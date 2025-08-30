use bytes::Bytes;
use chrono::Utc;
use golem_stt::runtime::AsyncRuntime;

use http::{Request, StatusCode};
use log::trace;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use super::{
    aws_signer::AwsSignatureV4,
    request::{AudioConfig, TranscriptionConfig},
};

// https://docs.aws.amazon.com/transcribe/latest/APIReference/API_CreateVocabulary.html
#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct CreateVocabularyRequest {
    pub vocabulary_name: String,
    pub language_code: String,
    pub phrases: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vocabulary_file_uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_access_role_arn: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<Tag>>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct Tag {
    pub key: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct CreateVocabularyResponse {
    pub vocabulary_name: String,
    pub language_code: String,
    pub vocabulary_state: String,
    pub last_modified_time: Option<f64>,
    pub failure_reason: Option<String>,
}

// https://docs.aws.amazon.com/transcribe/latest/APIReference/API_GetVocabulary.html
#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct GetVocabularyRequest {
    pub vocabulary_name: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct GetVocabularyResponse {
    pub vocabulary_name: String,
    pub language_code: String,
    pub vocabulary_state: String,
    pub last_modified_time: Option<f64>,
    pub failure_reason: Option<String>,
    pub download_uri: Option<String>,
}

// https://docs.aws.amazon.com/transcribe/latest/APIReference/API_DeleteVocabulary.html
#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct DeleteVocabularyRequest {
    pub vocabulary_name: String,
}

// https://docs.aws.amazon.com/transcribe/latest/APIReference/API_StartTranscriptionJob.html
#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct StartTranscriptionJobRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_redaction: Option<ContentRedaction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identify_language: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identify_multiple_languages: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_execution_settings: Option<JobExecutionSettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kms_encryption_context: Option<std::collections::HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language_id_settings: Option<std::collections::HashMap<String, LanguageIdSettings>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language_options: Option<Vec<String>>,
    pub media: Media,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_sample_rate_hertz: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_settings: Option<ModelSettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_bucket_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_encryption_kms_key_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<Settings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtitles: Option<Subtitles>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<Tag>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub toxicity_detection: Option<Vec<ToxicityDetectionSettings>>,
    pub transcription_job_name: String,
}

// ContentRedaction - https://docs.aws.amazon.com/transcribe/latest/APIReference/API_ContentRedaction.html
#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct ContentRedaction {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pii_entity_types: Option<Vec<String>>,
    pub redaction_output: String,
    pub redaction_type: String,
}

// JobExecutionSettings - https://docs.aws.amazon.com/transcribe/latest/APIReference/API_JobExecutionSettings.html
#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct JobExecutionSettings {
    pub allow_deferred_execution: bool,
    pub data_access_role_arn: String,
}

// LanguageIdSettings - https://docs.aws.amazon.com/transcribe/latest/APIReference/API_LanguageIdSettings.html
#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct LanguageIdSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language_model_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vocabulary_filter_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vocabulary_name: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct Media {
    pub media_file_uri: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redacted_media_file_uri: Option<String>,
}

// ModelSettings - https://docs.aws.amazon.com/transcribe/latest/APIReference/API_ModelSettings.html
#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct ModelSettings {
    pub language_model_name: String,
}

// Subtitles - https://docs.aws.amazon.com/transcribe/latest/APIReference/API_Subtitles.html
#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct Subtitles {
    pub formats: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_start_index: Option<i32>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Default)]
#[serde(rename_all = "PascalCase")]
pub struct Settings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_identification: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_alternatives: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_speaker_labels: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_alternatives: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_speaker_labels: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vocabulary_filter_method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vocabulary_filter_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vocabulary_name: Option<String>,
}

// ToxicityDetectionSettings - https://docs.aws.amazon.com/transcribe/latest/APIReference/API_ToxicityDetectionSettings.html
#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct ToxicityDetectionSettings {
    pub toxicity_categories: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct StartTranscriptionJobResponse {
    pub transcription_job: TranscriptionJob,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct TranscriptionJob {
    pub transcription_job_name: String,
    pub transcription_job_status: String,
    pub language_code: Option<String>,
    pub media: Option<Media>,
    pub media_format: Option<String>,
    pub media_sample_rate_hertz: Option<i32>,
    pub creation_time: Option<f64>,
    pub completion_time: Option<f64>,
    pub start_time: Option<f64>,
    pub failure_reason: Option<String>,
    pub settings: Option<Settings>,
    pub transcript: Option<Transcript>,
    pub tags: Option<Vec<Tag>>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct Transcript {
    pub transcript_file_uri: Option<String>,
    pub redacted_transcript_file_uri: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct DeleteTranscriptionJobRequest {
    pub transcription_job_name: String,
}

// https://docs.aws.amazon.com/transcribe/latest/APIReference/API_GetTranscriptionJob.html
#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct GetTranscriptionJobRequest {
    pub transcription_job_name: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct GetTranscriptionJobResponse {
    pub transcription_job: TranscriptionJob,
}

// https://docs.aws.amazon.com/transcribe/latest/dg/how-input.html#how-output
#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TranscribeOutput {
    pub job_name: String,
    pub account_id: String,
    pub results: TranscribeResults,
    pub status: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct TranscribeResults {
    pub transcripts: Vec<TranscriptText>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speaker_labels: Option<SpeakerLabels>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_labels: Option<ChannelLabels>,
    pub items: Vec<TranscribeItem>,
    pub audio_segments: Vec<AudioSegment>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptText {
    pub transcript: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct SpeakerLabels {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_label: Option<String>,
    pub speakers: i32,
    pub segments: Vec<SpeakerSegment>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct SpeakerSegment {
    pub start_time: String,
    pub speaker_label: String,
    pub end_time: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Vec<SpeakerItem>>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct SpeakerItem {
    pub start_time: String,
    pub speaker_label: String,
    pub end_time: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct TranscribeItem {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speaker_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_label: Option<String>,
    pub alternatives: Vec<TranscribeAlternative>,
    #[serde(rename = "type")]
    pub item_type: String, // "pronunciation" or "punctuation"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vocabulary_filter_match: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct TranscribeAlternative {
    pub confidence: String, // Note: AWS returns this as a string, not a number
    pub content: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct AudioSegment {
    pub id: i32,
    pub transcript: String,
    pub start_time: String,
    pub end_time: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speaker_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_label: Option<String>,
    pub items: Vec<i32>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct ChannelLabels {
    pub channels: Vec<Channel>,
    pub number_of_channels: i32,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct Channel {
    pub channel_label: String,
    pub items: Vec<TranscribeItem>,
}

pub trait TranscribeService {
    async fn create_vocabulary(
        &self,
        vocabulary_name: String,
        language_code: String,
        phrases: Vec<String>,
    ) -> Result<CreateVocabularyResponse, golem_stt::error::Error>;

    async fn get_vocabulary(
        &self,
        vocabulary_name: &str,
    ) -> Result<GetVocabularyResponse, golem_stt::error::Error>;

    async fn wait_for_vocabulary_ready(
        &self,
        request_id: &str,
        max_wait_time: Duration,
    ) -> Result<(), golem_stt::error::Error>;

    async fn delete_vocabulary(&self, vocabulary_name: &str)
        -> Result<(), golem_stt::error::Error>;

    async fn start_transcription_job(
        &self,
        transcription_job_name: &str,
        media_file_uri: &str,
        audio_config: &AudioConfig,
        transcription_config: Option<&TranscriptionConfig>,
        vocabulary_name: Option<&str>,
    ) -> Result<StartTranscriptionJobResponse, golem_stt::error::Error>;

    async fn delete_transcription_job(
        &self,
        transcription_job_name: &str,
    ) -> Result<(), golem_stt::error::Error>;

    async fn get_transcription_job(
        &self,
        transcription_job_name: &str,
    ) -> Result<GetTranscriptionJobResponse, golem_stt::error::Error>;

    async fn wait_for_transcription_job_completion(
        &self,
        transcription_job_name: &str,
        max_wait_time: Duration,
    ) -> Result<GetTranscriptionJobResponse, golem_stt::error::Error>;

    async fn download_transcript_json(
        &self,
        transcription_job_name: &str,
        transcript_uri: &str,
    ) -> Result<TranscribeOutput, golem_stt::error::Error>;
}

pub struct TranscribeClient<HC: golem_stt::http::HttpClient, RT: AsyncRuntime> {
    http_client: HC,
    signer: AwsSignatureV4,
    runtime: RT,
}

// marker trait to handle empty http response body in case of Delete requests
#[derive(Debug, Clone, PartialEq)]
pub struct EmptyResponse;

impl<'de> serde::Deserialize<'de> for EmptyResponse {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(EmptyResponse)
    }
}

impl<HC: golem_stt::http::HttpClient, RT: AsyncRuntime> TranscribeClient<HC, RT> {
    pub fn new(
        access_key: String,
        secret_key: String,
        region: String,
        http_client: HC,
        runtime: RT,
    ) -> Self {
        Self {
            http_client,
            signer: AwsSignatureV4::for_transcribe(access_key, secret_key, region),
            runtime,
        }
    }

    async fn make_authenticated_request<R, T>(
        &self,
        target: &str,
        request_body: &R,
        request_id: String,
        operation_name: &str,
    ) -> Result<T, golem_stt::error::Error>
    where
        T: for<'de> serde::Deserialize<'de>,
        R: serde::Serialize,
    {
        let timestamp = Utc::now();
        let uri = format!(
            "https://transcribe.{}.amazonaws.com/",
            self.signer.get_region()
        );

        let json_body = serde_json::to_string(request_body).map_err(|e| {
            (
                request_id.clone(),
                golem_stt::http::Error::Generic(format!("Failed to serialize request: {e}")),
            )
        })?;

        let json_bytes = Bytes::from(json_body);

        let request = Request::builder()
            .method("POST")
            .uri(&uri)
            .header("Content-Type", "application/x-amz-json-1.1")
            .header("X-Amz-Target", target)
            .body(json_bytes)
            .map_err(|e| (request_id.clone(), golem_stt::http::Error::HttpError(e)))?;

        let signed_request = self
            .signer
            .sign_request(request, timestamp)
            .map_err(|err| {
                (
                    request_id.clone(),
                    golem_stt::http::Error::Generic(format!("Failed to sign request: {err}")),
                )
            })?;

        trace!("Sending request to AWS Transcribe API: {uri}");

        let response = self
            .http_client
            .execute(signed_request)
            .await
            .map_err(|err| (request_id.clone(), err))?;

        if response.status().is_success() {
            let transcribe_response: T = serde_json::from_slice(response.body()).map_err(|e| {
                (
                    request_id.clone(),
                    golem_stt::http::Error::Generic(
                        format!("Failed to deserialize response: {e}",),
                    ),
                )
            })?;

            Ok(transcribe_response)
        } else {
            let error_body = String::from_utf8(response.body().to_vec())
                .unwrap_or_else(|_| "Unknown error".to_string());

            let status = response.status();

            match status {
                StatusCode::BAD_REQUEST => Err(golem_stt::error::Error::APIBadRequest {
                    request_id,
                    provider_error: format!(
                        "Transcribe {operation_name} bad request: {error_body}"
                    ),
                }),
                StatusCode::FORBIDDEN => Err(golem_stt::error::Error::APIForbidden {
                    request_id,
                    provider_error: format!("Transcribe {operation_name} forbidden: {error_body}"),
                }),
                StatusCode::INTERNAL_SERVER_ERROR => {
                    Err(golem_stt::error::Error::APIInternalServerError {
                        request_id,
                        provider_error: format!(
                            "Transcribe {operation_name} server error: {error_body}",
                        ),
                    })
                }
                StatusCode::SERVICE_UNAVAILABLE => {
                    Err(golem_stt::error::Error::APIInternalServerError {
                        request_id,
                        provider_error: format!(
                            "Transcribe {operation_name} service unavailable: {error_body}",
                        ),
                    })
                }
                _ => Err(golem_stt::error::Error::APIUnknown {
                    request_id,
                    provider_error: format!(
                        "Transcribe {operation_name} unknown error ({status}): {error_body}"
                    ),
                }),
            }
        }
    }
}
impl<HC: golem_stt::http::HttpClient, RT: AsyncRuntime> TranscribeService
    for TranscribeClient<HC, RT>
{
    async fn create_vocabulary(
        &self,
        vocabulary_name: String,
        language_code: String,
        phrases: Vec<String>,
    ) -> Result<CreateVocabularyResponse, golem_stt::error::Error> {
        let request_body = CreateVocabularyRequest {
            vocabulary_name: vocabulary_name.clone(),
            language_code,
            phrases,
            vocabulary_file_uri: None,
            data_access_role_arn: None,
            tags: None,
        };

        self.make_authenticated_request(
            "com.amazonaws.transcribe.Transcribe.CreateVocabulary",
            &request_body,
            vocabulary_name,
            "CreateVocabulary",
        )
        .await
    }

    async fn get_vocabulary(
        &self,
        vocabulary_name: &str,
    ) -> Result<GetVocabularyResponse, golem_stt::error::Error> {
        let request_body = GetVocabularyRequest {
            vocabulary_name: vocabulary_name.to_string(),
        };

        self.make_authenticated_request(
            "com.amazonaws.transcribe.Transcribe.GetVocabulary",
            &request_body,
            vocabulary_name.to_string(),
            "GetVocabulary",
        )
        .await
    }

    async fn wait_for_vocabulary_ready(
        &self,
        vocabulary_name: &str,
        max_wait_time: Duration,
    ) -> Result<(), golem_stt::error::Error> {
        let start_time = std::time::Instant::now();
        let poll_interval = Duration::from_secs(10);

        loop {
            if start_time.elapsed() > max_wait_time {
                return Err(golem_stt::error::Error::APIBadRequest {
                    request_id: vocabulary_name.to_string(),
                    provider_error: "Vocabulary creation timed out".to_string(),
                });
            }

            self.runtime.sleep(poll_interval).await;

            let res = self.get_vocabulary(vocabulary_name).await?;

            match res.vocabulary_state.as_str() {
                "READY" => return Ok(()),
                "FAILED" => {
                    return Err(golem_stt::error::Error::APIBadRequest {
                        request_id: vocabulary_name.to_string(),
                        provider_error: format!(
                            "Vocabulary creation failed: {}",
                            res.failure_reason
                                .unwrap_or_else(|| "Unknown error".to_string())
                        ),
                    });
                }
                "PENDING" => {}
                other => {
                    return Err(golem_stt::error::Error::APIBadRequest {
                        request_id: vocabulary_name.to_string(),
                        provider_error: format!("Unexpected vocabulary state: {other}"),
                    });
                }
            }
        }
    }

    async fn delete_vocabulary(
        &self,
        vocabulary_name: &str,
    ) -> Result<(), golem_stt::error::Error> {
        let request_body = DeleteVocabularyRequest {
            vocabulary_name: vocabulary_name.to_string(),
        };

        let _: EmptyResponse = self
            .make_authenticated_request(
                "com.amazonaws.transcribe.Transcribe.DeleteVocabulary",
                &request_body,
                vocabulary_name.to_string(),
                "DeleteVocabulary",
            )
            .await?;

        Ok(())
    }

    async fn start_transcription_job(
        &self,
        transcription_job_name: &str,
        media_file_uri: &str,
        audio_config: &AudioConfig,
        transcription_config: Option<&TranscriptionConfig>,
        vocabulary_name: Option<&str>,
    ) -> Result<StartTranscriptionJobResponse, golem_stt::error::Error> {
        let mut request_body = StartTranscriptionJobRequest {
            transcription_job_name: transcription_job_name.to_string(),
            media: Media {
                media_file_uri: media_file_uri.to_string(),
                redacted_media_file_uri: None,
            },
            media_format: Some(audio_config.format.to_string()),
            content_redaction: None,
            identify_language: None,
            identify_multiple_languages: None,
            job_execution_settings: None,
            kms_encryption_context: None,
            language_code: None,
            language_id_settings: None,
            language_options: None,
            media_sample_rate_hertz: None,
            model_settings: None,
            output_bucket_name: None,
            output_encryption_kms_key_id: None,
            output_key: None,
            settings: None,
            subtitles: None,
            tags: None,
            toxicity_detection: None,
        };

        let mut settings: Option<Settings> = None;

        if let Some(config) = transcription_config {
            if let Some(language) = &config.language {
                request_body.language_code = Some(language.to_string());

                if let Some(model) = &config.model {
                    request_body.model_settings = Some(ModelSettings {
                        language_model_name: model.to_string(),
                    });
                }

                if let Some(vocab_name) = vocabulary_name {
                    settings
                        .get_or_insert_with(Settings::default)
                        .vocabulary_name = Some(vocab_name.to_string());
                }
            } else {
                request_body.identify_language = Some(true);
            }

            if audio_config.channels.is_some_and(|c| c == 2) && config.enable_multi_channel {
                settings
                    .get_or_insert_with(Settings::default)
                    .channel_identification = Some(true);
            }

            if let Some(diarization) = &config.diarization {
                if diarization.enabled {
                    let settings_ref = settings.get_or_insert_with(Settings::default);
                    settings_ref.show_speaker_labels = Some(true);
                    settings_ref.max_speaker_labels = Some(diarization.max_speakers as i32);
                }
            }
        } else {
            request_body.identify_language = Some(true);
        }

        request_body.settings = settings;

        self.make_authenticated_request(
            "com.amazonaws.transcribe.Transcribe.StartTranscriptionJob",
            &request_body,
            transcription_job_name.to_string(),
            "StartTranscriptionJob",
        )
        .await
    }

    async fn delete_transcription_job(
        &self,
        transcription_job_name: &str,
    ) -> Result<(), golem_stt::error::Error> {
        let request_body = DeleteTranscriptionJobRequest {
            transcription_job_name: transcription_job_name.to_string(),
        };

        let _: EmptyResponse = self
            .make_authenticated_request(
                "com.amazonaws.transcribe.Transcribe.DeleteTranscriptionJob",
                &request_body,
                transcription_job_name.to_string(),
                "DeleteTranscriptionJob",
            )
            .await?;

        Ok(())
    }

    async fn get_transcription_job(
        &self,
        transcription_job_name: &str,
    ) -> Result<GetTranscriptionJobResponse, golem_stt::error::Error> {
        let request_body = GetTranscriptionJobRequest {
            transcription_job_name: transcription_job_name.to_string(),
        };

        self.make_authenticated_request(
            "com.amazonaws.transcribe.Transcribe.GetTranscriptionJob",
            &request_body,
            transcription_job_name.to_string(),
            "GetTranscriptionJob",
        )
        .await
    }

    async fn wait_for_transcription_job_completion(
        &self,
        transcription_job_name: &str,
        max_wait_time: Duration,
    ) -> Result<GetTranscriptionJobResponse, golem_stt::error::Error> {
        let start_time = std::time::Instant::now();
        let poll_interval = Duration::from_secs(10);

        loop {
            if start_time.elapsed() > max_wait_time {
                return Err(golem_stt::error::Error::APIBadRequest {
                    request_id: transcription_job_name.to_string(),
                    provider_error: "Transcription job timed out".to_string(),
                });
            }

            self.runtime.sleep(poll_interval).await;

            let res = self.get_transcription_job(transcription_job_name).await?;

            match res.transcription_job.transcription_job_status.as_str() {
                "COMPLETED" => {
                    trace!("transcription job {transcription_job_name} completed");
                    return Ok(res);
                }
                "FAILED" => {
                    trace!("tracription job {transcription_job_name} failed");
                    return Err(golem_stt::error::Error::APIBadRequest {
                        request_id: transcription_job_name.to_string(),
                        provider_error: format!(
                            "Transcription job failed: {}",
                            res.transcription_job
                                .failure_reason
                                .as_ref()
                                .unwrap_or(&"Unknown error".to_string())
                        ),
                    });
                }
                "IN_PROGRESS" | "QUEUED" => {
                    trace!("transcription job {transcription_job_name} waiting for completion");
                }
                other => {
                    return Err(golem_stt::error::Error::APIBadRequest {
                        request_id: transcription_job_name.to_string(),
                        provider_error: format!("Unexpected transcription job status: {other}"),
                    });
                }
            }
        }
    }

    async fn download_transcript_json(
        &self,
        transcription_job_name: &str,
        transcript_uri: &str,
    ) -> Result<TranscribeOutput, golem_stt::error::Error> {
        let request = http::Request::builder()
            .method("GET")
            .uri(transcript_uri)
            .header("Accept", "application/json")
            .body(Bytes::new())
            .map_err(|e| {
                (
                    transcription_job_name.to_string(),
                    golem_stt::http::Error::HttpError(e),
                )
            })?;

        let response = self
            .http_client
            .execute(request)
            .await
            .map_err(|err| (transcription_job_name.to_string(), err))?;

        if response.status().is_success() {
            let transcript_json: TranscribeOutput = serde_json::from_slice(response.body())
                .map_err(|e| {
                    golem_stt::error::Error::Http(
                        transcription_job_name.to_string(),
                        golem_stt::http::Error::Generic(format!(
                            "Failed to deserialize response: {e}",
                        )),
                    )
                })?;

            Ok(transcript_json)
        } else {
            let error_body = String::from_utf8(response.body().to_owned())
                .unwrap_or_else(|_| "Unknown error".to_string());

            let status = response.status();
            let request_id = transcription_job_name.to_string();

            // Map HTTP status codes based on S3 GET object behavior for transcript download
            match status {
                StatusCode::BAD_REQUEST => Err(golem_stt::error::Error::APIBadRequest {
                    request_id,
                    provider_error: format!("Transcript download bad request: {error_body}"),
                }),
                StatusCode::FORBIDDEN => Err(golem_stt::error::Error::APIForbidden {
                    request_id,
                    provider_error: format!("Transcript download forbidden (expired URL or insufficient permissions): {error_body}"),
                }),
                StatusCode::NOT_FOUND => Err(golem_stt::error::Error::APINotFound {
                    request_id,
                    provider_error: format!("Transcript file not found: {error_body}"),
                }),
                s if s.is_server_error() => Err(golem_stt::error::Error::APIInternalServerError {
                    request_id,
                    provider_error: format!("Transcript download server error ({status}): {error_body}"),
                }),
                _ => Err(golem_stt::error::Error::APIUnknown {
                    request_id,
                    provider_error: format!("Transcript download unknown error ({status}): {error_body}"),
                }),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        cell::{Ref, RefCell},
        collections::VecDeque,
    };

    use crate::transcription::request::{AudioFormat, DiarizationConfig};

    use super::*;
    use golem_stt::http::HttpClient;
    use http::{Request, Response, StatusCode};

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

    struct MockRuntime {
        sleep_calls: std::cell::RefCell<Vec<Duration>>,
    }

    impl MockRuntime {
        fn new() -> Self {
            Self {
                sleep_calls: std::cell::RefCell::new(Vec::new()),
            }
        }

        fn get_sleep_calls(&self) -> Vec<Duration> {
            self.sleep_calls.borrow().clone()
        }
    }

    impl golem_stt::runtime::AsyncRuntime for MockRuntime {
        async fn sleep(&self, duration: Duration) {
            self.sleep_calls.borrow_mut().push(duration);
        }
    }

    #[wstd::test]
    async fn test_transcribe_create_vocabulary_request() {
        let access_key = "AKIAIOSFODNN7EXAMPLE";
        let secret_key = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
        let region = "us-east-1";

        let expected_response = CreateVocabularyResponse {
            vocabulary_name: "test-vocabulary".to_string(),
            language_code: "en-US".to_string(),
            vocabulary_state: "PENDING".to_string(),
            last_modified_time: None,
            failure_reason: None,
        };

        let mock_client = MockHttpClient::new();

        let response_json_str = serde_json::to_string(&expected_response).unwrap();

        let body_bytes = response_json_str.into_bytes();

        mock_client.expect_response(
            Response::builder()
                .status(StatusCode::OK)
                .body(body_bytes)
                .unwrap(),
        );

        let mock_runtime = MockRuntime::new();

        let transcribe_client = TranscribeClient::new(
            access_key.to_string(),
            secret_key.to_string(),
            region.to_string(),
            mock_client,
            mock_runtime,
        );

        let vocabulary_name = "test-vocabulary".to_string();
        let language_code = "en-US".to_string();
        let phrases = vec![
            "hello world".to_string(),
            "machine learning".to_string(),
            "artificial intelligence".to_string(),
        ];

        let actual_response = transcribe_client
            .create_vocabulary(
                vocabulary_name.clone(),
                language_code.clone(),
                phrases.clone(),
            )
            .await
            .unwrap();

        assert_eq!(actual_response, expected_response);

        let request = transcribe_client
            .http_client
            .last_captured_request()
            .unwrap();
        assert_eq!(request.method(), "POST");
        assert_eq!(
            request.uri().to_string(),
            format!("https://transcribe.{region}.amazonaws.com/")
        );
        assert_eq!(
            request.headers().get("content-type").unwrap(),
            "application/x-amz-json-1.1"
        );
        assert_eq!(
            request.headers().get("x-amz-target").unwrap(),
            "com.amazonaws.transcribe.Transcribe.CreateVocabulary"
        );

        let expected_request = CreateVocabularyRequest {
            vocabulary_name,
            language_code,
            phrases,
            vocabulary_file_uri: None,
            data_access_role_arn: None,
            tags: None,
        };

        let actual_request: CreateVocabularyRequest =
            serde_json::from_slice(request.body()).unwrap();
        assert_eq!(actual_request, expected_request);

        assert!(request.headers().contains_key("x-amz-date"));
        assert!(request.headers().contains_key("x-amz-content-sha256"));
        assert!(request.headers().contains_key("authorization"));

        let auth_header = request
            .headers()
            .get("authorization")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(auth_header.starts_with("AWS4-HMAC-SHA256"));
        assert!(auth_header.contains("Credential="));
        assert!(auth_header.contains("SignedHeaders="));
        assert!(auth_header.contains("Signature="));
    }

    #[wstd::test]
    async fn test_transcribe_delete_vocabulary_request() {
        let access_key = "AKIAIOSFODNN7EXAMPLE";
        let secret_key = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
        let region = "us-east-1";

        let mock_client = MockHttpClient::new();
        mock_client.expect_response(
            Response::builder()
                .status(StatusCode::OK)
                .body(vec![])
                .unwrap(),
        );

        let mock_runtime = MockRuntime::new();

        let transcribe_client = TranscribeClient::new(
            access_key.to_string(),
            secret_key.to_string(),
            region.to_string(),
            mock_client,
            mock_runtime,
        );

        let vocabulary_name = "test-vocabulary".to_string();
        let result = transcribe_client.delete_vocabulary(&vocabulary_name).await;

        assert!(result.is_ok());

        let request = transcribe_client
            .http_client
            .last_captured_request()
            .unwrap();
        assert_eq!(request.method(), "POST");
        assert_eq!(
            request.uri().to_string(),
            format!("https://transcribe.{region}.amazonaws.com/")
        );
        assert_eq!(
            request.headers().get("content-type").unwrap(),
            "application/x-amz-json-1.1"
        );
        assert_eq!(
            request.headers().get("x-amz-target").unwrap(),
            "com.amazonaws.transcribe.Transcribe.DeleteVocabulary"
        );

        let expected_request = DeleteVocabularyRequest { vocabulary_name };

        let actual_request: DeleteVocabularyRequest =
            serde_json::from_slice(request.body()).unwrap();
        assert_eq!(actual_request, expected_request);

        assert!(request.headers().contains_key("x-amz-date"));
        assert!(request.headers().contains_key("x-amz-content-sha256"));
        assert!(request.headers().contains_key("authorization"));

        let auth_header = request
            .headers()
            .get("authorization")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(auth_header.starts_with("AWS4-HMAC-SHA256"));
        assert!(auth_header.contains("Credential="));
        assert!(auth_header.contains("SignedHeaders="));
        assert!(auth_header.contains("Signature="));
    }

    #[wstd::test]
    async fn test_start_transcription_job_basic_request() {
        let access_key = "AKIAIOSFODNN7EXAMPLE";
        let secret_key = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
        let region = "us-east-1";

        let mock_client = MockHttpClient::new();

        let mock_response = r#"{
               "TranscriptionJob": {
                   "TranscriptionJobName": "test-job-basic",
                   "TranscriptionJobStatus": "COMPLETED"
               }
           }"#;

        mock_client.expect_response(
            Response::builder()
                .status(200)
                .body(mock_response.as_bytes().to_vec())
                .unwrap(),
        );

        let mock_runtime = MockRuntime::new();

        let transcribe_client = TranscribeClient::new(
            access_key.to_string(),
            secret_key.to_string(),
            region.to_string(),
            mock_client,
            mock_runtime,
        );

        // Test basic audio config with no transcription config (should use identify_language)
        let audio_config = AudioConfig {
            format: AudioFormat::Wav,
            channels: Some(1),
        };

        let _result = transcribe_client
            .start_transcription_job(
                "test-job-basic",
                "s3://bucket/audio.wav",
                &audio_config,
                None, // No transcription config
                None, // No vocabulary
            )
            .await
            .unwrap();

        // Verify the request was constructed correctly
        let request = transcribe_client
            .http_client
            .last_captured_request()
            .unwrap();
        let actual_request: StartTranscriptionJobRequest =
            serde_json::from_slice(request.body()).unwrap();

        let expected_request = StartTranscriptionJobRequest {
            content_redaction: None,
            identify_language: Some(true), // Should be true when no language specified
            identify_multiple_languages: None,
            job_execution_settings: None,
            kms_encryption_context: None,
            language_code: None, // Should be None when using identify_language
            language_id_settings: None,
            language_options: None,
            media: Media {
                media_file_uri: "s3://bucket/audio.wav".to_string(),
                redacted_media_file_uri: None,
            },
            media_format: Some("wav".to_string()),
            media_sample_rate_hertz: None,
            model_settings: None, // Should be None when using identify_language
            output_bucket_name: None,
            output_encryption_kms_key_id: None,
            output_key: None,
            settings: None, // Should be None for basic single-channel audio
            subtitles: None,
            tags: None,
            toxicity_detection: None,
            transcription_job_name: "test-job-basic".to_string(),
        };

        assert_eq!(
            actual_request, expected_request,
            "Basic request should match expected structure"
        );
    }

    #[wstd::test]
    async fn test_start_transcription_job_with_explicit_language() {
        let access_key = "AKIAIOSFODNN7EXAMPLE";
        let secret_key = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
        let region = "us-west-2";

        let mock_client = MockHttpClient::new();

        let mock_response = r#"{
                "TranscriptionJob": {
                    "TranscriptionJobName": "test-job-lang",
                    "TranscriptionJobStatus": "IN_PROGRESS"
                }
            }"#;

        mock_client.expect_response(
            Response::builder()
                .status(200)
                .body(mock_response.as_bytes().to_vec())
                .unwrap(),
        );

        let mock_runtime = MockRuntime::new();

        let transcribe_client = TranscribeClient::new(
            access_key.to_string(),
            secret_key.to_string(),
            region.to_string(),
            mock_client,
            mock_runtime,
        );

        let audio_config = AudioConfig {
            format: AudioFormat::Mp3,
            channels: Some(1),
        };

        let transcription_config = TranscriptionConfig {
            language: Some("en-US".to_string()),
            model: None,
            diarization: None,
            enable_multi_channel: false,
            vocabulary: vec![],
        };

        let _result = transcribe_client
            .start_transcription_job(
                "test-job-lang",
                "s3://bucket/audio.mp3",
                &audio_config,
                Some(&transcription_config),
                None,
            )
            .await
            .unwrap();

        let request = transcribe_client
            .http_client
            .last_captured_request()
            .unwrap();
        let actual_request: StartTranscriptionJobRequest =
            serde_json::from_slice(request.body()).unwrap();

        let expected_request = StartTranscriptionJobRequest {
            content_redaction: None,
            identify_language: None, // Should be None when explicit language provided
            identify_multiple_languages: None,
            job_execution_settings: None,
            kms_encryption_context: None,
            language_code: Some("en-US".to_string()), // Should be set
            language_id_settings: None,
            language_options: None,
            media: Media {
                media_file_uri: "s3://bucket/audio.mp3".to_string(),
                redacted_media_file_uri: None,
            },
            media_format: Some("mp3".to_string()),
            media_sample_rate_hertz: None,
            model_settings: None, // No model specified
            output_bucket_name: None,
            output_encryption_kms_key_id: None,
            output_key: None,
            settings: None, // No settings should be set
            subtitles: None,
            tags: None,
            toxicity_detection: None,
            transcription_job_name: "test-job-lang".to_string(),
        };

        assert_eq!(
            actual_request, expected_request,
            "Request with explicit language should match expected structure"
        );
    }

    #[wstd::test]
    async fn test_start_transcription_job_with_model_and_vocabulary() {
        let access_key = "AKIAIOSFODNN7EXAMPLE";
        let secret_key = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
        let region = "eu-west-1";

        let mock_client = MockHttpClient::new();

        let mock_response = r#"{
               "TranscriptionJob": {
                   "TranscriptionJobName": "test-job-advanced",
                   "TranscriptionJobStatus": "COMPLETED"
               }
           }"#;

        mock_client.expect_response(
            Response::builder()
                .status(200)
                .body(mock_response.as_bytes().to_vec())
                .unwrap(),
        );

        let mock_runtime = MockRuntime::new();

        let transcribe_client = TranscribeClient::new(
            access_key.to_string(),
            secret_key.to_string(),
            region.to_string(),
            mock_client,
            mock_runtime,
        );

        let audio_config = AudioConfig {
            format: AudioFormat::Flac,
            channels: Some(1),
        };

        let transcription_config = TranscriptionConfig {
            language: Some("fr-FR".to_string()),
            model: Some("custom-medical-model".to_string()),
            diarization: None,
            enable_multi_channel: false,
            vocabulary: vec!["A".to_string(), "B".to_string()],
        };

        let _result = transcribe_client
            .start_transcription_job(
                "test-job-advanced",
                "s3://bucket/audio.flac",
                &audio_config,
                Some(&transcription_config),
                Some("custom-medical-vocab-123"), // Vocabulary name provided
            )
            .await
            .unwrap();

        let request = transcribe_client
            .http_client
            .last_captured_request()
            .unwrap();
        let actual_request: StartTranscriptionJobRequest =
            serde_json::from_slice(request.body()).unwrap();

        let expected_request = StartTranscriptionJobRequest {
            content_redaction: None,
            identify_language: None, // Explicit language provided
            identify_multiple_languages: None,
            job_execution_settings: None,
            kms_encryption_context: None,
            language_code: Some("fr-FR".to_string()),
            language_id_settings: None,
            language_options: None,
            media: Media {
                media_file_uri: "s3://bucket/audio.flac".to_string(),
                redacted_media_file_uri: None,
            },
            media_format: Some("flac".to_string()),
            media_sample_rate_hertz: None,
            model_settings: Some(ModelSettings {
                language_model_name: "custom-medical-model".to_string(),
            }),
            output_bucket_name: None,
            output_encryption_kms_key_id: None,
            output_key: None,
            settings: Some(Settings {
                channel_identification: None,
                max_alternatives: None,
                max_speaker_labels: None,
                show_alternatives: None,
                show_speaker_labels: None,
                vocabulary_filter_method: None,
                vocabulary_filter_name: None,
                vocabulary_name: Some("custom-medical-vocab-123".to_string()),
            }),
            subtitles: None,
            tags: None,
            toxicity_detection: None,
            transcription_job_name: "test-job-advanced".to_string(),
        };

        assert_eq!(
            actual_request, expected_request,
            "Request with model and vocabulary should match expected structure"
        );
    }

    #[wstd::test]
    async fn test_start_transcription_job_with_speaker_diarization() {
        let access_key = "AKIAIOSFODNN7EXAMPLE";
        let secret_key = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
        let region = "ap-southeast-1";

        let mock_client = MockHttpClient::new();

        let mock_response = r#"{
                "TranscriptionJob": {
                    "TranscriptionJobName": "test-job-speakers",
                    "TranscriptionJobStatus": "IN_PROGRESS"
                }
            }"#;

        mock_client.expect_response(
            Response::builder()
                .status(200)
                .body(mock_response.as_bytes().to_vec())
                .unwrap(),
        );

        let mock_runtime = MockRuntime::new();

        let transcribe_client = TranscribeClient::new(
            access_key.to_string(),
            secret_key.to_string(),
            region.to_string(),
            mock_client,
            mock_runtime,
        );

        let audio_config = AudioConfig {
            format: AudioFormat::Ogg,
            channels: Some(1), // Single channel
        };

        let transcription_config = TranscriptionConfig {
            language: Some("en-AU".to_string()),
            model: None,
            diarization: Some(DiarizationConfig {
                enabled: true,
                max_speakers: 2,
            }),
            enable_multi_channel: false,
            vocabulary: vec![],
        };

        let _result = transcribe_client
            .start_transcription_job(
                "test-job-speakers",
                "s3://bucket/meeting.ogg",
                &audio_config,
                Some(&transcription_config),
                None,
            )
            .await
            .unwrap();

        let request = transcribe_client
            .http_client
            .last_captured_request()
            .unwrap();
        let actual_request: StartTranscriptionJobRequest =
            serde_json::from_slice(request.body()).unwrap();

        let expected_request = StartTranscriptionJobRequest {
            content_redaction: None,
            identify_language: None,
            identify_multiple_languages: None,
            job_execution_settings: None,
            kms_encryption_context: None,
            language_code: Some("en-AU".to_string()),
            language_id_settings: None,
            language_options: None,
            media: Media {
                media_file_uri: "s3://bucket/meeting.ogg".to_string(),
                redacted_media_file_uri: None,
            },
            media_format: Some("ogg".to_string()),
            media_sample_rate_hertz: None,
            model_settings: None,
            output_bucket_name: None,
            output_encryption_kms_key_id: None,
            output_key: None,
            settings: Some(Settings {
                channel_identification: None, // Single channel
                max_alternatives: None,
                max_speaker_labels: Some(2), // Set for speaker diarization
                show_alternatives: None,
                show_speaker_labels: Some(true), // Enable speaker labels
                vocabulary_filter_method: None,
                vocabulary_filter_name: None,
                vocabulary_name: None,
            }),
            subtitles: None,
            tags: None,
            toxicity_detection: None,
            transcription_job_name: "test-job-speakers".to_string(),
        };

        assert_eq!(
            actual_request, expected_request,
            "Request with speaker diarization should match expected structure"
        );
    }

    #[wstd::test]
    async fn test_start_transcription_job_with_multi_channel() {
        let access_key = "AKIAIOSFODNN7EXAMPLE";
        let secret_key = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
        let region = "ca-central-1";

        let mock_client = MockHttpClient::new();

        let mock_response = r#"{
                "TranscriptionJob": {
                    "TranscriptionJobName": "test-job-channels",
                    "TranscriptionJobStatus": "COMPLETED"
                }
            }"#;

        mock_client.expect_response(
            Response::builder()
                .status(200)
                .body(mock_response.as_bytes().to_vec())
                .unwrap(),
        );

        let mock_runtime = MockRuntime::new();

        let transcribe_client = TranscribeClient::new(
            access_key.to_string(),
            secret_key.to_string(),
            region.to_string(),
            mock_client,
            mock_runtime,
        );

        let audio_config = AudioConfig {
            format: AudioFormat::Wav,
            channels: Some(2), // Multi-channel audio
        };

        let transcription_config = TranscriptionConfig {
            language: Some("en-CA".to_string()),
            model: Some("telephony-model".to_string()),
            diarization: Some(DiarizationConfig {
                enabled: true,
                max_speakers: 4,
            }),
            enable_multi_channel: true, // Both multi-channel and speaker diarization
            vocabulary: vec!["A".to_string(), "B".to_string()],
        };

        let _result = transcribe_client
            .start_transcription_job(
                "test-job-channels",
                "s3://bucket/call-recording.wav",
                &audio_config,
                Some(&transcription_config),
                Some("telephony-vocab"),
            )
            .await
            .unwrap();

        let request = transcribe_client
            .http_client
            .last_captured_request()
            .unwrap();
        let actual_request: StartTranscriptionJobRequest =
            serde_json::from_slice(request.body()).unwrap();

        let expected_request = StartTranscriptionJobRequest {
            content_redaction: None,
            identify_language: None,
            identify_multiple_languages: None,
            job_execution_settings: None,
            kms_encryption_context: None,
            language_code: Some("en-CA".to_string()),
            language_id_settings: None,
            language_options: None,
            media: Media {
                media_file_uri: "s3://bucket/call-recording.wav".to_string(),
                redacted_media_file_uri: None,
            },
            media_format: Some("wav".to_string()),
            media_sample_rate_hertz: None,
            model_settings: Some(ModelSettings {
                language_model_name: "telephony-model".to_string(),
            }),
            output_bucket_name: None,
            output_encryption_kms_key_id: None,
            output_key: None,
            settings: Some(Settings {
                channel_identification: Some(true), // Multi-channel
                max_alternatives: None,
                max_speaker_labels: Some(4), // Speaker diarization
                show_alternatives: None,
                show_speaker_labels: Some(true), // Speaker diarization
                vocabulary_filter_method: None,
                vocabulary_filter_name: None,
                vocabulary_name: Some("telephony-vocab".to_string()),
            }),
            subtitles: None,
            tags: None,
            toxicity_detection: None,
            transcription_job_name: "test-job-channels".to_string(),
        };

        assert_eq!(
            actual_request, expected_request,
            "Request with multi-channel and speaker diarization should match expected structure"
        );
    }

    #[wstd::test]
    async fn test_start_transcription_job_identify_language_ignores_vocabulary_and_model_settings()
    {
        let access_key = "AKIAIOSFODNN7EXAMPLE";
        let secret_key = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
        let region = "us-east-1";

        let mock_client = MockHttpClient::new();

        let mock_response = r#"{
               "TranscriptionJob": {
                   "TranscriptionJobName": "test-job-identify",
                   "TranscriptionJobStatus": "IN_PROGRESS"
               }
           }"#;

        mock_client.expect_response(
            Response::builder()
                .status(200)
                .body(mock_response.as_bytes().to_vec())
                .unwrap(),
        );

        let mock_runtime = MockRuntime::new();

        let transcribe_client = TranscribeClient::new(
            access_key.to_string(),
            secret_key.to_string(),
            region.to_string(),
            mock_client,
            mock_runtime,
        );

        let audio_config = AudioConfig {
            format: AudioFormat::Mp3,
            channels: Some(1),
        };

        let transcription_config = TranscriptionConfig {
            language: None, // means identify language
            model: Some("telephony-model".to_string()),
            diarization: None,
            enable_multi_channel: false,
            vocabulary: vec!["A".to_string(), "B".to_string()],
        };

        let _result = transcribe_client
            .start_transcription_job(
                "test-job-identify",
                "s3://bucket/unknown-language.mp3",
                &audio_config,
                Some(&transcription_config),
                Some("some-vocab"), // This should be ignored
            )
            .await
            .unwrap();

        let request = transcribe_client
            .http_client
            .last_captured_request()
            .unwrap();
        let actual_request: StartTranscriptionJobRequest =
            serde_json::from_slice(request.body()).unwrap();

        let expected_request = StartTranscriptionJobRequest {
            content_redaction: None,
            identify_language: Some(true), // Should be true
            identify_multiple_languages: None,
            job_execution_settings: None,
            kms_encryption_context: None,
            language_code: None, // Should be None
            language_id_settings: None,
            language_options: None,
            media: Media {
                media_file_uri: "s3://bucket/unknown-language.mp3".to_string(),
                redacted_media_file_uri: None,
            },
            media_format: Some("mp3".to_string()),
            media_sample_rate_hertz: None,
            model_settings: None, // Should be None for identify_language
            output_bucket_name: None,
            output_encryption_kms_key_id: None,
            output_key: None,
            settings: None, // Vocabulary should be ignored, so settings should be None
            subtitles: None,
            tags: None,
            toxicity_detection: None,
            transcription_job_name: "test-job-identify".to_string(),
        };

        assert_eq!(
            actual_request, expected_request,
            "Request with identify_language should ignore vocabulary and model settings"
        );
    }

    #[wstd::test]
    async fn test_transcribe_delete_transcription_job_request() {
        let access_key = "AKIAIOSFODNN7EXAMPLE";
        let secret_key = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
        let region = "us-east-1";

        let mock_client = MockHttpClient::new();
        mock_client.expect_response(
            Response::builder()
                .status(StatusCode::OK)
                .body(vec![])
                .unwrap(),
        );

        let mock_runtime = MockRuntime::new();

        let transcribe_client = TranscribeClient::new(
            access_key.to_string(),
            secret_key.to_string(),
            region.to_string(),
            mock_client,
            mock_runtime,
        );

        let transcription_job_name = "test-job-name".to_string();
        let result = transcribe_client
            .delete_transcription_job(&transcription_job_name)
            .await;

        assert!(result.is_ok());

        let request = transcribe_client
            .http_client
            .last_captured_request()
            .unwrap();
        assert_eq!(request.method(), "POST");
        assert_eq!(
            request.uri().to_string(),
            format!("https://transcribe.{region}.amazonaws.com/")
        );
        assert_eq!(
            request.headers().get("content-type").unwrap(),
            "application/x-amz-json-1.1"
        );
        assert_eq!(
            request.headers().get("x-amz-target").unwrap(),
            "com.amazonaws.transcribe.Transcribe.DeleteTranscriptionJob"
        );

        let expected_request = DeleteTranscriptionJobRequest {
            transcription_job_name,
        };

        let actual_request: DeleteTranscriptionJobRequest =
            serde_json::from_slice(request.body()).unwrap();
        assert_eq!(actual_request, expected_request);

        assert!(request.headers().contains_key("x-amz-date"));
        assert!(request.headers().contains_key("x-amz-content-sha256"));
        assert!(request.headers().contains_key("authorization"));

        let auth_header = request
            .headers()
            .get("authorization")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(auth_header.starts_with("AWS4-HMAC-SHA256"));
        assert!(auth_header.contains("Credential="));
        assert!(auth_header.contains("SignedHeaders="));
        assert!(auth_header.contains("Signature="));
    }

    #[wstd::test]
    async fn test_wait_for_vocabulary_ready_success() {
        let access_key = "AKIAIOSFODNN7EXAMPLE";
        let secret_key = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
        let region = "us-east-1";

        let mock_client = MockHttpClient::new();

        // First call - vocabulary is still PENDING
        let pending_response = r#"{
               "VocabularyName": "test-vocab",
               "LanguageCode": "en-US",
               "VocabularyState": "PENDING",
               "LastModifiedTime": 1234567890.0
           }"#;

        // Second call - vocabulary is READY
        let ready_response = r#"{
               "VocabularyName": "test-vocab",
               "LanguageCode": "en-US",
               "VocabularyState": "READY",
               "LastModifiedTime": 1234567891.0,
               "DownloadUri": "s3://bucket/vocab.txt"
           }"#;

        mock_client.expect_response(
            Response::builder()
                .status(200)
                .body(pending_response.as_bytes().to_vec())
                .unwrap(),
        );

        mock_client.expect_response(
            Response::builder()
                .status(200)
                .body(ready_response.as_bytes().to_vec())
                .unwrap(),
        );

        let mock_runtime = MockRuntime::new();

        let transcribe_client = TranscribeClient::new(
            access_key.to_string(),
            secret_key.to_string(),
            region.to_string(),
            mock_client,
            mock_runtime,
        );

        transcribe_client
            .wait_for_vocabulary_ready("test-vocab", Duration::from_secs(300))
            .await
            .unwrap();

        assert_eq!(transcribe_client.http_client.captured_request_count(), 2);

        let sleep_calls = transcribe_client.runtime.get_sleep_calls();
        assert!(
            !sleep_calls.is_empty(),
            "Should have called sleep at least once"
        );

        let captured_requests = transcribe_client.http_client.get_captured_requests();
        for request in captured_requests.iter() {
            assert_eq!(request.method(), "POST");
            assert_eq!(
                request.headers().get("x-amz-target").unwrap(),
                "com.amazonaws.transcribe.Transcribe.GetVocabulary"
            );

            assert!(request.headers().contains_key("x-amz-date"));
            assert!(request.headers().contains_key("x-amz-content-sha256"));
            assert!(request.headers().contains_key("authorization"));

            let auth_header = request
                .headers()
                .get("authorization")
                .unwrap()
                .to_str()
                .unwrap();
            assert!(auth_header.starts_with("AWS4-HMAC-SHA256"));
            assert!(auth_header.contains("Credential="));
            assert!(auth_header.contains("SignedHeaders="));
            assert!(auth_header.contains("Signature="));

            let request_body: GetVocabularyRequest =
                serde_json::from_slice(request.body()).unwrap();
            assert_eq!(request_body.vocabulary_name, "test-vocab");
        }
    }

    #[wstd::test]
    async fn test_wait_for_vocabulary_ready_failure() {
        let access_key = "AKIAIOSFODNN7EXAMPLE";
        let secret_key = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
        let region = "us-east-1";

        let mock_client = MockHttpClient::new();

        // Vocabulary creation failed
        let failed_response = r#"{
                "VocabularyName": "test-vocab",
                "LanguageCode": "en-US",
                "VocabularyState": "FAILED",
                "LastModifiedTime": 1234567890.0,
                "FailureReason": "Invalid vocabulary format"
            }"#;

        mock_client.expect_response(
            Response::builder()
                .status(200)
                .body(failed_response.as_bytes().to_vec())
                .unwrap(),
        );

        let mock_runtime = MockRuntime::new();

        let transcribe_client = TranscribeClient::new(
            access_key.to_string(),
            secret_key.to_string(),
            region.to_string(),
            mock_client,
            mock_runtime,
        );

        let result = transcribe_client
            .wait_for_vocabulary_ready("test-vocab", Duration::from_secs(300))
            .await;

        // Should fail with the specific error
        assert!(result.is_err());
        let error = result.unwrap_err();
        match error {
            golem_stt::error::Error::APIBadRequest {
                request_id,
                provider_error,
            } => {
                assert_eq!(request_id, "test-vocab");
                assert!(provider_error.contains("Invalid vocabulary format"));
            }
            _ => panic!("Expected APIBadRequest error"),
        }
    }

    #[wstd::test]
    async fn test_wait_for_vocabulary_ready_timeout() {
        let access_key = "AKIAIOSFODNN7EXAMPLE";
        let secret_key = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
        let region = "us-east-1";

        let mock_client = MockHttpClient::new();

        // Always return PENDING to simulate timeout
        let pending_response = r#"{
                "VocabularyName": "test-vocab",
                "LanguageCode": "en-US",
                "VocabularyState": "PENDING",
                "LastModifiedTime": 1234567890.0
            }"#;

        // Add multiple responses to allow polling before timeout
        for _ in 0..100 {
            mock_client.expect_response(
                Response::builder()
                    .status(200)
                    .body(pending_response.as_bytes().to_vec())
                    .unwrap(),
            );
        }

        struct MockRuntime {
            elapsed_time: std::cell::RefCell<Duration>,
        }

        impl MockRuntime {
            fn new() -> Self {
                Self {
                    elapsed_time: std::cell::RefCell::new(Duration::from_secs(0)),
                }
            }
        }

        impl golem_stt::runtime::AsyncRuntime for MockRuntime {
            async fn sleep(&self, duration: Duration) {
                // Simulate time passing
                let mut elapsed = self.elapsed_time.borrow_mut();
                *elapsed += duration;
            }
        }

        let mock_runtime = MockRuntime::new();

        let transcribe_client = TranscribeClient::new(
            access_key.to_string(),
            secret_key.to_string(),
            region.to_string(),
            mock_client,
            mock_runtime,
        );

        let result = transcribe_client
            .wait_for_vocabulary_ready(
                "test-vocab",
                Duration::from_millis(5), // Very short timeout
            )
            .await;

        assert!(
            transcribe_client.runtime.elapsed_time.borrow().as_millis() > 0,
            "Elapsed time should be greater than zero"
        );

        assert!(result.is_err());
        let error = result.unwrap_err();
        match error {
            golem_stt::error::Error::APIBadRequest {
                request_id,
                provider_error,
            } => {
                assert_eq!(request_id, "test-vocab");
                assert!(provider_error.contains("timed out"));
            }
            _ => panic!("Expected APIBadRequest timeout error"),
        }
    }

    #[wstd::test]
    async fn test_wait_for_transcription_job_completion_success() {
        let access_key = "AKIAIOSFODNN7EXAMPLE";
        let secret_key = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
        let region = "us-east-1";

        let mock_client = MockHttpClient::new();

        // First call - job is IN_PROGRESS
        let in_progress_response = r#"{
                "TranscriptionJob": {
                    "TranscriptionJobName": "test-job",
                    "TranscriptionJobStatus": "IN_PROGRESS",
                    "LanguageCode": "en-US",
                    "Media": {
                        "MediaFileUri": "s3://bucket/audio.wav"
                    },
                    "MediaFormat": "wav",
                    "CreationTime": 1234567890.0,
                    "StartTime": 1234567891.0
                }
            }"#;

        // Second call - job is COMPLETED
        let completed_response = r#"{
                "TranscriptionJob": {
                    "TranscriptionJobName": "test-job",
                    "TranscriptionJobStatus": "COMPLETED",
                    "LanguageCode": "en-US",
                    "Media": {
                        "MediaFileUri": "s3://bucket/audio.wav"
                    },
                    "MediaFormat": "wav",
                    "CreationTime": 1234567890.0,
                    "CompletionTime": 1234567920.0,
                    "StartTime": 1234567891.0,
                    "Transcript": {
                        "TranscriptFileUri": "s3://output/transcript.json"
                    }
                }
            }"#;

        mock_client.expect_response(
            Response::builder()
                .status(200)
                .body(in_progress_response.as_bytes().to_vec())
                .unwrap(),
        );

        mock_client.expect_response(
            Response::builder()
                .status(200)
                .body(completed_response.as_bytes().to_vec())
                .unwrap(),
        );

        let mock_runtime = MockRuntime::new();

        let transcribe_client = TranscribeClient::new(
            access_key.to_string(),
            secret_key.to_string(),
            region.to_string(),
            mock_client,
            mock_runtime,
        );

        let response = transcribe_client
            .wait_for_transcription_job_completion("test-job", Duration::from_secs(3600))
            .await
            .unwrap();

        assert_eq!(
            response.transcription_job.transcription_job_status,
            "COMPLETED"
        );
        assert!(response.transcription_job.transcript.is_some());
        assert!(response
            .transcription_job
            .transcript
            .unwrap()
            .transcript_file_uri
            .is_some());

        // Should have made exactly 2 API calls
        assert_eq!(transcribe_client.http_client.captured_request_count(), 2);

        // Should have called sleep at least once
        let sleep_calls = transcribe_client.runtime.get_sleep_calls();
        assert!(!sleep_calls.is_empty());

        // Verify the requests were get_transcription_job calls
        let captured_requests = transcribe_client.http_client.get_captured_requests();
        for request in captured_requests.iter() {
            assert_eq!(request.method(), "POST");
            assert_eq!(
                request.headers().get("x-amz-target").unwrap(),
                "com.amazonaws.transcribe.Transcribe.GetTranscriptionJob"
            );

            assert!(request.headers().contains_key("x-amz-date"));
            assert!(request.headers().contains_key("x-amz-content-sha256"));
            assert!(request.headers().contains_key("authorization"));

            let auth_header = request
                .headers()
                .get("authorization")
                .unwrap()
                .to_str()
                .unwrap();
            assert!(auth_header.starts_with("AWS4-HMAC-SHA256"));
            assert!(auth_header.contains("Credential="));
            assert!(auth_header.contains("SignedHeaders="));
            assert!(auth_header.contains("Signature="));

            let request_body: GetTranscriptionJobRequest =
                serde_json::from_slice(request.body()).unwrap();
            assert_eq!(request_body.transcription_job_name, "test-job");
        }
    }

    #[wstd::test]
    async fn test_wait_for_transcription_job_completion_failure() {
        let access_key = "AKIAIOSFODNN7EXAMPLE";
        let secret_key = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
        let region = "us-east-1";

        let mock_client = MockHttpClient::new();

        // Job failed
        let failed_response = r#"{
                "TranscriptionJob": {
                    "TranscriptionJobName": "test-job",
                    "TranscriptionJobStatus": "FAILED",
                    "LanguageCode": "en-US",
                    "Media": {
                        "MediaFileUri": "s3://bucket/audio.wav"
                    },
                    "MediaFormat": "wav",
                    "CreationTime": 1234567890.0,
                    "CompletionTime": 1234567920.0,
                    "StartTime": 1234567891.0,
                    "FailureReason": "Unsupported audio format"
                }
            }"#;

        mock_client.expect_response(
            Response::builder()
                .status(200)
                .body(failed_response.as_bytes().to_vec())
                .unwrap(),
        );

        let mock_runtime = MockRuntime::new();

        let transcribe_client = TranscribeClient::new(
            access_key.to_string(),
            secret_key.to_string(),
            region.to_string(),
            mock_client,
            mock_runtime,
        );

        let result = transcribe_client
            .wait_for_transcription_job_completion("test-job", Duration::from_secs(3600))
            .await;

        // Should fail with the specific error
        assert!(result.is_err());
        let error = result.unwrap_err();
        match error {
            golem_stt::error::Error::APIBadRequest {
                request_id,
                provider_error,
            } => {
                assert_eq!(request_id, "test-job");
                assert!(provider_error.contains("Unsupported audio format"));
            }
            _ => panic!("Expected APIBadRequest error"),
        }
    }

    #[wstd::test]
    async fn test_wait_for_transcription_job_completion_timeout() {
        let access_key = "AKIAIOSFODNN7EXAMPLE";
        let secret_key = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
        let region = "us-east-1";

        let mock_client = MockHttpClient::new();

        // Always return IN_PROGRESS to simulate timeout
        let in_progress_response = r#"{
            "TranscriptionJob": {
                "TranscriptionJobName": "test-job",
                "TranscriptionJobStatus": "IN_PROGRESS",
                "LanguageCode": "en-US",
                "Media": {
                    "MediaFileUri": "s3://bucket/audio.wav"
                },
                "MediaFormat": "wav",
                "CreationTime": 1234567890.0,
                "StartTime": 1234567891.0
            }
        }"#;

        // Add multiple responses to allow polling before timeout
        for _ in 0..100 {
            mock_client.expect_response(
                Response::builder()
                    .status(200)
                    .body(in_progress_response.as_bytes().to_vec())
                    .unwrap(),
            );
        }

        struct MockRuntime {
            elapsed_time: std::cell::RefCell<Duration>,
        }

        impl MockRuntime {
            fn new() -> Self {
                Self {
                    elapsed_time: std::cell::RefCell::new(Duration::from_secs(0)),
                }
            }
        }

        impl golem_stt::runtime::AsyncRuntime for MockRuntime {
            async fn sleep(&self, duration: Duration) {
                // Simulate time passing
                let mut elapsed = self.elapsed_time.borrow_mut();
                *elapsed += duration;
            }
        }

        let mock_runtime = MockRuntime::new();

        let transcribe_client = TranscribeClient::new(
            access_key.to_string(),
            secret_key.to_string(),
            region.to_string(),
            mock_client,
            mock_runtime,
        );

        let result = transcribe_client
            .wait_for_transcription_job_completion(
                "test-job",
                Duration::from_millis(5), // Very short timeout
            )
            .await;

        assert!(
            transcribe_client.runtime.elapsed_time.borrow().as_millis() > 0,
            "Elapsed time should be greater than zero"
        );

        assert!(result.is_err());
        let error = result.unwrap_err();
        match error {
            golem_stt::error::Error::APIBadRequest {
                request_id,
                provider_error,
            } => {
                assert_eq!(request_id, "test-job");
                assert!(provider_error.contains("timed out"));
            }
            _ => panic!("Expected APIBadRequest timeout error"),
        }
    }

    #[wstd::test]
    async fn test_download_transcript_json_with_diarization() {
        let access_key = "AKIAIOSFODNN7EXAMPLE";
        let secret_key = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
        let region = "us-east-1";

        let json_response = r#"{
            "jobName": "my-first-transcription-job",
            "accountId": "111122223333",
            "results": {
                "transcripts": [
                    {
                        "transcript": "I've been on hold for an hour. Sorry about that."
                    }
                ],
                "speaker_labels": {
                    "channel_label": "ch_0",
                    "speakers": 2,
                    "segments": [
                        {
                            "start_time": "4.87",
                            "speaker_label": "spk_0",
                            "end_time": "6.88",
                            "items": [
                                {
                                    "start_time": "4.87",
                                    "speaker_label": "spk_0",
                                    "end_time": "5.02"
                                },
                                {
                                    "start_time": "5.02",
                                    "speaker_label": "spk_0",
                                    "end_time": "5.17"
                                },
                                {
                                    "start_time": "5.17",
                                    "speaker_label": "spk_0",
                                    "end_time": "5.29"
                                },
                                {
                                    "start_time": "5.29",
                                    "speaker_label": "spk_0",
                                    "end_time": "5.64"
                                },
                                {
                                    "start_time": "5.64",
                                    "speaker_label": "spk_0",
                                    "end_time": "5.84"
                                },
                                {
                                    "start_time": "6.11",
                                    "speaker_label": "spk_0",
                                    "end_time": "6.26"
                                },
                                {
                                    "start_time": "6.26",
                                    "speaker_label": "spk_0",
                                    "end_time": "6.88"
                                }
                            ]
                        },
                        {
                            "start_time": "8.49",
                            "speaker_label": "spk_1",
                            "end_time": "9.24",
                            "items": [
                                {
                                    "start_time": "8.49",
                                    "speaker_label": "spk_1",
                                    "end_time": "8.88"
                                },
                                {
                                    "start_time": "8.88",
                                    "speaker_label": "spk_1",
                                    "end_time": "9.05"
                                },
                                {
                                    "start_time": "9.05",
                                    "speaker_label": "spk_1",
                                    "end_time": "9.24"
                                }
                            ]
                        }
                    ]
                },
                "items": [
                    {
                        "id": 0,
                        "start_time": "4.87",
                        "speaker_label": "spk_0",
                        "end_time": "5.02",
                        "alternatives": [
                            {
                                "confidence": "1.0",
                                "content": "I've"
                            }
                        ],
                        "type": "pronunciation"
                    },
                    {
                        "id": 1,
                        "start_time": "5.02",
                        "speaker_label": "spk_0",
                        "end_time": "5.17",
                        "alternatives": [
                            {
                                "confidence": "1.0",
                                "content": "been"
                            }
                        ],
                        "type": "pronunciation"
                    },
                    {
                        "id": 2,
                        "start_time": "5.17",
                        "speaker_label": "spk_0",
                        "end_time": "5.29",
                        "alternatives": [
                            {
                                "confidence": "1.0",
                                "content": "on"
                            }
                        ],
                        "type": "pronunciation"
                    },
                    {
                        "id": 3,
                        "start_time": "5.29",
                        "speaker_label": "spk_0",
                        "end_time": "5.64",
                        "alternatives": [
                            {
                                "confidence": "1.0",
                                "content": "hold"
                            }
                        ],
                        "type": "pronunciation"
                    },
                    {
                        "id": 4,
                        "start_time": "5.64",
                        "speaker_label": "spk_0",
                        "end_time": "5.84",
                        "alternatives": [
                            {
                                "confidence": "1.0",
                                "content": "for"
                            }
                        ],
                        "type": "pronunciation"
                    },
                    {
                        "id": 5,
                        "start_time": "6.11",
                        "speaker_label": "spk_0",
                        "end_time": "6.26",
                        "alternatives": [
                            {
                                "confidence": "1.0",
                                "content": "an"
                            }
                        ],
                        "type": "pronunciation"
                    },
                    {
                        "id": 6,
                        "start_time": "6.26",
                        "speaker_label": "spk_0",
                        "end_time": "6.88",
                        "alternatives": [
                            {
                                "confidence": "1.0",
                                "content": "hour"
                            }
                        ],
                        "type": "pronunciation"
                    },
                    {
                        "id": 7,
                        "speaker_label": "spk_0",
                        "alternatives": [
                            {
                                "confidence": "0.0",
                                "content": "."
                            }
                        ],
                        "type": "punctuation"
                    },
                    {
                        "id": 8,
                        "start_time": "8.49",
                        "speaker_label": "spk_1",
                        "end_time": "8.88",
                        "alternatives": [
                            {
                                "confidence": "1.0",
                                "content": "Sorry"
                            }
                        ],
                        "type": "pronunciation"
                    },
                    {
                        "id": 9,
                        "start_time": "8.88",
                        "speaker_label": "spk_1",
                        "end_time": "9.05",
                        "alternatives": [
                            {
                                "confidence": "0.902",
                                "content": "about"
                            }
                        ],
                        "type": "pronunciation"
                    },
                    {
                        "id": 10,
                        "start_time": "9.05",
                        "speaker_label": "spk_1",
                        "end_time": "9.24",
                        "alternatives": [
                            {
                                "confidence": "1.0",
                                "content": "that"
                            }
                        ],
                        "type": "pronunciation"
                    },
                    {
                        "id": 11,
                        "speaker_label": "spk_1",
                        "alternatives": [
                            {
                                "confidence": "0.0",
                                "content": "."
                            }
                        ],
                        "type": "punctuation"
                    }
                ],
                "audio_segments": [
                    {
                        "id": 0,
                        "transcript": "I've been on hold for an hour.",
                        "start_time": "4.87",
                        "end_time": "6.88",
                        "speaker_label": "spk_0",
                        "items": [0, 1, 2, 3, 4, 5, 6, 7]
                    },
                    {
                        "id": 1,
                        "transcript": "Sorry about that.",
                        "start_time": "8.49",
                        "end_time": "9.24",
                        "speaker_label": "spk_1",
                        "items": [8, 9, 10, 11]
                    }
                ]
            },
            "status": "COMPLETED"
        }"#;

        let expected = TranscribeOutput {
            job_name: "my-first-transcription-job".to_string(),
            account_id: "111122223333".to_string(),
            results: TranscribeResults {
                transcripts: vec![TranscriptText {
                    transcript: "I've been on hold for an hour. Sorry about that.".to_string(),
                }],
                speaker_labels: Some(SpeakerLabels {
                    channel_label: Some("ch_0".to_string()),
                    speakers: 2,
                    segments: vec![
                        SpeakerSegment {
                            start_time: "4.87".to_string(),
                            speaker_label: "spk_0".to_string(),
                            end_time: "6.88".to_string(),
                            items: Some(vec![
                                SpeakerItem {
                                    start_time: "4.87".to_string(),
                                    speaker_label: "spk_0".to_string(),
                                    end_time: "5.02".to_string(),
                                },
                                SpeakerItem {
                                    start_time: "5.02".to_string(),
                                    speaker_label: "spk_0".to_string(),
                                    end_time: "5.17".to_string(),
                                },
                                SpeakerItem {
                                    start_time: "5.17".to_string(),
                                    speaker_label: "spk_0".to_string(),
                                    end_time: "5.29".to_string(),
                                },
                                SpeakerItem {
                                    start_time: "5.29".to_string(),
                                    speaker_label: "spk_0".to_string(),
                                    end_time: "5.64".to_string(),
                                },
                                SpeakerItem {
                                    start_time: "5.64".to_string(),
                                    speaker_label: "spk_0".to_string(),
                                    end_time: "5.84".to_string(),
                                },
                                SpeakerItem {
                                    start_time: "6.11".to_string(),
                                    speaker_label: "spk_0".to_string(),
                                    end_time: "6.26".to_string(),
                                },
                                SpeakerItem {
                                    start_time: "6.26".to_string(),
                                    speaker_label: "spk_0".to_string(),
                                    end_time: "6.88".to_string(),
                                },
                            ]),
                        },
                        SpeakerSegment {
                            start_time: "8.49".to_string(),
                            speaker_label: "spk_1".to_string(),
                            end_time: "9.24".to_string(),
                            items: Some(vec![
                                SpeakerItem {
                                    start_time: "8.49".to_string(),
                                    speaker_label: "spk_1".to_string(),
                                    end_time: "8.88".to_string(),
                                },
                                SpeakerItem {
                                    start_time: "8.88".to_string(),
                                    speaker_label: "spk_1".to_string(),
                                    end_time: "9.05".to_string(),
                                },
                                SpeakerItem {
                                    start_time: "9.05".to_string(),
                                    speaker_label: "spk_1".to_string(),
                                    end_time: "9.24".to_string(),
                                },
                            ]),
                        },
                    ],
                }),
                items: vec![
                    TranscribeItem {
                        id: Some(0),
                        start_time: Some("4.87".to_string()),
                        end_time: Some("5.02".to_string()),
                        speaker_label: Some("spk_0".to_string()),
                        channel_label: None,
                        alternatives: vec![TranscribeAlternative {
                            confidence: "1.0".to_string(),
                            content: "I've".to_string(),
                        }],
                        item_type: "pronunciation".to_string(),
                        vocabulary_filter_match: None,
                    },
                    TranscribeItem {
                        id: Some(1),
                        start_time: Some("5.02".to_string()),
                        end_time: Some("5.17".to_string()),
                        speaker_label: Some("spk_0".to_string()),
                        channel_label: None,
                        alternatives: vec![TranscribeAlternative {
                            confidence: "1.0".to_string(),
                            content: "been".to_string(),
                        }],
                        item_type: "pronunciation".to_string(),
                        vocabulary_filter_match: None,
                    },
                    TranscribeItem {
                        id: Some(2),
                        start_time: Some("5.17".to_string()),
                        end_time: Some("5.29".to_string()),
                        speaker_label: Some("spk_0".to_string()),
                        channel_label: None,
                        alternatives: vec![TranscribeAlternative {
                            confidence: "1.0".to_string(),
                            content: "on".to_string(),
                        }],
                        item_type: "pronunciation".to_string(),
                        vocabulary_filter_match: None,
                    },
                    TranscribeItem {
                        id: Some(3),
                        start_time: Some("5.29".to_string()),
                        end_time: Some("5.64".to_string()),
                        speaker_label: Some("spk_0".to_string()),
                        channel_label: None,
                        alternatives: vec![TranscribeAlternative {
                            confidence: "1.0".to_string(),
                            content: "hold".to_string(),
                        }],
                        item_type: "pronunciation".to_string(),
                        vocabulary_filter_match: None,
                    },
                    TranscribeItem {
                        id: Some(4),
                        start_time: Some("5.64".to_string()),
                        end_time: Some("5.84".to_string()),
                        speaker_label: Some("spk_0".to_string()),
                        channel_label: None,
                        alternatives: vec![TranscribeAlternative {
                            confidence: "1.0".to_string(),
                            content: "for".to_string(),
                        }],
                        item_type: "pronunciation".to_string(),
                        vocabulary_filter_match: None,
                    },
                    TranscribeItem {
                        id: Some(5),
                        start_time: Some("6.11".to_string()),
                        end_time: Some("6.26".to_string()),
                        speaker_label: Some("spk_0".to_string()),
                        channel_label: None,
                        alternatives: vec![TranscribeAlternative {
                            confidence: "1.0".to_string(),
                            content: "an".to_string(),
                        }],
                        item_type: "pronunciation".to_string(),
                        vocabulary_filter_match: None,
                    },
                    TranscribeItem {
                        id: Some(6),
                        start_time: Some("6.26".to_string()),
                        end_time: Some("6.88".to_string()),
                        speaker_label: Some("spk_0".to_string()),
                        channel_label: None,
                        alternatives: vec![TranscribeAlternative {
                            confidence: "1.0".to_string(),
                            content: "hour".to_string(),
                        }],
                        item_type: "pronunciation".to_string(),
                        vocabulary_filter_match: None,
                    },
                    TranscribeItem {
                        id: Some(7),
                        start_time: None,
                        end_time: None,
                        speaker_label: Some("spk_0".to_string()),
                        channel_label: None,
                        alternatives: vec![TranscribeAlternative {
                            confidence: "0.0".to_string(),
                            content: ".".to_string(),
                        }],
                        item_type: "punctuation".to_string(),
                        vocabulary_filter_match: None,
                    },
                    TranscribeItem {
                        id: Some(8),
                        start_time: Some("8.49".to_string()),
                        end_time: Some("8.88".to_string()),
                        speaker_label: Some("spk_1".to_string()),
                        channel_label: None,
                        alternatives: vec![TranscribeAlternative {
                            confidence: "1.0".to_string(),
                            content: "Sorry".to_string(),
                        }],
                        item_type: "pronunciation".to_string(),
                        vocabulary_filter_match: None,
                    },
                    TranscribeItem {
                        id: Some(9),
                        start_time: Some("8.88".to_string()),
                        end_time: Some("9.05".to_string()),
                        speaker_label: Some("spk_1".to_string()),
                        channel_label: None,
                        alternatives: vec![TranscribeAlternative {
                            confidence: "0.902".to_string(),
                            content: "about".to_string(),
                        }],
                        item_type: "pronunciation".to_string(),
                        vocabulary_filter_match: None,
                    },
                    TranscribeItem {
                        id: Some(10),
                        start_time: Some("9.05".to_string()),
                        end_time: Some("9.24".to_string()),
                        speaker_label: Some("spk_1".to_string()),
                        channel_label: None,
                        alternatives: vec![TranscribeAlternative {
                            confidence: "1.0".to_string(),
                            content: "that".to_string(),
                        }],
                        item_type: "pronunciation".to_string(),
                        vocabulary_filter_match: None,
                    },
                    TranscribeItem {
                        id: Some(11),
                        start_time: None,
                        end_time: None,
                        speaker_label: Some("spk_1".to_string()),
                        channel_label: None,
                        alternatives: vec![TranscribeAlternative {
                            confidence: "0.0".to_string(),
                            content: ".".to_string(),
                        }],
                        item_type: "punctuation".to_string(),
                        vocabulary_filter_match: None,
                    },
                ],
                audio_segments: vec![
                    AudioSegment {
                        id: 0,
                        transcript: "I've been on hold for an hour.".to_string(),
                        start_time: "4.87".to_string(),
                        end_time: "6.88".to_string(),
                        speaker_label: Some("spk_0".to_string()),
                        channel_label: None,
                        items: vec![0, 1, 2, 3, 4, 5, 6, 7],
                    },
                    AudioSegment {
                        id: 1,
                        transcript: "Sorry about that.".to_string(),
                        start_time: "8.49".to_string(),
                        end_time: "9.24".to_string(),
                        speaker_label: Some("spk_1".to_string()),
                        channel_label: None,
                        items: vec![8, 9, 10, 11],
                    },
                ],
                channel_labels: None,
            },
            status: "COMPLETED".to_string(),
        };

        let mock_client = MockHttpClient::new();
        mock_client.expect_response(
            http::Response::builder()
                .status(200)
                .body(json_response.as_bytes().to_vec())
                .unwrap(),
        );

        let mock_runtime = MockRuntime::new();

        let transcribe_client = TranscribeClient::new(
            access_key.to_string(),
            secret_key.to_string(),
            region.to_string(),
            mock_client,
            mock_runtime,
        );

        let result = transcribe_client
            .download_transcript_json("test-job", "https://example.com/transcript.json")
            .await
            .expect("Failed to download transcript");

        assert_eq!(result, expected);
    }

    #[wstd::test]
    async fn test_download_transcript_json_with_multi_channel() {
        let access_key = "AKIAIOSFODNN7EXAMPLE";
        let secret_key = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
        let region = "us-east-1";

        let json_response = r#"{
        "jobName": "my-first-transcription-job",
        "accountId": "111122223333",
        "results": {
            "transcripts": [
                {
                    "transcript": "I've been on hold for an hour. Sorry about that."
                }
            ],
            "channel_labels": {
                "channels": [
                    {
                        "channel_label": "ch_0",
                        "items": [
                            {
                                "channel_label": "ch_0",
                                "start_time": "4.86",
                                "end_time": "5.01",
                                "alternatives": [
                                    {
                                        "confidence": "1.0",
                                        "content": "I've"
                                    }
                                ],
                                "type": "pronunciation"
                            },
                            {
                                "channel_label": "ch_0",
                                "start_time": "5.01",
                                "end_time": "5.16",
                                "alternatives": [
                                    {
                                        "confidence": "1.0",
                                        "content": "been"
                                    }
                                ],
                                "type": "pronunciation"
                            },
                            {
                                "channel_label": "ch_0",
                                "start_time": "5.16",
                                "end_time": "5.28",
                                "alternatives": [
                                    {
                                        "confidence": "1.0",
                                        "content": "on"
                                    }
                                ],
                                "type": "pronunciation"
                            },
                            {
                                "channel_label": "ch_0",
                                "start_time": "5.28",
                                "end_time": "5.62",
                                "alternatives": [
                                    {
                                        "confidence": "1.0",
                                        "content": "hold"
                                    }
                                ],
                                "type": "pronunciation"
                            },
                            {
                                "channel_label": "ch_0",
                                "start_time": "5.62",
                                "end_time": "5.83",
                                "alternatives": [
                                    {
                                        "confidence": "1.0",
                                        "content": "for"
                                    }
                                ],
                                "type": "pronunciation"
                            },
                            {
                                "channel_label": "ch_0",
                                "start_time": "6.1",
                                "end_time": "6.25",
                                "alternatives": [
                                    {
                                        "confidence": "1.0",
                                        "content": "an"
                                    }
                                ],
                                "type": "pronunciation"
                            },
                            {
                                "channel_label": "ch_0",
                                "start_time": "6.25",
                                "end_time": "6.87",
                                "alternatives": [
                                    {
                                        "confidence": "1.0",
                                        "content": "hour"
                                    }
                                ],
                                "type": "pronunciation"
                            },
                            {
                                "channel_label": "ch_0",
                                "language_code": "en-US",
                                "alternatives": [
                                    {
                                        "confidence": "0.0",
                                        "content": "."
                                    }
                                ],
                                "type": "punctuation"
                            }
                        ]
                    },
                    {
                    "channel_label": "ch_1",
                        "items": [
                            {
                                "channel_label": "ch_1",
                                "start_time": "8.5",
                                "end_time": "8.89",
                                "alternatives": [
                                    {
                                        "confidence": "1.0",
                                        "content": "Sorry"
                                    }
                                ],
                                "type": "pronunciation"
                            },
                            {
                                "channel_label": "ch_1",
                                "start_time": "8.89",
                                "end_time": "9.06",
                                "alternatives": [
                                    {
                                        "confidence": "0.9176",
                                        "content": "about"
                                    }
                                ],
                                "type": "pronunciation"
                            },
                            {
                                "channel_label": "ch_1",
                                "start_time": "9.06",
                                "end_time": "9.25",
                                "alternatives": [
                                    {
                                        "confidence": "1.0",
                                        "content": "that"
                                    }
                                ],
                                "type": "pronunciation"
                            },
                            {
                                "channel_label": "ch_1",
                                "alternatives": [
                                    {
                                        "confidence": "0.0",
                                        "content": "."
                                    }
                                ],
                                "type": "punctuation"
                            }
                        ]
                    }
                ],
                "number_of_channels": 2
            },
            "items": [
                {
                    "id": 0,
                    "channel_label": "ch_0",
                    "start_time": "4.86",
                    "end_time": "5.01",
                    "alternatives": [
                        {
                            "confidence": "1.0",
                            "content": "I've"
                        }
                    ],
                    "type": "pronunciation"
                },
                {
                    "id": 1,
                    "channel_label": "ch_0",
                    "start_time": "5.01",
                    "end_time": "5.16",
                    "alternatives": [
                        {
                            "confidence": "1.0",
                            "content": "been"
                        }
                    ],
                    "type": "pronunciation"
                },
                {
                    "id": 2,
                    "channel_label": "ch_0",
                    "start_time": "5.16",
                    "end_time": "5.28",
                    "alternatives": [
                        {
                            "confidence": "1.0",
                            "content": "on"
                        }
                    ],
                    "type": "pronunciation"
                },
                {
                    "id": 3,
                    "channel_label": "ch_0",
                    "start_time": "5.28",
                    "end_time": "5.62",
                    "alternatives": [
                        {
                            "confidence": "1.0",
                            "content": "hold"
                        }
                    ],
                    "type": "pronunciation"
                },
                {
                    "id": 4,
                    "channel_label": "ch_0",
                    "start_time": "5.62",
                    "end_time": "5.83",
                    "alternatives": [
                        {
                            "confidence": "1.0",
                            "content": "for"
                        }
                    ],
                    "type": "pronunciation"
                },
                {
                    "id": 5,
                    "channel_label": "ch_0",
                    "start_time": "6.1",
                    "end_time": "6.25",
                    "alternatives": [
                        {
                            "confidence": "1.0",
                            "content": "an"
                        }
                    ],
                    "type": "pronunciation"
                },
                {
                    "id": 6,
                    "channel_label": "ch_0",
                    "start_time": "6.25",
                    "end_time": "6.87",
                    "alternatives": [
                        {
                            "confidence": "1.0",
                            "content": "hour"
                        }
                    ],
                    "type": "pronunciation"
                },
                {
                    "id": 7,
                    "channel_label": "ch_0",
                    "alternatives": [
                        {
                            "confidence": "0.0",
                            "content": "."
                        }
                    ],
                    "type": "punctuation"
                },
                {
                    "id": 8,
                    "channel_label": "ch_1",
                    "start_time": "8.5",
                    "end_time": "8.89",
                    "alternatives": [
                        {
                            "confidence": "1.0",
                            "content": "Sorry"
                        }
                    ],
                    "type": "pronunciation"
                },
                {
                    "id": 9,
                    "channel_label": "ch_1",
                    "start_time": "8.89",
                    "end_time": "9.06",
                    "alternatives": [
                        {
                            "confidence": "0.9176",
                            "content": "about"
                        }
                    ],
                    "type": "pronunciation"
                },
                {
                    "id": 10,
                    "channel_label": "ch_1",
                    "start_time": "9.06",
                    "end_time": "9.25",
                    "alternatives": [
                        {
                            "confidence": "1.0",
                            "content": "that"
                        }
                    ],
                    "type": "pronunciation"
                },
                {
                    "id": 11,
                    "channel_label": "ch_1",
                    "alternatives": [
                        {
                            "confidence": "0.0",
                            "content": "."
                        }
                    ],
                    "type": "punctuation"
                }
            ],
            "audio_segments": [
                {
                    "id": 0,
                    "transcript": "I've been on hold for an hour.",
                    "start_time": "4.86",
                    "end_time": "6.87",
                    "channel_label": "ch_0",
                    "items": [
                        0,
                        1,
                        2,
                        3,
                        4,
                        5,
                        6,
                        7
                    ]
                },
                {
                    "id": 1,
                    "transcript": "Sorry about that.",
                    "start_time": "8.5",
                    "end_time": "9.25",
                    "channel_label": "ch_1",
                    "items": [
                        8,
                        9,
                        10,
                        11
                    ]
                }
            ]
        },
        "status": "COMPLETED"
    }"#;

        let expected = TranscribeOutput {
            job_name: "my-first-transcription-job".to_string(),
            account_id: "111122223333".to_string(),
            results: TranscribeResults {
                transcripts: vec![TranscriptText {
                    transcript: "I've been on hold for an hour. Sorry about that.".to_string(),
                }],
                speaker_labels: None,
                channel_labels: Some(ChannelLabels {
                    channels: vec![
                        Channel {
                            channel_label: "ch_0".to_string(),
                            items: vec![
                                TranscribeItem {
                                    id: None,
                                    start_time: Some("4.86".to_string()),
                                    end_time: Some("5.01".to_string()),
                                    speaker_label: None,
                                    channel_label: Some("ch_0".to_string()),
                                    alternatives: vec![TranscribeAlternative {
                                        confidence: "1.0".to_string(),
                                        content: "I've".to_string(),
                                    }],
                                    item_type: "pronunciation".to_string(),
                                    vocabulary_filter_match: None,
                                },
                                TranscribeItem {
                                    id: None,
                                    start_time: Some("5.01".to_string()),
                                    end_time: Some("5.16".to_string()),
                                    speaker_label: None,
                                    channel_label: Some("ch_0".to_string()),
                                    alternatives: vec![TranscribeAlternative {
                                        confidence: "1.0".to_string(),
                                        content: "been".to_string(),
                                    }],
                                    item_type: "pronunciation".to_string(),
                                    vocabulary_filter_match: None,
                                },
                                TranscribeItem {
                                    id: None,
                                    start_time: Some("5.16".to_string()),
                                    end_time: Some("5.28".to_string()),
                                    speaker_label: None,
                                    channel_label: Some("ch_0".to_string()),
                                    alternatives: vec![TranscribeAlternative {
                                        confidence: "1.0".to_string(),
                                        content: "on".to_string(),
                                    }],
                                    item_type: "pronunciation".to_string(),
                                    vocabulary_filter_match: None,
                                },
                                TranscribeItem {
                                    id: None,
                                    start_time: Some("5.28".to_string()),
                                    end_time: Some("5.62".to_string()),
                                    speaker_label: None,
                                    channel_label: Some("ch_0".to_string()),
                                    alternatives: vec![TranscribeAlternative {
                                        confidence: "1.0".to_string(),
                                        content: "hold".to_string(),
                                    }],
                                    item_type: "pronunciation".to_string(),
                                    vocabulary_filter_match: None,
                                },
                                TranscribeItem {
                                    id: None,
                                    start_time: Some("5.62".to_string()),
                                    end_time: Some("5.83".to_string()),
                                    speaker_label: None,
                                    channel_label: Some("ch_0".to_string()),
                                    alternatives: vec![TranscribeAlternative {
                                        confidence: "1.0".to_string(),
                                        content: "for".to_string(),
                                    }],
                                    item_type: "pronunciation".to_string(),
                                    vocabulary_filter_match: None,
                                },
                                TranscribeItem {
                                    id: None,
                                    start_time: Some("6.1".to_string()),
                                    end_time: Some("6.25".to_string()),
                                    speaker_label: None,
                                    channel_label: Some("ch_0".to_string()),
                                    alternatives: vec![TranscribeAlternative {
                                        confidence: "1.0".to_string(),
                                        content: "an".to_string(),
                                    }],
                                    item_type: "pronunciation".to_string(),
                                    vocabulary_filter_match: None,
                                },
                                TranscribeItem {
                                    id: None,
                                    start_time: Some("6.25".to_string()),
                                    end_time: Some("6.87".to_string()),
                                    speaker_label: None,
                                    channel_label: Some("ch_0".to_string()),
                                    alternatives: vec![TranscribeAlternative {
                                        confidence: "1.0".to_string(),
                                        content: "hour".to_string(),
                                    }],
                                    item_type: "pronunciation".to_string(),
                                    vocabulary_filter_match: None,
                                },
                                TranscribeItem {
                                    id: None,
                                    start_time: None,
                                    end_time: None,
                                    speaker_label: None,
                                    channel_label: Some("ch_0".to_string()),
                                    alternatives: vec![TranscribeAlternative {
                                        confidence: "0.0".to_string(),
                                        content: ".".to_string(),
                                    }],
                                    item_type: "punctuation".to_string(),
                                    vocabulary_filter_match: None,
                                },
                            ],
                        },
                        Channel {
                            channel_label: "ch_1".to_string(),
                            items: vec![
                                TranscribeItem {
                                    id: None,
                                    start_time: Some("8.5".to_string()),
                                    end_time: Some("8.89".to_string()),
                                    speaker_label: None,
                                    channel_label: Some("ch_1".to_string()),
                                    alternatives: vec![TranscribeAlternative {
                                        confidence: "1.0".to_string(),
                                        content: "Sorry".to_string(),
                                    }],
                                    item_type: "pronunciation".to_string(),
                                    vocabulary_filter_match: None,
                                },
                                TranscribeItem {
                                    id: None,
                                    start_time: Some("8.89".to_string()),
                                    end_time: Some("9.06".to_string()),
                                    speaker_label: None,
                                    channel_label: Some("ch_1".to_string()),
                                    alternatives: vec![TranscribeAlternative {
                                        confidence: "0.9176".to_string(),
                                        content: "about".to_string(),
                                    }],
                                    item_type: "pronunciation".to_string(),
                                    vocabulary_filter_match: None,
                                },
                                TranscribeItem {
                                    id: None,
                                    start_time: Some("9.06".to_string()),
                                    end_time: Some("9.25".to_string()),
                                    speaker_label: None,
                                    channel_label: Some("ch_1".to_string()),
                                    alternatives: vec![TranscribeAlternative {
                                        confidence: "1.0".to_string(),
                                        content: "that".to_string(),
                                    }],
                                    item_type: "pronunciation".to_string(),
                                    vocabulary_filter_match: None,
                                },
                                TranscribeItem {
                                    id: None,
                                    start_time: None,
                                    end_time: None,
                                    speaker_label: None,
                                    channel_label: Some("ch_1".to_string()),
                                    alternatives: vec![TranscribeAlternative {
                                        confidence: "0.0".to_string(),
                                        content: ".".to_string(),
                                    }],
                                    item_type: "punctuation".to_string(),
                                    vocabulary_filter_match: None,
                                },
                            ],
                        },
                    ],
                    number_of_channels: 2,
                }),
                items: vec![
                    TranscribeItem {
                        id: Some(0),
                        start_time: Some("4.86".to_string()),
                        end_time: Some("5.01".to_string()),
                        speaker_label: None,
                        channel_label: Some("ch_0".to_string()),
                        alternatives: vec![TranscribeAlternative {
                            confidence: "1.0".to_string(),
                            content: "I've".to_string(),
                        }],
                        item_type: "pronunciation".to_string(),
                        vocabulary_filter_match: None,
                    },
                    TranscribeItem {
                        id: Some(1),
                        start_time: Some("5.01".to_string()),
                        end_time: Some("5.16".to_string()),
                        speaker_label: None,
                        channel_label: Some("ch_0".to_string()),
                        alternatives: vec![TranscribeAlternative {
                            confidence: "1.0".to_string(),
                            content: "been".to_string(),
                        }],
                        item_type: "pronunciation".to_string(),
                        vocabulary_filter_match: None,
                    },
                    TranscribeItem {
                        id: Some(2),
                        start_time: Some("5.16".to_string()),
                        end_time: Some("5.28".to_string()),
                        speaker_label: None,
                        channel_label: Some("ch_0".to_string()),
                        alternatives: vec![TranscribeAlternative {
                            confidence: "1.0".to_string(),
                            content: "on".to_string(),
                        }],
                        item_type: "pronunciation".to_string(),
                        vocabulary_filter_match: None,
                    },
                    TranscribeItem {
                        id: Some(3),
                        start_time: Some("5.28".to_string()),
                        end_time: Some("5.62".to_string()),
                        speaker_label: None,
                        channel_label: Some("ch_0".to_string()),
                        alternatives: vec![TranscribeAlternative {
                            confidence: "1.0".to_string(),
                            content: "hold".to_string(),
                        }],
                        item_type: "pronunciation".to_string(),
                        vocabulary_filter_match: None,
                    },
                    TranscribeItem {
                        id: Some(4),
                        start_time: Some("5.62".to_string()),
                        end_time: Some("5.83".to_string()),
                        speaker_label: None,
                        channel_label: Some("ch_0".to_string()),
                        alternatives: vec![TranscribeAlternative {
                            confidence: "1.0".to_string(),
                            content: "for".to_string(),
                        }],
                        item_type: "pronunciation".to_string(),
                        vocabulary_filter_match: None,
                    },
                    TranscribeItem {
                        id: Some(5),
                        start_time: Some("6.1".to_string()),
                        end_time: Some("6.25".to_string()),
                        speaker_label: None,
                        channel_label: Some("ch_0".to_string()),
                        alternatives: vec![TranscribeAlternative {
                            confidence: "1.0".to_string(),
                            content: "an".to_string(),
                        }],
                        item_type: "pronunciation".to_string(),
                        vocabulary_filter_match: None,
                    },
                    TranscribeItem {
                        id: Some(6),
                        start_time: Some("6.25".to_string()),
                        end_time: Some("6.87".to_string()),
                        speaker_label: None,
                        channel_label: Some("ch_0".to_string()),
                        alternatives: vec![TranscribeAlternative {
                            confidence: "1.0".to_string(),
                            content: "hour".to_string(),
                        }],
                        item_type: "pronunciation".to_string(),
                        vocabulary_filter_match: None,
                    },
                    TranscribeItem {
                        id: Some(7),
                        start_time: None,
                        end_time: None,
                        speaker_label: None,
                        channel_label: Some("ch_0".to_string()),
                        alternatives: vec![TranscribeAlternative {
                            confidence: "0.0".to_string(),
                            content: ".".to_string(),
                        }],
                        item_type: "punctuation".to_string(),
                        vocabulary_filter_match: None,
                    },
                    TranscribeItem {
                        id: Some(8),
                        start_time: Some("8.5".to_string()),
                        end_time: Some("8.89".to_string()),
                        speaker_label: None,
                        channel_label: Some("ch_1".to_string()),
                        alternatives: vec![TranscribeAlternative {
                            confidence: "1.0".to_string(),
                            content: "Sorry".to_string(),
                        }],
                        item_type: "pronunciation".to_string(),
                        vocabulary_filter_match: None,
                    },
                    TranscribeItem {
                        id: Some(9),
                        start_time: Some("8.89".to_string()),
                        end_time: Some("9.06".to_string()),
                        speaker_label: None,
                        channel_label: Some("ch_1".to_string()),
                        alternatives: vec![TranscribeAlternative {
                            confidence: "0.9176".to_string(),
                            content: "about".to_string(),
                        }],
                        item_type: "pronunciation".to_string(),
                        vocabulary_filter_match: None,
                    },
                    TranscribeItem {
                        id: Some(10),
                        start_time: Some("9.06".to_string()),
                        end_time: Some("9.25".to_string()),
                        speaker_label: None,
                        channel_label: Some("ch_1".to_string()),
                        alternatives: vec![TranscribeAlternative {
                            confidence: "1.0".to_string(),
                            content: "that".to_string(),
                        }],
                        item_type: "pronunciation".to_string(),
                        vocabulary_filter_match: None,
                    },
                    TranscribeItem {
                        id: Some(11),
                        start_time: None,
                        end_time: None,
                        speaker_label: None,
                        channel_label: Some("ch_1".to_string()),
                        alternatives: vec![TranscribeAlternative {
                            confidence: "0.0".to_string(),
                            content: ".".to_string(),
                        }],
                        item_type: "punctuation".to_string(),
                        vocabulary_filter_match: None,
                    },
                ],
                audio_segments: vec![
                    AudioSegment {
                        id: 0,
                        transcript: "I've been on hold for an hour.".to_string(),
                        start_time: "4.86".to_string(),
                        end_time: "6.87".to_string(),
                        speaker_label: None,
                        channel_label: Some("ch_0".to_string()),
                        items: vec![0, 1, 2, 3, 4, 5, 6, 7],
                    },
                    AudioSegment {
                        id: 1,
                        transcript: "Sorry about that.".to_string(),
                        start_time: "8.5".to_string(),
                        end_time: "9.25".to_string(),
                        speaker_label: None,
                        channel_label: Some("ch_1".to_string()),
                        items: vec![8, 9, 10, 11],
                    },
                ],
            },
            status: "COMPLETED".to_string(),
        };

        let mock_client = MockHttpClient::new();
        mock_client.expect_response(
            http::Response::builder()
                .status(200)
                .body(json_response.as_bytes().to_vec())
                .unwrap(),
        );

        let mock_runtime = MockRuntime::new();

        // Create transcribe client
        let transcribe_client = TranscribeClient::new(
            access_key.to_string(),
            secret_key.to_string(),
            region.to_string(),
            mock_client,
            mock_runtime,
        );

        let result = transcribe_client
            .download_transcript_json("test-job", "https://example.com/transcript.json")
            .await
            .expect("Failed to download transcript");

        assert_eq!(result, expected);
    }
}
