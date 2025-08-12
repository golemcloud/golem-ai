use crate::client::{AwsTranscribeClient, AwsStreamingSession};
use crate::conversions::{
    get_supported_languages, generate_job_name, create_transcription_job_request,
    convert_aws_response_to_transcription_result,
};
use golem_stt::durability::{DurableSTT, ExtendedTranscriptionGuest, ExtendedVocabulariesGuest, ExtendedLanguagesGuest, ExtendedGuest};
use golem_stt::golem::stt::languages::{Guest as LanguagesGuest, LanguageInfo};
use golem_stt::golem::stt::transcription::{
    Guest as TranscriptionGuest, TranscribeOptions, TranscriptionStream,
};
use golem_stt::golem::stt::types::{AudioConfig, SttError, TranscriptionResult, TranscriptAlternative};
use golem_stt::golem::stt::vocabularies::{Guest as VocabulariesGuest, Vocabulary};
use log::{error, trace, warn};
use std::cell::RefCell;

mod client;
mod conversions;

pub struct AwsTranscriptionStream {
    session: Option<AwsStreamingSession>,
    is_finished: RefCell<bool>,
}

impl AwsTranscriptionStream {
    pub fn new(session: AwsStreamingSession) -> Self {
        Self {
            session: Some(session),
            is_finished: RefCell::new(false),
        }
    }
}

impl golem_stt::golem::stt::transcription::GuestTranscriptionStream for AwsTranscriptionStream {
    fn send_audio(&self, chunk: Vec<u8>) -> Result<(), SttError> {
        if *self.is_finished.borrow() {
            return Err(SttError::InternalError("Stream already finished".to_string()));
        }

        if let Some(session) = &self.session {
            session.send_audio(chunk)?;
            trace!("Sent audio chunk to AWS streaming session");
            Ok(())
        } else {
            Err(SttError::InternalError("Streaming session not initialized".to_string()))
        }
    }

    fn finish(&self) -> Result<(), SttError> {
        let mut is_finished = self.is_finished.borrow_mut();
        if *is_finished {
            return Err(SttError::InternalError("Stream already finished".to_string()));
        }

        if let Some(session) = &self.session {
            trace!("Finishing AWS real-time streaming session");
            
            // For real-time streaming, we don't need to wait for a final result
            // The session has been processing audio chunks in real-time
            // Just mark as finished so no more audio can be sent
            session.close();
            *is_finished = true;
            trace!("AWS real-time streaming session finished successfully");
            Ok(())
        } else {
            Err(SttError::InternalError("Streaming session not initialized".to_string()))
        }
    }

    fn receive_alternative(&self) -> Result<Option<TranscriptAlternative>, SttError> {
        // For real-time streaming, check for results even if not finished
        if let Some(session) = &self.session {
            // Get latest streaming results
            let streaming_results = session.get_latest_results()?;
            
            // Process streaming results and convert to alternatives
            for streaming_result in streaming_results {
                if let Some(alternative) = streaming_result.alternatives.first() {
                    if !alternative.transcript.trim().is_empty() {
                        // Convert AWS items to standard format if available
                        let words = if let Some(ref items) = alternative.items {
                            items.iter().filter_map(|item| {
                                if item.r#type == "pronunciation" {
                                    Some(golem_stt::golem::stt::types::WordSegment {
                                        text: item.content.clone(),
                                        start_time: item.start_time.unwrap_or(0.0) as f32,
                                        end_time: item.end_time.unwrap_or(0.0) as f32,
                                        confidence: None, // AWS doesn't provide word-level confidence in this format
                                        speaker_id: None, // Would need to be extracted from speaker labels
                                    })
                                } else {
                                    None
                                }
                            }).collect()
                        } else {
                            vec![]
                        };
                        
                        trace!("Returning AWS real-time streaming alternative: {} (partial: {})", 
                               alternative.transcript, streaming_result.is_partial);
                        
                        return Ok(Some(TranscriptAlternative {
                            text: alternative.transcript.clone(),
                            confidence: alternative.confidence.unwrap_or(0.0) as f32,
                            words,
                        }));
                    }
                }
            }
        }
        
        Ok(None) // No alternatives available
    }

    fn close(&self) {
        if let Some(session) = &self.session {
            session.close();
        }
        trace!("AWS streaming session closed");
    }
}

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
}

pub struct AwsVocabulary {
    name: String,
    language_code: String,
    phrases: Vec<String>,
    client: AwsTranscribeClient,
}

impl AwsVocabulary {
    /// Get the language code for this vocabulary
    pub fn get_language_code(&self) -> &str {
        &self.language_code
    }
}

impl golem_stt::golem::stt::vocabularies::GuestVocabulary for AwsVocabulary {
    fn get_name(&self) -> String {
        self.name.clone()
    }

    fn get_phrases(&self) -> Vec<String> {
        // Try to fetch current phrases from AWS Transcribe service
        // Fall back to stored phrases if AWS call fails
        match self.client.get_vocabulary(self.name.clone()) {
            Ok(response) => {
                trace!("Retrieved AWS vocabulary '{}' with state: {} (language: {})", 
                       response.vocabulary_name, response.vocabulary_state, response.language_code);
                
                // If vocabulary is ready and has a download URI, we could fetch the actual phrases
                // For now, return stored phrases since parsing the download URI is complex
                self.phrases.clone()
            }
            Err(e) => {
                trace!("Failed to get AWS vocabulary status: {:?}, returning stored phrases", e);
                self.phrases.clone()
            }
        }
    }

    fn delete(&self) -> Result<(), SttError> {
        // Delete vocabulary from AWS Transcribe
        trace!("Deleting AWS vocabulary: {}", self.name);
        self.client.delete_vocabulary(self.name.clone())?;
        trace!("AWS vocabulary '{}' deleted successfully", self.name);
        Ok(())
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

        trace!("Using AWS Transcribe with S3 batch processing");
        
        // Generate job name using conversions
        let job_name = generate_job_name();
        
        // Create transcription request using conversions
        let request = create_transcription_job_request(&config, &options, &job_name)?;
        
        // Use the client for raw API operations
        let aws_response = client.transcribe_audio_batch(&audio, request)
            .map_err(|e| {
                error!("AWS Transcribe API failed: {:?}", e);
                e
            })?;

        // Convert AWS response to standard format using conversions
        convert_aws_response_to_transcription_result(aws_response, audio.len(), language, &job_name)
    }

    fn transcribe_stream(
        config: AudioConfig,
        options: Option<TranscribeOptions>,
    ) -> Result<TranscriptionStream, SttError> {
        golem_stt::init_logging();
        trace!("Starting AWS Transcribe real-time streaming transcription");

        let client = Self::get_client()?;
        let session = client.start_streaming_session(&config, &options)?;
        let stream = AwsTranscriptionStream::new(session);
        
        Ok(TranscriptionStream::new(stream))
    }
}

impl LanguagesGuest for AwsSTTComponent {
    fn list_languages() -> Result<Vec<LanguageInfo>, SttError> {
        Ok(get_supported_languages())
    }
}

impl VocabulariesGuest for AwsSTTComponent {
    type Vocabulary = AwsVocabulary;

    fn create_vocabulary(
        name: String,
        phrases: Vec<String>,
    ) -> Result<Vocabulary, SttError> {
        golem_stt::init_logging();
        trace!("Creating AWS vocabulary '{}' with {} phrases", name, phrases.len());
        
        // Get AWS client
        let client = Self::get_client()?;
        
        // Default to en-US if no language context is available
        // In a more sophisticated implementation, we could pass language as a parameter
        let language_code = "en-US".to_string();
        
        // Create vocabulary on AWS Transcribe
        let response = client.create_vocabulary(name.clone(), language_code.clone(), phrases.clone())?;
        
        // Check if vocabulary creation was successful
        match response.vocabulary_state.as_str() {
            "PENDING" | "READY" => {
                trace!("AWS vocabulary '{}' created successfully with state: {}", response.vocabulary_name, response.vocabulary_state);
                
                // Return the vocabulary object
                Ok(Vocabulary::new(AwsVocabulary { 
                    name: response.vocabulary_name, 
                    language_code: response.language_code,
                    phrases,
                    client,
                }))
            }
            "FAILED" => {
                let error_msg = response.failure_reason.unwrap_or_else(|| "Unknown failure".to_string());
                Err(SttError::InternalError(format!("AWS vocabulary creation failed: {}", error_msg)))
            }
            _ => {
                warn!("AWS vocabulary '{}' has unexpected state: {}", response.vocabulary_name, response.vocabulary_state);
                
                // Still return the vocabulary object as it might become ready later
                Ok(Vocabulary::new(AwsVocabulary { 
                    name: response.vocabulary_name, 
                    language_code: response.language_code,
                    phrases,
                    client,
                }))
            }
        }
    }
}

impl ExtendedTranscriptionGuest for AwsSTTComponent {}
impl ExtendedVocabulariesGuest for AwsSTTComponent {}
impl ExtendedLanguagesGuest for AwsSTTComponent {}
impl ExtendedGuest for AwsSTTComponent {}

type DurableAwsSTTComponent = DurableSTT<AwsSTTComponent>;

golem_stt::export_stt!(DurableAwsSTTComponent with_types_in golem_stt);