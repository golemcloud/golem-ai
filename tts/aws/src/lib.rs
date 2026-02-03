mod aws_signer;

use golem_tts::durability::{DurableTts, ExtendedGuest};
use golem_tts::guest::{TtsGuest, TtsRequest};
use golem_tts::exports::golem::tts::voices::{Voice, Guest as WitVoicesGuest, Error as TtsError};
use golem_tts::exports::golem::tts::synthesis::Response;
use golem_tts::http::WstdHttpClient;

use once_cell::sync::OnceCell;
use wstd::runtime::block_on;
use http::Request;
use bytes::Bytes;
use crate::aws_signer::{AwsSignatureV4, AwsService};

static CLIENT: OnceCell<AwsPollyProvider> = OnceCell::new();

struct AwsPollyComponent;

struct AwsPollyProvider {
    access_key: String,
    secret_key: String,
    region: String,
    client: WstdHttpClient,
}

impl AwsPollyComponent {
    fn get_provider() -> Result<&'static AwsPollyProvider, TtsError> {
        CLIENT.get_or_try_init(|| {
            let access_key = std::env::var("AWS_ACCESS_KEY")
                .map_err(|_| TtsError::ServiceUnavailable("AWS_ACCESS_KEY not set".to_string()))?;
            let secret_key = std::env::var("AWS_SECRET_KEY")
                .map_err(|_| TtsError::ServiceUnavailable("AWS_SECRET_KEY not set".to_string()))?;
            let region = std::env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".to_string());
            
            Ok(AwsPollyProvider {
                access_key,
                secret_key,
                region,
                client: WstdHttpClient::new(),
            })
        })
    }
}

impl WitVoicesGuest for AwsPollyComponent {
    fn list_voices() -> Result<Vec<Voice>, TtsError> {
        // Mocking for now as Polly voice list is often static or requires complex XML parsing
        Ok(vec![
            Voice { id: "Amy".to_string(), name: "Amy".to_string(), provider: "aws".to_string(), language: "en-GB".to_string() },
            Voice { id: "Joey".to_string(), name: "Joey".to_string(), provider: "aws".to_string(), language: "en-US".to_string() },
        ])
    }
}

impl TtsGuest for AwsPollyComponent {
    fn synthesize(req: TtsRequest) -> Result<Response, TtsError> {
        block_on(async {
            let provider = Self::get_provider()?;
            let url = format!("https://polly.{}.amazonaws.com/v1/speech", provider.region);
            
            let body_json = serde_json::json!({
                "OutputFormat": "mp3",
                "Text": req.text,
                "VoiceId": req.voice_id,
            });

            let body_bytes = Bytes::from(serde_json::to_vec(&body_json).unwrap());

            let mut http_req = Request::post(&url)
                .body(body_bytes.clone())
                .map_err(|e| TtsError::ServiceUnavailable(e.to_string()))?;

            let signer = AwsSignatureV4::new(
                provider.access_key.clone(),
                provider.secret_key.clone(),
                provider.region.clone(),
                AwsService::Polly
            );

            signer.sign_request("POST", &url, http_req.headers_mut(), &body_bytes)
                .map_err(|e| TtsError::ServiceUnavailable(e))?;

            let resp = provider.client.execute(http_req).await
                .map_err(|e| TtsError::ServiceUnavailable(format!("{:?}", e)))?;

            Ok(Response {
                audio: resp.body().clone(),
                content_type: "audio/mpeg".to_string(),
            })
        })
    }
}

impl ExtendedGuest for AwsPollyComponent {}

type DurableAwsPollyComponent = DurableTts<AwsPollyComponent>;

golem_tts::export_tts!(DurableAwsPollyComponent with_types_in golem_tts);
