use bytes::Bytes;
use golem_stt::{
    error::Error as SttError,
    http::{Error as HttpError, HttpClient, MultipartBuilder},
    languages::Language,
    transcription::SttProviderClient,
};
use http::{Method, Request, StatusCode};
use log::trace;
use serde::{Deserialize, Serialize};

const BASE_URL_TEMPLATE: &str = "https://{region}.api.cognitive.microsoft.com/speechtotext/transcriptions:transcribe?api-version=2024-11-15";

const AZURE_SUPPORTED_LANGUAGES: [Language; 16] = [
    Language::new("de-DE", "German (Germany)", "Deutsch (Deutschland)"),
    Language::new("en-AU", "English (Australia)", "English (Australia)"),
    Language::new("en-CA", "English (Canada)", "English (Canada)"),
    Language::new(
        "en-GB",
        "English (United Kingdom)",
        "English (United Kingdom)",
    ),
    Language::new("en-IN", "English (India)", "English (India)"),
    Language::new(
        "en-US",
        "English (United States)",
        "English (United States)",
    ),
    Language::new("es-ES", "Spanish (Spain)", "Español (España)"),
    Language::new("es-MX", "Spanish (Mexico)", "Español (México)"),
    Language::new("fr-CA", "French (Canada)", "Français (Canada)"),
    Language::new("fr-FR", "French (France)", "Français (France)"),
    Language::new("hi-IN", "Hindi (India)", "हिन्दी (भारत)"),
    Language::new("it-IT", "Italian (Italy)", "Italiano (Italia)"),
    Language::new("ja-JP", "Japanese (Japan)", "日本語 (日本)"),
    Language::new("ko-KR", "Korean (South Korea)", "한국어 (대한민국)"),
    Language::new("pt-BR", "Portuguese (Brazil)", "Português (Brasil)"),
    Language::new("zh-CN", "Chinese (Simplified)", "中文 (简体)"),
];

pub fn is_supported_language(language_code: &str) -> bool {
    AZURE_SUPPORTED_LANGUAGES
        .iter()
        .any(|lang| lang.code == language_code)
}

pub fn get_supported_languages() -> &'static [Language] {
    &AZURE_SUPPORTED_LANGUAGES
}

#[allow(unused)]
#[derive(Debug, Clone)]
pub enum AudioFormat {
    Wav,
    Mp3,
    Flac,
    Ogg,
    Wma,
    Aac,
    Webm,
    Speex,
}

impl std::fmt::Display for AudioFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AudioFormat::Wav => write!(f, "wav"),
            AudioFormat::Mp3 => write!(f, "mp3"),
            AudioFormat::Flac => write!(f, "flac"),
            AudioFormat::Ogg => write!(f, "ogg"),
            AudioFormat::Wma => write!(f, "wma"),
            AudioFormat::Aac => write!(f, "aac"),
            AudioFormat::Webm => write!(f, "webm"),
            AudioFormat::Speex => write!(f, "speex"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AudioConfig {
    pub format: AudioFormat,
    pub channels: Option<u8>,
}

#[derive(Debug, Clone)]
pub struct DiarizationConfig {
    pub enabled: bool,
    pub max_speakers: u8,
}

#[allow(unused)]
#[derive(Debug, Clone)]
pub enum ProfanityFilterMode {
    None,
    Masked,
    Removed,
    Tags,
}

impl std::fmt::Display for ProfanityFilterMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProfanityFilterMode::None => write!(f, "None"),
            ProfanityFilterMode::Masked => write!(f, "Masked"),
            ProfanityFilterMode::Removed => write!(f, "Removed"),
            ProfanityFilterMode::Tags => write!(f, "Tags"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TranscriptionConfig {
    pub locales: Vec<String>,
    pub diarization: Option<DiarizationConfig>,
    pub profanity_filter_mode: Option<ProfanityFilterMode>,
    pub enable_multi_channel: bool,
}

#[derive(Clone)]
pub struct TranscriptionRequest {
    pub request_id: String,
    pub audio: Bytes,
    pub audio_config: AudioConfig,
    pub transcription_config: Option<TranscriptionConfig>,
}

impl std::fmt::Debug for TranscriptionRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TranscriptionRequest")
            .field("request_id", &self.request_id)
            .field("audio_size", &self.audio.len())
            .field("audio_config", &self.audio_config)
            .field("transcription_config", &self.transcription_config)
            .finish()
    }
}

#[derive(Debug)]
pub struct FastTranscriptionApi<HC: HttpClient> {
    subscription_key: String,
    region: String,
    http_client: HC,
}

impl<HC: HttpClient> FastTranscriptionApi<HC> {
    pub fn new(subscription_key: String, region: String, http_client: HC) -> Self {
        Self {
            subscription_key,
            region,
            http_client,
        }
    }
}

// implementation based on https://learn.microsoft.com/en-us/azure/ai-services/speech-service/fast-transcription-create?tabs=locale-specified
impl<HC: HttpClient> SttProviderClient<TranscriptionRequest, TranscriptionResponse, SttError>
    for FastTranscriptionApi<HC>
{
    async fn transcribe_audio(
        &self,
        request: TranscriptionRequest,
    ) -> Result<TranscriptionResponse, SttError> {
        let url = BASE_URL_TEMPLATE.replace("{region}", &self.region);

        let request_id = request.request_id; // figure this out, there has to better way than cloning it all over the place

        trace!(
            "Making transcription request to Azure Fast Transcription API for request_id: {}",
            request_id.clone()
        );

        let file_name = format!("audio.{}", request.audio_config.format);
        let mime_type = get_mime_type(&request.audio_config.format);
        let req_locales = request
            .transcription_config
            .as_ref()
            .map(|config| config.locales.clone())
            .unwrap_or_default();

        let audio_size_bytes = request.audio.len();

        let definition_json = if let Some(config) = request.transcription_config {
            let channels = request
                .audio_config
                .channels
                .as_ref()
                .and_then(|&channels| {
                    if config.diarization.as_ref().is_none_or(|d| !d.enabled)
                        && config.enable_multi_channel
                        && channels == 2
                    {
                        Some(vec![0, 1])
                    } else {
                        None
                    }
                });

            let definition = AzureTranscriptionDefinition {
                locales: if !config.locales.is_empty() {
                    Some(config.locales)
                } else {
                    None
                },
                diarization: config.diarization.as_ref().map(|d| AzureDiarizationConfig {
                    enabled: d.enabled,
                    max_speakers: d.max_speakers as u32,
                }),
                channels,
                profanity_filter_mode: config.profanity_filter_mode.map(|pfm| pfm.to_string()),
            };

            serde_json::to_string(&definition).map_err(|e| {
                SttError::Http(
                    request_id.clone(),
                    HttpError::Generic(format!("Failed to serialize definition: {e}")),
                )
            })?
        } else {
            "{}".to_string()
        };

        let mut form = MultipartBuilder::new_with_capacity(request.audio.len() + 4096);
        form.add_bytes("audio", &file_name, &mime_type, &request.audio);
        form.add_field("definition", &definition_json);

        let (content_type, body) = form.finish();

        let http_request = Request::builder()
            .method(Method::POST)
            .uri(&url)
            .header("Content-Type", content_type)
            .header("Ocp-Apim-Subscription-Key", &self.subscription_key)
            .body(body)
            .map_err(|e| SttError::Http(request_id.clone(), HttpError::from(e)))?;

        let response = self
            .http_client
            .execute(http_request)
            .await
            .map_err(|e| SttError::Http(request_id.clone(), e))?;

        if response.status().is_success() {
            let azure_transcription: AzureTranscription = serde_json::from_slice(response.body())
                .map_err(|e| {
                SttError::Http(
                    request_id.clone(),
                    HttpError::Generic(format!("Failed to parse response: {e}")),
                )
            })?;

            Ok(TranscriptionResponse {
                request_id,
                audio_size_bytes,
                locales: req_locales,
                azure_transcription,
            })
        } else {
            let provider_error = String::from_utf8(response.body().to_vec()).map_err(|e| {
                SttError::Http(
                    request_id.clone(),
                    golem_stt::http::Error::Generic(format!(
                        "Failed to parse response as UTF-8: {e}"
                    )),
                )
            })?;

            match response.status() {
                StatusCode::BAD_REQUEST => Err(SttError::APIBadRequest {
                    request_id,
                    provider_error,
                }),
                StatusCode::UNAUTHORIZED => Err(SttError::APIUnauthorized {
                    request_id,
                    provider_error,
                }),
                StatusCode::FORBIDDEN => Err(SttError::APIForbidden {
                    request_id,
                    provider_error,
                }),
                StatusCode::NOT_FOUND => Err(SttError::APINotFound {
                    request_id,
                    provider_error,
                }),
                StatusCode::CONFLICT => Err(SttError::APIConflict {
                    request_id,
                    provider_error,
                }),
                StatusCode::UNPROCESSABLE_ENTITY => Err(SttError::APIUnprocessableEntity {
                    request_id,
                    provider_error,
                }),
                StatusCode::TOO_MANY_REQUESTS => Err(SttError::APIRateLimit {
                    request_id,
                    provider_error,
                }),
                status if status.is_server_error() => Err(SttError::APIInternalServerError {
                    request_id,
                    provider_error,
                }),
                _ => Err(SttError::APIUnknown {
                    request_id,
                    provider_error,
                }),
            }
        }
    }
}

#[derive(Debug)]
pub struct TranscriptionResponse {
    pub request_id: String,
    pub audio_size_bytes: usize,
    pub locales: Vec<String>,
    pub azure_transcription: AzureTranscription,
}

// https://learn.microsoft.com/en-us/rest/api/speechtotext/transcriptions/transcribe?view=rest-speechtotext-2024-11-15&tabs=HTTP#transcriberesult
#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AzureTranscription {
    pub combined_phrases: Vec<CombinedPhrase>,
    pub duration_milliseconds: u64,
    pub phrases: Vec<Phrase>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct CombinedPhrase {
    pub text: String,
    pub channel: Option<u32>,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Phrase {
    pub channel: Option<u32>,
    pub confidence: f64,
    pub duration_milliseconds: u64,
    pub locale: String,
    pub offset_milliseconds: u64,
    pub speaker: Option<u32>,
    pub text: String,
    pub words: Vec<Word>,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Word {
    pub duration_milliseconds: u64,
    pub offset_milliseconds: u64,
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct AzureTranscriptionDefinition {
    #[serde(skip_serializing_if = "Option::is_none")]
    locales: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    diarization: Option<AzureDiarizationConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    channels: Option<Vec<u32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    profanity_filter_mode: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct AzureDiarizationConfig {
    pub enabled: bool,
    pub max_speakers: u32,
}

fn get_mime_type(format: &AudioFormat) -> String {
    match format {
        AudioFormat::Wav => "audio/wav".to_string(),
        AudioFormat::Mp3 => "audio/mpeg".to_string(),
        AudioFormat::Flac => "audio/flac".to_string(),
        AudioFormat::Ogg => "audio/ogg".to_string(),
        AudioFormat::Wma => "audio/x-ms-wma".to_string(),
        AudioFormat::Aac => "audio/aac".to_string(),
        AudioFormat::Webm => "audio/webm".to_string(),
        AudioFormat::Speex => "audio/speex".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use golem_stt::http::{Error as HttpError, HttpClient};
    use http::{Request, Response, StatusCode};
    use std::cell::RefCell;
    use std::collections::VecDeque;

    use multipart_2021::server::Multipart;
    use std::collections::HashMap;
    use std::io::{Cursor, Read};

    const TEST_SUBSCRIPTION_KEY: &str = "test-subscription-key";
    const TEST_REGION: &str = "eastus";

    #[derive(Debug)]
    struct MockHttpClient {
        pub responses: RefCell<VecDeque<Result<Response<Vec<u8>>, HttpError>>>,
        pub captured_requests: RefCell<Vec<Request<Bytes>>>,
    }

    #[allow(unused)]
    impl MockHttpClient {
        pub fn new() -> Self {
            Self {
                responses: RefCell::new(VecDeque::new()),
                captured_requests: RefCell::new(Vec::new()),
            }
        }

        pub fn expect_response(&self, response: Result<Response<Vec<u8>>, HttpError>) {
            self.responses.borrow_mut().push_back(response);
        }

        pub fn get_captured_requests(&self) -> Vec<Request<Bytes>> {
            self.captured_requests.borrow().clone()
        }

        pub fn clear_captured_requests(&self) {
            self.captured_requests.borrow_mut().clear();
        }

        pub fn captured_request_count(&self) -> usize {
            self.captured_requests.borrow().len()
        }

        pub fn last_captured_request(&self) -> Option<Request<Bytes>> {
            self.captured_requests.borrow().last().cloned()
        }
    }

    impl HttpClient for MockHttpClient {
        async fn execute(&self, request: Request<Bytes>) -> Result<Response<Vec<u8>>, HttpError> {
            self.captured_requests.borrow_mut().push(request);
            self.responses.borrow_mut().pop_front().unwrap()
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    struct MultipartField {
        pub data: Vec<u8>,
        pub filename: Option<String>,
        pub content_type: Option<String>,
    }

    #[wstd::test]
    async fn test_subscription_key_gets_passed_as_header() {
        let mock_client = MockHttpClient::new();
        let api = FastTranscriptionApi::new(
            TEST_SUBSCRIPTION_KEY.to_string(),
            TEST_REGION.to_string(),
            mock_client,
        );

        let success_response = r#"
        {
            "durationMilliseconds": 1000,
            "combinedPhrases": [
                {
                    "text": "Hello world"
                }
            ],
            "phrases": [
                {
                    "offsetMilliseconds": 0,
                    "durationMilliseconds": 1000,
                    "text": "Hello world",
                    "words": [
                        {
                            "text": "Hello",
                            "offsetMilliseconds": 0,
                            "durationMilliseconds": 500
                        },
                        {
                            "text": "world",
                            "offsetMilliseconds": 500,
                            "durationMilliseconds": 500
                        }
                    ],
                    "locale": "en-US",
                    "confidence": 0.95
                }
            ]
        }
        "#;

        api.http_client.expect_response(Ok(Response::builder()
            .status(StatusCode::OK)
            .body(success_response.as_bytes().to_vec())
            .unwrap()));

        let request = TranscriptionRequest {
            request_id: "test-id".to_string(),
            audio: "test audio data".into(),
            audio_config: AudioConfig {
                format: AudioFormat::Wav,
                channels: Some(1),
            },
            transcription_config: Some(TranscriptionConfig {
                locales: vec!["en-US".to_string()],
                diarization: None,
                profanity_filter_mode: None,
                enable_multi_channel: false,
            }),
        };

        let result = api.transcribe_audio(request).await;
        assert!(result.is_ok());

        assert_eq!(api.http_client.captured_request_count(), 1);
        let captured_request = api.http_client.last_captured_request().unwrap();

        let auth_header = captured_request
            .headers()
            .get("Ocp-Apim-Subscription-Key")
            .unwrap()
            .to_str()
            .unwrap();
        assert_eq!(auth_header, TEST_SUBSCRIPTION_KEY);
    }

    #[wstd::test]
    async fn test_diarization_config_is_passed_correctly() {
        let mock_client = MockHttpClient::new();
        let api = FastTranscriptionApi::new(
            TEST_SUBSCRIPTION_KEY.to_string(),
            TEST_REGION.to_string(),
            mock_client,
        );

        let success_response = r#"
            {
                "durationMilliseconds": 1000,
                "combinedPhrases": [
                    {
                        "text": "Hello world"
                    }
                ],
                "phrases": [
                    {
                        "offsetMilliseconds": 0,
                        "durationMilliseconds": 1000,
                        "text": "Hello world",
                        "words": [
                            {
                                "text": "Hello",
                                "offsetMilliseconds": 0,
                                "durationMilliseconds": 500
                            }
                        ],
                        "locale": "en-US",
                        "confidence": 0.95
                    }
                ]
            }
            "#;

        api.http_client.expect_response(Ok(Response::builder()
            .status(StatusCode::OK)
            .body(success_response.as_bytes().to_vec())
            .unwrap()));

        let request = TranscriptionRequest {
            request_id: "test-id".to_string(),
            audio: "test audio data".into(),
            audio_config: AudioConfig {
                format: AudioFormat::Wav,
                channels: None,
            },
            transcription_config: Some(TranscriptionConfig {
                locales: vec!["en-US".to_string()],
                diarization: Some(DiarizationConfig {
                    enabled: true,
                    max_speakers: 3,
                }),
                profanity_filter_mode: None,
                enable_multi_channel: false,
            }),
        };

        let _result = api.transcribe_audio(request).await.unwrap();

        let captured_request = api.http_client.last_captured_request().unwrap();
        let content_type_header = captured_request
            .headers()
            .get("Content-Type")
            .unwrap()
            .to_str()
            .unwrap();
        let boundary = content_type_header.split("boundary=").nth(1).unwrap();
        let body_bytes = captured_request.body().to_vec();

        let cursor = Cursor::new(body_bytes);
        let mut multipart = Multipart::with_body(cursor, boundary);
        let mut fields: HashMap<String, MultipartField> = HashMap::new();

        while let Ok(Some(mut field)) = multipart.read_entry() {
            let field_name = field.headers.name.clone();
            let mut data = Vec::new();
            field.data.read_to_end(&mut data).unwrap();
            let multipart_field = MultipartField {
                data,
                filename: field.headers.filename.clone(),
                content_type: field.headers.content_type.as_ref().map(|ct| ct.to_string()),
            };
            fields.insert(field_name.to_string(), multipart_field);
        }

        let definition_field = fields.get("definition").unwrap();
        let definition_json = String::from_utf8(definition_field.data.clone()).unwrap();
        let actual_definition: AzureTranscriptionDefinition =
            serde_json::from_str(&definition_json).unwrap();

        let expected_definition = AzureTranscriptionDefinition {
            locales: Some(vec!["en-US".to_string()]),
            diarization: Some(AzureDiarizationConfig {
                enabled: true,
                max_speakers: 3,
            }),
            channels: None,
            profanity_filter_mode: None,
        };

        assert_eq!(actual_definition, expected_definition);
    }

    #[wstd::test]
    async fn test_channels_config_is_passed_correctly() {
        let mock_client = MockHttpClient::new();
        let api = FastTranscriptionApi::new(
            TEST_SUBSCRIPTION_KEY.to_string(),
            TEST_REGION.to_string(),
            mock_client,
        );

        let multichannel_response = r#"
           {
               "durationMilliseconds": 9920,
               "combinedPhrases": [
                   {
                       "channel": 0,
                       "text": "Hello. Thank you for calling Contoso."
                   },
                   {
                       "channel": 1,
                       "text": "Hi, my name is Mary Rondo."
                   }
               ],
               "phrases": [
                   {
                       "channel": 0,
                       "offsetMilliseconds": 720,
                       "durationMilliseconds": 480,
                       "text": "Hello.",
                       "words": [
                           {
                               "text": "Hello.",
                               "offsetMilliseconds": 720,
                               "durationMilliseconds": 480
                           }
                       ],
                       "locale": "en-US",
                       "confidence": 0.9177142
                   },
                   {
                       "channel": 1,
                       "offsetMilliseconds": 4480,
                       "durationMilliseconds": 1600,
                       "text": "Hi, my name is Mary Rondo.",
                       "words": [
                           {
                               "text": "Hi,",
                               "offsetMilliseconds": 4480,
                               "durationMilliseconds": 400
                           },
                           {
                               "text": "my",
                               "offsetMilliseconds": 4880,
                               "durationMilliseconds": 120
                           },
                           {
                               "text": "name",
                               "offsetMilliseconds": 5000,
                               "durationMilliseconds": 120
                           },
                           {
                               "text": "is",
                               "offsetMilliseconds": 5120,
                               "durationMilliseconds": 160
                           },
                           {
                               "text": "Mary",
                               "offsetMilliseconds": 5280,
                               "durationMilliseconds": 240
                           },
                           {
                               "text": "Rondo.",
                               "offsetMilliseconds": 5520,
                               "durationMilliseconds": 560
                           }
                       ],
                       "locale": "en-US",
                       "confidence": 0.8989456
                   }
               ]
           }
           "#;

        api.http_client.expect_response(Ok(Response::builder()
            .status(StatusCode::OK)
            .body(multichannel_response.as_bytes().to_vec())
            .unwrap()));

        let request = TranscriptionRequest {
            request_id: "test-id".to_string(),
            audio: "test audio data".into(),
            audio_config: AudioConfig {
                format: AudioFormat::Wav,
                channels: Some(2),
            },
            transcription_config: Some(TranscriptionConfig {
                locales: vec!["en-US".to_string()],
                diarization: None,
                profanity_filter_mode: None,
                enable_multi_channel: true,
            }),
        };

        let _result = api.transcribe_audio(request).await.unwrap();

        let captured_request = api.http_client.last_captured_request().unwrap();
        let content_type_header = captured_request
            .headers()
            .get("Content-Type")
            .unwrap()
            .to_str()
            .unwrap();
        let boundary = content_type_header.split("boundary=").nth(1).unwrap();
        let body_bytes = captured_request.body().to_vec();

        let cursor = Cursor::new(body_bytes);
        let mut multipart = Multipart::with_body(cursor, boundary);
        let mut fields: HashMap<String, MultipartField> = HashMap::new();

        while let Ok(Some(mut field)) = multipart.read_entry() {
            let field_name = field.headers.name.clone();
            let mut data = Vec::new();
            field.data.read_to_end(&mut data).unwrap();
            let multipart_field = MultipartField {
                data,
                filename: field.headers.filename.clone(),
                content_type: field.headers.content_type.as_ref().map(|ct| ct.to_string()),
            };
            fields.insert(field_name.to_string(), multipart_field);
        }

        let definition_field = fields.get("definition").unwrap();
        let definition_json = String::from_utf8(definition_field.data.clone()).unwrap();
        let actual_definition: AzureTranscriptionDefinition =
            serde_json::from_str(&definition_json).unwrap();

        let expected_definition = AzureTranscriptionDefinition {
            locales: Some(vec!["en-US".to_string()]),
            diarization: None,
            channels: Some(vec![0, 1]),
            profanity_filter_mode: None,
        };

        assert_eq!(actual_definition, expected_definition);
    }

    #[wstd::test]
    async fn test_profanity_filter_mode_is_passed_correctly() {
        let mock_client = MockHttpClient::new();
        let api = FastTranscriptionApi::new(
            TEST_SUBSCRIPTION_KEY.to_string(),
            TEST_REGION.to_string(),
            mock_client,
        );

        let success_response = r#"
           {
               "durationMilliseconds": 1000,
               "combinedPhrases": [
                   {
                       "text": "Hello world"
                   }
               ],
               "phrases": [
                   {
                       "offsetMilliseconds": 0,
                       "durationMilliseconds": 1000,
                       "text": "Hello world",
                       "words": [
                           {
                               "text": "Hello",
                               "offsetMilliseconds": 0,
                               "durationMilliseconds": 500
                           }
                       ],
                       "locale": "en-US",
                       "confidence": 0.95
                   }
               ]
           }
           "#;

        api.http_client.expect_response(Ok(Response::builder()
            .status(StatusCode::OK)
            .body(success_response.as_bytes().to_vec())
            .unwrap()));

        let request = TranscriptionRequest {
            request_id: "test-id".to_string(),
            audio: "test audio data".into(),
            audio_config: AudioConfig {
                format: AudioFormat::Wav,
                channels: None,
            },
            transcription_config: Some(TranscriptionConfig {
                locales: vec!["en-US".to_string()],
                diarization: None,
                profanity_filter_mode: Some(ProfanityFilterMode::Masked),
                enable_multi_channel: false,
            }),
        };

        let _result = api.transcribe_audio(request).await.unwrap();

        let captured_request = api.http_client.last_captured_request().unwrap();
        let content_type_header = captured_request
            .headers()
            .get("Content-Type")
            .unwrap()
            .to_str()
            .unwrap();
        let boundary = content_type_header.split("boundary=").nth(1).unwrap();
        let body_bytes = captured_request.body().to_vec();

        let cursor = Cursor::new(body_bytes);
        let mut multipart = Multipart::with_body(cursor, boundary);
        let mut fields: HashMap<String, MultipartField> = HashMap::new();

        while let Ok(Some(mut field)) = multipart.read_entry() {
            let field_name = field.headers.name.clone();
            let mut data = Vec::new();
            field.data.read_to_end(&mut data).unwrap();
            let multipart_field = MultipartField {
                data,
                filename: field.headers.filename.clone(),
                content_type: field.headers.content_type.as_ref().map(|ct| ct.to_string()),
            };
            fields.insert(field_name.to_string(), multipart_field);
        }

        let definition_field = fields.get("definition").unwrap();
        let definition_json = String::from_utf8(definition_field.data.clone()).unwrap();
        let actual_definition: AzureTranscriptionDefinition =
            serde_json::from_str(&definition_json).unwrap();

        let expected_definition = AzureTranscriptionDefinition {
            locales: Some(vec!["en-US".to_string()]),
            diarization: None,
            channels: None,
            profanity_filter_mode: Some("Masked".to_string()),
        };

        assert_eq!(actual_definition, expected_definition);
    }

    #[wstd::test]
    async fn test_audio_bytes_are_handled_correctly() {
        let mock_client = MockHttpClient::new();
        let api = FastTranscriptionApi::new(
            TEST_SUBSCRIPTION_KEY.to_string(),
            TEST_REGION.to_string(),
            mock_client,
        );

        let success_response = r#"
            {
                "durationMilliseconds": 1000,
                "combinedPhrases": [
                    {
                        "text": "Hello world"
                    }
                ],
                "phrases": [
                    {
                        "offsetMilliseconds": 0,
                        "durationMilliseconds": 1000,
                        "text": "Hello world",
                        "words": [
                            {
                                "text": "Hello",
                                "offsetMilliseconds": 0,
                                "durationMilliseconds": 500
                            }
                        ],
                        "locale": "en-US",
                        "confidence": 0.95
                    }
                ]
            }
            "#;

        api.http_client.expect_response(Ok(Response::builder()
            .status(StatusCode::OK)
            .body(success_response.as_bytes().to_vec())
            .unwrap()));

        let audio_bytes = Bytes::from("test audio data");
        let request = TranscriptionRequest {
            request_id: "test-id".to_string(),
            audio: audio_bytes.clone(),
            audio_config: AudioConfig {
                format: AudioFormat::Flac,
                channels: None,
            },
            transcription_config: None,
        };

        let _result = api.transcribe_audio(request).await.unwrap();

        let captured_request = api.http_client.last_captured_request().unwrap();
        let content_type_header = captured_request
            .headers()
            .get("Content-Type")
            .unwrap()
            .to_str()
            .unwrap();

        assert!(
            content_type_header.starts_with("multipart/form-data"),
            "should be multipart/form-data"
        );

        let boundary = content_type_header.split("boundary=").nth(1).unwrap();
        let body_bytes = captured_request.body().to_vec();

        let cursor = Cursor::new(body_bytes);
        let mut multipart = Multipart::with_body(cursor, boundary);
        let mut fields: HashMap<String, MultipartField> = HashMap::new();

        while let Ok(Some(mut field)) = multipart.read_entry() {
            let field_name = field.headers.name.clone();
            let mut data = Vec::new();
            field.data.read_to_end(&mut data).unwrap();
            let multipart_field = MultipartField {
                data,
                filename: field.headers.filename.clone(),
                content_type: field.headers.content_type.as_ref().map(|ct| ct.to_string()),
            };
            fields.insert(field_name.to_string(), multipart_field);
        }

        let audio_field = fields.get("audio").unwrap();
        assert_eq!(
            audio_field,
            &MultipartField {
                data: audio_bytes.to_vec(),
                filename: Some("audio.flac".to_string()),
                content_type: Some("audio/flac".to_string()),
            }
        );

        let definition_field = fields.get("definition").unwrap();
        let definition_json = String::from_utf8(definition_field.data.clone()).unwrap();
        let actual_definition: AzureTranscriptionDefinition =
            serde_json::from_str(&definition_json).unwrap();

        let expected_definition = AzureTranscriptionDefinition {
            locales: None, // Should be None when empty vec is provided
            diarization: None,
            channels: None,
            profanity_filter_mode: None,
        };

        assert_eq!(actual_definition, expected_definition);
    }

    #[wstd::test]
    async fn test_transcribe_audio_success() {
        let mock_client = MockHttpClient::new();
        let api = FastTranscriptionApi::new(
            TEST_SUBSCRIPTION_KEY.to_string(),
            TEST_REGION.to_string(),
            mock_client,
        );

        let success_response = r#"
        {
            "durationMilliseconds": 9920,
            "combinedPhrases": [
                {
                    "channel": 0,
                    "text": "Hello. Thank you for calling Contoso. Who am I speaking with today? Hi, Mary."
                },
                {
                    "channel": 1,
                    "text": "Hi, my name is Mary Rondo. I'm trying to enroll myself with Contuso."
                }
            ],
            "phrases": [
                {
                    "channel": 0,
                    "offsetMilliseconds": 720,
                    "durationMilliseconds": 480,
                    "text": "Hello.",
                    "words": [
                        {
                            "text": "Hello.",
                            "offsetMilliseconds": 720,
                            "durationMilliseconds": 480
                        }
                    ],
                    "locale": "en-US",
                    "confidence": 0.9177142
                },
                {
                    "channel": 0,
                    "offsetMilliseconds": 1200,
                    "durationMilliseconds": 1120,
                    "text": "Thank you for calling Contoso.",
                    "words": [
                        {
                            "text": "Thank",
                            "offsetMilliseconds": 1200,
                            "durationMilliseconds": 200
                        },
                        {
                            "text": "you",
                            "offsetMilliseconds": 1400,
                            "durationMilliseconds": 80
                        },
                        {
                            "text": "for",
                            "offsetMilliseconds": 1480,
                            "durationMilliseconds": 120
                        },
                        {
                            "text": "calling",
                            "offsetMilliseconds": 1600,
                            "durationMilliseconds": 240
                        },
                        {
                            "text": "Contoso.",
                            "offsetMilliseconds": 1840,
                            "durationMilliseconds": 480
                        }
                    ],
                    "locale": "en-US",
                    "confidence": 0.9177142
                },
                {
                    "channel": 0,
                    "offsetMilliseconds": 2320,
                    "durationMilliseconds": 1120,
                    "text": "Who am I speaking with today?",
                    "words": [
                        {
                            "text": "Who",
                            "offsetMilliseconds": 2320,
                            "durationMilliseconds": 160
                        },
                        {
                            "text": "am",
                            "offsetMilliseconds": 2480,
                            "durationMilliseconds": 80
                        },
                        {
                            "text": "I",
                            "offsetMilliseconds": 2560,
                            "durationMilliseconds": 80
                        },
                        {
                            "text": "speaking",
                            "offsetMilliseconds": 2640,
                            "durationMilliseconds": 320
                        },
                        {
                            "text": "with",
                            "offsetMilliseconds": 2960,
                            "durationMilliseconds": 160
                        },
                        {
                            "text": "today?",
                            "offsetMilliseconds": 3120,
                            "durationMilliseconds": 320
                        }
                    ],
                    "locale": "en-US",
                    "confidence": 0.9177142
                },
                {
                    "channel": 0,
                    "offsetMilliseconds": 9520,
                    "durationMilliseconds": 400,
                    "text": "Hi, Mary.",
                    "words": [
                        {
                            "text": "Hi,",
                            "offsetMilliseconds": 9520,
                            "durationMilliseconds": 80
                        },
                        {
                            "text": "Mary.",
                            "offsetMilliseconds": 9600,
                            "durationMilliseconds": 320
                        }
                    ],
                    "locale": "en-US",
                    "confidence": 0.9177142
                },
                {
                    "channel": 1,
                    "offsetMilliseconds": 4480,
                    "durationMilliseconds": 1600,
                    "text": "Hi, my name is Mary Rondo.",
                    "words": [
                        {
                            "text": "Hi,",
                            "offsetMilliseconds": 4480,
                            "durationMilliseconds": 400
                        },
                        {
                            "text": "my",
                            "offsetMilliseconds": 4880,
                            "durationMilliseconds": 120
                        },
                        {
                            "text": "name",
                            "offsetMilliseconds": 5000,
                            "durationMilliseconds": 120
                        },
                        {
                            "text": "is",
                            "offsetMilliseconds": 5120,
                            "durationMilliseconds": 160
                        },
                        {
                            "text": "Mary",
                            "offsetMilliseconds": 5280,
                            "durationMilliseconds": 240
                        },
                        {
                            "text": "Rondo.",
                            "offsetMilliseconds": 5520,
                            "durationMilliseconds": 560
                        }
                    ],
                    "locale": "en-US",
                    "confidence": 0.8989456
                },
                {
                    "channel": 1,
                    "offsetMilliseconds": 6080,
                    "durationMilliseconds": 1920,
                    "text": "I'm trying to enroll myself with Contuso.",
                    "words": [
                        {
                            "text": "I'm",
                            "offsetMilliseconds": 6080,
                            "durationMilliseconds": 160
                        },
                        {
                            "text": "trying",
                            "offsetMilliseconds": 6240,
                            "durationMilliseconds": 200
                        },
                        {
                            "text": "to",
                            "offsetMilliseconds": 6440,
                            "durationMilliseconds": 80
                        },
                        {
                            "text": "enroll",
                            "offsetMilliseconds": 6520,
                            "durationMilliseconds": 200
                        },
                        {
                            "text": "myself",
                            "offsetMilliseconds": 6720,
                            "durationMilliseconds": 360
                        },
                        {
                            "text": "with",
                            "offsetMilliseconds": 7080,
                            "durationMilliseconds": 120
                        },
                        {
                            "text": "Contuso.",
                            "offsetMilliseconds": 7200,
                            "durationMilliseconds": 800
                        }
                    ],
                    "locale": "en-US",
                    "confidence": 0.8989456
                }
            ]
        }
        "#;

        api.http_client.expect_response(Ok(Response::builder()
            .status(StatusCode::OK)
            .body(success_response.as_bytes().to_vec())
            .unwrap()));

        let audio_bytes = Bytes::from("test audio data");
        let request = TranscriptionRequest {
            request_id: "test-id".to_string(),
            audio: audio_bytes.clone(),
            audio_config: AudioConfig {
                format: AudioFormat::Wav,
                channels: None,
            },
            transcription_config: Some(TranscriptionConfig {
                locales: vec!["en-US".to_string()],
                diarization: Some(DiarizationConfig {
                    enabled: true,
                    max_speakers: 2,
                }),
                profanity_filter_mode: None,
                enable_multi_channel: false,
            }),
        };

        let result = api.transcribe_audio(request).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.audio_size_bytes, audio_bytes.len());

        let expected_response = AzureTranscription {
            duration_milliseconds: 9920,
            combined_phrases: vec![
                CombinedPhrase {
                    channel: Some(0),
                    text: "Hello. Thank you for calling Contoso. Who am I speaking with today? Hi, Mary.".to_string(),
                },
                CombinedPhrase {
                    channel: Some(1),
                    text: "Hi, my name is Mary Rondo. I'm trying to enroll myself with Contuso.".to_string(),
                },
            ],
            phrases: vec![
                Phrase {
                    channel: Some(0),
                    offset_milliseconds: 720,
                    duration_milliseconds: 480,
                    text: "Hello.".to_string(),
                    words: vec![
                        Word {
                            text: "Hello.".to_string(),
                            offset_milliseconds: 720,
                            duration_milliseconds: 480,
                        }
                    ],
                    locale: "en-US".to_string(),
                    confidence: 0.9177142,
                    speaker: None,
                },
                Phrase {
                    channel: Some(0),
                    offset_milliseconds: 1200,
                    duration_milliseconds: 1120,
                    text: "Thank you for calling Contoso.".to_string(),
                    words: vec![
                        Word { text: "Thank".to_string(), offset_milliseconds: 1200, duration_milliseconds: 200 },
                        Word { text: "you".to_string(), offset_milliseconds: 1400, duration_milliseconds: 80 },
                        Word { text: "for".to_string(), offset_milliseconds: 1480, duration_milliseconds: 120 },
                        Word { text: "calling".to_string(), offset_milliseconds: 1600, duration_milliseconds: 240 },
                        Word { text: "Contoso.".to_string(), offset_milliseconds: 1840, duration_milliseconds: 480 },
                    ],
                    locale: "en-US".to_string(),
                    confidence: 0.9177142,
                    speaker: None,
                },
                Phrase {
                    channel: Some(0),
                    offset_milliseconds: 2320,
                    duration_milliseconds: 1120,
                    text: "Who am I speaking with today?".to_string(),
                    words: vec![
                        Word { text: "Who".to_string(), offset_milliseconds: 2320, duration_milliseconds: 160 },
                        Word { text: "am".to_string(), offset_milliseconds: 2480, duration_milliseconds: 80 },
                        Word { text: "I".to_string(), offset_milliseconds: 2560, duration_milliseconds: 80 },
                        Word { text: "speaking".to_string(), offset_milliseconds: 2640, duration_milliseconds: 320 },
                        Word { text: "with".to_string(), offset_milliseconds: 2960, duration_milliseconds: 160 },
                        Word { text: "today?".to_string(), offset_milliseconds: 3120, duration_milliseconds: 320 },
                    ],
                    locale: "en-US".to_string(),
                    confidence: 0.9177142,
                    speaker: None,
                },
                Phrase {
                    channel: Some(0),
                    offset_milliseconds: 9520,
                    duration_milliseconds: 400,
                    text: "Hi, Mary.".to_string(),
                    words: vec![
                        Word { text: "Hi,".to_string(), offset_milliseconds: 9520, duration_milliseconds: 80 },
                        Word { text: "Mary.".to_string(), offset_milliseconds: 9600, duration_milliseconds: 320 },
                    ],
                    locale: "en-US".to_string(),
                    confidence: 0.9177142,
                    speaker: None,
                },
                Phrase {
                    channel: Some(1),
                    offset_milliseconds: 4480,
                    duration_milliseconds: 1600,
                    text: "Hi, my name is Mary Rondo.".to_string(),
                    words: vec![
                        Word { text: "Hi,".to_string(), offset_milliseconds: 4480, duration_milliseconds: 400 },
                        Word { text: "my".to_string(), offset_milliseconds: 4880, duration_milliseconds: 120 },
                        Word { text: "name".to_string(), offset_milliseconds: 5000, duration_milliseconds: 120 },
                        Word { text: "is".to_string(), offset_milliseconds: 5120, duration_milliseconds: 160 },
                        Word { text: "Mary".to_string(), offset_milliseconds: 5280, duration_milliseconds: 240 },
                        Word { text: "Rondo.".to_string(), offset_milliseconds: 5520, duration_milliseconds: 560 },
                    ],
                    locale: "en-US".to_string(),
                    confidence: 0.8989456,
                    speaker: None,
                },
                Phrase {
                    channel: Some(1),
                    offset_milliseconds: 6080,
                    duration_milliseconds: 1920,
                    text: "I'm trying to enroll myself with Contuso.".to_string(),
                    words: vec![
                        Word { text: "I'm".to_string(), offset_milliseconds: 6080, duration_milliseconds: 160 },
                        Word { text: "trying".to_string(), offset_milliseconds: 6240, duration_milliseconds: 200 },
                        Word { text: "to".to_string(), offset_milliseconds: 6440, duration_milliseconds: 80 },
                        Word { text: "enroll".to_string(), offset_milliseconds: 6520, duration_milliseconds: 200 },
                        Word { text: "myself".to_string(), offset_milliseconds: 6720, duration_milliseconds: 360 },
                        Word { text: "with".to_string(), offset_milliseconds: 7080, duration_milliseconds: 120 },
                        Word { text: "Contuso.".to_string(), offset_milliseconds: 7200, duration_milliseconds: 800 },
                    ],
                    locale: "en-US".to_string(),
                    confidence: 0.8989456,
                    speaker: None,
                },
            ],
        };

        assert_eq!(response.azure_transcription, expected_response);
    }
    #[wstd::test]
    async fn test_transcribe_audio_error_bad_request() {
        let mock_client = MockHttpClient::new();
        let api = FastTranscriptionApi::new(
            TEST_SUBSCRIPTION_KEY.to_string(),
            TEST_REGION.to_string(),
            mock_client,
        );

        let error_body = r#"
           {
               "code": "InvalidRequest",
               "message": "The request is invalid",
               "innerError": {
                   "code": "InvalidAudioFormat",
                   "message": "The audio format is not supported"
               }
           }
           "#;

        api.http_client.expect_response(Ok(Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(error_body.as_bytes().to_vec())
            .unwrap()));

        let request = TranscriptionRequest {
            request_id: "some-transcription-id".to_string(),
            audio: "test audio data".into(),
            audio_config: AudioConfig {
                format: AudioFormat::Wav,
                channels: None,
            },
            transcription_config: Some(TranscriptionConfig {
                locales: vec!["en-US".to_string()],
                diarization: None,
                profanity_filter_mode: None,
                enable_multi_channel: false,
            }),
        };

        let result = api.transcribe_audio(request).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            SttError::APIBadRequest {
                request_id,
                provider_error,
            } => {
                assert_eq!(request_id, "some-transcription-id");
                assert_eq!(provider_error, error_body);
            }
            _ => panic!("Expected APIBadRequest error"),
        }
    }

    #[wstd::test]
    async fn test_transcribe_audio_error_unauthorized() {
        let mock_client = MockHttpClient::new();
        let api = FastTranscriptionApi::new(
            TEST_SUBSCRIPTION_KEY.to_string(),
            TEST_REGION.to_string(),
            mock_client,
        );

        let error_body = r#"
        {
            "code": "Unauthorized",
            "message": "Access denied due to invalid subscription key",
            "innerError": {
                "code": "InvalidSubscription",
                "message": "The provided subscription key is not valid"
            }
        }
        "#;

        api.http_client.expect_response(Ok(Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(error_body.as_bytes().to_vec())
            .unwrap()));

        let request = TranscriptionRequest {
            request_id: "some-transcription-id".to_string(),
            audio: "test audio data".into(),
            audio_config: AudioConfig {
                format: AudioFormat::Wav,
                channels: None,
            },
            transcription_config: Some(TranscriptionConfig {
                locales: vec!["en-US".to_string()],
                diarization: None,
                profanity_filter_mode: None,
                enable_multi_channel: false,
            }),
        };

        let result = api.transcribe_audio(request).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            SttError::APIUnauthorized {
                request_id,
                provider_error,
            } => {
                assert_eq!(request_id, "some-transcription-id");
                assert_eq!(provider_error, error_body);
            }
            _ => panic!("Expected APIUnauthorized error"),
        }
    }

    #[wstd::test]
    async fn test_transcribe_audio_error_forbidden() {
        let mock_client = MockHttpClient::new();
        let api = FastTranscriptionApi::new(
            TEST_SUBSCRIPTION_KEY.to_string(),
            TEST_REGION.to_string(),
            mock_client,
        );

        let error_body = r#"
        {
            "code": "Forbidden",
            "message": "The subscription does not have access to this feature",
            "innerError": {
                "code": "Forbidden",
                "message": "Access to fast transcription is not enabled for this subscription"
            }
        }
        "#;

        api.http_client.expect_response(Ok(Response::builder()
            .status(StatusCode::FORBIDDEN)
            .body(error_body.as_bytes().to_vec())
            .unwrap()));

        let request = TranscriptionRequest {
            request_id: "some-transcription-id".to_string(),
            audio: "test audio data".into(),
            audio_config: AudioConfig {
                format: AudioFormat::Wav,
                channels: None,
            },
            transcription_config: Some(TranscriptionConfig {
                locales: vec!["en-US".to_string()],
                diarization: None,
                profanity_filter_mode: None,
                enable_multi_channel: false,
            }),
        };

        let result = api.transcribe_audio(request).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            SttError::APIForbidden {
                request_id,
                provider_error,
            } => {
                assert_eq!(request_id, "some-transcription-id");
                assert_eq!(provider_error, error_body);
            }
            _ => panic!("Expected APIForbidden error"),
        }
    }

    #[wstd::test]
    async fn test_transcribe_audio_error_not_found() {
        let mock_client = MockHttpClient::new();
        let api = FastTranscriptionApi::new(
            TEST_SUBSCRIPTION_KEY.to_string(),
            TEST_REGION.to_string(),
            mock_client,
        );

        let error_body = r#"
        {
            "code": "NotFound",
            "message": "The specified resource was not found",
            "innerError": {
                "code": "NotFound",
                "message": "The transcription endpoint was not found"
            }
        }
        "#;

        api.http_client.expect_response(Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(error_body.as_bytes().to_vec())
            .unwrap()));

        let request = TranscriptionRequest {
            request_id: "some-transcription-id".to_string(),
            audio: "test audio data".into(),
            audio_config: AudioConfig {
                format: AudioFormat::Wav,
                channels: None,
            },
            transcription_config: Some(TranscriptionConfig {
                locales: vec!["en-US".to_string()],
                diarization: None,
                profanity_filter_mode: None,
                enable_multi_channel: false,
            }),
        };

        let result = api.transcribe_audio(request).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            SttError::APINotFound {
                request_id,
                provider_error,
            } => {
                assert_eq!(request_id, "some-transcription-id");
                assert_eq!(provider_error, error_body);
            }
            _ => panic!("Expected APINotFound error"),
        }
    }

    #[wstd::test]
    async fn test_transcribe_audio_error_unprocessable_entity() {
        let mock_client = MockHttpClient::new();
        let api = FastTranscriptionApi::new(
            TEST_SUBSCRIPTION_KEY.to_string(),
            TEST_REGION.to_string(),
            mock_client,
        );

        let error_body = r#"
        {
            "code": "UnprocessableEntity",
            "message": "The audio file is corrupted or in an unsupported format",
            "innerError": {
                "code": "InvalidAudioFormat",
                "message": "The audio encoding is not supported for transcription"
            }
        }
        "#;

        api.http_client.expect_response(Ok(Response::builder()
            .status(StatusCode::UNPROCESSABLE_ENTITY)
            .body(error_body.as_bytes().to_vec())
            .unwrap()));

        let request = TranscriptionRequest {
            request_id: "some-transcription-id".to_string(),
            audio: "test audio data".into(),
            audio_config: AudioConfig {
                format: AudioFormat::Wav,
                channels: None,
            },
            transcription_config: Some(TranscriptionConfig {
                locales: vec!["en-US".to_string()],
                diarization: None,
                profanity_filter_mode: None,
                enable_multi_channel: false,
            }),
        };

        let result = api.transcribe_audio(request).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            SttError::APIUnprocessableEntity {
                request_id,
                provider_error,
            } => {
                assert_eq!(request_id, "some-transcription-id");
                assert_eq!(provider_error, error_body);
            }
            _ => panic!("Expected APIUnprocessableEntity error"),
        }
    }

    #[wstd::test]
    async fn test_transcribe_audio_error_rate_limit() {
        let mock_client = MockHttpClient::new();
        let api = FastTranscriptionApi::new(
            TEST_SUBSCRIPTION_KEY.to_string(),
            TEST_REGION.to_string(),
            mock_client,
        );

        let error_body = r#"
        {
            "code": "TooManyRequests",
            "message": "Rate limit exceeded. Please retry after some time",
            "innerError": {
                "code": "TooManyRequests",
                "message": "The request rate limit has been exceeded for this subscription"
            }
        }
        "#;

        api.http_client.expect_response(Ok(Response::builder()
            .status(StatusCode::TOO_MANY_REQUESTS)
            .body(error_body.as_bytes().to_vec())
            .unwrap()));

        let request = TranscriptionRequest {
            request_id: "some-transcription-id".to_string(),
            audio: "test audio data".into(),
            audio_config: AudioConfig {
                format: AudioFormat::Wav,
                channels: None,
            },
            transcription_config: Some(TranscriptionConfig {
                locales: vec!["en-US".to_string()],
                diarization: None,
                profanity_filter_mode: None,
                enable_multi_channel: false,
            }),
        };

        let result = api.transcribe_audio(request).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            SttError::APIRateLimit {
                request_id,
                provider_error,
            } => {
                assert_eq!(request_id, "some-transcription-id");
                assert_eq!(provider_error, error_body);
            }
            _ => panic!("Expected APIRateLimit error"),
        }
    }

    #[wstd::test]
    async fn test_transcribe_audio_error_internal_server_error() {
        let mock_client = MockHttpClient::new();
        let api = FastTranscriptionApi::new(
            TEST_SUBSCRIPTION_KEY.to_string(),
            TEST_REGION.to_string(),
            mock_client,
        );

        let error_body = r#"
        {
            "code": "InternalServerError",
            "message": "An internal server error occurred",
            "innerError": {
                "code": "UnexpectedError",
                "message": "An unexpected error occurred during processing"
            }
        }
        "#;

        api.http_client.expect_response(Ok(Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(error_body.as_bytes().to_vec())
            .unwrap()));

        let request = TranscriptionRequest {
            request_id: "some-transcription-id".to_string(),
            audio: "test audio data".into(),
            audio_config: AudioConfig {
                format: AudioFormat::Wav,
                channels: None,
            },
            transcription_config: Some(TranscriptionConfig {
                locales: vec!["en-US".to_string()],
                diarization: None,
                profanity_filter_mode: None,
                enable_multi_channel: false,
            }),
        };

        let result = api.transcribe_audio(request).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            SttError::APIInternalServerError {
                request_id,
                provider_error,
            } => {
                assert_eq!(request_id, "some-transcription-id");
                assert_eq!(provider_error, error_body);
            }
            _ => panic!("Expected APIInternalServerError error"),
        }
    }

    #[wstd::test]
    async fn test_transcribe_audio_error_unknown_status() {
        let mock_client = MockHttpClient::new();
        let api = FastTranscriptionApi::new(
            TEST_SUBSCRIPTION_KEY.to_string(),
            TEST_REGION.to_string(),
            mock_client,
        );

        let error_body = "I'm a teapot";

        api.http_client.expect_response(Ok(Response::builder()
            .status(StatusCode::IM_A_TEAPOT)
            .body(error_body.as_bytes().to_vec())
            .unwrap()));

        let request = TranscriptionRequest {
            request_id: "some-transcription-id".to_string(),
            audio: "test audio data".into(),
            audio_config: AudioConfig {
                format: AudioFormat::Wav,
                channels: None,
            },
            transcription_config: Some(TranscriptionConfig {
                locales: vec!["en-US".to_string()],
                diarization: None,
                profanity_filter_mode: None,
                enable_multi_channel: false,
            }),
        };

        let result = api.transcribe_audio(request).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            SttError::APIUnknown {
                request_id,
                provider_error,
            } => {
                assert_eq!(request_id, "some-transcription-id");
                assert_eq!(provider_error, error_body);
            }
            _ => panic!("Expected APIUnknown error"),
        }
    }
}
