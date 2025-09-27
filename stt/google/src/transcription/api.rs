use std::time::Duration;

use bytes::Bytes;
use golem_stt::{error::Error as SttError, languages::Language, transcription::SttProviderClient};

use super::{
    gcp_cloud_storage::CloudStorageService,
    gcp_speech_to_text::{BatchRecognizeOperationResponse, RecognizeResults, SpeechToTextService},
    request::TranscriptionRequest,
};

const MAX_SHORT_AUDIO_SIZE: usize = 10 * 1024 * 1024; // 10MB

// https://cloud.google.com/speech-to-text/v2/docs/speech-to-text-supported-languages
// different models support different languages so here is a common set of languages Google Speech to Text supports accross regions
const GOOGLE_SPEECH_SUPPORTED_LANGUAGES: [Language; 117] = [
    Language::new("af-ZA", "Afrikaans (South Africa)", "Afrikaans"),
    Language::new("am-ET", "Amharic (Ethiopia)", "አማርኛ"),
    Language::new("ar-EG", "Arabic (Egypt)", "العربية"),
    Language::new("as-IN", "Assamese (India)", "অসমীয়া"),
    Language::new("ast-ES", "Asturian (Spain)", "asturianu"),
    Language::new("az-AZ", "Azerbaijani (Azerbaijan)", "azərbaycan dili"),
    Language::new("be-BY", "Belarusian (Belarus)", "беларуская"),
    Language::new("bg-BG", "Bulgarian (Bulgaria)", "български"),
    Language::new("bn-BD", "Bengali (Bangladesh)", "বাংলা"),
    Language::new("bn-IN", "Bengali (India)", "বাংলা"),
    Language::new("bs-BA", "Bosnian (Bosnia and Herzegovina)", "bosanski"),
    Language::new("ca-ES", "Catalan (Spain)", "català"),
    Language::new("ceb-PH", "Cebuano (Philippines)", "Cebuano"),
    Language::new("ckb-IQ", "Central Kurdish (Iraq)", "کوردیی ناوەندی"),
    Language::new("cmn-Hans-CN", "Chinese (Simplified, China)", "中文（简体）"),
    Language::new(
        "cmn-Hant-TW",
        "Chinese, Mandarin (Traditional, Taiwan)",
        "中文（繁體）",
    ),
    Language::new("cs-CZ", "Czech (Czech Republic)", "čeština"),
    Language::new("cy-GB", "Welsh (United Kingdom)", "Cymraeg"),
    Language::new("da-DK", "Danish (Denmark)", "dansk"),
    Language::new("de-DE", "German (Germany)", "Deutsch"),
    Language::new("el-GR", "Greek (Greece)", "ελληνικά"),
    Language::new("en-AU", "English (Australia)", "English"),
    Language::new("en-GB", "English (United Kingdom)", "English"),
    Language::new("en-IN", "English (India)", "English"),
    Language::new("en-US", "English (United States)", "English"),
    Language::new("es-419", "Spanish (Latin American)", "español"),
    Language::new("es-ES", "Spanish (Spain)", "español"),
    Language::new("es-US", "Spanish (United States)", "español"),
    Language::new("et-EE", "Estonian (Estonia)", "eesti"),
    Language::new("eu-ES", "Basque (Spain)", "euskera"),
    Language::new("fa-IR", "Persian (Iran)", "فارسی"),
    Language::new("ff-SN", "Fulah (Senegal)", "Fulfulde"),
    Language::new("fi-FI", "Finnish (Finland)", "suomi"),
    Language::new("fil-PH", "Filipino (Philippines)", "Filipino"),
    Language::new("fr-CA", "French (Canada)", "français"),
    Language::new("fr-FR", "French (France)", "français"),
    Language::new("ga-IE", "Irish (Ireland)", "Gaeilge"),
    Language::new("gl-ES", "Galician (Spain)", "galego"),
    Language::new("gu-IN", "Gujarati (India)", "ગુજરાતી"),
    Language::new("ha-NG", "Hausa (Nigeria)", "Hausa"),
    Language::new("hi-IN", "Hindi (India)", "हिन्दी"),
    Language::new("hr-HR", "Croatian (Croatia)", "hrvatski"),
    Language::new("hu-HU", "Hungarian (Hungary)", "magyar"),
    Language::new("hy-AM", "Armenian (Armenia)", "հայերեն"),
    Language::new("id-ID", "Indonesian (Indonesia)", "Bahasa Indonesia"),
    Language::new("ig-NG", "Igbo (Nigeria)", "Igbo"),
    Language::new("is-IS", "Icelandic (Iceland)", "íslenska"),
    Language::new("it-IT", "Italian (Italy)", "italiano"),
    Language::new("iw-IL", "Hebrew (Israel)", "עברית"),
    Language::new("ja-JP", "Japanese (Japan)", "日本語"),
    Language::new("jv-ID", "Javanese (Indonesia)", "basa Jawa"),
    Language::new("ka-GE", "Georgian (Georgia)", "ქართული"),
    Language::new("kam-KE", "Kamba (Kenya)", "Kikamba"),
    Language::new("kea-CV", "Kabuverdianu (Cape Verde)", "Kabuverdianu"),
    Language::new("kk-KZ", "Kazakh (Kazakhstan)", "қазақ тілі"),
    Language::new("km-KH", "Khmer (Cambodia)", "ខ្មែរ"),
    Language::new("kn-IN", "Kannada (India)", "ಕನ್ನಡ"),
    Language::new("ko-KR", "Korean (South Korea)", "한국어"),
    Language::new("ky-KG", "Kyrgyz (Cyrillic)", "кыргызча"),
    Language::new("lb-LU", "Luxembourgish (Luxembourg)", "Lëtzebuergesch"),
    Language::new("lg-UG", "Ganda (Uganda)", "Luganda"),
    Language::new("ln-CD", "Lingala (Congo-Kinshasa)", "Lingála"),
    Language::new("lo-LA", "Lao (Laos)", "ລາວ"),
    Language::new("lt-LT", "Lithuanian (Lithuania)", "lietuvių"),
    Language::new("luo-KE", "Luo (Kenya)", "Luo"),
    Language::new("lv-LV", "Latvian (Latvia)", "latviešu"),
    Language::new("mi-NZ", "Maori (New Zealand)", "te reo Māori"),
    Language::new("mk-MK", "Macedonian (North Macedonia)", "македонски"),
    Language::new("ml-IN", "Malayalam (India)", "മലയാളം"),
    Language::new("mn-MN", "Mongolian (Mongolia)", "монгол"),
    Language::new("mr-IN", "Marathi (India)", "मराठी"),
    Language::new("ms-MY", "Malay (Malaysia)", "Bahasa Melayu"),
    Language::new("mt-MT", "Maltese (Malta)", "Malti"),
    Language::new("my-MM", "Burmese (Myanmar)", "ဗမာ"),
    Language::new("ne-NP", "Nepali (Nepal)", "नेपाली"),
    Language::new("nl-NL", "Dutch (Netherlands)", "Nederlands"),
    Language::new("no-NO", "Norwegian Bokmål (Norway)", "norsk"),
    Language::new("nso-ZA", "Sepedi (South Africa)", "Sesotho sa Leboa"),
    Language::new("ny-MW", "Nyanja (Malawi)", "Chinyanja"),
    Language::new("oc-FR", "Occitan (France)", "occitan"),
    Language::new("om-ET", "Oromo (Ethiopia)", "Afaan Oromoo"),
    Language::new("or-IN", "Oriya (India)", "ଓଡ଼ିଆ"),
    Language::new("pa-Guru-IN", "Punjabi (Gurmukhi India)", "ਪੰਜਾਬੀ"),
    Language::new("pl-PL", "Polish (Poland)", "polski"),
    Language::new("ps-AF", "Pashto", "پښتو"),
    Language::new("pt-BR", "Portuguese (Brazil)", "português"),
    Language::new("pt-PT", "Portuguese (Portugal)", "português"),
    Language::new("ro-RO", "Romanian (Romania)", "română"),
    Language::new("ru-RU", "Russian (Russia)", "русский"),
    Language::new("rup-BG", "Aromanian (Bulgaria)", "armãneashti"),
    Language::new("sd-IN", "Sindhi (India)", "سنڌي"),
    Language::new("si-LK", "Sinhala (Sri Lanka)", "සිංහල"),
    Language::new("sk-SK", "Slovak (Slovakia)", "slovenčina"),
    Language::new("sl-SI", "Slovenian (Slovenia)", "slovenščina"),
    Language::new("sn-ZW", "Shona (Zimbabwe)", "chiShona"),
    Language::new("so-SO", "Somali", "Soomaali"),
    Language::new("sq-AL", "Albanian (Albania)", "shqip"),
    Language::new("sr-RS", "Serbian (Serbia)", "српски"),
    Language::new("su-ID", "Sundanese (Indonesia)", "basa Sunda"),
    Language::new("sv-SE", "Swedish (Sweden)", "svenska"),
    Language::new("sw", "Swahili", "Kiswahili"),
    Language::new("sw-KE", "Swahili (Kenya)", "Kiswahili"),
    Language::new("ta-IN", "Tamil (India)", "தமிழ்"),
    Language::new("te-IN", "Telugu (India)", "తెలుగు"),
    Language::new("tg-TJ", "Tajik (Tajikistan)", "тоҷикӣ"),
    Language::new("th-TH", "Thai (Thailand)", "ไทย"),
    Language::new("tr-TR", "Turkish (Turkey)", "Türkçe"),
    Language::new("uk-UA", "Ukrainian (Ukraine)", "українська"),
    Language::new("umb-AO", "Umbundu (Angola)", "Umbundu"),
    Language::new("ur-PK", "Urdu (Pakistan)", "اردو"),
    Language::new("uz-UZ", "Uzbek (Uzbekistan)", "o'zbekcha"),
    Language::new("vi-VN", "Vietnamese (Vietnam)", "Tiếng Việt"),
    Language::new("wo-SN", "Wolof (Senegal)", "Wolof"),
    Language::new("xh-ZA", "Xhosa (South Africa)", "isiXhosa"),
    Language::new("yo-NG", "Yoruba (Nigeria)", "Yorùbá"),
    Language::new(
        "yue-Hant-HK",
        "Chinese, Cantonese (Traditional Hong Kong)",
        "廣東話",
    ),
    Language::new("zu-ZA", "Zulu (South Africa)", "isiZulu"),
];

pub fn is_supported_language(language_code: &str) -> bool {
    GOOGLE_SPEECH_SUPPORTED_LANGUAGES
        .iter()
        .any(|lang| lang.code == language_code)
}

pub fn get_supported_languages() -> &'static [Language] {
    &GOOGLE_SPEECH_SUPPORTED_LANGUAGES
}

pub struct SpeechToTextApi<GC: CloudStorageService, ST: SpeechToTextService> {
    bucket_name: String,
    cloud_storage_service: GC,
    speech_to_text_service: ST,
}

impl<GC: CloudStorageService, ST: SpeechToTextService> SpeechToTextApi<GC, ST> {
    pub fn new(bucket_name: String, cloud_storage_service: GC, speech_to_text_service: ST) -> Self {
        Self {
            bucket_name,
            cloud_storage_service,
            speech_to_text_service,
        }
    }

    async fn run_synchronous_transcription(
        &self,
        request_id: &str,
        audio_content: &[u8],
        audio_config: &super::request::AudioConfig,
        transcription_config: Option<&super::request::TranscriptionConfig>,
    ) -> Result<RecognizeResults, SttError> {
        let recognize_response = self
            .speech_to_text_service
            .recognize(
                request_id,
                audio_content,
                audio_config,
                transcription_config,
            )
            .await?;

        // Convert RecognizeResults for consistency
        let batch_results = RecognizeResults {
            results: recognize_response.results,
            metadata: recognize_response.metadata,
        };

        Ok(batch_results)
    }

    async fn upload_audio_to_gcs(
        &self,
        request_id: &str,
        object_name: &str,
        audio_data: Bytes,
    ) -> Result<(), SttError> {
        self.cloud_storage_service
            .put_object(request_id, &self.bucket_name, object_name, audio_data)
            .await
    }

    async fn run_transcription_job(
        &self,
        request_id: &str,
        gcs_uri: &str,
        audio_config: &super::request::AudioConfig,
        transcription_config: Option<&super::request::TranscriptionConfig>,
    ) -> Result<BatchRecognizeOperationResponse, SttError> {
        let operation_response = self
            .speech_to_text_service
            .start_batch_recognize(
                request_id,
                vec![gcs_uri.to_string()],
                audio_config,
                transcription_config,
            )
            .await?;

        let max_wait_time = Duration::from_secs(3600 * 6);
        let completed_operation = self
            .speech_to_text_service
            .wait_for_batch_recognize_completion(
                request_id,
                &operation_response.name,
                max_wait_time,
            )
            .await?;

        Ok(completed_operation)
    }
}

// implementation based on https://cloud.google.com/speech-to-text/v2/docs/reference/rest
impl<GC: CloudStorageService, ST: SpeechToTextService>
    SttProviderClient<TranscriptionRequest, TranscriptionResponse, SttError>
    for SpeechToTextApi<GC, ST>
{
    async fn transcribe_audio(
        &self,
        request: TranscriptionRequest,
    ) -> Result<TranscriptionResponse, SttError> {
        let request_id = request.request_id;

        validate_request_id(&request_id).map_err(|validation_error| SttError::APIBadRequest {
            request_id: request_id.clone(),
            provider_error: format!("Invalid request ID: {validation_error}"),
        })?;

        let audio_size = request.audio.len();

        let use_sync_recognition = audio_size < MAX_SHORT_AUDIO_SIZE
            && request
                .transcription_config
                .as_ref()
                .and_then(|config| config.model.as_ref())
                .map(|model| {
                    model.eq_ignore_ascii_case("short")
                        || model.eq_ignore_ascii_case("latest_short")
                        || model.eq_ignore_ascii_case("command_and_search")
                })
                .unwrap_or(false);
        let gcp_transcription = if use_sync_recognition {
            self.run_synchronous_transcription(
                &request_id,
                &request.audio,
                &request.audio_config,
                request.transcription_config.as_ref(),
            )
            .await?
        } else {
            let extension = determine_audio_extension(&request.audio_config.format);
            let object_name = format!("{}/audio{}", request_id.clone(), extension);

            self.upload_audio_to_gcs(&request_id, &object_name, request.audio)
                .await?;

            let gcs_uri = format!("gs://{}/{}", self.bucket_name, object_name);
            let transcription_result = self
                .run_transcription_job(
                    &request_id,
                    &gcs_uri,
                    &request.audio_config,
                    request.transcription_config.as_ref(),
                )
                .await;

            let cleanup_result = self
                .cloud_storage_service
                .delete_object(&request_id, &self.bucket_name, &object_name)
                .await;

            if let Err(cleanup_error) = cleanup_result {
                // Log cleanup error but don't fail the operation
                log::warn!(
                    "Failed to cleanup audio file for request {request_id}: {cleanup_error:?}",
                );
            }

            let gcp_transcription = transcription_result?;

            let mut transcription_response =
                gcp_transcription
                    .response
                    .ok_or_else(|| golem_stt::error::Error::APIUnknown {
                        request_id: request_id.to_string(),
                        provider_error: "Transcription completed but no transcript found"
                            .to_string(),
                    })?;

            let transcription =
                transcription_response
                    .results
                    .remove(&gcs_uri)
                    .ok_or_else(|| golem_stt::error::Error::APIUnknown {
                        request_id: request_id.to_string(),
                        provider_error: format!(
                        "Transcription completed but no transcript found for expected file path {gcs_uri}",
                    ),
                    })?;

            let inline_result =
                transcription
                    .inline_result
                    .ok_or_else(|| golem_stt::error::Error::APIUnknown {
                        request_id: request_id.to_string(),
                        provider_error: "Transcription completed but no InlineResult found"
                            .to_string(),
                    })?;

            inline_result
                .transcript
                .ok_or_else(|| golem_stt::error::Error::APIUnknown {
                    request_id: request_id.to_string(),
                    provider_error:
                        "Transcription completed but transcript is empty (no recognition results)"
                            .to_string(),
                })?
        };

        // Determine language from response or use the first provided language
        let language = request
            .transcription_config
            .as_ref()
            .and_then(|config| config.language_codes.as_ref())
            .and_then(|codes| codes.first())
            .cloned()
            .unwrap_or_else(|| "unknown".to_string());

        // Determine model from configuration
        let model = request
            .transcription_config
            .and_then(|config| config.model.clone())
            .or(Some("latest_long".to_string()));

        Ok(TranscriptionResponse {
            request_id,
            audio_size_bytes: audio_size,
            language,
            model,
            gcp_transcription,
        })
    }
}

fn validate_request_id(request_id: &str) -> Result<(), String> {
    if request_id.is_empty() {
        return Err("Request ID cannot be empty".to_string());
    }

    // Check length - GCS object names have a limit of 1024 bytes, but keep it reasonable
    if request_id.len() > 256 {
        return Err(
            "Request ID too long (max 256 characters for GCS object naming compatibility)"
                .to_string(),
        );
    }

    // GCS object naming requirements: https://cloud.google.com/storage/docs/objects#naming
    // Must contain only valid Unicode characters (we'll be more restrictive for safety)
    let is_valid_char = |c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.';

    if !request_id.chars().all(is_valid_char) {
        return Err(
            "Request ID contains invalid characters. Only alphanumeric characters, hyphens (-), underscores (_), and dots (.) are allowed for GCS object naming".to_string()
        );
    }

    // GCS best practices - start with alphanumeric
    if !request_id.chars().next().unwrap().is_ascii_alphanumeric() {
        return Err("Request ID must start with an alphanumeric character".to_string());
    }

    // Avoid problematic endings for file-like naming
    if request_id.ends_with('-') || request_id.ends_with('_') || request_id.ends_with('.') {
        return Err("Request ID cannot end with hyphens, underscores, or dots".to_string());
    }

    // Avoid consecutive special characters that might cause issues
    let problematic_patterns = ["--", "__", "..", "-_", "_-", "-.", "._", "_.", ".-"];
    for pattern in &problematic_patterns {
        if request_id.contains(pattern) {
            return Err("Request ID cannot contain consecutive special characters".to_string());
        }
    }

    Ok(())
}

fn determine_audio_extension(format: &super::request::AudioFormat) -> &'static str {
    use super::request::AudioFormat;

    match format {
        AudioFormat::LinearPcm => ".pcm",
        AudioFormat::Flac => ".flac",
        AudioFormat::Mp3 => ".mp3",
        AudioFormat::OggOpus => ".ogg",
        AudioFormat::WebmOpus => ".webm",
        AudioFormat::AmrNb => ".amr",
        AudioFormat::AmrWb => ".awb",
        AudioFormat::Wav => ".wav",
        AudioFormat::Mp4 => ".mp4",
        AudioFormat::M4a => ".m4a",
        AudioFormat::Mov => ".mov",
    }
}

#[allow(unused)]
#[derive(Debug, PartialEq)]
pub struct TranscriptionResponse {
    pub request_id: String,
    pub audio_size_bytes: usize,
    pub language: String,
    pub model: Option<String>,
    pub gcp_transcription: RecognizeResults,
}

#[cfg(test)]
mod tests {
    use std::{
        cell::{Ref, RefCell},
        collections::VecDeque,
        time::Duration,
    };

    use super::*;
    use crate::transcription::{
        gcp_speech_to_text::*,
        request::{AudioConfig, AudioFormat, DiarizationConfig, Phrase, TranscriptionConfig},
    };

    #[test]
    fn test_validate_request_id_valid_cases() {
        let valid_ids = vec![
            "abc123",
            "request-123",
            "my_request_456",
            "test-request_789",
            "a1b2c3",
            "RequestID123",
            "user123-session456",
            "batch_job_001",
            "request.123",
            "user.session.456",
            "v1.2.3",
            "api.request.789",
            "test-1.0_beta",
            "service.endpoint.call",
            "user123.session-456_temp",
            "namespace.resource.id",
            "a",
            "1",
            "A1",
            "test123_final.v1",
        ];

        for id in valid_ids {
            assert!(
                validate_request_id(id).is_ok(),
                "Expected '{}' to be valid for GCS, but validation failed: {:?}",
                id,
                validate_request_id(id)
            );
        }
    }

    #[test]
    fn test_validate_request_id_invalid_cases() {
        let long_id = "a".repeat(257);

        let test_cases = vec![
            ("", "Request ID cannot be empty"),
            (&long_id, "Request ID too long"),
            ("request id", "Request ID contains invalid characters"),
            ("request@123", "Request ID contains invalid characters"),
            ("request/123", "Request ID contains invalid characters"),
            ("request#123", "Request ID contains invalid characters"),
            ("request+123", "Request ID contains invalid characters"),
            ("request%123", "Request ID contains invalid characters"),
            ("request&123", "Request ID contains invalid characters"),
            ("request*123", "Request ID contains invalid characters"),
            ("request(123", "Request ID contains invalid characters"),
            ("request)123", "Request ID contains invalid characters"),
            ("request[123", "Request ID contains invalid characters"),
            ("request]123", "Request ID contains invalid characters"),
            ("request{123", "Request ID contains invalid characters"),
            ("request}123", "Request ID contains invalid characters"),
            (
                "-request",
                "Request ID must start with an alphanumeric character",
            ),
            (
                "_request",
                "Request ID must start with an alphanumeric character",
            ),
            (
                ".request",
                "Request ID must start with an alphanumeric character",
            ),
            (
                "request-",
                "Request ID cannot end with hyphens, underscores, or dots",
            ),
            (
                "request_",
                "Request ID cannot end with hyphens, underscores, or dots",
            ),
            (
                "request.",
                "Request ID cannot end with hyphens, underscores, or dots",
            ),
            (
                "request--123",
                "Request ID cannot contain consecutive special characters",
            ),
            (
                "request__123",
                "Request ID cannot contain consecutive special characters",
            ),
            (
                "request..123",
                "Request ID cannot contain consecutive special characters",
            ),
            (
                "request-_123",
                "Request ID cannot contain consecutive special characters",
            ),
            (
                "request_-123",
                "Request ID cannot contain consecutive special characters",
            ),
            (
                "request-.123",
                "Request ID cannot contain consecutive special characters",
            ),
            (
                "request._123",
                "Request ID cannot contain consecutive special characters",
            ),
            (
                "request_.123",
                "Request ID cannot contain consecutive special characters",
            ),
            (
                "request.-123",
                "Request ID cannot contain consecutive special characters",
            ),
        ];

        for (invalid_id, expected_error_part) in test_cases {
            let result = validate_request_id(invalid_id);
            assert!(
                result.is_err(),
                "Expected '{invalid_id}' to be invalid, but validation passed",
            );

            // Your implementation returns String error, not SttError
            if let Err(error_msg) = result {
                assert!(
                    error_msg.contains(expected_error_part),
                    "For input '{invalid_id}', expected error containing '{expected_error_part}', but got: '{error_msg}'"
                );
            }
        }
    }

    #[test]
    fn test_determine_audio_extension() {
        assert_eq!(determine_audio_extension(&AudioFormat::Flac), ".flac");
        assert_eq!(determine_audio_extension(&AudioFormat::Mp3), ".mp3");
        assert_eq!(determine_audio_extension(&AudioFormat::OggOpus), ".ogg");
        assert_eq!(determine_audio_extension(&AudioFormat::WebmOpus), ".webm");
        assert_eq!(determine_audio_extension(&AudioFormat::AmrNb), ".amr");
        assert_eq!(determine_audio_extension(&AudioFormat::AmrWb), ".awb");
        assert_eq!(determine_audio_extension(&AudioFormat::Wav), ".wav");
        assert_eq!(determine_audio_extension(&AudioFormat::Mp4), ".mp4");
        assert_eq!(determine_audio_extension(&AudioFormat::M4a), ".m4a");
        assert_eq!(determine_audio_extension(&AudioFormat::Mov), ".mov");
    }

    #[derive(Debug, PartialEq, Eq)]
    struct GcsPutOperation {
        request_id: String,
        bucket: String,
        object_name: String,
        content_size: usize,
    }

    #[derive(Debug, PartialEq, Eq)]
    struct GcsDeleteOperation {
        request_id: String,
        bucket: String,
        object_name: String,
    }

    #[derive(Debug, PartialEq)]
    struct RecognizeOperation {
        request_id: String,
        audio_size: usize,
        audio_config: AudioConfig,
        transcription_config: Option<TranscriptionConfig>,
    }

    #[derive(Debug, PartialEq)]
    struct StartBatchRecognizeOperation {
        request_id: String,
        audio_gcs_uris: Vec<String>,
        audio_config: AudioConfig,
        transcription_config: Option<TranscriptionConfig>,
    }

    #[derive(Debug, PartialEq, Eq)]
    struct WaitForCompletionOperation {
        request_id: String,
        operation_name: String,
        max_wait_time: Duration,
    }

    struct MockCloudStorageService {
        pub put_object_responses: RefCell<VecDeque<Result<(), SttError>>>,
        pub delete_object_responses: RefCell<VecDeque<Result<(), SttError>>>,
        pub captured_put_operations: RefCell<Vec<GcsPutOperation>>,
        pub captured_delete_operations: RefCell<Vec<GcsDeleteOperation>>,
    }

    #[allow(unused)]
    impl MockCloudStorageService {
        pub fn new() -> Self {
            Self {
                put_object_responses: RefCell::new(VecDeque::new()),
                delete_object_responses: RefCell::new(VecDeque::new()),
                captured_put_operations: RefCell::new(Vec::new()),
                captured_delete_operations: RefCell::new(Vec::new()),
            }
        }

        pub fn expect_put_object_response(&self, response: Result<(), SttError>) {
            self.put_object_responses.borrow_mut().push_back(response);
        }

        pub fn expect_delete_object_response(&self, response: Result<(), SttError>) {
            self.delete_object_responses
                .borrow_mut()
                .push_back(response);
        }

        pub fn get_captured_put_operations(&self) -> Ref<'_, Vec<GcsPutOperation>> {
            self.captured_put_operations.borrow()
        }

        pub fn get_captured_delete_operations(&self) -> Ref<'_, Vec<GcsDeleteOperation>> {
            self.captured_delete_operations.borrow()
        }

        pub fn clear_captured_operations(&self) {
            self.captured_put_operations.borrow_mut().clear();
            self.captured_delete_operations.borrow_mut().clear();
        }
    }

    impl CloudStorageService for MockCloudStorageService {
        async fn put_object(
            &self,
            request_id: &str,
            bucket: &str,
            object_name: &str,
            content: Bytes,
        ) -> Result<(), SttError> {
            self.captured_put_operations
                .borrow_mut()
                .push(GcsPutOperation {
                    request_id: request_id.to_string(),
                    bucket: bucket.to_string(),
                    object_name: object_name.to_string(),
                    content_size: content.len(),
                });

            self.put_object_responses
                .borrow_mut()
                .pop_front()
                .unwrap_or(Err((
                    request_id.to_string(),
                    golem_stt::http::Error::Generic("unexpected error".to_string()),
                )
                    .into()))
        }

        async fn delete_object(
            &self,
            request_id: &str,
            bucket: &str,
            object_name: &str,
        ) -> Result<(), SttError> {
            self.captured_delete_operations
                .borrow_mut()
                .push(GcsDeleteOperation {
                    request_id: request_id.to_string(),
                    bucket: bucket.to_string(),
                    object_name: object_name.to_string(),
                });

            self.delete_object_responses
                .borrow_mut()
                .pop_front()
                .unwrap_or(Err((
                    request_id.to_string(),
                    golem_stt::http::Error::Generic("unexpected error".to_string()),
                )
                    .into()))
        }
    }

    struct MockSpeechToTextService {
        pub recognize_responses: RefCell<VecDeque<Result<RecognizeResponse, SttError>>>,
        pub start_batch_recognize_responses:
            RefCell<VecDeque<Result<BatchRecognizeOperationResponse, SttError>>>,
        pub wait_for_completion_responses:
            RefCell<VecDeque<Result<BatchRecognizeOperationResponse, SttError>>>,
        pub captured_recognize: RefCell<Vec<RecognizeOperation>>,
        pub captured_start_batch_recognize: RefCell<Vec<StartBatchRecognizeOperation>>,
        pub captured_wait_for_completion: RefCell<Vec<WaitForCompletionOperation>>,
    }

    #[allow(unused)]
    impl MockSpeechToTextService {
        pub fn new() -> Self {
            Self {
                recognize_responses: RefCell::new(VecDeque::new()),
                start_batch_recognize_responses: RefCell::new(VecDeque::new()),
                wait_for_completion_responses: RefCell::new(VecDeque::new()),
                captured_recognize: RefCell::new(Vec::new()),
                captured_start_batch_recognize: RefCell::new(Vec::new()),
                captured_wait_for_completion: RefCell::new(Vec::new()),
            }
        }

        pub fn expect_recognize_response(&self, response: Result<RecognizeResponse, SttError>) {
            self.recognize_responses.borrow_mut().push_back(response);
        }

        pub fn expect_start_batch_recognize_response(
            &self,
            response: Result<BatchRecognizeOperationResponse, SttError>,
        ) {
            self.start_batch_recognize_responses
                .borrow_mut()
                .push_back(response);
        }

        pub fn expect_wait_for_completion_response(
            &self,
            response: Result<BatchRecognizeOperationResponse, SttError>,
        ) {
            self.wait_for_completion_responses
                .borrow_mut()
                .push_back(response);
        }

        pub fn get_captured_recognize(&self) -> Ref<'_, Vec<RecognizeOperation>> {
            self.captured_recognize.borrow()
        }

        pub fn get_captured_start_batch_recognize(
            &self,
        ) -> Ref<'_, Vec<StartBatchRecognizeOperation>> {
            self.captured_start_batch_recognize.borrow()
        }

        pub fn get_captured_wait_for_completion(&self) -> Ref<'_, Vec<WaitForCompletionOperation>> {
            self.captured_wait_for_completion.borrow()
        }

        pub fn clear_captured_operations(&self) {
            self.captured_recognize.borrow_mut().clear();
            self.captured_start_batch_recognize.borrow_mut().clear();
            self.captured_wait_for_completion.borrow_mut().clear();
        }
    }

    impl SpeechToTextService for MockSpeechToTextService {
        async fn recognize(
            &self,
            request_id: &str,
            audio_content: &[u8],
            audio_config: &AudioConfig,
            transcription_config: Option<&TranscriptionConfig>,
        ) -> Result<RecognizeResponse, SttError> {
            self.captured_recognize
                .borrow_mut()
                .push(RecognizeOperation {
                    request_id: request_id.to_string(),
                    audio_size: audio_content.len(),
                    audio_config: audio_config.clone(),
                    transcription_config: transcription_config.cloned(),
                });

            self.recognize_responses
                .borrow_mut()
                .pop_front()
                .unwrap_or(Err((
                    request_id.to_string(),
                    golem_stt::http::Error::Generic("unexpected error".to_string()),
                )
                    .into()))
        }

        async fn start_batch_recognize(
            &self,
            request_id: &str,
            audio_gcs_uris: Vec<String>,
            audio_config: &AudioConfig,
            transcription_config: Option<&TranscriptionConfig>,
        ) -> Result<BatchRecognizeOperationResponse, SttError> {
            self.captured_start_batch_recognize
                .borrow_mut()
                .push(StartBatchRecognizeOperation {
                    request_id: request_id.to_string(),
                    audio_gcs_uris,
                    audio_config: audio_config.clone(),
                    transcription_config: transcription_config.cloned(),
                });

            self.start_batch_recognize_responses
                .borrow_mut()
                .pop_front()
                .unwrap_or(Err((
                    request_id.to_string(),
                    golem_stt::http::Error::Generic("unexpected error".to_string()),
                )
                    .into()))
        }

        async fn get_batch_recognize(
            &self,
            request_id: &str,
            _operation_name: &str,
        ) -> Result<BatchRecognizeOperationResponse, SttError> {
            Err((
                request_id.to_string(),
                golem_stt::http::Error::Generic("should not be called by mock".to_string()),
            )
                .into())
        }

        async fn wait_for_batch_recognize_completion(
            &self,
            request_id: &str,
            operation_name: &str,
            max_wait_time: Duration,
        ) -> Result<BatchRecognizeOperationResponse, SttError> {
            self.captured_wait_for_completion
                .borrow_mut()
                .push(WaitForCompletionOperation {
                    request_id: request_id.to_string(),
                    operation_name: operation_name.to_string(),
                    max_wait_time,
                });

            self.wait_for_completion_responses
                .borrow_mut()
                .pop_front()
                .unwrap_or(Err((
                    request_id.to_string(),
                    golem_stt::http::Error::Generic("unexpected error".to_string()),
                )
                    .into()))
        }

        async fn delete_batch_recognize(
            &self,
            _request_id: &str,
            _operation_name: &str,
        ) -> Result<(), SttError> {
            Ok(())
        }
    }

    fn create_mock_speech_to_text_api(
    ) -> SpeechToTextApi<MockCloudStorageService, MockSpeechToTextService> {
        SpeechToTextApi {
            bucket_name: "test-bucket".to_string(),
            cloud_storage_service: MockCloudStorageService::new(),
            speech_to_text_service: MockSpeechToTextService::new(),
        }
    }

    fn create_successful_recognize_response() -> RecognizeResponse {
        RecognizeResponse {
            results: vec![SpeechRecognitionResult {
                alternatives: vec![SpeechRecognitionAlternative {
                    transcript: "Hello world from sync".to_string(),
                    confidence: Some(0.98),
                    words: vec![],
                }],
                channel_tag: None,
                result_end_offset: None,
                language_code: Some("en-US".to_string()),
            }],
            metadata: Some(RecognitionResponseMetadata {
                request_id: Some("sync-test-request".to_string()),
                total_billed_duration: None,
            }),
        }
    }

    fn create_successful_batch_response_for_request(
        request_id: &str,
        bucket_name: &str,
        audio_format: &AudioFormat,
    ) -> BatchRecognizeOperationResponse {
        use std::collections::HashMap;

        let mut results = HashMap::new();

        // Generate the GCS URI that matches what the actual implementation creates
        let extension = determine_audio_extension(audio_format);
        let object_name = format!("{request_id}/audio{extension}");
        let gcs_uri = format!("gs://{bucket_name}/{object_name}");

        let file_result = BatchRecognizeFileResult {
            error: None,
            metadata: Some(RecognitionResponseMetadata {
                request_id: Some("some-gcp-request-id".to_string()),
                total_billed_duration: None,
            }),
            inline_result: Some(InlineResult {
                transcript: Some(RecognizeResults {
                    results: vec![SpeechRecognitionResult {
                        alternatives: vec![SpeechRecognitionAlternative {
                            transcript: "Hello world".to_string(),
                            confidence: Some(0.95),
                            words: vec![],
                        }],
                        channel_tag: None,
                        result_end_offset: None,
                        language_code: Some("en-US".to_string()),
                    }],
                    metadata: None,
                }),
            }),
        };

        results.insert(gcs_uri, file_result);

        BatchRecognizeOperationResponse {
            name: "operations/test-operation".to_string(),
            metadata: None,
            done: Some(true),
            error: None,
            response: Some(BatchRecognizeResponse {
                results,
                total_billed_duration: None,
            }),
        }
    }

    #[wstd::test]
    async fn test_transcribe_audio_invalid_request_id_returns_error() {
        let api = create_mock_speech_to_text_api();

        let request = TranscriptionRequest {
            request_id: "invalid request id".to_string(), // spaces are invalid
            audio: "test audio".into(),
            audio_config: AudioConfig {
                format: AudioFormat::Wav,
                sample_rate_hertz: Some(16000),
                channels: Some(1),
            },
            transcription_config: None,
        };

        let result = api.transcribe_audio(request).await;
        assert!(result.is_err());

        let expected_error = SttError::APIBadRequest {
                request_id: "invalid request id".to_string(),
                provider_error: "Invalid request ID: Request ID contains invalid characters. Only alphanumeric characters, hyphens (-), underscores (_), and dots (.) are allowed for GCS object naming".to_string(),
            };
        assert_eq!(
            format!("{:?}", result.unwrap_err()),
            format!("{:?}", expected_error)
        );
    }

    #[wstd::test]
    async fn test_transcribe_audio_uploads_to_gcs() {
        let api = create_mock_speech_to_text_api();

        api.cloud_storage_service.expect_put_object_response(Ok(()));
        api.speech_to_text_service
            .expect_start_batch_recognize_response(Ok(BatchRecognizeOperationResponse {
                name: "operations/test-operation".to_string(),
                metadata: None,
                done: Some(false),
                error: None,
                response: None,
            }));

        let expected_response = create_successful_batch_response_for_request(
            "test-123",
            "test-bucket",
            &AudioFormat::Mp3,
        );

        api.speech_to_text_service
            .expect_wait_for_completion_response(Ok(expected_response));
        api.cloud_storage_service
            .expect_delete_object_response(Ok(()));

        let request = TranscriptionRequest {
            request_id: "test-123".to_string(),
            audio: "test audio data".into(),
            audio_config: AudioConfig {
                format: AudioFormat::Mp3,
                sample_rate_hertz: Some(16000),
                channels: Some(1),
            },
            transcription_config: None,
        };

        let _ = api.transcribe_audio(request).await.unwrap();

        let captured_puts = api.cloud_storage_service.get_captured_put_operations();
        assert_eq!(captured_puts.len(), 1);

        let expected_put_op = GcsPutOperation {
            request_id: "test-123".to_string(),
            bucket: "test-bucket".to_string(),
            object_name: "test-123/audio.mp3".to_string(),
            content_size: 15,
        };
        assert_eq!(captured_puts[0], expected_put_op);
    }

    #[wstd::test]
    async fn test_transcribe_audio_starts_batch_recognize_job() {
        let api = create_mock_speech_to_text_api();

        api.cloud_storage_service.expect_put_object_response(Ok(()));
        api.speech_to_text_service
            .expect_start_batch_recognize_response(Ok(BatchRecognizeOperationResponse {
                name: "operations/test-operation".to_string(),
                metadata: None,
                done: None,
                error: None,
                response: None,
            }));
        api.speech_to_text_service
            .expect_wait_for_completion_response(Ok(create_successful_batch_response_for_request(
                "test-456",
                "test-bucket",
                &AudioFormat::Wav,
            )));
        api.cloud_storage_service
            .expect_delete_object_response(Ok(()));

        let transcription_config = TranscriptionConfig {
            language_codes: Some(vec!["en-US".to_string()]),
            model: Some("latest_long".to_string()),
            enable_profanity_filter: true,
            diarization: Some(DiarizationConfig {
                enabled: true,
                min_speaker_count: Some(2),
                max_speaker_count: Some(5),
            }),
            enable_multi_channel: true,
            phrases: vec![Phrase {
                value: "Google Cloud".to_string(),
                boost: Some(10.0),
            }],
        };

        let request = TranscriptionRequest {
            request_id: "test-456".to_string(),
            audio: "test audio data".into(),
            audio_config: AudioConfig {
                format: AudioFormat::Wav,
                sample_rate_hertz: Some(44100),
                channels: Some(2),
            },
            transcription_config: Some(transcription_config.clone()),
        };

        let _ = api.transcribe_audio(request).await.unwrap();

        let captured_starts = api
            .speech_to_text_service
            .get_captured_start_batch_recognize();
        assert_eq!(captured_starts.len(), 1);

        let expected_start_op = StartBatchRecognizeOperation {
            request_id: "test-456".to_string(),
            audio_gcs_uris: vec!["gs://test-bucket/test-456/audio.wav".to_string()],
            audio_config: AudioConfig {
                format: AudioFormat::Wav,
                sample_rate_hertz: Some(44100),
                channels: Some(2),
            },
            transcription_config: Some(transcription_config),
        };
        assert_eq!(captured_starts[0], expected_start_op);
    }

    #[wstd::test]
    async fn test_transcribe_audio_uses_synchronous_transcription_for_short_model() {
        let api = create_mock_speech_to_text_api();

        let expected_recognize_response = create_successful_recognize_response();
        api.speech_to_text_service
            .expect_recognize_response(Ok(expected_recognize_response.clone()));

        // Create a small audio file (< 10MB) with "short" model
        let small_audio = Bytes::from(vec![0u8; 1024]); // 1KB audio file
        let request = TranscriptionRequest {
            request_id: "sync-test".to_string(),
            audio: small_audio,
            audio_config: AudioConfig {
                format: AudioFormat::Wav,
                sample_rate_hertz: Some(16000),
                channels: Some(1),
            },
            transcription_config: Some(TranscriptionConfig {
                language_codes: Some(vec!["en-US".to_string()]),
                model: Some("short".to_string()),
                enable_profanity_filter: false,
                diarization: None,
                enable_multi_channel: false,
                phrases: vec![],
            }),
        };

        let result = api.transcribe_audio(request).await.unwrap();

        let captured_recognize = api.speech_to_text_service.get_captured_recognize();
        assert_eq!(captured_recognize.len(), 1);

        let expected_recognize_op = RecognizeOperation {
            request_id: "sync-test".to_string(),
            audio_size: 1024,
            audio_config: AudioConfig {
                format: AudioFormat::Wav,
                sample_rate_hertz: Some(16000),
                channels: Some(1),
            },
            transcription_config: Some(TranscriptionConfig {
                language_codes: Some(vec!["en-US".to_string()]),
                model: Some("short".to_string()),
                enable_profanity_filter: false,
                diarization: None,
                enable_multi_channel: false,
                phrases: vec![],
            }),
        };
        assert_eq!(captured_recognize[0], expected_recognize_op);

        let captured_batch = api
            .speech_to_text_service
            .get_captured_start_batch_recognize();
        assert_eq!(captured_batch.len(), 0);

        let captured_puts = api.cloud_storage_service.get_captured_put_operations();
        let captured_deletes = api.cloud_storage_service.get_captured_delete_operations();
        assert_eq!(captured_puts.len(), 0);
        assert_eq!(captured_deletes.len(), 0);

        // Verify the response is correct
        let expected_response = TranscriptionResponse {
            request_id: "sync-test".to_string(),
            audio_size_bytes: 1024,
            language: "en-US".to_string(),
            model: Some("short".to_string()),
            gcp_transcription: RecognizeResults {
                results: expected_recognize_response.results,
                metadata: expected_recognize_response.metadata,
            },
        };
        assert_eq!(result, expected_response);
    }

    #[wstd::test]
    async fn test_transcribe_audio_returns_response_with_transcription() {
        let api = create_mock_speech_to_text_api();

        api.cloud_storage_service.expect_put_object_response(Ok(()));
        api.speech_to_text_service
            .expect_start_batch_recognize_response(Ok(BatchRecognizeOperationResponse {
                name: "operations/test-operation".to_string(),
                metadata: None,
                done: Some(false),
                error: None,
                response: None,
            }));

        let expected_gcp_response = create_successful_batch_response_for_request(
            "test-789",
            "test-bucket",
            &AudioFormat::Flac,
        );
        api.speech_to_text_service
            .expect_wait_for_completion_response(Ok(expected_gcp_response.clone()));
        api.cloud_storage_service
            .expect_delete_object_response(Ok(()));

        let request = TranscriptionRequest {
            request_id: "test-789".to_string(),
            audio: "audio content".into(),
            audio_config: AudioConfig {
                format: AudioFormat::Flac,
                sample_rate_hertz: None,
                channels: None,
            },
            transcription_config: Some(TranscriptionConfig {
                language_codes: Some(vec!["fr-FR".to_string()]),
                model: Some("long".to_string()),
                enable_profanity_filter: false,
                diarization: None,
                enable_multi_channel: false,
                phrases: vec![],
            }),
        };

        let result = api.transcribe_audio(request).await.unwrap();

        let expected_gcp_transcription = expected_gcp_response
            .response
            .unwrap()
            .results
            .into_iter()
            .next()
            .unwrap()
            .1
            .inline_result
            .unwrap()
            .transcript
            .unwrap();

        let expected_response = TranscriptionResponse {
            request_id: "test-789".to_string(),
            audio_size_bytes: 13,
            language: "fr-FR".to_string(),
            model: Some("long".to_string()),
            gcp_transcription: expected_gcp_transcription,
        };
        assert_eq!(result, expected_response);
    }

    #[wstd::test]
    async fn test_transcribe_audio_cleans_up_gcs_object() {
        let api = create_mock_speech_to_text_api();

        api.cloud_storage_service.expect_put_object_response(Ok(()));
        api.speech_to_text_service
            .expect_start_batch_recognize_response(Ok(BatchRecognizeOperationResponse {
                name: "operations/test-operation".to_string(),
                metadata: None,
                done: Some(false),
                error: None,
                response: None,
            }));
        api.speech_to_text_service
            .expect_wait_for_completion_response(Ok(create_successful_batch_response_for_request(
                "cleanup-test",
                "test-bucket",
                &AudioFormat::Mp4,
            )));
        api.cloud_storage_service
            .expect_delete_object_response(Ok(()));

        let request = TranscriptionRequest {
            request_id: "cleanup-test".to_string(),
            audio: "test audio".into(),
            audio_config: AudioConfig {
                format: AudioFormat::Mp4,
                sample_rate_hertz: Some(16000),
                channels: Some(1),
            },
            transcription_config: None,
        };

        let _ = api.transcribe_audio(request).await.unwrap();

        let captured_deletes = api.cloud_storage_service.get_captured_delete_operations();
        assert_eq!(captured_deletes.len(), 1);

        let expected_delete_op = GcsDeleteOperation {
            request_id: "cleanup-test".to_string(),
            bucket: "test-bucket".to_string(),
            object_name: "cleanup-test/audio.mp4".to_string(),
        };
        assert_eq!(captured_deletes[0], expected_delete_op);
    }

    #[wstd::test]
    async fn test_transcribe_audio_gcs_upload_failure() {
        let api = create_mock_speech_to_text_api();

        api.cloud_storage_service.expect_put_object_response(Err(
            SttError::APIInternalServerError {
                request_id: "upload-fail".to_string(),
                provider_error: "GCS upload failed".to_string(),
            },
        ));

        let request = TranscriptionRequest {
            request_id: "upload-fail".to_string(),
            audio: "test audio".into(),
            audio_config: AudioConfig {
                format: AudioFormat::Wav,
                sample_rate_hertz: Some(16000),
                channels: Some(1),
            },
            transcription_config: None,
        };

        let result = api.transcribe_audio(request).await;
        assert!(result.is_err());

        let expected_error = SttError::APIInternalServerError {
            request_id: "upload-fail".to_string(),
            provider_error: "GCS upload failed".to_string(),
        };
        assert_eq!(
            format!("{:?}", result.unwrap_err()),
            format!("{:?}", expected_error)
        );
    }

    #[wstd::test]
    async fn test_transcribe_audio_batch_recognize_failure() {
        let api = create_mock_speech_to_text_api();

        api.cloud_storage_service.expect_put_object_response(Ok(()));
        api.speech_to_text_service
            .expect_start_batch_recognize_response(Err(SttError::APIRateLimit {
                request_id: "batch-fail".to_string(),
                provider_error: "Speech-to-Text API rate limit exceeded".to_string(),
            }));
        api.cloud_storage_service
            .expect_delete_object_response(Ok(()));

        let request = TranscriptionRequest {
            request_id: "batch-fail".to_string(),
            audio: "test audio".into(),
            audio_config: AudioConfig {
                format: AudioFormat::Wav,
                sample_rate_hertz: Some(16000),
                channels: Some(1),
            },
            transcription_config: None,
        };

        let result = api.transcribe_audio(request).await;
        assert!(result.is_err());

        let expected_error = SttError::APIRateLimit {
            request_id: "batch-fail".to_string(),
            provider_error: "Speech-to-Text API rate limit exceeded".to_string(),
        };
        assert_eq!(
            format!("{:?}", result.unwrap_err()),
            format!("{:?}", expected_error)
        );

        let captured_deletes = api.cloud_storage_service.get_captured_delete_operations();
        assert_eq!(captured_deletes.len(), 1);

        let expected_delete_op = GcsDeleteOperation {
            request_id: "batch-fail".to_string(),
            bucket: "test-bucket".to_string(),
            object_name: "batch-fail/audio.wav".to_string(),
        };
        assert_eq!(captured_deletes[0], expected_delete_op);
    }

    #[wstd::test]
    async fn test_transcribe_audio_wait_for_completion_failure() {
        let api = create_mock_speech_to_text_api();

        api.cloud_storage_service.expect_put_object_response(Ok(()));
        api.speech_to_text_service
            .expect_start_batch_recognize_response(Ok(BatchRecognizeOperationResponse {
                name: "operations/test-operation".to_string(),
                metadata: None,
                done: None,
                error: None,
                response: None,
            }));
        api.speech_to_text_service
            .expect_wait_for_completion_response(Err(SttError::APIInternalServerError {
                request_id: "timeout-test".to_string(),
                provider_error: "Transcription timeout".to_string(),
            }));
        api.cloud_storage_service
            .expect_delete_object_response(Ok(()));

        let request = TranscriptionRequest {
            request_id: "timeout-test".to_string(),
            audio: "test audio".into(),
            audio_config: AudioConfig {
                format: AudioFormat::Wav,
                sample_rate_hertz: Some(16000),
                channels: Some(1),
            },
            transcription_config: None,
        };

        let result = api.transcribe_audio(request).await;
        assert!(result.is_err());

        let expected_error = SttError::APIInternalServerError {
            request_id: "timeout-test".to_string(),
            provider_error: "Transcription timeout".to_string(),
        };

        assert_eq!(
            format!("{:?}", result.unwrap_err()),
            format!("{:?}", expected_error)
        );

        // Verify cleanup still happened
        let captured_deletes = api.cloud_storage_service.get_captured_delete_operations();
        assert_eq!(captured_deletes.len(), 1);

        let expected_delete_op = GcsDeleteOperation {
            request_id: "timeout-test".to_string(),
            bucket: "test-bucket".to_string(),
            object_name: "timeout-test/audio.wav".to_string(),
        };

        assert_eq!(captured_deletes[0], expected_delete_op);
    }
}
