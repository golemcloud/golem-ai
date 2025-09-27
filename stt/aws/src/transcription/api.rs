use std::time::Duration;

use bytes::Bytes;
use golem_stt::{error::Error, languages::Language, transcription::SttProviderClient};

use log::trace;

use crate::transcription::{
    aws_transcribe::TranscriptionJob,
    request::{AudioConfig, TranscriptionConfig},
};

use super::{
    aws_s3::S3Service,
    aws_transcribe::{TranscribeOutput, TranscribeService},
    request::TranscriptionRequest,
};

const AWS_TRANSCRIBE_SUPPORTED_LANGUAGES: [Language; 104] = [
    Language::new("ab-GE", "Abkhaz", "აფხაზური"),
    Language::new("af-ZA", "Afrikaans", "Afrikaans"),
    Language::new("ar-AE", "Arabic, Gulf", "العربية الخليجية"),
    Language::new("ar-SA", "Arabic, Modern Standard", "العربية الفصحى"),
    Language::new("hy-AM", "Armenian", "հայերեն"),
    Language::new("ast-ES", "Asturian", "asturianu"),
    Language::new("az-AZ", "Azerbaijani", "azərbaycan dili"),
    Language::new("ba-RU", "Bashkir", "башҡорт теле"),
    Language::new("eu-ES", "Basque", "euskera"),
    Language::new("be-BY", "Belarusian", "беларуская"),
    Language::new("bn-IN", "Bengali", "বাংলা"),
    Language::new("bs-BA", "Bosnian", "bosanski"),
    Language::new("bg-BG", "Bulgarian", "български"),
    Language::new("ca-ES", "Catalan", "català"),
    Language::new("ckb-IR", "Central Kurdish, Iran", "کوردیی ناوەندی"),
    Language::new("ckb-IQ", "Central Kurdish, Iraq", "کوردیی ناوەندی"),
    Language::new("zh-HK", "Chinese, Cantonese", "廣東話"),
    Language::new("zh-CN", "Chinese, Simplified", "中文（简体）"),
    Language::new("zh-TW", "Chinese, Traditional", "中文（繁體）"),
    Language::new("hr-HR", "Croatian", "hrvatski"),
    Language::new("cs-CZ", "Czech", "čeština"),
    Language::new("da-DK", "Danish", "dansk"),
    Language::new("nl-NL", "Dutch", "Nederlands"),
    Language::new("en-AU", "English, Australian", "English (Australia)"),
    Language::new("en-GB", "English, British", "English (United Kingdom)"),
    Language::new("en-IN", "English, Indian", "English (India)"),
    Language::new("en-IE", "English, Irish", "English (Ireland)"),
    Language::new("en-NZ", "English, New Zealand", "English (New Zealand)"),
    Language::new("en-AB", "English, Scottish", "English (Scotland)"),
    Language::new("en-ZA", "English, South African", "English (South Africa)"),
    Language::new("en-US", "English, US", "English (United States)"),
    Language::new("en-WL", "English, Welsh", "English (Wales)"),
    Language::new("et-EE", "Estonian", "eesti"),
    Language::new("et-ET", "Estonian", "eesti"),
    Language::new("fa-IR", "Farsi", "فارسی"),
    Language::new("fi-FI", "Finnish", "suomi"),
    Language::new("fr-FR", "French", "français"),
    Language::new("fr-CA", "French, Canadian", "français (Canada)"),
    Language::new("gl-ES", "Galician", "galego"),
    Language::new("ka-GE", "Georgian", "ქართული"),
    Language::new("de-DE", "German", "Deutsch"),
    Language::new("de-CH", "German, Swiss", "Deutsch (Schweiz)"),
    Language::new("el-GR", "Greek", "ελληνικά"),
    Language::new("gu-IN", "Gujarati", "ગુજરાતી"),
    Language::new("ha-NG", "Hausa", "Hausa"),
    Language::new("he-IL", "Hebrew", "עברית"),
    Language::new("hi-IN", "Hindi, Indian", "हिन्दी"),
    Language::new("hu-HU", "Hungarian", "magyar"),
    Language::new("is-IS", "Icelandic", "íslenska"),
    Language::new("id-ID", "Indonesian", "Bahasa Indonesia"),
    Language::new("it-IT", "Italian", "italiano"),
    Language::new("ja-JP", "Japanese", "日本語"),
    Language::new("kab-DZ", "Kabyle", "Taqbaylit"),
    Language::new("kn-IN", "Kannada", "ಕನ್ನಡ"),
    Language::new("kk-KZ", "Kazakh", "қазақ тілі"),
    Language::new("rw-RW", "Kinyarwanda", "Ikinyarwanda"),
    Language::new("ko-KR", "Korean", "한국어"),
    Language::new("ky-KG", "Kyrgyz", "кыргызча"),
    Language::new("lv-LV", "Latvian", "latviešu"),
    Language::new("lt-LT", "Lithuanian", "lietuvių"),
    Language::new("lg-IN", "Luganda", "Luganda"),
    Language::new("mk-MK", "Macedonian", "македонски"),
    Language::new("ms-MY", "Malay", "Bahasa Melayu"),
    Language::new("ml-IN", "Malayalam", "മലയാളം"),
    Language::new("mt-MT", "Maltese", "Malti"),
    Language::new("mr-IN", "Marathi", "मराठी"),
    Language::new("mhr-RU", "Meadow Mari", "олык марий"),
    Language::new("mn-MN", "Mongolian", "монгол"),
    Language::new("no-NO", "Norwegian Bokmål", "norsk"),
    Language::new("or-IN", "Odia/Oriya", "ଓଡ଼ିଆ"),
    Language::new("ps-AF", "Pashto", "پښتو"),
    Language::new("pl-PL", "Polish", "polski"),
    Language::new("pt-PT", "Portuguese", "português"),
    Language::new("pt-BR", "Portuguese, Brazilian", "português (Brasil)"),
    Language::new("pa-IN", "Punjabi", "ਪੰਜਾਬੀ"),
    Language::new("ro-RO", "Romanian", "română"),
    Language::new("ru-RU", "Russian", "русский"),
    Language::new("sr-RS", "Serbian", "српски"),
    Language::new("si-LK", "Sinhala", "සිංහල"),
    Language::new("sk-SK", "Slovak", "slovenčina"),
    Language::new("sl-SI", "Slovenian", "slovenščina"),
    Language::new("so-SO", "Somali", "Soomaali"),
    Language::new("es-ES", "Spanish", "español"),
    Language::new("es-US", "Spanish, US", "español (Estados Unidos)"),
    Language::new("su-ID", "Sundanese", "basa Sunda"),
    Language::new("sw-KE", "Swahili, Kenya", "Kiswahili (Kenya)"),
    Language::new("sw-BI", "Swahili, Burundi", "Kiswahili (Burundi)"),
    Language::new("sw-RW", "Swahili, Rwanda", "Kiswahili (Rwanda)"),
    Language::new("sw-TZ", "Swahili, Tanzania", "Kiswahili (Tanzania)"),
    Language::new("sw-UG", "Swahili, Uganda", "Kiswahili (Uganda)"),
    Language::new("sv-SE", "Swedish", "svenska"),
    Language::new("tl-PH", "Tagalog/Filipino", "Tagalog"),
    Language::new("ta-IN", "Tamil", "தமிழ்"),
    Language::new("tt-RU", "Tatar", "татарча"),
    Language::new("te-IN", "Telugu", "తెలుగు"),
    Language::new("th-TH", "Thai", "ไทย"),
    Language::new("tr-TR", "Turkish", "Türkçe"),
    Language::new("uk-UA", "Ukrainian", "українська"),
    Language::new("ug-CN", "Uyghur", "ئۇيغۇرچە"),
    Language::new("uz-UZ", "Uzbek", "oʻzbekcha"),
    Language::new("vi-VN", "Vietnamese", "Tiếng Việt"),
    Language::new("cy-WL", "Welsh", "Cymraeg"),
    Language::new("wo-SN", "Wolof", "Wolof"),
    Language::new("zu-ZA", "Zulu", "isiZulu"),
];

pub fn is_supported_language(language_code: &str) -> bool {
    AWS_TRANSCRIBE_SUPPORTED_LANGUAGES
        .iter()
        .any(|lang| lang.code == language_code)
}

pub fn get_supported_languages() -> &'static [Language] {
    &AWS_TRANSCRIBE_SUPPORTED_LANGUAGES
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

pub struct TranscribeApi<S3: S3Service, TC: TranscribeService> {
    bucket_name: String,
    s3_client: S3,
    transcribe_client: TC,
}

#[allow(unused)]
impl<S3: S3Service, TC: TranscribeService> TranscribeApi<S3, TC> {
    pub fn new(bucket_name: String, s3_client: S3, transcribe_client: TC) -> Self {
        Self {
            bucket_name,
            s3_client,
            transcribe_client,
        }
    }

    async fn upload_audio_to_s3(
        &self,
        request_id: &str,
        object_key: &str,
        audio_bytes: Bytes,
    ) -> Result<(), Error> {
        self.s3_client
            .put_object(request_id, &self.bucket_name, object_key, audio_bytes)
            .await?;

        Ok(())
    }

    async fn create_vocabulary<'a>(
        &self,
        request_id: &'a str,
        language_code: &str,
        vocabulary: Vec<String>,
    ) -> Result<&'a str, Error> {
        let res = self
            .transcribe_client
            .create_vocabulary(
                request_id.to_string(),
                language_code.to_string(),
                vocabulary,
            )
            .await?;

        if res.vocabulary_state == "FAILED" {
            return Err(Error::APIBadRequest {
                request_id: request_id.to_string(),
                provider_error: format!(
                    "Vocabulary creation failed: {}",
                    res.failure_reason
                        .unwrap_or_else(|| "Unknown error".to_string())
                ),
            });
        }

        if res.vocabulary_state == "PENDING" {
            self.transcribe_client
                .wait_for_vocabulary_ready(request_id, Duration::from_secs(300))
                .await?;
        }

        Ok(request_id)
    }

    async fn run_transcription_job(
        &self,
        request_id: &str,
        object_key: &str,
        audio_config: &AudioConfig,
        transcription_config: Option<&TranscriptionConfig>,
        vocabulary_name: Option<&str>,
    ) -> Result<TranscriptionJob, Error> {
        let bucket = &self.bucket_name;
        let res = self
            .transcribe_client
            .start_transcription_job(
                request_id,
                &format!("s3://{}/{object_key}", &bucket),
                audio_config,
                transcription_config,
                vocabulary_name,
            )
            .await?;

        if res.transcription_job.transcription_job_status == "FAILED" {
            log::error!("transcription job {request_id} failed");
            return Err(Error::APIBadRequest {
                request_id: request_id.to_string(),
                provider_error: format!(
                    "Transcription job creation failed: {}",
                    res.transcription_job
                        .failure_reason
                        .unwrap_or_else(|| "Unknown error".to_string())
                ),
            });
        }

        trace!("waiting for {request_id} to complete");
        let completed_transcription_job = if res.transcription_job.transcription_job_status
            != "COMPLETED"
        {
            self.transcribe_client
                .wait_for_transcription_job_completion(request_id, Duration::from_secs(3600 * 6))
                .await?
                .transcription_job
        } else {
            res.transcription_job
        };

        Ok(completed_transcription_job)
    }

    async fn download_transcription(
        &self,
        request_id: &str,
        completed_transcription_job: &TranscriptionJob,
    ) -> Result<TranscribeOutput, Error> {
        trace!("retrieveing transcription job {request_id} result",);
        if let Some(ref transcript) = completed_transcription_job.transcript {
            if let Some(ref transcript_uri) = transcript.transcript_file_uri {
                let transcribe_output = self
                    .transcribe_client
                    .download_transcript_json(request_id.as_ref(), transcript_uri)
                    .await?;

                Ok(transcribe_output)
            } else {
                Err(golem_stt::error::Error::APIUnknown {
                    request_id: request_id.to_string(),
                    provider_error: "Transcription completed but no transcript file URI found"
                        .to_string(),
                })
            }
        } else {
            Err(golem_stt::error::Error::APIUnknown {
                request_id: request_id.to_string(),
                provider_error: "Transcription completed but no transcript found".to_string(),
            })
        }
    }
}

impl<S3: S3Service, TC: TranscribeService>
    SttProviderClient<TranscriptionRequest, TranscriptionResponse, Error>
    for TranscribeApi<S3, TC>
{
    async fn transcribe_audio(
        &self,
        request: TranscriptionRequest,
    ) -> Result<TranscriptionResponse, Error> {
        trace!("Sending request to AWS Transcribe API: {request:?}");

        let request_id = request.request_id;

        let audio_size_bytes = request.audio.len();
        let req_language = request
            .transcription_config
            .as_ref()
            .and_then(|config| config.language.clone());
        let model = request
            .transcription_config
            .as_ref()
            .and_then(|config| config.model.clone());

        let vocabulary_size = request
            .transcription_config
            .as_ref()
            .map_or_else(|| 0, |config| config.vocabulary.len());

        let object_key = format!("{request_id}/audio.{}", &request.audio_config.format);

        validate_request_id(&request_id).map_err(|validation_error| Error::APIBadRequest {
            request_id: request_id.clone(),
            provider_error: format!("Invalid request ID: {validation_error}"),
        })?;

        let maybe_existing_job = self
            .transcribe_client
            .get_transcription_job(&request_id)
            .await
            .ok();

        let completed_transcription_job: TranscriptionJob = match maybe_existing_job {
            Some(job) if job.transcription_job.transcription_job_status == "COMPLETED" => {
                job.transcription_job
            }
            Some(job)
                if job.transcription_job.transcription_job_status == "IN_PROGRESS"
                    || job.transcription_job.transcription_job_status == "QUEUED" =>
            {
                self.transcribe_client
                    .wait_for_transcription_job_completion(
                        &request_id,
                        Duration::from_secs(3600 * 6),
                    )
                    .await?
                    .transcription_job
            }
            _ => {
                // allow to retry failed jobs
                if maybe_existing_job
                    .is_some_and(|job| job.transcription_job.transcription_job_status == "FAILED")
                {
                    // clear all the state if there is any
                    match self.transcribe_client.delete_vocabulary(&request_id).await {
                        Ok(_) => (),
                        Err(e) => {
                            log::warn!("Failed to delete vocabulary: {e}");
                        }
                    }

                    match self
                        .s3_client
                        .delete_object(&request_id, &self.bucket_name, &object_key)
                        .await
                    {
                        Ok(_) => (),
                        Err(e) => {
                            log::warn!("Failed to delete object: {e}");
                        }
                    }

                    self.transcribe_client
                        .delete_transcription_job(&request_id)
                        .await?;
                }

                if let Some(ref config) = request.transcription_config {
                    if !config.vocabulary.is_empty() && config.language.is_none() {
                        return Err(Error::APIBadRequest {
                                    request_id: request_id.to_string(),
                                    provider_error: "Vocabulary can only be used when a specific language is provided. Cannot be used with automatic language detection.".to_string(),
                                });
                    }

                    if config.model.is_some() && config.language.is_none() {
                        return Err(Error::APIBadRequest {
                                    request_id: request_id.to_string(),
                                    provider_error: "Model settings can only be used when a specific language is provided. Cannot be used with automatic language detection.".to_string(),
                                });
                    }
                }

                self.upload_audio_to_s3(&request_id, &object_key, request.audio)
                    .await?;

                let vocabulary_name = if let Some(ref config) = request.transcription_config {
                    if !config.vocabulary.is_empty() {
                        let language_code = config.language.as_ref().unwrap(); // Safe due to validation above

                        let vocabulary_name = self
                            .create_vocabulary(
                                &request_id,
                                language_code,
                                config.vocabulary.clone(),
                            )
                            .await;

                        match vocabulary_name {
                            Ok(name) => Some(name),
                            Err(err) => {
                                match self
                                    .s3_client
                                    .delete_object(&request_id, &self.bucket_name, &object_key)
                                    .await
                                {
                                    Ok(_) => (),
                                    Err(e) => {
                                        log::error!("Failed to delete object: {e}");
                                    }
                                }
                                return Err(err);
                            }
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };

                let completed_transcription_job = self
                    .run_transcription_job(
                        &request_id,
                        &object_key,
                        &request.audio_config,
                        request.transcription_config.as_ref(),
                        vocabulary_name,
                    )
                    .await;

                if vocabulary_size > 0 {
                    match self.transcribe_client.delete_vocabulary(&request_id).await {
                        Ok(_) => (),
                        Err(e) => {
                            log::error!("Failed to delete vocabulary: {e}");
                        }
                    }
                }

                match self
                    .s3_client
                    .delete_object(&request_id, &self.bucket_name, &object_key)
                    .await
                {
                    Ok(_) => (),
                    Err(e) => {
                        log::error!("Failed to delete object: {e}");
                    }
                }

                completed_transcription_job?
            }
        };

        let aws_transcription = self
            .download_transcription(&request_id, &completed_transcription_job)
            .await?;

        Ok(TranscriptionResponse {
            request_id: request_id.to_string(),
            audio_size_bytes,
            language: req_language.unwrap_or_default(),
            model,
            aws_transcription,
        })
    }
}

#[allow(unused)]
#[derive(Debug, PartialEq)]
pub struct TranscriptionResponse {
    pub request_id: String,
    pub audio_size_bytes: usize,
    pub language: String,
    pub model: Option<String>,
    pub aws_transcription: TranscribeOutput,
}

fn validate_request_id(request_id: &str) -> Result<(), String> {
    if request_id.is_empty() {
        return Err("Request ID cannot be empty".to_string());
    }

    // Check length - Transcribe support up to 200 characters for our use case
    if request_id.len() > 200 {
        return Err(
            "Request ID too long (max 200 characters for S3 and Transcribe compatibility)"
                .to_string(),
        );
    }

    // https://docs.aws.amazon.com/transcribe/latest/APIReference/API_CreateVocabulary.html#transcribe-CreateVocabulary-request-VocabularyName
    // AWS Transcribe vocabulary name pattern: ^[0-9a-zA-Z._-]+$ which is also S3-safe
    let is_valid_char = |c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.';

    if !request_id.chars().all(is_valid_char) {
        return Err(
                "Request ID contains invalid characters. Only alphanumeric characters, hyphens (-), underscores (_), and dots (.) are allowed for S3 and Transcribe compatibility".to_string()
            );
    }

    if request_id.to_lowercase().starts_with("aws-") {
        return Err(
            "Request ID cannot start with 'aws-' (reserved prefix for AWS services)".to_string(),
        );
    }

    // Ensure it starts with an alphanumeric character (good practice for both S3 and Transcribe)
    if !request_id.chars().next().unwrap().is_ascii_alphanumeric() {
        return Err("Request ID must start with an alphanumeric character".to_string());
    }

    if request_id.ends_with('-') || request_id.ends_with('_') || request_id.ends_with('.') {
        return Err("Request ID cannot end with hyphens, underscores, or dots".to_string());
    }

    let problematic_patterns = ["--", "__", "..", "-_", "_-", "-.", "._", "_.", ".-"];
    for pattern in &problematic_patterns {
        if request_id.contains(pattern) {
            return Err("Request ID cannot contain consecutive special characters".to_string());
        }
    }

    if request_id.starts_with('.') {
        return Err(
            "Request ID cannot start with a dot (reserved for file extensions)".to_string(),
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        cell::{Ref, RefCell},
        collections::VecDeque,
    };

    use crate::transcription::{
        aws_transcribe::{
            CreateVocabularyResponse, GetTranscriptionJobResponse, GetVocabularyResponse,
            StartTranscriptionJobResponse, TranscribeResults, Transcript, TranscriptText,
            TranscriptionJob,
        },
        request::{AudioConfig, AudioFormat, DiarizationConfig, TranscriptionConfig},
    };

    use super::*;

    #[test]
    fn test_validate_request_id_valid_cases() {
        // Valid request IDs for both S3 and Transcribe
        let valid_ids = vec![
            "abc123",
            "request-123",
            "my_request_456",
            "test-request_789",
            "a1b2c3",
            "RequestID123",
            "user123-session456",
            "batch_job_001",
            // Valid cases with dots (Transcribe allows these)
            "request.123",
            "user.session.456",
            "v1.2.3",
            "api.request.789",
            "test-1.0_beta",
            "service.endpoint.call",
            "user123.session-456_temp",
            "namespace.resource.id",
            // Edge cases
            "a",                // single character
            "1",                // single digit
            "A1",               // mixed case
            "test123_final.v1", // complex but valid
        ];

        for id in valid_ids {
            assert!(
                validate_request_id(id).is_ok(),
                "Expected '{}' to be valid for both S3 and Transcribe, but validation failed: {:?}",
                id,
                validate_request_id(id)
            );
        }
    }

    #[test]
    fn test_validate_request_id_invalid_cases() {
        let long_id = "a".repeat(201);

        let test_cases = vec![
            // Empty string
            ("", "Request ID cannot be empty"),
            // Too long
            (&long_id, "Request ID too long"),
            // Invalid characters (not in ^[0-9a-zA-Z._-]+$ pattern)
            ("request id", "Request ID contains invalid characters"), // space
            ("request@123", "Request ID contains invalid characters"), // @
            ("request/123", "Request ID contains invalid characters"), // slash
            ("request#123", "Request ID contains invalid characters"), // hash
            ("request+123", "Request ID contains invalid characters"), // plus
            ("request%123", "Request ID contains invalid characters"), // percent
            ("request&123", "Request ID contains invalid characters"), // ampersand
            ("request*123", "Request ID contains invalid characters"), // asterisk
            ("request(123", "Request ID contains invalid characters"), // parentheses
            ("request)123", "Request ID contains invalid characters"), // parentheses
            ("request[123", "Request ID contains invalid characters"), // brackets
            ("request]123", "Request ID contains invalid characters"), // brackets
            ("request{123", "Request ID contains invalid characters"), // braces
            ("request}123", "Request ID contains invalid characters"), // braces
            ("request$123", "Request ID contains invalid characters"), // dollar
            ("request!123", "Request ID contains invalid characters"), // exclamation
            // AWS reserved prefix (case insensitive)
            ("aws-service", "Request ID cannot start with 'aws-'"),
            ("AWS-Service", "Request ID cannot start with 'aws-'"),
            ("Aws-test", "Request ID cannot start with 'aws-'"),
            ("awS-test", "Request ID cannot start with 'aws-'"),
            // Starting with non-alphanumeric
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
            ("!request", "Request ID contains invalid characters."),
            // Ending with special characters
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
            // Consecutive special characters
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

        for (id, expected_error_substring) in test_cases {
            let result = validate_request_id(id);
            assert!(
                result.is_err(),
                "Expected '{id}' to be invalid, but validation passed",
            );

            let error_msg = result.unwrap_err();
            assert!(
                error_msg.contains(expected_error_substring),
                "Expected error for '{id}' to contain '{expected_error_substring}', but got: '{error_msg}'",
            );
        }
    }

    #[derive(Debug, Clone)]
    struct S3PutOperation {
        request_id: String,
        bucket: String,
        object_name: String,
        content_size: usize,
    }

    #[derive(Debug, Clone)]
    struct S3DeleteOperation {
        request_id: String,
        bucket: String,
        object_name: String,
    }

    #[derive(Debug, Clone)]
    struct CreateVocabularyOperation {
        vocabulary_name: String,
        language_code: String,
        phrases: Vec<String>,
    }

    #[derive(Debug, Clone)]
    struct StartTranscriptionOperation {
        job_name: String,
        media_uri: String,
        audio_config: AudioConfig,
        transcription_config: Option<TranscriptionConfig>,
        vocabulary_name: Option<String>,
    }

    #[derive(Debug, Clone)]
    struct DownloadTranscriptOperation {
        job_name: String,
        transcript_uri: String,
    }

    struct MockS3Client {
        pub put_object_responses: RefCell<VecDeque<Result<(), Error>>>,
        pub delete_object_responses: RefCell<VecDeque<Result<(), Error>>>,
        pub captured_put_operations: RefCell<Vec<S3PutOperation>>,
        pub captured_delete_operations: RefCell<Vec<S3DeleteOperation>>,
    }

    #[allow(unused)]
    impl MockS3Client {
        pub fn new() -> Self {
            Self {
                put_object_responses: RefCell::new(VecDeque::new()),
                delete_object_responses: RefCell::new(VecDeque::new()),
                captured_put_operations: RefCell::new(Vec::new()),
                captured_delete_operations: RefCell::new(Vec::new()),
            }
        }

        pub fn expect_put_object_response(&self, response: Result<(), Error>) {
            self.put_object_responses.borrow_mut().push_back(response);
        }

        pub fn expect_delete_object_response(&self, response: Result<(), Error>) {
            self.delete_object_responses
                .borrow_mut()
                .push_back(response);
        }

        pub fn get_captured_put_operations(&self) -> Ref<'_, Vec<S3PutOperation>> {
            self.captured_put_operations.borrow()
        }

        pub fn get_captured_delete_operations(&self) -> Ref<'_, Vec<S3DeleteOperation>> {
            self.captured_delete_operations.borrow()
        }

        pub fn clear_captured_operations(&self) {
            self.captured_put_operations.borrow_mut().clear();
            self.captured_delete_operations.borrow_mut().clear();
        }
    }
    #[allow(unused)]
    impl S3Service for MockS3Client {
        async fn put_object(
            &self,
            request_id: &str,
            bucket: &str,
            object_name: &str,
            content: Bytes,
        ) -> Result<(), Error> {
            self.captured_put_operations
                .borrow_mut()
                .push(S3PutOperation {
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
        ) -> Result<(), Error> {
            self.captured_delete_operations
                .borrow_mut()
                .push(S3DeleteOperation {
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

    struct MockTranscribeClient {
        pub create_vocabulary_responses: RefCell<VecDeque<Result<CreateVocabularyResponse, Error>>>,
        pub get_vocabulary_responses: RefCell<VecDeque<Result<GetVocabularyResponse, Error>>>,
        pub start_transcription_responses:
            RefCell<VecDeque<Result<StartTranscriptionJobResponse, Error>>>,
        pub get_transcription_responses:
            RefCell<VecDeque<Result<GetTranscriptionJobResponse, Error>>>,
        pub download_transcript_responses: RefCell<VecDeque<Result<TranscribeOutput, Error>>>,
        pub delete_vocabulary_responses: RefCell<VecDeque<Result<(), Error>>>,
        pub captured_create_vocabulary: RefCell<Vec<CreateVocabularyOperation>>,
        pub captured_get_vocabulary: RefCell<Vec<String>>,
        pub captured_start_transcription: RefCell<Vec<StartTranscriptionOperation>>,
        pub captured_get_transcription: RefCell<Vec<String>>,
        pub captured_download_transcript: RefCell<Vec<DownloadTranscriptOperation>>,
        pub captured_delete_vocabulary: RefCell<Vec<String>>,
    }

    #[allow(unused)]
    impl MockTranscribeClient {
        pub fn new() -> Self {
            Self {
                create_vocabulary_responses: RefCell::new(VecDeque::new()),
                get_vocabulary_responses: RefCell::new(VecDeque::new()),
                start_transcription_responses: RefCell::new(VecDeque::new()),
                get_transcription_responses: RefCell::new(VecDeque::new()),
                download_transcript_responses: RefCell::new(VecDeque::new()),
                delete_vocabulary_responses: RefCell::new(VecDeque::new()),
                captured_create_vocabulary: RefCell::new(Vec::new()),
                captured_get_vocabulary: RefCell::new(Vec::new()),
                captured_start_transcription: RefCell::new(Vec::new()),
                captured_get_transcription: RefCell::new(Vec::new()),
                captured_download_transcript: RefCell::new(Vec::new()),
                captured_delete_vocabulary: RefCell::new(Vec::new()),
            }
        }

        pub fn expect_create_vocabulary_response(
            &self,
            response: Result<CreateVocabularyResponse, Error>,
        ) {
            self.create_vocabulary_responses
                .borrow_mut()
                .push_back(response);
        }

        pub fn expect_get_vocabulary_response(
            &self,
            response: Result<GetVocabularyResponse, Error>,
        ) {
            self.get_vocabulary_responses
                .borrow_mut()
                .push_back(response);
        }

        pub fn expect_start_transcription_response(
            &self,
            response: Result<StartTranscriptionJobResponse, Error>,
        ) {
            self.start_transcription_responses
                .borrow_mut()
                .push_back(response);
        }

        pub fn expect_get_transcription_response(
            &self,
            response: Result<GetTranscriptionJobResponse, Error>,
        ) {
            self.get_transcription_responses
                .borrow_mut()
                .push_back(response);
        }

        pub fn expect_download_transcript_response(
            &self,
            response: Result<TranscribeOutput, Error>,
        ) {
            self.download_transcript_responses
                .borrow_mut()
                .push_back(response);
        }

        pub fn expect_delete_vocabulary_response(&self, response: Result<(), Error>) {
            self.delete_vocabulary_responses
                .borrow_mut()
                .push_back(response);
        }

        pub fn get_captured_create_vocabulary(&self) -> Ref<'_, Vec<CreateVocabularyOperation>> {
            self.captured_create_vocabulary.borrow()
        }

        pub fn get_captured_start_transcription(
            &self,
        ) -> Ref<'_, Vec<StartTranscriptionOperation>> {
            self.captured_start_transcription.borrow()
        }

        pub fn get_captured_download_transcript(
            &self,
        ) -> Ref<'_, Vec<DownloadTranscriptOperation>> {
            self.captured_download_transcript.borrow()
        }

        pub fn clear_captured_operations(&self) {
            self.captured_create_vocabulary.borrow_mut().clear();
            self.captured_get_vocabulary.borrow_mut().clear();
            self.captured_start_transcription.borrow_mut().clear();
            self.captured_get_transcription.borrow_mut().clear();
            self.captured_download_transcript.borrow_mut().clear();
            self.captured_delete_vocabulary.borrow_mut().clear();
        }
    }

    impl TranscribeService for MockTranscribeClient {
        async fn create_vocabulary(
            &self,
            vocabulary_name: String,
            language_code: String,
            phrases: Vec<String>,
        ) -> Result<CreateVocabularyResponse, Error> {
            self.captured_create_vocabulary
                .borrow_mut()
                .push(CreateVocabularyOperation {
                    vocabulary_name: vocabulary_name.clone(),
                    language_code: language_code.clone(),
                    phrases: phrases.clone(),
                });

            self.create_vocabulary_responses
                .borrow_mut()
                .pop_front()
                .unwrap_or(Err((
                    vocabulary_name.clone(),
                    golem_stt::http::Error::Generic("unexpected error".to_string()),
                )
                    .into()))
        }

        async fn get_vocabulary(
            &self,
            vocabulary_name: &str,
        ) -> Result<GetVocabularyResponse, Error> {
            self.captured_get_vocabulary
                .borrow_mut()
                .push(vocabulary_name.to_string());

            self.get_vocabulary_responses
                .borrow_mut()
                .pop_front()
                .unwrap_or(Err((
                    vocabulary_name.to_string(),
                    golem_stt::http::Error::Generic("unexpected error".to_string()),
                )
                    .into()))
        }

        async fn wait_for_vocabulary_ready(
            &self,
            _request_id: &str,
            _max_wait_time: std::time::Duration,
        ) -> Result<(), Error> {
            Ok(())
        }

        async fn delete_vocabulary(&self, vocabulary_name: &str) -> Result<(), Error> {
            self.captured_delete_vocabulary
                .borrow_mut()
                .push(vocabulary_name.to_string());

            self.delete_vocabulary_responses
                .borrow_mut()
                .pop_front()
                .unwrap_or(Err((
                    vocabulary_name.to_string(),
                    golem_stt::http::Error::Generic("unexpected error".to_string()),
                )
                    .into()))
        }

        async fn start_transcription_job(
            &self,
            transcription_job_name: &str,
            media_file_uri: &str,
            audio_config: &AudioConfig,
            transcription_config: Option<&TranscriptionConfig>,
            vocabulary_name: Option<&str>,
        ) -> Result<StartTranscriptionJobResponse, Error> {
            self.captured_start_transcription
                .borrow_mut()
                .push(StartTranscriptionOperation {
                    job_name: transcription_job_name.to_string(),
                    media_uri: media_file_uri.to_string(),
                    audio_config: audio_config.clone(),
                    transcription_config: transcription_config.cloned(),
                    vocabulary_name: vocabulary_name.map(|s| s.to_string()),
                });

            self.start_transcription_responses
                .borrow_mut()
                .pop_front()
                .unwrap_or(Err((
                    transcription_job_name.to_string(),
                    golem_stt::http::Error::Generic("unexpected error".to_string()),
                )
                    .into()))
        }

        async fn get_transcription_job(
            &self,
            transcription_job_name: &str,
        ) -> Result<GetTranscriptionJobResponse, Error> {
            self.captured_get_transcription
                .borrow_mut()
                .push(transcription_job_name.to_string());

            self.get_transcription_responses
                .borrow_mut()
                .pop_front()
                .unwrap_or(Err((
                    transcription_job_name.to_string(),
                    golem_stt::http::Error::Generic("unexpected error".to_string()),
                )
                    .into()))
        }

        async fn wait_for_transcription_job_completion(
            &self,
            transcription_job_name: &str,
            _max_wait_time: std::time::Duration,
        ) -> Result<GetTranscriptionJobResponse, Error> {
            self.get_transcription_job(transcription_job_name).await
        }

        async fn download_transcript_json(
            &self,
            transcription_job_name: &str,
            transcript_uri: &str,
        ) -> Result<TranscribeOutput, Error> {
            self.captured_download_transcript
                .borrow_mut()
                .push(DownloadTranscriptOperation {
                    job_name: transcription_job_name.to_string(),
                    transcript_uri: transcript_uri.to_string(),
                });

            self.download_transcript_responses
                .borrow_mut()
                .pop_front()
                .unwrap_or(Err((
                    transcription_job_name.to_string(),
                    golem_stt::http::Error::Generic("unexpected error".to_string()),
                )
                    .into()))
        }

        async fn delete_transcription_job(
            &self,
            _transcription_job_name: &str,
        ) -> Result<(), golem_stt::error::Error> {
            Ok(())
        }
    }

    fn create_mock_transcribe_api() -> TranscribeApi<MockS3Client, MockTranscribeClient> {
        TranscribeApi {
            bucket_name: "test-bucket".to_string(),
            s3_client: MockS3Client::new(),
            transcribe_client: MockTranscribeClient::new(),
        }
    }

    #[wstd::test]
    async fn test_transcribe_audio_invalid_request_id_returns_error() {
        let api = create_mock_transcribe_api();

        let request = TranscriptionRequest {
            request_id: "invalid request id".to_string(), // spaces are invalid
            audio: "test audio".into(),
            audio_config: AudioConfig {
                format: AudioFormat::Wav,
                channels: Some(1),
            },
            transcription_config: None,
        };

        let result = api.transcribe_audio(request).await;
        assert!(result.is_err());

        if let Err(Error::APIBadRequest { provider_error, .. }) = result {
            assert!(provider_error.contains("Invalid request ID"));
        } else {
            panic!("Expected APIBadRequest error");
        }
    }

    #[wstd::test]
    async fn test_transcribe_audio_vocabulary_without_language_returns_error() {
        let api = create_mock_transcribe_api();

        let request = TranscriptionRequest {
            request_id: "test-123".to_string(),
            audio: "test audio".into(),
            audio_config: AudioConfig {
                format: AudioFormat::Wav,
                channels: Some(1),
            },
            transcription_config: Some(TranscriptionConfig {
                language: None, // No language specified
                model: None,
                diarization: None,
                enable_multi_channel: false,
                vocabulary: vec!["word1".to_string(), "word2".to_string()], // But vocabulary provided
            }),
        };

        let result = api.transcribe_audio(request).await;
        assert!(result.is_err());

        if let Err(Error::APIBadRequest { provider_error, .. }) = result {
            assert!(provider_error
                .contains("Vocabulary can only be used when a specific language is provided"));
        } else {
            panic!("Expected APIBadRequest error");
        }
    }

    #[wstd::test]
    async fn test_transcribe_audio_model_without_language_returns_error() {
        let api = create_mock_transcribe_api();

        let request = TranscriptionRequest {
            request_id: "test-123".to_string(),
            audio: "test audio".into(),
            audio_config: AudioConfig {
                format: AudioFormat::Wav,
                channels: Some(1),
            },
            transcription_config: Some(TranscriptionConfig {
                language: None,                             // No language specified
                model: Some("en-US_Telephony".to_string()), // But model provided
                diarization: None,
                enable_multi_channel: false,
                vocabulary: vec![],
            }),
        };

        let result = api.transcribe_audio(request).await;
        assert!(result.is_err());

        if let Err(Error::APIBadRequest { provider_error, .. }) = result {
            assert!(provider_error
                .contains("Model settings can only be used when a specific language is provided"));
        } else {
            panic!("Expected APIBadRequest error");
        }
    }

    #[wstd::test]
    async fn test_transcribe_audio_uploads_to_s3() {
        let api = create_mock_transcribe_api();

        api.s3_client.expect_put_object_response(Ok(()));
        api.transcribe_client
            .expect_start_transcription_response(Ok(StartTranscriptionJobResponse {
                transcription_job: TranscriptionJob {
                    transcription_job_name: "test-123".to_string(),
                    transcription_job_status: "COMPLETED".to_string(),
                    language_code: None,
                    media: None,
                    media_format: None,
                    media_sample_rate_hertz: None,
                    creation_time: None,
                    completion_time: None,
                    start_time: None,
                    failure_reason: None,
                    settings: None,
                    transcript: Some(Transcript {
                        transcript_file_uri: Some(
                            "https://example.com/transcript.json".to_string(),
                        ),
                        redacted_transcript_file_uri: None,
                    }),
                    tags: None,
                },
            }));
        api.transcribe_client
            .expect_download_transcript_response(Ok(TranscribeOutput {
                job_name: "test-123".to_string(),
                account_id: "123456789".to_string(),
                results: TranscribeResults {
                    transcripts: vec![TranscriptText {
                        transcript: "Hello world".to_string(),
                    }],
                    speaker_labels: None,
                    channel_labels: None,
                    items: vec![],
                    audio_segments: vec![],
                },
                status: "COMPLETED".to_string(),
            }));
        api.transcribe_client
            .expect_delete_vocabulary_response(Ok(()));
        api.s3_client.expect_delete_object_response(Ok(()));

        let request = TranscriptionRequest {
            request_id: "test-123".to_string(),
            audio: "test audio data".into(),
            audio_config: AudioConfig {
                format: AudioFormat::Wav,
                channels: Some(1),
            },
            transcription_config: None,
        };

        let _ = api.transcribe_audio(request).await.unwrap();

        let captured_puts = api.s3_client.get_captured_put_operations();
        assert_eq!(captured_puts.len(), 1);
        let put_op = &captured_puts[0];
        assert_eq!(put_op.request_id, "test-123");
        assert_eq!(put_op.bucket, "test-bucket");
        assert_eq!(put_op.object_name, "test-123/audio.wav");
        assert_eq!(put_op.content_size, 15);
    }

    #[wstd::test]
    async fn test_transcribe_audio_creates_vocabulary_when_provided() {
        let api = create_mock_transcribe_api();

        api.s3_client.expect_put_object_response(Ok(()));
        api.transcribe_client
            .expect_create_vocabulary_response(Ok(CreateVocabularyResponse {
                vocabulary_name: "test-123".to_string(),
                language_code: "en-US".to_string(),
                vocabulary_state: "READY".to_string(),
                last_modified_time: None,
                failure_reason: None,
            }));
        api.transcribe_client
            .expect_start_transcription_response(Ok(StartTranscriptionJobResponse {
                transcription_job: TranscriptionJob {
                    transcription_job_name: "test-123".to_string(),
                    transcription_job_status: "COMPLETED".to_string(),
                    language_code: None,
                    media: None,
                    media_format: None,
                    media_sample_rate_hertz: None,
                    creation_time: None,
                    completion_time: None,
                    start_time: None,
                    failure_reason: None,
                    settings: None,
                    transcript: Some(Transcript {
                        transcript_file_uri: Some(
                            "https://example.com/transcript.json".to_string(),
                        ),
                        redacted_transcript_file_uri: None,
                    }),
                    tags: None,
                },
            }));
        api.transcribe_client
            .expect_download_transcript_response(Ok(TranscribeOutput {
                job_name: "test-123".to_string(),
                account_id: "123456789".to_string(),
                results: TranscribeResults {
                    transcripts: vec![TranscriptText {
                        transcript: "Hello world".to_string(),
                    }],
                    speaker_labels: None,
                    channel_labels: None,
                    items: vec![],
                    audio_segments: vec![],
                },
                status: "COMPLETED".to_string(),
            }));
        api.transcribe_client
            .expect_delete_vocabulary_response(Ok(()));
        api.s3_client.expect_delete_object_response(Ok(()));

        let request = TranscriptionRequest {
            request_id: "test-123".to_string(),
            audio: "test audio".into(),
            audio_config: AudioConfig {
                format: AudioFormat::Mp3,
                channels: Some(1),
            },
            transcription_config: Some(TranscriptionConfig {
                language: Some("en-US".to_string()),
                model: None,
                diarization: None,
                enable_multi_channel: false,
                vocabulary: vec!["custom".to_string(), "words".to_string()],
            }),
        };

        let _ = api.transcribe_audio(request).await.unwrap();

        let captured_vocabulary = api.transcribe_client.get_captured_create_vocabulary();
        assert_eq!(captured_vocabulary.len(), 1);
        let vocab_op = &captured_vocabulary[0];
        assert_eq!(vocab_op.vocabulary_name, "test-123");
        assert_eq!(vocab_op.language_code, "en-US");
        assert_eq!(
            vocab_op.phrases,
            vec!["custom".to_string(), "words".to_string()]
        );
    }

    #[wstd::test]
    async fn test_transcribe_audio_starts_transcription_job() {
        let api = create_mock_transcribe_api();

        api.s3_client.expect_put_object_response(Ok(()));
        api.transcribe_client
            .expect_start_transcription_response(Ok(StartTranscriptionJobResponse {
                transcription_job: TranscriptionJob {
                    transcription_job_name: "test-123".to_string(),
                    transcription_job_status: "COMPLETED".to_string(),
                    language_code: None,
                    media: None,
                    media_format: None,
                    media_sample_rate_hertz: None,
                    creation_time: None,
                    completion_time: None,
                    start_time: None,
                    failure_reason: None,
                    settings: None,
                    transcript: Some(Transcript {
                        transcript_file_uri: Some(
                            "https://example.com/transcript.json".to_string(),
                        ),
                        redacted_transcript_file_uri: None,
                    }),
                    tags: None,
                },
            }));
        api.transcribe_client
            .expect_download_transcript_response(Ok(TranscribeOutput {
                job_name: "test-123".to_string(),
                account_id: "123456789".to_string(),
                results: TranscribeResults {
                    transcripts: vec![TranscriptText {
                        transcript: "Hello world".to_string(),
                    }],
                    speaker_labels: None,
                    channel_labels: None,
                    items: vec![],
                    audio_segments: vec![],
                },
                status: "COMPLETED".to_string(),
            }));
        api.transcribe_client
            .expect_delete_vocabulary_response(Ok(()));
        api.s3_client.expect_delete_object_response(Ok(()));

        let request = TranscriptionRequest {
            request_id: "test-123".to_string(),
            audio: "test audio".into(),
            audio_config: AudioConfig {
                format: AudioFormat::Flac,
                channels: Some(2),
            },
            transcription_config: Some(TranscriptionConfig {
                language: Some("es-ES".to_string()),
                model: Some("en-US_Telephony".to_string()),
                diarization: Some(DiarizationConfig {
                    enabled: true,
                    max_speakers: 2,
                }),
                enable_multi_channel: false,
                vocabulary: vec![],
            }),
        };

        let _ = api.transcribe_audio(request).await.unwrap();

        let captured_transcription = api.transcribe_client.get_captured_start_transcription();
        assert_eq!(captured_transcription.len(), 1);
        let transcription_op = &captured_transcription[0];
        assert_eq!(transcription_op.job_name, "test-123");
        assert_eq!(
            transcription_op.media_uri,
            "s3://test-bucket/test-123/audio.flac"
        );
        assert_eq!(transcription_op.audio_config.format.to_string(), "flac");
        assert_eq!(transcription_op.audio_config.channels, Some(2));
        assert!(transcription_op
            .transcription_config
            .as_ref()
            .unwrap()
            .diarization
            .as_ref()
            .is_some_and(|d| d.enabled));
        assert_eq!(transcription_op.vocabulary_name, None);
    }

    #[wstd::test]
    async fn test_transcribe_audio_downloads_transcript_and_returns_response() {
        let api = create_mock_transcribe_api();

        api.s3_client.expect_put_object_response(Ok(()));
        api.transcribe_client
            .expect_start_transcription_response(Ok(StartTranscriptionJobResponse {
                transcription_job: TranscriptionJob {
                    transcription_job_name: "test-123".to_string(),
                    transcription_job_status: "COMPLETED".to_string(),
                    language_code: None,
                    media: None,
                    media_format: None,
                    media_sample_rate_hertz: None,
                    creation_time: None,
                    completion_time: None,
                    start_time: None,
                    failure_reason: None,
                    settings: None,
                    transcript: Some(Transcript {
                        transcript_file_uri: Some(
                            "https://example.com/transcript.json".to_string(),
                        ),
                        redacted_transcript_file_uri: None,
                    }),
                    tags: None,
                },
            }));
        api.transcribe_client
            .expect_download_transcript_response(Ok(TranscribeOutput {
                job_name: "test-123".to_string(),
                account_id: "123456789".to_string(),
                results: TranscribeResults {
                    transcripts: vec![TranscriptText {
                        transcript: "Hello world".to_string(),
                    }],
                    speaker_labels: None,
                    channel_labels: None,
                    items: vec![],
                    audio_segments: vec![],
                },
                status: "COMPLETED".to_string(),
            }));
        api.transcribe_client
            .expect_delete_vocabulary_response(Ok(()));
        api.s3_client.expect_delete_object_response(Ok(()));

        let request = TranscriptionRequest {
            request_id: "test-123".to_string(),
            audio: "test audio data".into(),
            audio_config: AudioConfig {
                format: AudioFormat::Ogg,
                channels: Some(1),
            },
            transcription_config: Some(TranscriptionConfig {
                language: Some("fr-FR".to_string()),
                model: None,
                diarization: None,
                enable_multi_channel: false,
                vocabulary: vec![],
            }),
        };

        let response = api.transcribe_audio(request).await.unwrap();

        assert_eq!(response.audio_size_bytes, 15);
        assert_eq!(response.language, "fr-FR");

        let captured_downloads = api.transcribe_client.get_captured_download_transcript();
        assert_eq!(captured_downloads.len(), 1);
        let download_op = &captured_downloads[0];
        assert_eq!(download_op.job_name, "test-123");
        assert_eq!(
            download_op.transcript_uri,
            "https://example.com/transcript.json"
        );
    }

    #[wstd::test]
    async fn test_transcribe_audio_cleans_up_resources() {
        let api = create_mock_transcribe_api();

        api.s3_client.expect_put_object_response(Ok(()));
        api.transcribe_client
            .expect_create_vocabulary_response(Ok(CreateVocabularyResponse {
                vocabulary_name: "test-123".to_string(),
                language_code: "en-US".to_string(),
                vocabulary_state: "READY".to_string(),
                last_modified_time: None,
                failure_reason: None,
            }));
        api.transcribe_client
            .expect_start_transcription_response(Ok(StartTranscriptionJobResponse {
                transcription_job: TranscriptionJob {
                    transcription_job_name: "test-123".to_string(),
                    transcription_job_status: "COMPLETED".to_string(),
                    language_code: None,
                    media: None,
                    media_format: None,
                    media_sample_rate_hertz: None,
                    creation_time: None,
                    completion_time: None,
                    start_time: None,
                    failure_reason: None,
                    settings: None,
                    transcript: Some(Transcript {
                        transcript_file_uri: Some(
                            "https://example.com/transcript.json".to_string(),
                        ),
                        redacted_transcript_file_uri: None,
                    }),
                    tags: None,
                },
            }));
        api.transcribe_client
            .expect_download_transcript_response(Ok(TranscribeOutput {
                job_name: "test-123".to_string(),
                account_id: "123456789".to_string(),
                results: TranscribeResults {
                    transcripts: vec![TranscriptText {
                        transcript: "Hello world".to_string(),
                    }],
                    speaker_labels: None,
                    channel_labels: None,
                    items: vec![],
                    audio_segments: vec![],
                },
                status: "COMPLETED".to_string(),
            }));
        api.transcribe_client
            .expect_delete_vocabulary_response(Ok(()));
        api.s3_client.expect_delete_object_response(Ok(()));

        let request = TranscriptionRequest {
            request_id: "test-123".to_string(),
            audio: "test audio".into(),
            audio_config: AudioConfig {
                format: AudioFormat::Wav,
                channels: Some(1),
            },
            transcription_config: Some(TranscriptionConfig {
                language: Some("en-US".to_string()),
                model: None,
                diarization: None,
                enable_multi_channel: false,
                vocabulary: vec!["word1".to_string()],
            }),
        };

        let _ = api.transcribe_audio(request).await.unwrap();

        // Check vocabulary was deleted
        let captured_vocab_deletes = api.transcribe_client.captured_delete_vocabulary.borrow();
        assert_eq!(captured_vocab_deletes.len(), 1);
        assert_eq!(captured_vocab_deletes[0], "test-123");

        // Check S3 object was deleted
        let captured_deletes = api.s3_client.get_captured_delete_operations();
        assert_eq!(captured_deletes.len(), 1);
        let delete_op = &captured_deletes[0];
        assert_eq!(delete_op.request_id, "test-123");
        assert_eq!(delete_op.bucket, "test-bucket");
        assert_eq!(delete_op.object_name, "test-123/audio.wav");
    }

    #[wstd::test]
    async fn test_transcribe_audio_s3_upload_failure() {
        let api = create_mock_transcribe_api();

        api.s3_client
            .expect_put_object_response(Err(Error::APIInternalServerError {
                request_id: "test-123".to_string(),
                provider_error: "S3 upload failed".to_string(),
            }));

        let request = TranscriptionRequest {
            request_id: "test-123".to_string(),
            audio: "test audio".into(),
            audio_config: AudioConfig {
                format: AudioFormat::Wav,
                channels: Some(1),
            },
            transcription_config: None,
        };

        let result = api.transcribe_audio(request).await;
        assert!(result.is_err());

        if let Err(Error::APIInternalServerError { provider_error, .. }) = result {
            assert!(provider_error.contains("S3 upload failed"));
        } else {
            panic!("Expected APIInternalServerError");
        }
    }

    #[wstd::test]
    async fn test_transcribe_audio_vocabulary_creation_failure() {
        let api = create_mock_transcribe_api();

        api.s3_client.expect_put_object_response(Ok(()));
        api.transcribe_client
            .expect_create_vocabulary_response(Ok(CreateVocabularyResponse {
                vocabulary_name: "test-123".to_string(),
                language_code: "en-US".to_string(),
                vocabulary_state: "FAILED".to_string(),
                last_modified_time: None,
                failure_reason: Some("Invalid vocabulary words".to_string()),
            }));

        let request = TranscriptionRequest {
            request_id: "test-123".to_string(),
            audio: "test audio".into(),
            audio_config: AudioConfig {
                format: AudioFormat::Wav,
                channels: Some(1),
            },
            transcription_config: Some(TranscriptionConfig {
                language: Some("en-US".to_string()),
                model: None,
                diarization: None,
                enable_multi_channel: false,
                vocabulary: vec!["invalid".to_string()],
            }),
        };

        let result = api.transcribe_audio(request).await;
        assert!(result.is_err());

        if let Err(Error::APIBadRequest { provider_error, .. }) = result {
            assert!(provider_error.contains("Vocabulary creation failed"));
            assert!(provider_error.contains("Invalid vocabulary words"));
        } else {
            panic!("Expected APIBadRequest error");
        }

        let captured_s3_deletes = api.s3_client.get_captured_delete_operations();
        assert_eq!(captured_s3_deletes.len(), 1);
    }

    #[wstd::test]
    async fn test_transcribe_audio_transcription_job_failure() {
        let api = create_mock_transcribe_api();

        api.s3_client.expect_put_object_response(Ok(()));
        api.transcribe_client
            .expect_start_transcription_response(Ok(StartTranscriptionJobResponse {
                transcription_job: TranscriptionJob {
                    transcription_job_name: "test-123".to_string(),
                    transcription_job_status: "FAILED".to_string(),
                    language_code: None,
                    media: None,
                    media_format: None,
                    media_sample_rate_hertz: None,
                    creation_time: None,
                    completion_time: None,
                    start_time: None,
                    failure_reason: Some("Audio format not supported".to_string()),
                    settings: None,
                    transcript: None,
                    tags: None,
                },
            }));

        let request = TranscriptionRequest {
            request_id: "test-123".to_string(),
            audio: "test audio".into(),
            audio_config: AudioConfig {
                format: AudioFormat::Wav,
                channels: Some(1),
            },
            transcription_config: None,
        };

        let result = api.transcribe_audio(request).await;
        assert!(result.is_err());

        if let Err(Error::APIBadRequest { provider_error, .. }) = result {
            assert!(provider_error.contains("Transcription job creation failed"));
            assert!(provider_error.contains("Audio format not supported"));
        } else {
            panic!("Expected APIBadRequest error");
        }

        let captured_s3_deletes = api.s3_client.get_captured_delete_operations();
        assert_eq!(captured_s3_deletes.len(), 1);
    }

    #[wstd::test]
    async fn test_transcribe_audio_transcript_download_failure() {
        let api = create_mock_transcribe_api();

        api.s3_client.expect_put_object_response(Ok(()));
        api.transcribe_client
            .expect_start_transcription_response(Ok(StartTranscriptionJobResponse {
                transcription_job: TranscriptionJob {
                    transcription_job_name: "test-123".to_string(),
                    transcription_job_status: "COMPLETED".to_string(),
                    language_code: None,
                    media: None,
                    media_format: None,
                    media_sample_rate_hertz: None,
                    creation_time: None,
                    completion_time: None,
                    start_time: None,
                    failure_reason: None,
                    settings: None,
                    transcript: Some(Transcript {
                        transcript_file_uri: Some(
                            "https://example.com/transcript.json".to_string(),
                        ),
                        redacted_transcript_file_uri: None,
                    }),
                    tags: None,
                },
            }));
        api.transcribe_client
            .expect_download_transcript_response(Err(Error::APINotFound {
                request_id: "test-123".to_string(),
                provider_error: "Transcript file not found".to_string(),
            }));

        let request = TranscriptionRequest {
            request_id: "test-123".to_string(),
            audio: "test audio".into(),
            audio_config: AudioConfig {
                format: AudioFormat::Wav,
                channels: Some(1),
            },
            transcription_config: None,
        };

        let result = api.transcribe_audio(request).await;
        assert!(result.is_err());

        if let Err(Error::APINotFound { provider_error, .. }) = result {
            assert!(provider_error.contains("Transcript file not found"));
        } else {
            panic!("Expected APINotFound error");
        }
    }
}
