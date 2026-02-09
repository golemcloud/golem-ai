use crate::async_utils::UnsafeFuture;
use crate::conversions::converse_output_to_complete_response;
use crate::conversions::{from_converse_sdk_error, from_converse_stream_sdk_error, BedrockInput};
use crate::stream::BedrockChatStream;
use crate::wasi_client::WasiClient;
use aws_config::BehaviorVersion;
use aws_sdk_bedrockruntime as bedrock;
use aws_sdk_bedrockruntime::config::{AsyncSleep, Sleep};
use aws_sdk_bedrockruntime::operation::converse::builders::ConverseFluentBuilder;
use aws_sdk_bedrockruntime::operation::converse_stream::builders::ConverseStreamFluentBuilder;
use aws_types::region;
use golem_llm::config::{get_config_key, get_config_key_or_none};
use golem_llm::model::{Config, Error, Event, Response};
use log::trace;
use wasi::clocks::monotonic_clock;
use wstd::runtime::Reactor;

#[derive(Debug)]
pub struct Bedrock {
    client: bedrock::Client,
}

impl Bedrock {
    pub async fn new() -> Result<Self, Error> {
        let environment = BedrockEnvironment::load_from_env()?;

        let sdk_config = aws_config::defaults(BehaviorVersion::latest())
            .region(environment.aws_region())
            .http_client(WasiClient::new())
            .credentials_provider(environment.aws_credentials())
            .sleep_impl(WasiSleep::new())
            .load()
            .await;
        let client = bedrock::Client::new(&sdk_config);
        Ok(Self { client })
    }

    pub async fn converse(&self, events: Vec<Event>, config: Config) -> Result<Response, Error> {
        let input = BedrockInput::from_events(config, events).await?;

        trace!("Sending request to AWS Bedrock: {input:?}");

        let model_id = input.model_id.clone();
        let response = self
            .init_converse(input)
            .send()
            .await
            .map_err(|e| from_converse_sdk_error(model_id, e))?;

        converse_output_to_complete_response(response)
    }

    pub async fn converse_stream(&self, events: Vec<Event>, config: Config) -> BedrockChatStream {
        let bedrock_input = BedrockInput::from_events(config, events).await;

        match bedrock_input {
            Err(err) => BedrockChatStream::failed(err),
            Ok(input) => {
                trace!("Sending request to AWS Bedrock: {input:?}");
                let model_id = input.model_id.clone();
                let response = self
                    .init_converse_stream(input)
                    .send()
                    .await
                    .map_err(|e| from_converse_stream_sdk_error(model_id, e));

                trace!("Creating AWS Bedrock event stream");
                match response {
                    Ok(response) => BedrockChatStream::new(response.stream),
                    Err(error) => BedrockChatStream::failed(error),
                }
            }
        }
    }

    fn init_converse(&self, input: BedrockInput) -> ConverseFluentBuilder {
        self.client
            .converse()
            .model_id(input.model_id)
            .set_system(Some(input.system_instructions))
            .set_messages(Some(input.messages))
            .inference_config(input.inference_configuration)
            .set_tool_config(input.tools)
            .additional_model_request_fields(input.additional_fields)
    }

    fn init_converse_stream(&self, input: BedrockInput) -> ConverseStreamFluentBuilder {
        self.client
            .converse_stream()
            .model_id(input.model_id)
            .set_system(Some(input.system_instructions))
            .set_messages(Some(input.messages))
            .inference_config(input.inference_configuration)
            .set_tool_config(input.tools)
            .additional_model_request_fields(input.additional_fields)
    }
}

#[derive(Debug)]
pub struct BedrockEnvironment {
    access_key_id: String,
    region: String,
    secret_access_key: String,
    session_token: Option<String>,
}

impl BedrockEnvironment {
    pub fn load_from_env() -> Result<Self, Error> {
        Ok(Self {
            access_key_id: get_config_key("AWS_ACCESS_KEY_ID")?,
            region: get_config_key("AWS_REGION")?,
            secret_access_key: get_config_key("AWS_SECRET_ACCESS_KEY")?,
            session_token: get_config_key_or_none("AWS_SESSION_TOKEN"),
        })
    }

    fn aws_region(&self) -> region::Region {
        region::Region::new(self.region.clone())
    }

    fn aws_credentials(&self) -> bedrock::config::Credentials {
        bedrock::config::Credentials::new(
            self.access_key_id.clone(),
            self.secret_access_key.clone(),
            self.session_token.clone(),
            None,
            "llm-bedrock",
        )
    }
}

#[derive(Debug, Clone)]
struct WasiSleep;

impl WasiSleep {
    fn new() -> Self {
        Self
    }
}

impl AsyncSleep for WasiSleep {
    fn sleep(&self, duration: std::time::Duration) -> Sleep {
        let reactor = Reactor::current();
        let nanos = duration.as_nanos() as u64;
        let pollable = reactor.schedule(monotonic_clock::subscribe_duration(nanos));

        let fut = pollable.wait_for();
        Sleep::new(Box::pin(UnsafeFuture::new(fut)))
    }
}
