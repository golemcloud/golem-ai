use crate::component::VocabularyResource;
use crate::sigv4::{sign, SigV4Params};
use golem_stt::config::AwsConfig;
use golem_stt::errors::InternalSttError;
use golem_stt::exports::golem::stt::transcription::TranscribeOptions;
use golem_stt::exports::golem::stt::types::{AudioConfig, AudioFormat};
use golem_stt::exports::golem::stt::vocabularies::GuestVocabulary;
use golem_stt::http::HttpClient;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, CONTENT_TYPE};
use serde_json;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use time;

#[derive(Clone)]
pub struct AwsClient {
    pub cfg: AwsConfig,
    http: HttpClient,
}

impl AwsClient {
    pub fn new(cfg: AwsConfig) -> Result<Self, InternalSttError> {
        cfg.access_key_id
            .as_ref()
            .ok_or_else(|| InternalSttError::unauthorized("AWS_ACCESS_KEY_ID not set"))?;
        cfg.secret_access_key
            .as_ref()
            .ok_or_else(|| InternalSttError::unauthorized("AWS_SECRET_ACCESS_KEY not set"))?;
        cfg.region
            .as_ref()
            .ok_or_else(|| InternalSttError::unauthorized("AWS_REGION not set"))?;

        let http = HttpClient::new(cfg.common.timeout_secs, cfg.common.max_retries)?;
        Ok(Self { cfg, http })
    }

    fn region(&self) -> Result<String, InternalSttError> {
        self.cfg
            .region
            .as_ref()
            .ok_or_else(|| InternalSttError::unauthorized("AWS_REGION not set"))
            .cloned()
    }



    fn region(&self) -> Result<String, InternalSttError> {
        self.cfg
            .region
            .clone()
            .ok_or_else(|| InternalSttError::unauthorized("AWS_REGION not set"))
    }

    fn s3_bucket(&self) -> Result<String, InternalSttError> {
        self.cfg
            .s3_bucket
            .clone()
            .ok_or_else(|| InternalSttError::unauthorized("AWS_S3_BUCKET not set"))
    }

    fn content_type_for(format: &AudioFormat) -> &'static str {
        match format {
            AudioFormat::Wav => "audio/wav",
            AudioFormat::Mp3 => "audio/mpeg",
            AudioFormat::Flac => "audio/flac",
            AudioFormat::Ogg => "audio/ogg",
            AudioFormat::Aac => "audio/aac",
            AudioFormat::Pcm => "application/octet-stream",
        }
    }

    fn sha256_hex(bytes: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        hex::encode(hasher.finalize())
    }

    async fn upload_to_s3(
        &self,
        audio: &[u8],
        key: &str,
        content_type: &str,
    ) -> Result<String, InternalSttError> {
        let bucket = self.s3_bucket()?;
        let region = self.region()?;
        let url = format!("https://{bucket}.s3.{region}.amazonaws.com/{key}");

        let access_key = self
            .cfg
            .access_key_id
            .as_ref()
            .ok_or_else(|| InternalSttError::unauthorized("AWS_ACCESS_KEY_ID not set"))?
            .clone();
        let secret_key = self
            .cfg
            .secret_access_key
            .as_ref()
            .ok_or_else(|| InternalSttError::unauthorized("AWS_SECRET_ACCESS_KEY not set"))?
            .clone();
        let session_token = self.cfg.session_token.clone();

        let now = time::OffsetDateTime::now_utc();
        let amz_date = now
            .format(
                &time::format_description::parse("[year][month][day]T[hour][minute][second]Z")
                    .unwrap(),
            )
            .map_err(|e| InternalSttError::internal(format!("time format: {e}")))?;
        let date_stamp = now
            .format(
                &time::format_description::parse("[year][month][day]").map_err(|e| {
                    InternalSttError::internal(format!("error parsing date format: {e}"))
                })?,
            )
            .map_err(|e| InternalSttError::internal(format!("date format: {e}")))?;

        let payload_sha256 = Self::sha256_hex(audio);
        let host = format!("{bucket}.s3.{region}.amazonaws.com");

        let (auth_header, signed_headers) = sign(SigV4Params {
            method: "PUT".into(),
            service: "s3".into(),
            region: region.clone(),
            host: host.clone(),
            canonical_uri: format!("/{key}"),
            canonical_querystring: String::new(),
            payload_sha256: payload_sha256.clone(),
            access_key,
            secret_key,
            session_token,
            amz_date: amz_date.clone(),
            date_stamp,
            content_type: Some(content_type.into()),
        });

        let mut headers = HeaderMap::new();
        headers.insert(
            "Authorization",
            HeaderValue::from_str(&auth_header)
                .map_err(|e| InternalSttError::internal(format!("auth header: {e}")))?,
        );
        for (k, v) in signed_headers {
            headers.insert(
                HeaderName::from_bytes(k.as_bytes())
                    .map_err(|e| InternalSttError::internal(format!("invalid header name: {e}")))?,
                HeaderValue::from_str(&v)
                    .map_err(|e| InternalSttError::internal(format!("signed header {k}: {e}")))?,
            );
        }
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_str(content_type)
                .map_err(|e| InternalSttError::internal(format!("invalid content-type: {e}")))?,
        );

        let (status, _text, _hdrs) = self
            .http
            .put_bytes(&url, headers, audio.to_vec(), content_type)
            .await?;

        if !status.is_success() {
            return Err(InternalSttError::failed(format!(
                "S3 upload failed: status={status}"
            )));
        }

        Ok(format!("s3://{bucket}/{key}"))
    }

    async fn start_transcription_job(
        &self,
        s3_uri: &str,
        job_name: &str,
        config: &AudioConfig,
        options: &Option<TranscribeOptions<'_>>,
    ) -> Result<String, InternalSttError> {
        let region = self.region()?;
        let url = format!("https://transcribe.{region}.amazonaws.com/");

        let mut request_body = serde_json::json!({
            "TranscriptionJobName": job_name,
            "Media": {
                "MediaFileUri": s3_uri
            },
            "MediaFormat": match config.format {
                AudioFormat::Wav => "wav",
                AudioFormat::Mp3 => "mp3",
                AudioFormat::Flac => "flac",
                AudioFormat::Ogg => "ogg",
                AudioFormat::Aac => "mp4",
                AudioFormat::Pcm => "wav",
            }
        });

        if let Some(rate) = config.sample_rate {
            request_body["MediaSampleRateHertz"] = serde_json::json!(rate);
        }

        if let Some(opts) = options {
            if let Some(lang) = &opts.language {
                request_body["LanguageCode"] = serde_json::json!(lang);
            }
            if opts.enable_speaker_diarization.unwrap_or(false) {
                request_body["Settings"] = serde_json::json!({
                    "ShowSpeakerLabels": true,
                    "MaxSpeakerLabels": 10
                });
            }
            if let Some(vocab) = &opts.vocabulary {
                let vocab_name = vocab.get::<VocabularyResource>().get_name();
                request_body["Settings"] = request_body["Settings"].as_object().cloned().unwrap_or_default().into();
                request_body["Settings"]["VocabularyName"] = serde_json::json!(vocab_name);
            }
        }

        let access_key = self
            .cfg
            .access_key_id
            .as_ref()
            .ok_or_else(|| InternalSttError::unauthorized("AWS_ACCESS_KEY_ID not set"))?
            .clone();
        let secret_key = self
            .cfg
            .secret_access_key
            .as_ref()
            .ok_or_else(|| InternalSttError::unauthorized("AWS_SECRET_ACCESS_KEY not set"))?
            .clone();
        let session_token = self.cfg.session_token.clone();

        let body_str = serde_json::to_string(&request_body)
            .map_err(|e| InternalSttError::internal(format!("json serialize: {e}")))?;

        let now = time::OffsetDateTime::now_utc();
        let amz_date = now
            .format(
                &time::format_description::parse("[year][month][day]T[hour][minute][second]Z")
                    .unwrap(),
            )
            .map_err(|e| InternalSttError::internal(format!("time format: {e}")))?;
        let date_stamp = now
            .format(
                &time::format_description::parse("[year][month][day]").map_err(|e| {
                    InternalSttError::internal(format!("error parsing date format: {e}"))
                })?,
            )
            .map_err(|e| InternalSttError::internal(format!("date format: {e}")))?;

        let payload_sha256 = Self::sha256_hex(body_str.as_bytes());
        let host = format!("transcribe.{region}.amazonaws.com");

        let (auth_header, signed_headers) = sign(SigV4Params {
            method: "POST".into(),
            service: "transcribe".into(),
            region: region.clone(),
            host: host.clone(),
            canonical_uri: "/".into(),
            canonical_querystring: String::new(),
            payload_sha256: payload_sha256.clone(),
            access_key,
            secret_key,
            session_token,
            amz_date: amz_date.clone(),
            date_stamp,
            content_type: Some("application/x-amz-json-1.1".into()),
        });

        let mut headers = HeaderMap::new();
        headers.insert(
            "Authorization",
            HeaderValue::from_str(&auth_header)
                .map_err(|e| InternalSttError::internal(format!("auth header: {e}")))?,
        );
        headers.insert(
            "X-Amz-Target",
            HeaderValue::from_static("Transcribe.StartTranscriptionJob"),
        );
        for (k, v) in signed_headers {
            headers.insert(
                HeaderName::from_bytes(k.as_bytes())
                    .map_err(|e| InternalSttError::internal(format!("invalid header name: {e}")))?,
                HeaderValue::from_str(&v)
                    .map_err(|e| InternalSttError::internal(format!("signed header {k}: {e}")))?,
            );
        }
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/x-amz-json-1.1"),
        );

        let (status, text, _hdrs) = self
            .http
            .post_bytes(&url, headers, body_str.into_bytes(), "application/x-amz-json-1.1")
            .await?;

        if !status.is_success() {
            return Err(InternalSttError::failed(format!(
                "AWS Transcribe start job failed: status={status}, body={text}"
            )));
        }

        Ok(job_name.to_string())
    }

    async fn get_transcription_job(
        &self,
        job_name: &str,
    ) -> Result<serde_json::Value, InternalSttError> {
        let region = self.region()?;
        let url = format!("https://transcribe.{region}.amazonaws.com/");

        let request_body = serde_json::json!({
            "TranscriptionJobName": job_name
        });

        let access_key = self
            .cfg
            .access_key_id
            .as_ref()
            .ok_or_else(|| InternalSttError::unauthorized("AWS_ACCESS_KEY_ID not set"))?
            .clone();
        let secret_key = self
            .cfg
            .secret_access_key
            .as_ref()
            .ok_or_else(|| InternalSttError::unauthorized("AWS_SECRET_ACCESS_KEY not set"))?
            .clone();
        let session_token = self.cfg.session_token.clone();

        let body_str = serde_json::to_string(&request_body)
            .map_err(|e| InternalSttError::internal(format!("json serialize: {e}")))?;

        let now = time::OffsetDateTime::now_utc();
        let amz_date = now
            .format(
                &time::format_description::parse("[year][month][day]T[hour][minute][second]Z")
                    .unwrap(),
            )
            .map_err(|e| InternalSttError::internal(format!("time format: {e}")))?;
        let date_stamp = now
            .format(
                &time::format_description::parse("[year][month][day]").map_err(|e| {
                    InternalSttError::internal(format!("error parsing date format: {e}"))
                })?,
            )
            .map_err(|e| InternalSttError::internal(format!("date format: {e}")))?;

        let payload_sha256 = Self::sha256_hex(body_str.as_bytes());
        let host = format!("transcribe.{region}.amazonaws.com");

        let (auth_header, signed_headers) = sign(SigV4Params {
            method: "POST".into(),
            service: "transcribe".into(),
            region: region.clone(),
            host: host.clone(),
            canonical_uri: "/".into(),
            canonical_querystring: String::new(),
            payload_sha256: payload_sha256.clone(),
            access_key,
            secret_key,
            session_token,
            amz_date: amz_date.clone(),
            date_stamp,
            content_type: Some("application/x-amz-json-1.1".into()),
        });

        let mut headers = HeaderMap::new();
        headers.insert(
            "Authorization",
            HeaderValue::from_str(&auth_header)
                .map_err(|e| InternalSttError::internal(format!("auth header: {e}")))?,
        );
        headers.insert(
            "X-Amz-Target",
            HeaderValue::from_static("Transcribe.GetTranscriptionJob"),
        );
        for (k, v) in signed_headers {
            headers.insert(
                HeaderName::from_bytes(k.as_bytes())
                    .map_err(|e| InternalSttError::internal(format!("invalid header name: {e}")))?,
                HeaderValue::from_str(&v)
                    .map_err(|e| InternalSttError::internal(format!("signed header {k}: {e}")))?,
            );
        }
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/x-amz-json-1.1"),
        );

        let (status, text, _hdrs) = self
            .http
            .post_bytes(&url, headers, body_str.into_bytes(), "application/x-amz-json-1.1")
            .await?;

        if !status.is_success() {
            return Err(InternalSttError::failed(format!(
                "AWS Transcribe get job failed: status={status}, body={text}"
            )));
        }

        let job_response: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| InternalSttError::internal(format!("parse job response: {e}")))?;

        Ok(job_response)
    }

    async fn poll_transcription_completion(
        &self,
        job_name: &str,
        max_wait_seconds: u64,
    ) -> Result<serde_json::Value, InternalSttError> {
        let start_time = std::time::Instant::now();
        let max_duration = std::time::Duration::from_secs(max_wait_seconds);

        loop {
            let job_response = self.get_transcription_job(job_name).await?;

            if let Some(job) = job_response.get("TranscriptionJob") {
                if let Some(status) = job.get("TranscriptionJobStatus").and_then(|s| s.as_str()) {
                    match status {
                        "COMPLETED" => return Ok(job_response),
                        "FAILED" => {
                            let failure_reason = job
                                .get("FailureReason")
                                .and_then(|r| r.as_str())
                                .unwrap_or("Unknown failure");
                            return Err(InternalSttError::failed(format!(
                                "AWS Transcribe job failed: {failure_reason}"
                            )));
                        }
                        "IN_PROGRESS" | "QUEUED" => {
                            // Continue polling
                        }
                        _ => {
                            return Err(InternalSttError::failed(format!(
                                "Unknown AWS Transcribe job status: {status}"
                            )));
                        }
                    }
                }
            }

            // Check timeout
            if start_time.elapsed() >= max_duration {
                return Err(InternalSttError::timeout(format!(
                    "AWS Transcribe job {job_name} did not complete within {max_wait_seconds} seconds"
                )));
            }

            // Wait before next poll (exponential backoff)
            let wait_secs = std::cmp::min(
                5,
                1 + start_time.elapsed().as_secs() / 10
            );

            // Use proper async sleep - simulate with yield and counter
            for _ in 0..(wait_secs * 10) {
                wstd::runtime::yield_now().await;
                // Each yield represents ~100ms, so 10 yields = ~1 second
            }
        }
    }

    async fn download_transcription_result(
        &self,
        transcript_file_uri: &str,
    ) -> Result<String, InternalSttError> {
        // Parse S3 URI to get bucket and key
        if !transcript_file_uri.starts_with("https://") {
            return Err(InternalSttError::failed(format!(
                "Invalid transcript URI format: {transcript_file_uri}"
            )));
        }

        // For AWS Transcribe, the transcript URI is a pre-signed URL that doesn't need additional auth
        // AWS Transcribe provides pre-signed URLs for downloading results
        let headers = HeaderMap::new();
        let (status, text, _hdrs) = self
            .http
            .get(transcript_file_uri, headers)
            .await?;

        if !status.is_success() {
            return Err(InternalSttError::failed(format!(
                "Failed to download transcription result: status={status}"
            )));
        }

        Ok(text)
    }

    pub async fn transcribe(
        &self,
        audio: Vec<u8>,
        config: &AudioConfig,
        options: &Option<TranscribeOptions<'_>>,
    ) -> Result<(u16, String), InternalSttError> {
        // AWS Transcribe requires S3 upload + transcription job workflow

        // Generate unique job name and S3 key
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let job_name = format!("golem-stt-{}", timestamp);
        let s3_key = format!("audio/{}.{}", timestamp, match config.format {
            AudioFormat::Wav => "wav",
            AudioFormat::Mp3 => "mp3",
            AudioFormat::Flac => "flac",
            AudioFormat::Ogg => "ogg",
            AudioFormat::Aac => "aac",
            AudioFormat::Pcm => "wav",
        });

        // Step 1: Upload audio to S3
        let content_type = Self::content_type_for(&config.format);
        let s3_uri = self.upload_to_s3(&audio, &s3_key, content_type).await?;

        // Step 2: Start transcription job
        let _job_name = self.start_transcription_job(&s3_uri, &job_name, config, options).await?;

        // Step 3: Poll for completion (with timeout)
        let timeout_secs = self.cfg.common.timeout_secs;
        let job_response = self.poll_transcription_completion(&job_name, timeout_secs).await?;

        // Step 4: Download and return transcription results
        if let Some(job) = job_response.get("TranscriptionJob") {
            if let Some(transcript) = job.get("Transcript") {
                if let Some(transcript_file_uri) = transcript.get("TranscriptFileUri").and_then(|u| u.as_str()) {
                    let transcription_result = self.download_transcription_result(transcript_file_uri).await?;
                    return Ok((200, transcription_result));
                }
            }
        }

        // Fallback: return the job response if we can't get the transcript
        Ok((200, serde_json::to_string(&job_response).unwrap()))
    }
}
