use bytes::Bytes;
use golem_stt::{
    http::{HttpClient, MultipartBuilder},
    transcription::SttProviderClient,
};
use log::trace;
use serde::{Deserialize, Serialize};

use golem_stt::error::Error;
use golem_stt::languages::Language;
use http::{Method, Request, StatusCode};

const BASE_URL: &str = "https://api.openai.com/v1/audio/transcriptions";

const WHISPER_SUPPORTED_LANGUAGES: [Language; 57] = [
    Language::new("af", "Afrikaans", "Afrikaans"),
    Language::new("ar", "Arabic", "العربية"),
    Language::new("hy", "Armenian", "հայերեն"),
    Language::new("az", "Azerbaijani", "azərbaycan dili"),
    Language::new("be", "Belarusian", "беларуская мова"),
    Language::new("bs", "Bosnian", "bosanski jezik"),
    Language::new("bg", "Bulgarian", "български език"),
    Language::new("ca", "Catalan", "català"),
    Language::new("zh", "Chinese", "中文"),
    Language::new("hr", "Croatian", "hrvatski jezik"),
    Language::new("cs", "Czech", "čeština"),
    Language::new("da", "Danish", "dansk"),
    Language::new("nl", "Dutch", "Nederlands"),
    Language::new("en", "English", "English"),
    Language::new("et", "Estonian", "eesti keel"),
    Language::new("fi", "Finnish", "suomi"),
    Language::new("fr", "French", "français"),
    Language::new("gl", "Galician", "galego"),
    Language::new("de", "German", "Deutsch"),
    Language::new("el", "Greek", "ελληνικά"),
    Language::new("he", "Hebrew", "עברית"),
    Language::new("hi", "Hindi", "हिन्दी"),
    Language::new("hu", "Hungarian", "magyar"),
    Language::new("is", "Icelandic", "íslenska"),
    Language::new("id", "Indonesian", "Bahasa Indonesia"),
    Language::new("it", "Italian", "italiano"),
    Language::new("ja", "Japanese", "日本語"),
    Language::new("kn", "Kannada", "ಕನ್ನಡ"),
    Language::new("kk", "Kazakh", "қазақ тілі"),
    Language::new("ko", "Korean", "한국어"),
    Language::new("lv", "Latvian", "latviešu valoda"),
    Language::new("lt", "Lithuanian", "lietuvių kalba"),
    Language::new("mk", "Macedonian", "македонски јазик"),
    Language::new("ms", "Malay", "Bahasa Melayu"),
    Language::new("mr", "Marathi", "मराठी"),
    Language::new("mi", "Maori", "te reo Māori"),
    Language::new("ne", "Nepali", "नेपाली"),
    Language::new("no", "Norwegian", "norsk"),
    Language::new("fa", "Persian", "فارسی"),
    Language::new("pl", "Polish", "polski"),
    Language::new("pt", "Portuguese", "português"),
    Language::new("ro", "Romanian", "română"),
    Language::new("ru", "Russian", "русский язык"),
    Language::new("sr", "Serbian", "српски језик"),
    Language::new("sk", "Slovak", "slovenčina"),
    Language::new("sl", "Slovenian", "slovenščina"),
    Language::new("es", "Spanish", "español"),
    Language::new("sw", "Swahili", "Kiswahili"),
    Language::new("sv", "Swedish", "svenska"),
    Language::new("tl", "Tagalog", "Tagalog"),
    Language::new("ta", "Tamil", "தமிழ்"),
    Language::new("th", "Thai", "ไทย"),
    Language::new("tr", "Turkish", "Türkçe"),
    Language::new("uk", "Ukrainian", "українська мова"),
    Language::new("ur", "Urdu", "اردو"),
    Language::new("vi", "Vietnamese", "Tiếng Việt"),
    Language::new("cy", "Welsh", "Cymraeg"),
];

pub fn is_supported_language(language_code: &str) -> bool {
    WHISPER_SUPPORTED_LANGUAGES
        .iter()
        .any(|lang| lang.code == language_code)
}

pub fn get_supported_languages() -> &'static [Language] {
    &WHISPER_SUPPORTED_LANGUAGES
}

#[allow(unused)]
#[derive(Debug, Clone)]
pub enum AudioFormat {
    Wav,
    Mp3,
    Mp4,
    Mpeg,
    Mpga,
    M4a,
    Flac,
    Ogg,
    Webm,
}

impl core::fmt::Display for AudioFormat {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string_representation = match self {
            AudioFormat::Wav => "wav",
            AudioFormat::Mp3 => "mp3",
            AudioFormat::Mp4 => "mp4",
            AudioFormat::Mpeg => "mpeg",
            AudioFormat::Mpga => "mpga",
            AudioFormat::M4a => "m4a",
            AudioFormat::Flac => "flac",
            AudioFormat::Ogg => "ogg",
            AudioFormat::Webm => "webm",
        };
        write!(fmt, "{string_representation}")
    }
}

#[derive(Debug, Clone)]
pub struct AudioConfig {
    pub format: AudioFormat,
}

#[derive(Debug, Clone)]
pub struct TranscriptionConfig {
    pub language: Option<String>,
    pub prompt: Option<String>,
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

/// The OpenAI API client for transcribing audio into the input language powered by their open source Whisper V2 model
///
/// https://platform.openai.com/docs/api-reference/audio/createTranscription
#[derive(Debug)]
pub struct TranscriptionsApi<HC: HttpClient> {
    openai_api_token: String,
    http_client: HC,
}

#[allow(unused)]
impl<HC: HttpClient> TranscriptionsApi<HC> {
    pub fn new(openai_api_key: String, http_client: HC) -> Self {
        Self {
            openai_api_token: format!("Bearer {openai_api_key}"),
            http_client,
        }
    }
}

impl<HC: HttpClient> SttProviderClient<TranscriptionRequest, TranscriptionResponse, Error>
    for TranscriptionsApi<HC>
{
    async fn transcribe_audio(
        &self,
        request: TranscriptionRequest,
    ) -> Result<TranscriptionResponse, Error> {
        trace!("Sending request to OpenAI API: {request:?}");

        let request_id = request.request_id;

        let file_name = format!("audio.{}", request.audio_config.format);
        let mime_type = get_mime_type(&request.audio_config.format);

        let audio_size_bytes = request.audio.len();

        let mut form = MultipartBuilder::new_with_capacity(audio_size_bytes + 2048);

        form.add_bytes("file", &file_name, &mime_type, &request.audio);

        form.add_field("model", "whisper-1");
        form.add_field("response_format", "verbose_json");
        form.add_field("timestamp_granularities[]", "word");

        if let Some(transcription_config) = request.transcription_config {
            if let Some(language) = transcription_config.language {
                form.add_field("language", &language);
            }

            if let Some(prompt) = transcription_config.prompt {
                form.add_field("prompt", &prompt);
            }
        }

        let (content_type, body) = form.finish();

        trace!("sending multipart form: {}", String::from_utf8_lossy(&body));

        let req = Request::builder()
            .method(Method::POST)
            .uri(BASE_URL)
            .header("Authorization", &self.openai_api_token)
            .header("Content-Type", content_type)
            .body(body)
            .map_err(|e| Error::Http(request_id.clone(), golem_stt::http::Error::HttpError(e)))?;

        let response = self
            .http_client
            .execute(req)
            .await
            .map_err(|e| Error::Http(request_id.clone(), e))?;

        // match what official OpenAI SDK does https://github.com/openai/openai-python/blob/0673da62f2f2476a3e5791122e75ec0cbfd03442/src/openai/_client.py#L343
        if response.status().is_success() {
            trace!("response: {}", String::from_utf8_lossy(response.body()));

            let whisper_transcription: WhisperTranscription =
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
                whisper_transcription,
            })
        } else {
            let provider_error = String::from_utf8(response.body().to_vec()).map_err(|e| {
                Error::Http(
                    request_id.clone(),
                    golem_stt::http::Error::Generic(format!(
                        "Failed to parse response as UTF-8: {e}"
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
                StatusCode::FORBIDDEN => Err(Error::APIForbidden {
                    request_id,
                    provider_error,
                }),
                StatusCode::NOT_FOUND => Err(Error::APINotFound {
                    request_id,
                    provider_error,
                }),
                StatusCode::CONFLICT => Err(Error::APIConflict {
                    request_id,
                    provider_error,
                }),
                StatusCode::UNPROCESSABLE_ENTITY => Err(Error::APIUnprocessableEntity {
                    request_id,
                    provider_error,
                }),
                StatusCode::TOO_MANY_REQUESTS => Err(Error::APIRateLimit {
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
    pub whisper_transcription: WhisperTranscription,
}

#[allow(unused)]
#[derive(Debug, Deserialize, PartialEq)]
pub struct WhisperTranscription {
    pub task: String,
    pub language: String,
    pub duration: f64,
    pub text: String,
    pub words: Vec<Word>,
    pub usage: Usage,
}

#[allow(unused)]
#[derive(Debug, Deserialize, PartialEq)]
pub struct Word {
    pub word: String,
    pub start: f64,
    pub end: f64,
}

#[allow(unused)]
#[derive(Debug, Deserialize, PartialEq)]
pub struct Segment {
    pub id: u32,
    pub seek: u32,
    pub start: f64,
    pub end: f64,
    pub text: String,
    pub temperature: f64,
    pub avg_logprob: f64,
    pub compression_ratio: f64,
    pub no_speech_prob: f64,
}

#[allow(unused)]
#[derive(Debug, Deserialize, PartialEq)]
pub struct Usage {
    pub r#type: String,
    pub seconds: u32,
}

#[allow(unused)]
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct ErrorBody {
    pub message: String,
    pub r#type: String,
    pub param: Option<String>,
    pub code: Option<String>,
}

fn get_mime_type(format: &AudioFormat) -> String {
    match format {
        AudioFormat::Wav => "audio/wav".to_string(),
        AudioFormat::Mp3 => "audio/mp3".to_string(),
        AudioFormat::Flac => "audio/flac".to_string(),
        AudioFormat::Ogg => "audio/ogg".to_string(),
        AudioFormat::Mp4 => "video/mp4".to_string(),
        AudioFormat::Mpeg => "audio/mpeg".to_string(),
        AudioFormat::Mpga => "audio/mpeg".to_string(),
        AudioFormat::M4a => "audio/mp4".to_string(),
        AudioFormat::Webm => "video/webm".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use http::Response;
    use std::cell::{Ref, RefCell};
    use std::collections::{HashMap, VecDeque};
    use std::io::{Cursor, Read};

    use multipart_2021::server::Multipart;

    use super::*;

    const TEST_API_KEY: &str = "test-api-key";

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

    #[derive(Debug, Clone, PartialEq)]
    struct MultipartField {
        pub data: Vec<u8>,
        pub filename: Option<String>,
        pub content_type: Option<String>,
    }

    #[wstd::test]
    async fn test_api_key_gets_passed_as_auth_header() {
        let response_body = r#"
               {
                   "task": "transcribe",
                   "language": "en",
                   "duration": 8.2,
                   "text": "Hello world",
                   "words": [
                       {
                           "word": "Hello",
                           "start": 0.0,
                           "end": 1.5
                       },
                       {
                           "word": "world",
                           "start": 1.5,
                           "end": 3.0
                       }
                   ],
                   "usage": {
                       "type": "transcribe",
                       "seconds": 10
                   }
               }
           "#;

        let mock_client = MockHttpClient::new();
        mock_client.expect_response(
            Response::builder()
                .status(StatusCode::OK)
                .body(response_body.as_bytes().to_vec())
                .unwrap(),
        );

        let api = TranscriptionsApi::new(TEST_API_KEY.to_string(), mock_client);

        let audio_bytes = "fake audio data".into();

        let request = TranscriptionRequest {
            request_id: "some-transcription-id".to_string(),
            audio: audio_bytes,
            audio_config: AudioConfig {
                format: AudioFormat::Mp3,
            },
            transcription_config: None,
        };

        api.transcribe_audio(request).await.unwrap();

        let captured_request = api.http_client.last_captured_request().unwrap();

        assert_eq!(captured_request.method(), &Method::POST);

        assert_eq!(captured_request.uri().path(), "/v1/audio/transcriptions");

        let auth_header = captured_request
            .headers()
            .get("Authorization")
            .and_then(|h| h.to_str().ok());

        assert_eq!(auth_header, Some("Bearer test-api-key"));

        assert_eq!(api.http_client.captured_request_count(), 1);
    }

    #[wstd::test]
    async fn test_resquest_gets_sent_as_multi_part_form_data() {
        let response_body = r#"
               {
                   "task": "transcribe",
                   "language": "en",
                   "duration": 8.2,
                   "text": "Hello world",
                   "words": [
                       {
                           "word": "Hello",
                           "start": 0.0,
                           "end": 1.5
                       },
                       {
                           "word": "world",
                           "start": 1.5,
                           "end": 3.0
                       }
                   ],
                   "usage": {
                       "type": "transcribe",
                       "seconds": 10
                   }
               }
           "#;

        let mock_client = MockHttpClient::new();
        mock_client.expect_response(
            Response::builder()
                .status(StatusCode::OK)
                .body(response_body.as_bytes().to_vec())
                .unwrap(),
        );

        let api = TranscriptionsApi::new(TEST_API_KEY.to_string(), mock_client);

        let audio_bytes = Bytes::from("fake audio data");

        let language = "en".to_string();
        let prompt = "foo".to_string();

        let request = TranscriptionRequest {
            request_id: "some-transcription-id".to_string(),
            audio: audio_bytes.clone(),
            audio_config: AudioConfig {
                format: AudioFormat::Mp3,
            },
            transcription_config: Some(TranscriptionConfig {
                language: Some(language.clone()),
                prompt: Some(prompt.clone()),
            }),
        };

        api.transcribe_audio(request).await.unwrap();

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

        let file_field = fields.get("file").unwrap();
        assert_eq!(
            file_field,
            &MultipartField {
                data: audio_bytes.to_vec(),
                filename: Some("audio.mp3".to_string()),
                content_type: Some("audio/mp3".to_string()),
            }
        );

        let model_field = fields.get("model").unwrap();
        assert_eq!(
            model_field,
            &MultipartField {
                data: b"whisper-1".to_vec(),
                filename: None,
                content_type: None,
            }
        );

        let response_format_field = fields.get("response_format").unwrap();
        assert_eq!(
            response_format_field,
            &MultipartField {
                data: b"verbose_json".to_vec(),
                filename: None,
                content_type: None,
            }
        );

        let language_field = fields.get("language").unwrap();
        assert_eq!(
            language_field,
            &MultipartField {
                data: language.as_bytes().to_vec(),
                filename: None,
                content_type: None,
            }
        );

        let verbose_field = fields.get("prompt").unwrap();
        assert_eq!(
            verbose_field,
            &MultipartField {
                data: prompt.as_bytes().to_vec(),
                filename: None,
                content_type: None,
            }
        );

        assert_eq!(api.http_client.captured_request_count(), 1);
    }

    #[wstd::test]
    async fn test_transcribe_audio_success() {
        let response_body = r#"
               {
                   "task": "transcribe",
                   "language": "en",
                   "duration": 8.2,
                   "text": "Hello world",
                   "words": [
                       {
                           "word": "Hello",
                           "start": 0.0,
                           "end": 1.5
                       },
                       {
                           "word": "world",
                           "start": 1.5,
                           "end": 3.0
                       }
                   ],
                   "usage": {
                       "type": "transcribe",
                       "seconds": 10
                   }
               }
           "#;

        let mock_client = MockHttpClient::new();
        mock_client.expect_response(
            Response::builder()
                .status(StatusCode::OK)
                .body(response_body.as_bytes().to_vec())
                .unwrap(),
        );

        let api = TranscriptionsApi::new(TEST_API_KEY.to_string(), mock_client);

        let audio_bytes = Bytes::from("fake audio data for words test");

        let audio_byte_len = audio_bytes.len();

        let request = TranscriptionRequest {
            request_id: "some-transcription-id".to_string(),
            audio: audio_bytes,
            audio_config: AudioConfig {
                format: AudioFormat::Wav,
            },
            transcription_config: Some(TranscriptionConfig {
                language: Some("en".to_string()),
                prompt: None,
            }),
        };

        let response = api.transcribe_audio(request).await.unwrap();

        let expected_response = TranscriptionResponse {
            request_id: "some-transcription-id".to_string(),
            audio_size_bytes: audio_byte_len,
            whisper_transcription: WhisperTranscription {
                task: "transcribe".to_string(),
                language: "en".to_string(),
                duration: 8.2,
                text: "Hello world".to_string(),
                words: vec![
                    Word {
                        word: "Hello".to_string(),
                        start: 0.0,
                        end: 1.5,
                    },
                    Word {
                        word: "world".to_string(),
                        start: 1.5,
                        end: 3.0,
                    },
                ],
                usage: Usage {
                    r#type: "transcribe".to_string(),
                    seconds: 10,
                },
            },
        };

        assert_eq!(response, expected_response);
    }

    #[wstd::test]
    async fn test_transcribe_audio_error_bad_request() {
        let error_body = r#"
                {
                    "error": {
                        "message": "[{'type': 'enum', 'loc': ('body', 'timestamp_granularities[]', 0), 'msg': \"Input should be 'segment' or 'word'\", 'input': 'word,segments', 'ctx': {'expected': \"'segment' or 'word'\"}}]",
                        "type": "invalid_request_error",
                        "param": null,
                        "code": null
                    }
                }
            "#;

        let mock_client = MockHttpClient::new();
        mock_client.expect_response(
            Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(error_body.as_bytes().to_vec())
                .unwrap(),
        );

        let api = TranscriptionsApi::new(TEST_API_KEY.to_string(), mock_client);

        let audio_bytes = "fake audio data".into();

        let request = TranscriptionRequest {
            request_id: "some-transcription-id".to_string(),
            audio: audio_bytes,
            audio_config: AudioConfig {
                format: AudioFormat::Mp3,
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
        let error_body = r#"
                {
                    "error": {
                        "message": "Incorrect API key provided",
                        "type": "invalid_request_error",
                        "param": null,
                        "code": "invalid_api_key"
                    }
                }
            "#;

        let mock_client = MockHttpClient::new();
        mock_client.expect_response(
            Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .body(error_body.as_bytes().to_vec())
                .unwrap(),
        );

        let api = TranscriptionsApi::new("invalid_key".to_string(), mock_client);

        let audio_bytes = "fake audio data".into();

        let request = TranscriptionRequest {
            request_id: "some-transcription-id".to_string(),
            audio: audio_bytes,
            audio_config: AudioConfig {
                format: AudioFormat::Wav,
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
    async fn test_transcribe_audio_error_forbidden() {
        let error_body = r#"
                {
                    "error": {
                        "message": "Your account does not have access to this resource",
                        "type": "insufficient_quota",
                        "param": null,
                        "code": "insufficient_quota"
                    }
                }
            "#;

        let mock_client = MockHttpClient::new();
        mock_client.expect_response(
            Response::builder()
                .status(StatusCode::FORBIDDEN)
                .body(error_body.as_bytes().to_vec())
                .unwrap(),
        );

        let api = TranscriptionsApi::new(TEST_API_KEY.to_string(), mock_client);

        let audio_bytes = "fake audio data".into();

        let request = TranscriptionRequest {
            request_id: "some-transcription-id".to_string(),
            audio: audio_bytes,
            audio_config: AudioConfig {
                format: AudioFormat::Flac,
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
    async fn test_transcribe_audio_error_not_found() {
        let error_body = r#"
                {
                    "error": {
                        "message": "The model 'xxxxxxx-2' does not exist",
                        "type": "invalid_request_error",
                        "param": "model",
                        "code": null
                    }
                }
            "#;

        let mock_client = MockHttpClient::new();
        mock_client.expect_response(
            Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(error_body.as_bytes().to_vec())
                .unwrap(),
        );

        let api = TranscriptionsApi::new(TEST_API_KEY.to_string(), mock_client);

        let audio_bytes = "fake audio data".into();

        let request = TranscriptionRequest {
            request_id: "some-transcription-id".to_string(),
            audio: audio_bytes,
            audio_config: AudioConfig {
                format: AudioFormat::Ogg,
            },
            transcription_config: None,
        };

        let result = api.transcribe_audio(request).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::APINotFound {
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
        let error_body = r#"
                {
                    "error": {
                        "message": "The audio file is too large. Maximum size is 25MB.",
                        "type": "invalid_request_error",
                        "param": "file",
                        "code": "file_too_large"
                    }
                }
            "#;

        let mock_client = MockHttpClient::new();
        mock_client.expect_response(
            Response::builder()
                .status(StatusCode::UNPROCESSABLE_ENTITY)
                .body(error_body.as_bytes().to_vec())
                .unwrap(),
        );

        let api = TranscriptionsApi::new(TEST_API_KEY.to_string(), mock_client);

        let audio_bytes = "fake large audio data".into();

        let request = TranscriptionRequest {
            request_id: "some-transcription-id".to_string(),
            audio: audio_bytes,
            audio_config: AudioConfig {
                format: AudioFormat::Mp3,
            },
            transcription_config: None,
        };

        let result = api.transcribe_audio(request).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::APIUnprocessableEntity {
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
        let error_body = r#"
                {
                    "error": {
                        "message": "Rate limit exceeded. Please try again later.",
                        "type": "requests",
                        "param": null,
                        "code": "rate_limit_exceeded"
                    }
                }
            "#;

        let mock_client = MockHttpClient::new();
        mock_client.expect_response(
            Response::builder()
                .status(StatusCode::TOO_MANY_REQUESTS)
                .body(error_body.as_bytes().to_vec())
                .unwrap(),
        );

        let api = TranscriptionsApi::new(TEST_API_KEY.to_string(), mock_client);

        let audio_bytes = "fake audio data".into();

        let request = TranscriptionRequest {
            request_id: "some-transcription-id".to_string(),
            audio: audio_bytes,
            audio_config: AudioConfig {
                format: AudioFormat::Wav,
            },
            transcription_config: None,
        };

        let result = api.transcribe_audio(request).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::APIRateLimit {
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
        let error_body = r#"
                {
                    "error": {
                        "message": "The server encountered an internal error and was unable to complete your request.",
                        "type": "server_error",
                        "param": null,
                        "code": null
                    }
                }
            "#;

        let mock_client = MockHttpClient::new();
        mock_client.expect_response(
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(error_body.as_bytes().to_vec())
                .unwrap(),
        );

        let api = TranscriptionsApi::new(TEST_API_KEY.to_string(), mock_client);

        let audio_bytes = "fake audio data".into();

        let request = TranscriptionRequest {
            request_id: "some-transcription-id".to_string(),
            audio: audio_bytes,
            audio_config: AudioConfig {
                format: AudioFormat::Mp3,
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
        let error_body = r#"
                {
                    "error": {
                        "message": "Unknown error occurred",
                        "type": "unknown_error",
                        "param": null,
                        "code": "unknown"
                    }
                }
            "#;

        let mock_client = MockHttpClient::new();
        mock_client.expect_response(
            Response::builder()
                .status(StatusCode::IM_A_TEAPOT)
                .body(error_body.as_bytes().to_vec())
                .unwrap(),
        );

        let api = TranscriptionsApi::new(TEST_API_KEY.to_string(), mock_client);

        let audio_bytes = "fake audio data".into();

        let request = TranscriptionRequest {
            request_id: "some-transcription-id".to_string(),
            audio: audio_bytes,
            audio_config: AudioConfig {
                format: AudioFormat::Flac,
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
