use crate::async_utils::UnsafeFuture;
use crate::config::BedrockConfig;
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
use golem_ai_llm::model::{Config, Error, Event, Response};
use log::trace;
use wasi::clocks::monotonic_clock;
use wstd::runtime::Reactor;

#[derive(Debug)]
pub struct Bedrock {
    client: bedrock::Client,
}

impl Bedrock {
    /// Builds a new Bedrock SDK client from the supplied
    /// [`BedrockConfig`].
    ///
    /// The credential [`SecretSource`](golem_ai_llm::config::SecretSource)s
    /// are resolved here, immediately before the AWS SDK client is
    /// constructed. The returned `bedrock::Client` then captures the
    /// resolved credentials internally for its own lifetime.
    ///
    /// This is fine because every top-level provider call
    /// (`LlmProvider::send` / `LlmProvider::stream`) constructs a fresh
    /// [`Bedrock`] instance, so each top-level call re-resolves the
    /// secrets via `SecretSource::get()`. This satisfies the
    /// per-request hot-rotation contract.
    pub async fn new(config: &BedrockConfig) -> Result<Self, Error> {
        let access_key_id = config.access_key_id.get();
        let secret_access_key = config.secret_access_key.get();
        let session_token = config.session_token.as_ref().map(|s| s.get());
        let region_str = config.region.clone();

        let sdk_config = aws_config::defaults(BehaviorVersion::latest())
            .region(region::Region::new(region_str))
            .http_client(WasiClient::new())
            .credentials_provider(bedrock::config::Credentials::new(
                access_key_id,
                secret_access_key,
                session_token,
                None,
                "llm-bedrock",
            ))
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
