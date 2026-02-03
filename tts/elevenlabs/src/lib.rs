use golem_tts::durability::{DurableTts, ExtendedGuest};
use golem_tts::guest::{TtsGuest, TtsRequest};
use golem_tts::exports::golem::tts::voices::{Voice, Guest as WitVoicesGuest, Error as TtsError};
use golem_tts::exports::golem::tts::synthesis::{Response, Guest as WitSynthesisGuest};
use golem_tts::http::WstdHttpClient;

use once_cell::sync::OnceCell;
use wstd::runtime::block_on;
use http::{Request, header};
use bytes::Bytes;

static CLIENT: OnceCell<ElevenLabsProvider> = OnceCell::new();

struct ElevenLabsComponent;

struct ElevenLabsProvider {
    api_key: String,
    client: WstdHttpClient,
}

impl ElevenLabsComponent {
    fn get_provider() -> Result<&'static ElevenLabsProvider, TtsError> {
        CLIENT.get_or_try_init(|| {
            let api_key = std::env::var("ELEVENLABS_API_KEY")
                .map_err(|_| TtsError::ServiceUnavailable("ELEVENLABS_API_KEY not set".to_string()))?;
            Ok(ElevenLabsProvider {
                api_key,
                client: WstdHttpClient::new(),
            })
        })
    }
}

impl WitVoicesGuest for ElevenLabsComponent {
    fn list_voices() -> Result<Vec<Voice>, TtsError> {
        block_on(async {
            let provider = Self::get_provider()?;
            let url = "https://api.elevenlabs.io/v1/voices";
            let req = Request::get(url)
                .header("xi-api-key", &provider.api_key)
                .body(Bytes::new())
                .map_err(|e| TtsError::ServiceUnavailable(e.to_string()))?;

            let resp = provider.client.execute(req).await
                .map_err(|e| TtsError::ServiceUnavailable(format!("{:?}", e)))?;

            let body: serde_json::Value = serde_json::from_slice(resp.body())
                .map_err(|e| TtsError::ServiceUnavailable(e.to_string()))?;

            let mut voices = Vec::new();
            if let Some(voices_arr) = body["voices"].as_array() {
                for v in voices_arr {
                    voices.push(Voice {
                        id: v["voice_id"].as_str().unwrap_or_default().to_string(),
                        name: v["name"].as_str().unwrap_or_default().to_string(),
                        provider: "elevenlabs".to_string(),
                        language: "en".to_string(),
                    });
                }
            }
            Ok(voices)
        })
    }
}

impl TtsGuest for ElevenLabsComponent {
    fn synthesize(req: TtsRequest) -> Result<Response, TtsError> {
        block_on(async {
            let provider = Self::get_provider()?;
            let url = format!("https://api.elevenlabs.io/v1/text-to-speech/{}/stream", req.voice_id);
            
            let body_json = serde_json::json!({
                "text": req.text,
                "model_id": "eleven_monolingual_v1",
            });

            let body_bytes = Bytes::from(serde_json::to_vec(&body_json).unwrap());

            let http_req = Request::post(url)
                .header("xi-api-key", &provider.api_key)
                .header(header::CONTENT_TYPE, "application/json")
                .body(body_bytes)
                .map_err(|e| TtsError::ServiceUnavailable(e.to_string()))?;

            let resp = provider.client.execute(http_req).await
                .map_err(|e| TtsError::ServiceUnavailable(format!("{:?}", e)))?;

            Ok(Response {
                audio: resp.body().clone(),
                content_type: "audio/mpeg".to_string(),
            })
        })
    }
}

impl ExtendedGuest for ElevenLabsComponent {}

type DurableElevenLabsComponent = DurableTts<ElevenLabsComponent>;

golem_tts::export_tts!(DurableElevenLabsComponent with_types_in golem_tts);
