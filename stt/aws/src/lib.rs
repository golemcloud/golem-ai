use crate::client::AwsTranscribeClient;
use crate::conversions::{convert_aws_response, create_transcription_job_request, get_supported_languages, generate_job_name};
// use golem_stt::config::with_config_key;
use golem_stt::durability::{DurableSTT, ExtendedTranscriptionGuest, ExtendedVocabulariesGuest, ExtendedLanguagesGuest, ExtendedGuest};
use golem_stt::golem::stt::languages::{Guest as LanguagesGuest, LanguageInfo};
use golem_stt::golem::stt::transcription::{
    Guest as TranscriptionGuest, TranscribeOptions, TranscriptionStream,
};
use golem_stt::golem::stt::types::{AudioConfig, SttError, TranscriptionResult, TranscriptAlternative, TranscriptionMetadata};
use golem_stt::golem::stt::vocabularies::{Guest as VocabulariesGuest, Vocabulary};
use log::{error, trace, warn};
use std::cell::RefCell;
use std::collections::HashMap;
use std::thread;
use std::time::Duration;

mod client;
mod conversions;

struct AwsSTTComponent;

impl AwsSTTComponent {
    const ACCESS_KEY_ENV_VAR: &'static str = "AWS_ACCESS_KEY_ID";
    const SECRET_KEY_ENV_VAR: &'static str = "AWS_SECRET_ACCESS_KEY";
    const REGION_ENV_VAR: &'static str = "AWS_REGION";

    fn get_client() -> Result<AwsTranscribeClient, SttError> {
        let access_key = std::env::var(Self::ACCESS_KEY_ENV_VAR)
            .map_err(|_| SttError::Unauthorized("AWS_ACCESS_KEY_ID not set".to_string()))?;
        
        let secret_key = std::env::var(Self::SECRET_KEY_ENV_VAR)
            .map_err(|_| SttError::Unauthorized("AWS_SECRET_ACCESS_KEY not set".to_string()))?;
            
        let region = std::env::var(Self::REGION_ENV_VAR)
            .unwrap_or_else(|_| "us-east-1".to_string());
        
        Ok(AwsTranscribeClient::new(access_key, secret_key, region))
    }

    fn poll_transcription_job(client: &AwsTranscribeClient, job_name: &str) -> Result<crate::client::TranscriptionJob, SttError> {
        let max_attempts = 60; // 5 minutes with 5-second intervals
        let poll_interval = Duration::from_secs(5);
        
        for attempt in 1..=max_attempts {
            trace!("Polling transcription job {}, attempt {}/{}", job_name, attempt, max_attempts);
            
            let response = client.get_transcription_job(job_name)?;
            
            if let Some(job) = response.transcription_job {
                match job.transcription_job_status.as_str() {
                    "COMPLETED" => {
                        trace!("Transcription job {} completed", job_name);
                        return Ok(job);
                    }
                    "FAILED" => {
                        error!("Transcription job {} failed", job_name);
                        return Err(SttError::TranscriptionFailed(
                            format!("AWS Transcribe job {} failed", job_name)
                        ));
                    }
                    "IN_PROGRESS" => {
                        trace!("Transcription job {} still in progress", job_name);
                        thread::sleep(poll_interval);
                        continue;
                    }
                    status => {
                        warn!("Unknown transcription job status: {}", status);
                        thread::sleep(poll_interval);
                        continue;
                    }
                }
            } else {
                return Err(SttError::InternalError(
                    format!("No transcription job found with name {}", job_name)
                ));
            }
        }
        
        Err(SttError::InternalError(
            format!("Transcription job {} timed out after {} attempts", job_name, max_attempts)
        ))
    }

    fn fetch_transcript_from_s3(transcript_uri: &str) -> Result<crate::client::AwsTranscriptResponse, SttError> {
        // In a real implementation, you would fetch the transcript from S3
        // For now, we'll return a mock response since we can't actually upload to S3
        // in this WebAssembly environment
        
        trace!("Would fetch transcript from S3 URI: {}", transcript_uri);
        
        // Mock response for demonstration
        Ok(crate::client::AwsTranscriptResponse {
            results: crate::client::Results {
                transcripts: vec![
                    crate::client::TranscriptItem {
                        transcript: "This is a mock transcription from AWS Transcribe.".to_string(),
                    }
                ],
                items: vec![
                    crate::client::Item {
                        start_time: Some("0.0".to_string()),
                        end_time: Some("1.5".to_string()),
                        alternatives: vec![
                            crate::client::Alternative {
                                confidence: Some("0.95".to_string()),
                                content: "This".to_string(),
                            }
                        ],
                        item_type: "pronunciation".to_string(),
                    },
                    crate::client::Item {
                        start_time: Some("1.5".to_string()),
                        end_time: Some("2.0".to_string()),
                        alternatives: vec![
                            crate::client::Alternative {
                                confidence: Some("0.98".to_string()),
                                content: "is".to_string(),
                            }
                        ],
                        item_type: "pronunciation".to_string(),
                    },
                    crate::client::Item {
                        start_time: Some("2.0".to_string()),
                        end_time: Some("2.5".to_string()),
                        alternatives: vec![
                            crate::client::Alternative {
                                confidence: Some("0.92".to_string()),
                                content: "a".to_string(),
                            }
                        ],
                        item_type: "pronunciation".to_string(),
                    },
                    crate::client::Item {
                        start_time: Some("2.5".to_string()),
                        end_time: Some("3.2".to_string()),
                        alternatives: vec![
                            crate::client::Alternative {
                                confidence: Some("0.96".to_string()),
                                content: "mock".to_string(),
                            }
                        ],
                        item_type: "pronunciation".to_string(),
                    },
                    crate::client::Item {
                        start_time: Some("3.2".to_string()),
                        end_time: Some("4.5".to_string()),
                        alternatives: vec![
                            crate::client::Alternative {
                                confidence: Some("0.94".to_string()),
                                content: "transcription".to_string(),
                            }
                        ],
                        item_type: "pronunciation".to_string(),
                    },
                ],
            },
        })
    }
}

// Placeholder for TranscriptionStream - AWS Transcribe streaming would require WebSocket
pub struct AwsTranscriptionStream;

impl golem_stt::golem::stt::transcription::GuestTranscriptionStream for AwsTranscriptionStream {
    fn send_audio(&self, _chunk: Vec<u8>) -> Result<(), SttError> {
        Err(SttError::UnsupportedOperation("AWS Transcribe streaming not yet implemented".to_string()))
    }

    fn finish(&self) -> Result<(), SttError> {
        Err(SttError::UnsupportedOperation("AWS Transcribe streaming not yet implemented".to_string()))
    }

    fn receive_alternative(&self) -> Result<Option<golem_stt::golem::stt::types::TranscriptAlternative>, SttError> {
        Err(SttError::UnsupportedOperation("AWS Transcribe streaming not yet implemented".to_string()))
    }

    fn close(&self) {
        // No-op for now
    }
}

impl TranscriptionGuest for AwsSTTComponent {
    type TranscriptionStream = AwsTranscriptionStream;

    fn transcribe(
        audio: Vec<u8>,
        config: AudioConfig,
        options: Option<TranscribeOptions>,
    ) -> Result<TranscriptionResult, SttError> {
        golem_stt::init_logging();
        trace!("Starting AWS Transcribe transcription, audio size: {} bytes", audio.len());

        let client = Self::get_client()?;
        
        let default_language = "en-US".to_string();
        let language = options
            .as_ref()
            .and_then(|opts| opts.language.as_ref())
            .unwrap_or(&default_language);

        let media_format = match config.format {
            golem_stt::golem::stt::types::AudioFormat::Wav => "wav",
            golem_stt::golem::stt::types::AudioFormat::Mp3 => "mp3",
            golem_stt::golem::stt::types::AudioFormat::Flac => "flac",
            golem_stt::golem::stt::types::AudioFormat::Aac => "mp4",
            _ => "wav", // Default fallback
        };

        trace!("Sending direct audio transcription to AWS Transcribe Streaming API");
        
        // Use direct streaming transcription
        let direct_response = client.transcribe_audio_directly(&audio, media_format, Some(language))
            .map_err(|e| {
                error!("AWS Transcribe streaming failed: {:?}", e);
                e
            })?;

        // Convert direct response to our format
        let alternatives = vec![TranscriptAlternative {
            text: direct_response.transcript,
            confidence: direct_response.confidence,
            words: vec![], // AWS streaming doesn't provide word-level timing in this simplified implementation
        }];

        Ok(TranscriptionResult {
            alternatives,
            metadata: TranscriptionMetadata {
                duration_seconds: 0.0, // Would need to calculate from audio
                audio_size_bytes: audio.len() as u32,
                request_id: generate_job_name(),
                model: Some("AWS Transcribe Streaming".to_string()),
                language: language.clone(),
            },
        })
    }

    fn transcribe_stream(
        _config: AudioConfig,
        _options: Option<TranscribeOptions>,
    ) -> Result<TranscriptionStream, SttError> {
        // AWS Transcribe streaming would require WebSocket connection
        Err(SttError::UnsupportedOperation(
            "AWS Transcribe streaming not yet implemented".to_string(),
        ))
    }
}

impl LanguagesGuest for AwsSTTComponent {
    fn list_languages() -> Result<Vec<LanguageInfo>, SttError> {
        Ok(get_supported_languages())
    }
}

// Simple in-memory vocabulary storage for this implementation
thread_local! {
    static VOCABULARIES: RefCell<HashMap<String, Vec<String>>> = RefCell::new(HashMap::new());
}

pub struct AwsVocabulary {
    name: String,
}

impl golem_stt::golem::stt::vocabularies::GuestVocabulary for AwsVocabulary {
    fn get_name(&self) -> String {
        self.name.clone()
    }

    fn get_phrases(&self) -> Vec<String> {
        VOCABULARIES.with(|v| {
            v.borrow()
                .get(&self.name)
                .cloned()
                .unwrap_or_default()
        })
    }

    fn delete(&self) -> Result<(), SttError> {
        VOCABULARIES.with(|v| {
            v.borrow_mut().remove(&self.name);
        });
        Ok(())
    }
}

impl VocabulariesGuest for AwsSTTComponent {
    type Vocabulary = AwsVocabulary;

    fn create_vocabulary(
        name: String,
        phrases: Vec<String>,
    ) -> Result<Vocabulary, SttError> {
        VOCABULARIES.with(|v| {
            v.borrow_mut().insert(name.clone(), phrases);
        });
        
        Ok(Vocabulary::new(AwsVocabulary { name }))
    }
}

impl ExtendedTranscriptionGuest for AwsSTTComponent {}
impl ExtendedVocabulariesGuest for AwsSTTComponent {}
impl ExtendedLanguagesGuest for AwsSTTComponent {}
impl ExtendedGuest for AwsSTTComponent {}

type DurableAwsSTTComponent = DurableSTT<AwsSTTComponent>;

golem_stt::export_stt!(DurableAwsSTTComponent with_types_in golem_stt);