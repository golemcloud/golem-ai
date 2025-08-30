use std::collections::HashMap;

use bytes::Bytes;
use golem_stt::{http::HttpClient, languages::Language, transcription::SttProviderClient};
use http::{header::CONTENT_TYPE, Method, Request, StatusCode};
use log::trace;
use serde::{Deserialize, Serialize};
use url::Url;

use golem_stt::error::Error;

const DEEPGRAM_SUPPORTED_LANGUAGES: [Language; 56] = [
    Language::new("multi", "Multilingual", "Multi"),
    Language::new("bg", "Bulgarian", "български"),
    Language::new("ca", "Catalan", "català"),
    Language::new("zh", "Chinese (Simplified)", "中文（简体）"),
    Language::new("zh-CN", "Chinese (China)", "中文（中国）"),
    Language::new("zh-Hans", "Chinese (Simplified Han)", "中文（简体字）"),
    Language::new("zh-TW", "Chinese (Taiwan)", "中文（台灣）"),
    Language::new("zh-Hant", "Chinese (Traditional Han)", "中文（繁體字）"),
    Language::new("zh-HK", "Chinese (Hong Kong)", "中文（香港）"),
    Language::new("cs", "Czech", "čeština"),
    Language::new("da", "Danish", "dansk"),
    Language::new("da-DK", "Danish (Denmark)", "dansk (Danmark)"),
    Language::new("nl", "Dutch", "Nederlands"),
    Language::new("nl-BE", "Flemish", "Vlaams"),
    Language::new("en", "English", "English"),
    Language::new(
        "en-US",
        "English (United States)",
        "English (United States)",
    ),
    Language::new("en-AU", "English (Australia)", "English (Australia)"),
    Language::new(
        "en-GB",
        "English (United Kingdom)",
        "English (United Kingdom)",
    ),
    Language::new("en-NZ", "English (New Zealand)", "English (New Zealand)"),
    Language::new("en-IN", "English (India)", "English (India)"),
    Language::new("et", "Estonian", "eesti"),
    Language::new("fi", "Finnish", "suomi"),
    Language::new("nl-BE", "Flemish", "Vlaams"),
    Language::new("fr", "French", "français"),
    Language::new("fr-CA", "French (Canada)", "français (Canada)"),
    Language::new("de", "German", "Deutsch"),
    Language::new("de-CH", "German (Switzerland)", "Deutsch (Schweiz)"),
    Language::new("el", "Greek", "ελληνικά"),
    Language::new("hi", "Hindi", "हिन्दी"),
    Language::new("hi-Latn", "Hindi (Roman Script)", "Hindi"),
    Language::new("hu", "Hungarian", "magyar"),
    Language::new("id", "Indonesian", "Bahasa Indonesia"),
    Language::new("it", "Italian", "italiano"),
    Language::new("ja", "Japanese", "日本語"),
    Language::new("ko", "Korean", "한국어"),
    Language::new("ko-KR", "Korean (South Korea)", "한국어 (대한민국)"),
    Language::new("lv", "Latvian", "latviešu"),
    Language::new("lt", "Lithuanian", "lietuvių"),
    Language::new("ms", "Malay", "Bahasa Melayu"),
    Language::new("no", "Norwegian", "norsk"),
    Language::new("pl", "Polish", "polski"),
    Language::new("pt", "Portuguese", "português"),
    Language::new("pt-BR", "Portuguese (Brazil)", "português (Brasil)"),
    Language::new("pt-PT", "Portuguese (Portugal)", "português (Portugal)"),
    Language::new("ro", "Romanian", "română"),
    Language::new("ru", "Russian", "русский"),
    Language::new("sk", "Slovak", "slovenčina"),
    Language::new("es", "Spanish", "español"),
    Language::new(
        "es-419",
        "Spanish (Latin America)",
        "español (Latinoamérica)",
    ),
    Language::new("sv", "Swedish", "svenska"),
    Language::new("sv-SE", "Swedish (Sweden)", "svenska (Sverige)"),
    Language::new("th", "Thai", "ไทย"),
    Language::new("th-TH", "Thai (Thailand)", "ไทย (ประเทศไทย)"),
    Language::new("tr", "Turkish", "Türkçe"),
    Language::new("uk", "Ukrainian", "українська"),
    Language::new("vi", "Vietnamese", "Tiếng Việt"),
];

pub fn is_supported_language(language_code: &str) -> bool {
    DEEPGRAM_SUPPORTED_LANGUAGES
        .iter()
        .any(|lang| lang.code == language_code)
}

pub fn get_supported_languages() -> &'static [Language] {
    &DEEPGRAM_SUPPORTED_LANGUAGES
}

#[allow(unused)]
#[derive(Debug, Clone)]
pub enum AudioFormat {
    Wav,
    Mp3,
    Flac,
    Ogg,
    Aac,
    Pcm,
}

impl core::fmt::Display for AudioFormat {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string_representation = match self {
            AudioFormat::Wav => "wav",
            AudioFormat::Mp3 => "mp3",
            AudioFormat::Flac => "flac",
            AudioFormat::Ogg => "ogg",
            AudioFormat::Aac => "aac",
            AudioFormat::Pcm => "pcm",
        };
        write!(fmt, "{string_representation}")
    }
}

#[derive(Debug, Clone)]
pub struct AudioConfig {
    pub format: AudioFormat,
    pub channels: Option<u8>,
}

#[derive(Debug, Clone)]
pub struct Keyword {
    pub value: String,
    pub boost: Option<f32>,
}

#[derive(Debug, Clone)]
pub struct TranscriptionConfig {
    pub language: Option<String>,
    pub model: Option<String>,
    pub enable_profanity_filter: bool,
    pub enable_speaker_diarization: bool,
    pub enable_multi_channel: bool,
    pub keywords: Vec<Keyword>,
    pub keyterms: Vec<String>, // only nova-3
}

pub struct TranscriptionRequest {
    pub request_id: String,
    pub audio: Bytes,
    pub audio_config: AudioConfig,
    pub transcription_config: Option<TranscriptionConfig>,
}

impl std::fmt::Debug for TranscriptionRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TranscriptionRequest")
            .field("audio_size", &self.audio.len())
            .field("audio_config", &self.audio_config)
            .field("transcription_config", &self.transcription_config)
            .finish()
    }
}

// The Deepgram Speech-to-Text API client for transcribing audio into the input language
///
/// https://developers.deepgram.com/reference/speech-to-text-api/listen
pub struct PreRecordedAudioApi<HC: HttpClient> {
    endpoint: String,
    deepgram_api_token: String,
    http_client: HC,
}

#[allow(unused)]
impl<HC: HttpClient> PreRecordedAudioApi<HC> {
    pub fn new(deepgram_api_key: String, endpoint: String, http_client: HC) -> Self {
        Self {
            endpoint,
            deepgram_api_token: format!("Token {deepgram_api_key}"),
            http_client,
        }
    }
}

impl<HC: HttpClient> SttProviderClient<TranscriptionRequest, TranscriptionResponse, Error>
    for PreRecordedAudioApi<HC>
{
    async fn transcribe_audio(
        &self,
        request: TranscriptionRequest,
    ) -> Result<TranscriptionResponse, Error> {
        trace!("Sending request to Deepgram API: {request:?}");

        let request_id = request.request_id;

        let mime_type = format!("audio/{}", request.audio_config.format);

        let audio_size_bytes = request.audio.len();
        let req_language = request
            .transcription_config
            .as_ref()
            .and_then(|config| config.language.clone());

        let mut query_params: Vec<(&str, String)> = vec![];

        query_params.push(("utterances", "true".to_string()));
        query_params.push(("punctuate", "true".to_string()));

        if let Some(channels) = request.audio_config.channels {
            if channels > 1
                && request
                    .transcription_config
                    .as_ref()
                    .is_some_and(|t| t.enable_multi_channel)
            {
                query_params.push(("multichannel", "true".to_string()));
            }
        }

        if let Some(transcription_config) = request.transcription_config {
            if let Some(language) = transcription_config.language {
                query_params.push(("language", language));
            }

            if transcription_config.enable_profanity_filter {
                query_params.push(("profanity_filter", "true".to_string()));
            }

            if transcription_config.enable_speaker_diarization {
                query_params.push(("diarize", "true".to_string()));
            }

            if transcription_config
                .model
                .as_ref()
                .is_some_and(|m| *m == "nova-3")
            {
                transcription_config.keyterms.iter().for_each(|keyterm| {
                    let encoded = keyterm.replace(" ", "+");
                    query_params.push(("keyterm", encoded));
                });
            }

            //Nova-2, Nova-1, Enhanced, and Base

            if transcription_config.model.as_ref().is_some_and(|m| {
                *m == "nova-2" || *m == "nova-1" || *m == "enhanced" || *m == "base"
            }) {
                transcription_config.keywords.iter().for_each(|keyword| {
                    let encoded = keyword.value.replace(" ", "+");
                    if let Some(boost) = keyword.boost {
                        query_params.push(("keyword", format!("{encoded}:{boost}")));
                    } else {
                        query_params.push(("keyword", encoded));
                    }
                });
            }

            if let Some(model) = transcription_config.model {
                query_params.push(("model", model));
            }
        }

        let mut url = Url::parse(&self.endpoint).map_err(|e| {
            Error::Http(
                request_id.clone(),
                golem_stt::http::Error::Generic(format!("Failed to parse uri: {e}")),
            )
        })?;

        for (key, value) in query_params {
            url.query_pairs_mut().append_pair(key, &value);
        }

        let req = Request::builder()
            .method(Method::POST)
            .uri(url.as_str())
            .header(CONTENT_TYPE, mime_type)
            .header("Authorization", &self.deepgram_api_token)
            .body(request.audio)
            .map_err(|e| Error::Http(request_id.clone(), golem_stt::http::Error::HttpError(e)))?;

        let response = self
            .http_client
            .execute(req)
            .await
            .map_err(|e| Error::Http(request_id.clone(), e))?;

        if response.status().is_success() {
            let deepgram_transcription: DeepgramTranscription =
                serde_json::from_slice(response.body()).map_err(|e| {
                    Error::Http(
                        request_id.clone(),
                        golem_stt::http::Error::Generic(format!(
                            "Failed to deserialize response: {e}"
                        )),
                    )
                })?;

            Ok(TranscriptionResponse {
                request_id,
                audio_size_bytes,
                language: req_language.unwrap_or_default(),
                deepgram_transcription,
            })
        } else {
            let provider_error = String::from_utf8(response.body().to_vec()).map_err(|e| {
                Error::Http(
                    request_id.clone(),
                    golem_stt::http::Error::Generic(format!(
                        "Failed to parse response as UTF-8: {e}",
                    )),
                )
            })?;

            match response.status() {
                StatusCode::BAD_REQUEST => Err(Error::APIBadRequest {
                    request_id,
                    provider_error,
                }),
                StatusCode::UNAUTHORIZED => Err(Error::APIUnauthorized {
                    request_id,
                    provider_error,
                }),
                StatusCode::PAYMENT_REQUIRED => Err(Error::APIAccessDenied {
                    request_id,
                    provider_error,
                }),
                StatusCode::FORBIDDEN => Err(Error::APIForbidden {
                    request_id,
                    provider_error,
                }),
                status if status.is_server_error() => Err(Error::APIInternalServerError {
                    request_id,
                    provider_error,
                }),
                _ => Err(Error::APIUnknown {
                    request_id,
                    provider_error,
                }),
            }
        }
    }
}

#[allow(unused)]
#[derive(Debug, PartialEq)]
pub struct TranscriptionResponse {
    pub request_id: String,
    pub audio_size_bytes: usize,
    pub language: String,
    pub deepgram_transcription: DeepgramTranscription,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct DeepgramTranscription {
    pub metadata: Metadata,
    pub results: Results,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Metadata {
    pub transaction_key: String,
    pub request_id: String,
    pub sha256: String,
    pub created: String,
    pub duration: f32,
    pub channels: u8,
    pub models: Vec<String>,
    pub model_info: HashMap<String, ModelInfo>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct ModelInfo {
    pub name: String,
    pub version: String,
    pub arch: String,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Results {
    pub channels: Vec<Channel>,
    pub utterances: Vec<Utterance>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Channel {
    pub alternatives: Vec<Alternative>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Alternative {
    pub transcript: String,
    pub confidence: f32,
    pub words: Vec<Word>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Word {
    pub word: String,
    pub start: f32,
    pub end: f32,
    pub confidence: f32,
    pub speaker: Option<u8>,
    pub speaker_confidence: Option<f32>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Utterance {
    pub start: f32,
    pub end: f32,
    pub confidence: f32,
    pub channel: u8,
    pub transcript: String,
    pub words: Vec<Word>,
    pub speaker: Option<u8>,
    pub id: String,
}

#[cfg(test)]
mod tests {

    use http::Response;

    use super::*;
    use std::cell::{Ref, RefCell};
    use std::collections::VecDeque;

    const TEST_API_KEY: &str = "test-deepgram-api-key";

    const TEST_ENDPOINT: &str = "https://example.com/v1/listen";

    struct MockHttpClient {
        pub responses: RefCell<VecDeque<Result<Response<Vec<u8>>, golem_stt::http::Error>>>,
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

        pub fn expect_response(&self, response: Response<Vec<u8>>) {
            self.responses.borrow_mut().push_back(Ok(response));
        }

        pub fn get_captured_requests(&self) -> Ref<'_, Vec<Request<Bytes>>> {
            self.captured_requests.borrow()
        }

        pub fn clear_captured_requests(&self) {
            self.captured_requests.borrow_mut().clear();
        }

        pub fn captured_request_count(&self) -> usize {
            self.captured_requests.borrow().len()
        }

        pub fn last_captured_request(&self) -> Option<Ref<'_, Request<Bytes>>> {
            let borrow = self.captured_requests.borrow();
            if borrow.is_empty() {
                None
            } else {
                Some(Ref::map(borrow, |requests| requests.last().unwrap()))
            }
        }
    }

    impl HttpClient for MockHttpClient {
        async fn execute(
            &self,
            request: Request<Bytes>,
        ) -> Result<Response<Vec<u8>>, golem_stt::http::Error> {
            self.captured_requests.borrow_mut().push(request);
            self.responses
                .borrow_mut()
                .pop_front()
                .unwrap_or(Err(golem_stt::http::Error::Generic(
                    "unexpected error".to_string(),
                )))
        }
    }

    fn create_mock_success_response() -> Response<Vec<u8>> {
        let response_body = r#"{
                "metadata": {
                    "transaction_key": "test-transaction-key",
                    "request_id": "test-request-id",
                    "sha256": "test-sha256",
                    "created": "2023-01-01T00:00:00Z",
                    "duration": 10.5,
                    "channels": 1,
                    "models": ["nova-2"],
                    "model_info": {
                        "nova-2": {
                            "name": "nova-2",
                            "version": "1.0.0",
                            "arch": "transformer"
                        }
                    }
                },
                "results": {
                    "channels": [{
                        "alternatives": [{
                            "transcript": "Hello world",
                            "confidence": 0.95,
                            "words": [{
                                "word": "Hello",
                                "start": 0.0,
                                "end": 0.5,
                                "confidence": 0.95,
                                "speaker": 0,
                                "speaker_confidence": 0.9
                            }, {
                                "word": "world",
                                "start": 0.6,
                                "end": 1.0,
                                "confidence": 0.95,
                                "speaker": 0,
                                "speaker_confidence": 0.9
                            }]
                        }]
                    }],
                    "utterances": [{
                        "start": 0.0,
                        "end": 1.0,
                        "confidence": 0.95,
                        "channel": 0,
                        "transcript": "Hello world",
                        "words": [{
                            "word": "Hello",
                            "start": 0.0,
                            "end": 0.5,
                            "confidence": 0.95,
                            "speaker": 0,
                            "speaker_confidence": 0.9
                        }, {
                            "word": "world",
                            "start": 0.6,
                            "end": 1.0,
                            "confidence": 0.95,
                            "speaker": 0,
                            "speaker_confidence": 0.9
                        }],
                        "speaker": 0,
                        "id": "test-utterance-id"
                    }]
                }
            }"#;

        Response::builder()
            .status(StatusCode::OK)
            .body(response_body.as_bytes().to_vec())
            .unwrap()
    }

    #[wstd::test]
    async fn test_api_key_gets_passed_as_auth_header() {
        let mock_client = MockHttpClient::new();

        mock_client.expect_response(create_mock_success_response());

        let api = PreRecordedAudioApi::new(
            TEST_API_KEY.to_string(),
            TEST_ENDPOINT.to_string(),
            mock_client,
        );

        let request = TranscriptionRequest {
            request_id: "some-transcription-id".to_string(),
            audio: "fake audio data".into(),
            audio_config: AudioConfig {
                format: AudioFormat::Wav,
                channels: None,
            },
            transcription_config: None,
        };

        api.transcribe_audio(request).await.unwrap();

        let captured_request = api.http_client.last_captured_request().unwrap();
        let auth_header = captured_request
            .headers()
            .get("Authorization")
            .and_then(|h| h.to_str().ok());

        assert_eq!(auth_header, Some("Token test-deepgram-api-key"));
    }

    #[wstd::test]
    async fn test_request_gets_sent_with_correct_content_type_and_body() {
        let mock_client = MockHttpClient::new();

        mock_client.expect_response(create_mock_success_response());

        let api = PreRecordedAudioApi::new(
            TEST_API_KEY.to_string(),
            TEST_ENDPOINT.to_string(),
            mock_client,
        );

        let audio_data = Bytes::from("fake audio data");
        let request = TranscriptionRequest {
            request_id: "some-transcription-id".to_string(),
            audio: audio_data.clone(),
            audio_config: AudioConfig {
                format: AudioFormat::Mp3,
                channels: Some(2),
            },
            transcription_config: None,
        };

        api.transcribe_audio(request).await.unwrap();

        let captured_request = api.http_client.last_captured_request().unwrap();

        let content_type = captured_request
            .headers()
            .get("content-type")
            .and_then(|h| h.to_str().ok());
        assert_eq!(content_type, Some("audio/mp3"));

        assert_eq!(captured_request.method(), &Method::POST);
        assert!(captured_request
            .uri()
            .to_string()
            .starts_with(&api.endpoint));

        let body_bytes = captured_request.body().to_vec();

        assert_eq!(body_bytes, audio_data)
    }

    #[wstd::test]
    async fn test_query_parameters_other_than_keywords_and_keyterms_set_correctly() {
        let mock_client = MockHttpClient::new();

        mock_client.expect_response(create_mock_success_response());

        let api = PreRecordedAudioApi::new(
            TEST_API_KEY.to_string(),
            TEST_ENDPOINT.to_string(),
            mock_client,
        );

        let request = TranscriptionRequest {
            request_id: "some-transcription-id".to_string(),
            audio: "fake audio data".into(),
            audio_config: AudioConfig {
                format: AudioFormat::Wav,
                channels: Some(2), // Should add multichannel=true
            },
            transcription_config: Some(TranscriptionConfig {
                language: Some("en".to_string()),
                model: Some("nova-2".to_string()),
                enable_profanity_filter: true,
                enable_speaker_diarization: true,
                enable_multi_channel: true,
                keywords: vec![],
                keyterms: vec![],
            }),
        };

        api.transcribe_audio(request).await.unwrap();

        let captured_request = api.http_client.last_captured_request().unwrap();
        let uri = captured_request.uri();
        let query_pairs: HashMap<String, String> = Url::parse(&uri.to_string())
            .unwrap()
            .query_pairs()
            .into_owned()
            .collect();

        assert_eq!(query_pairs.get("utterances"), Some(&"true".to_string()));
        assert_eq!(query_pairs.get("punctuate"), Some(&"true".to_string()));
        assert_eq!(query_pairs.get("multichannel"), Some(&"true".to_string()));
        assert_eq!(query_pairs.get("language"), Some(&"en".to_string()));
        assert_eq!(query_pairs.get("model"), Some(&"nova-2".to_string()));
        assert_eq!(
            query_pairs.get("profanity_filter"),
            Some(&"true".to_string())
        );
        assert_eq!(query_pairs.get("diarize"), Some(&"true".to_string()));
    }

    #[wstd::test]
    async fn test_query_keyterms_params_set_correctly_in_case_of_nova3_model() {
        let mock_client = MockHttpClient::new();

        mock_client.expect_response(create_mock_success_response());

        let api = PreRecordedAudioApi::new(
            TEST_API_KEY.to_string(),
            TEST_ENDPOINT.to_string(),
            mock_client,
        );

        let request = TranscriptionRequest {
            request_id: "some-transcription-id".to_string(),
            audio: "fake audio data".into(),
            audio_config: AudioConfig {
                format: AudioFormat::Wav,
                channels: Some(2),
            },
            transcription_config: Some(TranscriptionConfig {
                language: Some("en".to_string()),
                model: Some("nova-3".to_string()),
                enable_profanity_filter: true,
                enable_speaker_diarization: true,
                enable_multi_channel: false,
                keywords: vec![],
                keyterms: vec!["foo".to_string(), "bar".to_string(), "baz baz".to_string()],
            }),
        };

        api.transcribe_audio(request).await.unwrap();

        let captured_request = api.http_client.last_captured_request().unwrap();
        let uri = captured_request.uri();
        let keyterm_query_pairs: Vec<(String, String)> = Url::parse(&uri.to_string())
            .unwrap()
            .query_pairs()
            .into_owned()
            .filter(|(key, _)| key == "keyterm")
            .collect();

        assert_eq!(
            keyterm_query_pairs,
            vec![
                ("keyterm".to_string(), "foo".to_string()),
                ("keyterm".to_string(), "bar".to_string()),
                ("keyterm".to_string(), "baz+baz".to_string()),
            ]
        )
    }

    #[wstd::test]
    async fn test_query_keyterms_params_set_correctly_in_case_of_nova2_nova1_enhanced_and_base_model(
    ) {
        let mock_client = MockHttpClient::new();

        mock_client.expect_response(create_mock_success_response());

        let api = PreRecordedAudioApi::new(
            TEST_API_KEY.to_string(),
            TEST_ENDPOINT.to_string(),
            mock_client,
        );

        let request = TranscriptionRequest {
            request_id: "some-transcription-id".to_string(),
            audio: "fake audio data".into(),
            audio_config: AudioConfig {
                format: AudioFormat::Wav,
                channels: Some(2),
            },
            transcription_config: Some(TranscriptionConfig {
                language: Some("en".to_string()),
                model: Some("nova-2".to_string()),
                enable_profanity_filter: true,
                enable_speaker_diarization: true,
                enable_multi_channel: true,
                keywords: vec![
                    Keyword {
                        value: "foo".to_string(),
                        boost: None,
                    },
                    Keyword {
                        value: "bar".to_string(),
                        boost: Some(1.0),
                    },
                    Keyword {
                        value: "baz baz".to_string(),
                        boost: Some(2.5),
                    },
                ],
                keyterms: vec![],
            }),
        };

        api.transcribe_audio(request).await.unwrap();

        let captured_request = api.http_client.last_captured_request().unwrap();
        let uri = captured_request.uri();
        let keyterm_query_pairs: Vec<(String, String)> = Url::parse(&uri.to_string())
            .unwrap()
            .query_pairs()
            .into_owned()
            .filter(|(key, _)| key == "keyword")
            .collect();

        assert_eq!(
            keyterm_query_pairs,
            vec![
                ("keyword".to_string(), "foo".to_string()),
                ("keyword".to_string(), "bar:1".to_string()),
                ("keyword".to_string(), "baz+baz:2.5".to_string()),
            ]
        )
    }

    #[wstd::test]
    async fn test_query_parameters_not_set_when_disabled() {
        let mock_client = MockHttpClient::new();

        mock_client.expect_response(create_mock_success_response());

        let api = PreRecordedAudioApi::new(
            TEST_API_KEY.to_string(),
            TEST_ENDPOINT.to_string(),
            mock_client,
        );

        let request = TranscriptionRequest {
            request_id: "some-transcription-id".to_string(),
            audio: "fake audio data".into(),
            audio_config: AudioConfig {
                format: AudioFormat::Wav,
                channels: Some(2),
            },
            transcription_config: Some(TranscriptionConfig {
                language: None,
                model: None,
                enable_profanity_filter: false,
                enable_speaker_diarization: false,
                enable_multi_channel: false,
                keywords: vec![],
                keyterms: vec![],
            }),
        };

        api.transcribe_audio(request).await.unwrap();

        let captured_request = api.http_client.last_captured_request().unwrap();
        let uri = captured_request.uri();
        let query_pairs: HashMap<String, String> = Url::parse(&uri.to_string())
            .unwrap()
            .query_pairs()
            .into_owned()
            .collect();

        assert!(!query_pairs.contains_key("multichannel"));
        assert!(!query_pairs.contains_key("language"));
        assert!(!query_pairs.contains_key("model"));
        assert!(!query_pairs.contains_key("profanity_filter"));
        assert!(!query_pairs.contains_key("diarize"));
        assert!(!query_pairs.contains_key("keyterm"));
    }

    #[wstd::test]
    async fn test_transcribe_audio_without_diarization_success() {
        let mock_client = MockHttpClient::new();

        let response_body = r#"{
            "metadata": {
                "transaction_key": "test-transaction-key",
                "request_id": "test-request-id",
                "sha256": "test-sha256",
                "created": "2023-01-01T00:00:00Z",
                "duration": 10.5,
                "channels": 1,
                "models": ["nova-2"],
                "model_info": {
                    "nova-2": {
                        "name": "nova-2",
                        "version": "1.0.0",
                        "arch": "transformer"
                    }
                }
            },
            "results": {
                "channels": [{
                    "alternatives": [{
                        "transcript": "Hello world",
                        "confidence": 0.95,
                        "words": [{
                            "word": "Hello",
                            "start": 0.0,
                            "end": 0.5,
                            "confidence": 0.95
                        }, {
                            "word": "world",
                            "start": 0.6,
                            "end": 1.0,
                            "confidence": 0.95
                        }]
                    }]
                }],
                "utterances": [{
                    "start": 0.0,
                    "end": 1.0,
                    "confidence": 0.95,
                    "channel": 0,
                    "transcript": "Hello world",
                    "words": [{
                        "word": "Hello",
                        "start": 0.0,
                        "end": 0.5,
                        "confidence": 0.95
                    }, {
                        "word": "world",
                        "start": 0.6,
                        "end": 1.0,
                        "confidence": 0.95
                    }],
                    "id": "test-utterance-1"
                }]
            }
        }"#;

        mock_client.expect_response(
            Response::builder()
                .status(StatusCode::OK)
                .body(response_body.as_bytes().to_vec())
                .unwrap(),
        );

        let api = PreRecordedAudioApi::new(
            TEST_API_KEY.to_string(),
            TEST_ENDPOINT.to_string(),
            mock_client,
        );

        let audio_data = Bytes::from("fake audio data");
        let request = TranscriptionRequest {
            request_id: "some-transcription-id".to_string(),
            audio: audio_data.clone(),
            audio_config: AudioConfig {
                format: AudioFormat::Wav,
                channels: None,
            },
            transcription_config: None,
        };

        let response = api.transcribe_audio(request).await.unwrap();

        let expected_response = TranscriptionResponse {
            request_id: "some-transcription-id".to_string(),
            language: String::new(),
            audio_size_bytes: audio_data.len(),
            deepgram_transcription: DeepgramTranscription {
                metadata: Metadata {
                    transaction_key: "test-transaction-key".to_string(),
                    request_id: "test-request-id".to_string(),
                    sha256: "test-sha256".to_string(),
                    created: "2023-01-01T00:00:00Z".to_string(),
                    duration: 10.5,
                    channels: 1,
                    models: vec!["nova-2".to_string()],
                    model_info: HashMap::from([(
                        "nova-2".to_string(),
                        ModelInfo {
                            name: "nova-2".to_string(),
                            version: "1.0.0".to_string(),
                            arch: "transformer".to_string(),
                        },
                    )]),
                },
                results: Results {
                    channels: vec![Channel {
                        alternatives: vec![Alternative {
                            transcript: "Hello world".to_string(),
                            confidence: 0.95,
                            words: vec![
                                Word {
                                    word: "Hello".to_string(),
                                    start: 0.0,
                                    end: 0.5,
                                    confidence: 0.95,
                                    speaker: None,
                                    speaker_confidence: None,
                                },
                                Word {
                                    word: "world".to_string(),
                                    start: 0.6,
                                    end: 1.0,
                                    confidence: 0.95,
                                    speaker: None,
                                    speaker_confidence: None,
                                },
                            ],
                        }],
                    }],
                    utterances: vec![Utterance {
                        start: 0.0,
                        end: 1.0,
                        confidence: 0.95,
                        channel: 0,
                        transcript: "Hello world".to_string(),
                        words: vec![
                            Word {
                                word: "Hello".to_string(),
                                start: 0.0,
                                end: 0.5,
                                confidence: 0.95,
                                speaker: None,
                                speaker_confidence: None,
                            },
                            Word {
                                word: "world".to_string(),
                                start: 0.6,
                                end: 1.0,
                                confidence: 0.95,
                                speaker: None,
                                speaker_confidence: None,
                            },
                        ],
                        speaker: None,
                        id: "test-utterance-1".to_string(),
                    }],
                },
            },
        };

        assert_eq!(response, expected_response);
    }

    #[wstd::test]
    async fn test_transcribe_audio_with_diarization_success() {
        let mock_client = MockHttpClient::new();

        let response_body = r#"{
            "metadata": {
                "transaction_key": "test-transaction-key",
                "request_id": "test-request-id",
                "sha256": "test-sha256",
                "created": "2023-01-01T00:00:00Z",
                "duration": 10.5,
                "channels": 1,
                "models": ["nova-2"],
                "model_info": {
                    "nova-2": {
                        "name": "nova-2",
                        "version": "1.0.0",
                        "arch": "transformer"
                    }
                }
            },
            "results": {
                "channels": [{
                    "alternatives": [{
                        "transcript": "Hello world",
                        "confidence": 0.95,
                        "words": [{
                            "word": "Hello",
                            "start": 0.0,
                            "end": 0.5,
                            "confidence": 0.95,
                            "speaker": 0,
                            "speaker_confidence": 0.9
                        }, {
                            "word": "world",
                            "start": 0.6,
                            "end": 1.0,
                            "confidence": 0.95,
                            "speaker": 0,
                            "speaker_confidence": 0.9
                        }]
                    }]
                }],
                "utterances": [{
                    "start": 0.0,
                    "end": 1.0,
                    "confidence": 0.95,
                    "channel": 0,
                    "transcript": "Hello world",
                    "words": [{
                        "word": "Hello",
                        "start": 0.0,
                        "end": 0.5,
                        "confidence": 0.95,
                        "speaker": 0,
                        "speaker_confidence": 0.9
                    }, {
                        "word": "world",
                        "start": 0.6,
                        "end": 1.0,
                        "confidence": 0.95,
                        "speaker": 0,
                        "speaker_confidence": 0.9
                    }],
                    "speaker": 0,
                    "id": "test-utterance-2"
                }]
            }
        }"#;

        mock_client.expect_response(
            Response::builder()
                .status(StatusCode::OK)
                .body(response_body.as_bytes().to_vec())
                .unwrap(),
        );

        let api = PreRecordedAudioApi::new(
            TEST_API_KEY.to_string(),
            TEST_ENDPOINT.to_string(),
            mock_client,
        );

        let audio_data = Bytes::from("fake audio data");
        let request = TranscriptionRequest {
            request_id: "some-transcription-id".to_string(),
            audio: audio_data.clone(),
            audio_config: AudioConfig {
                format: AudioFormat::Wav,
                channels: None,
            },
            transcription_config: None,
        };

        let response = api.transcribe_audio(request).await.unwrap();

        let expected_response = TranscriptionResponse {
            request_id: "some-transcription-id".to_string(),
            language: String::new(),
            audio_size_bytes: audio_data.len(),
            deepgram_transcription: DeepgramTranscription {
                metadata: Metadata {
                    transaction_key: "test-transaction-key".to_string(),
                    request_id: "test-request-id".to_string(),
                    sha256: "test-sha256".to_string(),
                    created: "2023-01-01T00:00:00Z".to_string(),
                    duration: 10.5,
                    channels: 1,
                    models: vec!["nova-2".to_string()],
                    model_info: HashMap::from([(
                        "nova-2".to_string(),
                        ModelInfo {
                            name: "nova-2".to_string(),
                            version: "1.0.0".to_string(),
                            arch: "transformer".to_string(),
                        },
                    )]),
                },
                results: Results {
                    channels: vec![Channel {
                        alternatives: vec![Alternative {
                            transcript: "Hello world".to_string(),
                            confidence: 0.95,
                            words: vec![
                                Word {
                                    word: "Hello".to_string(),
                                    start: 0.0,
                                    end: 0.5,
                                    confidence: 0.95,
                                    speaker: Some(0),
                                    speaker_confidence: Some(0.9),
                                },
                                Word {
                                    word: "world".to_string(),
                                    start: 0.6,
                                    end: 1.0,
                                    confidence: 0.95,
                                    speaker: Some(0),
                                    speaker_confidence: Some(0.9),
                                },
                            ],
                        }],
                    }],
                    utterances: vec![Utterance {
                        start: 0.0,
                        end: 1.0,
                        confidence: 0.95,
                        channel: 0,
                        transcript: "Hello world".to_string(),
                        words: vec![
                            Word {
                                word: "Hello".to_string(),
                                start: 0.0,
                                end: 0.5,
                                confidence: 0.95,
                                speaker: Some(0),
                                speaker_confidence: Some(0.9),
                            },
                            Word {
                                word: "world".to_string(),
                                start: 0.6,
                                end: 1.0,
                                confidence: 0.95,
                                speaker: Some(0),
                                speaker_confidence: Some(0.9),
                            },
                        ],
                        speaker: Some(0),
                        id: "test-utterance-2".to_string(),
                    }],
                },
            },
        };

        assert_eq!(response, expected_response);
    }

    #[wstd::test]
    async fn test_transcribe_audio_error_bad_request() {
        let mock_client = MockHttpClient::new();

        let error_body = r#"{
          "err_code": "INVALID_AUDIO",
          "err_msg": "Invalid audio format.",
          "request_id": "32313879-0783-4b57-871d-69124a18373a"
        }"#;
        mock_client.expect_response(
            Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(error_body.as_bytes().to_vec())
                .unwrap(),
        );

        let api = PreRecordedAudioApi::new(
            TEST_API_KEY.to_string(),
            TEST_ENDPOINT.to_string(),
            mock_client,
        );

        let request = TranscriptionRequest {
            request_id: "some-transcription-id".to_string(),
            audio: "fake audio data".into(),
            audio_config: AudioConfig {
                format: AudioFormat::Wav,
                channels: None,
            },
            transcription_config: None,
        };

        let result = api.transcribe_audio(request).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            Error::APIBadRequest {
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

        let error_body = r#"{
          "err_code": "INVALID_AUTH",
          "err_msg": "Invalid credentials.",
          "request_id": "32313879-0783-4b57-871d-69124a18373a"
        }"#;

        mock_client.expect_response(
            Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .body(error_body.as_bytes().to_vec())
                .unwrap(),
        );

        let api = PreRecordedAudioApi::new(
            TEST_API_KEY.to_string(),
            TEST_ENDPOINT.to_string(),
            mock_client,
        );

        let request = TranscriptionRequest {
            request_id: "some-transcription-id".to_string(),
            audio: "fake audio data".into(),
            audio_config: AudioConfig {
                format: AudioFormat::Wav,
                channels: None,
            },
            transcription_config: None,
        };

        let result = api.transcribe_audio(request).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            Error::APIUnauthorized {
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
    async fn test_transcribe_audio_error_access_denied() {
        let mock_client = MockHttpClient::new();

        let error_body = r#"{
          "err_code": "OUT_OF_CREDITS",
          "err_msg": "Not enough credits.",
          "request_id": "32313879-0783-4b57-871d-69124a18373a"
        }"#;
        mock_client.expect_response(
            Response::builder()
                .status(StatusCode::PAYMENT_REQUIRED)
                .body(error_body.as_bytes().to_vec())
                .unwrap(),
        );

        let api = PreRecordedAudioApi::new(
            TEST_API_KEY.to_string(),
            TEST_ENDPOINT.to_string(),
            mock_client,
        );

        let request = TranscriptionRequest {
            request_id: "some-transcription-id".to_string(),
            audio: "fake audio data".into(),
            audio_config: AudioConfig {
                format: AudioFormat::Wav,
                channels: None,
            },
            transcription_config: None,
        };

        let result = api.transcribe_audio(request).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            Error::APIAccessDenied {
                request_id,
                provider_error,
            } => {
                assert_eq!(request_id, "some-transcription-id");
                assert_eq!(provider_error, error_body);
            }
            _ => panic!("Expected APIAccessDenied error"),
        }
    }

    #[wstd::test]
    async fn test_transcribe_audio_error_forbidden() {
        let mock_client = MockHttpClient::new();

        let error_body = r#"{
          "err_code": "ACCESS_DENIED",
          "err_msg": "Access denied.",
          "request_id": "32313879-0783-4b57-871d-69124a18373a"
        }"#;

        mock_client.expect_response(
            Response::builder()
                .status(StatusCode::FORBIDDEN)
                .body(error_body.as_bytes().to_vec())
                .unwrap(),
        );

        let api = PreRecordedAudioApi::new(
            TEST_API_KEY.to_string(),
            TEST_ENDPOINT.to_string(),
            mock_client,
        );

        let request = TranscriptionRequest {
            request_id: "some-transcription-id".to_string(),
            audio: "fake audio data".into(),
            audio_config: AudioConfig {
                format: AudioFormat::Wav,
                channels: None,
            },
            transcription_config: None,
        };

        let result = api.transcribe_audio(request).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            Error::APIForbidden {
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
    async fn test_transcribe_audio_error_internal_server_error() {
        let mock_client = MockHttpClient::new();

        let error_body = r#"{"error": "Internal server error"}"#;
        mock_client.expect_response(
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(error_body.as_bytes().to_vec())
                .unwrap(),
        );

        let api = PreRecordedAudioApi::new(
            TEST_API_KEY.to_string(),
            TEST_ENDPOINT.to_string(),
            mock_client,
        );

        let request = TranscriptionRequest {
            request_id: "some-transcription-id".to_string(),
            audio: "fake audio data".into(),
            audio_config: AudioConfig {
                format: AudioFormat::Wav,
                channels: None,
            },
            transcription_config: None,
        };

        let result = api.transcribe_audio(request).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            Error::APIInternalServerError {
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

        let error_body = r#"{"error": "Unknown error"}"#;
        mock_client.expect_response(
            Response::builder()
                .status(StatusCode::IM_A_TEAPOT)
                .body(error_body.as_bytes().to_vec())
                .unwrap(),
        );

        let api = PreRecordedAudioApi::new(
            TEST_API_KEY.to_string(),
            TEST_ENDPOINT.to_string(),
            mock_client,
        );

        let request = TranscriptionRequest {
            request_id: "some-transcription-id".to_string(),
            audio: "fake audio data".into(),
            audio_config: AudioConfig {
                format: AudioFormat::Wav,
                channels: None,
            },
            transcription_config: None,
        };

        let result = api.transcribe_audio(request).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            Error::APIUnknown {
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
