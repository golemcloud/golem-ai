use crate::client::AzureSpeechClient;
use crate::conversions::{
    convert_realtime_response, convert_detailed_transcript, create_realtime_transcription_request,
    create_batch_transcription_request, get_supported_languages, generate_transcription_name,
};
use golem_stt::durability::{DurableSTT, ExtendedTranscriptionGuest, ExtendedVocabulariesGuest, ExtendedLanguagesGuest, ExtendedGuest};
use golem_stt::golem::stt::languages::{Guest as LanguagesGuest, LanguageInfo};
use golem_stt::golem::stt::transcription::{
    Guest as TranscriptionGuest, TranscribeOptions, TranscriptionStream,
};
use golem_stt::golem::stt::types::{AudioConfig, SttError, TranscriptionResult};
use golem_stt::golem::stt::vocabularies::{Guest as VocabulariesGuest, Vocabulary};
use log::{error, trace, warn};
use std::cell::RefCell;
use std::collections::HashMap;
use std::thread;
use std::time::Duration;

mod client;
mod conversions;

struct AzureSTTComponent;

impl AzureSTTComponent {
    const SUBSCRIPTION_KEY_ENV_VAR: &'static str = "AZURE_SPEECH_KEY";
    const REGION_ENV_VAR: &'static str = "AZURE_SPEECH_REGION";

    fn get_client() -> Result<AzureSpeechClient, SttError> {
        let subscription_key = std::env::var(Self::SUBSCRIPTION_KEY_ENV_VAR)
            .map_err(|_| SttError::Unauthorized("AZURE_SPEECH_KEY not set".to_string()))?;
        
        let region = std::env::var(Self::REGION_ENV_VAR)
            .map_err(|_| SttError::Unauthorized("AZURE_SPEECH_REGION not set".to_string()))?;
        
        Ok(AzureSpeechClient::new(subscription_key, region))
    }

    fn transcribe_realtime(
        audio: Vec<u8>,
        config: AudioConfig,
        options: Option<TranscribeOptions>,
    ) -> Result<TranscriptionResult, SttError> {
        let client = Self::get_client()?;
        let request = create_realtime_transcription_request(&audio, &config, &options)?;
        
        let language = options
            .as_ref()
            .and_then(|opts| opts.language.as_ref())
            .unwrap_or(&"en-US".to_string())
            .clone();

        trace!("Sending audio to Azure Speech REST API");
        
        // Use Azure Speech REST API directly (not WebSocket)
        let azure_response = client.transcribe_audio(request)
            .map_err(|e| {
                error!("Azure Speech transcription failed: {:?}", e);
                e
            })?;

        convert_realtime_response(azure_response, audio.len(), &language)
    }

    fn transcribe_batch(
        audio: Vec<u8>,
        config: AudioConfig,
        options: Option<TranscribeOptions>,
    ) -> Result<TranscriptionResult, SttError> {
        let client = Self::get_client()?;
        let transcription_name = generate_transcription_name();
        let request = create_batch_transcription_request(&audio, &config, &options, &transcription_name)?;
        
        let language = options
            .as_ref()
            .and_then(|opts| opts.language.as_ref())
            .unwrap_or(&"en-US".to_string())
            .clone();

        // Note: This is a simplified implementation
        // In practice, Azure Speech requires uploading audio to Azure Blob Storage first
        warn!("Azure Speech batch transcription requires audio to be uploaded to Azure Blob Storage first. This is a mock implementation.");
        
        // Start the batch transcription
        let transcription_response = client.start_batch_transcription(request)
            .map_err(|e| {
                error!("Azure Speech batch transcription start failed: {:?}", e);
                e
            })?;

        // Extract transcription ID from the self URL
        let transcription_id = transcription_response.self_url
            .split('/')
            .last()
            .ok_or_else(|| SttError::InternalError("Could not extract transcription ID".to_string()))?;

        // Poll for completion
        Self::poll_transcription_completion(&client, transcription_id)?;
        
        // Get transcription files
        let files_response = client.get_transcription_files(transcription_id)?;
        
        // Find the transcript file
        for file in files_response.values {
            if file.kind == "Transcription" {
                if let Some(file_links) = file.links {
                    let transcript = client.download_transcript(&file_links.content_url)?;
                    return convert_detailed_transcript(transcript, audio.len(), &language);
                }
            }
        }

        Err(SttError::InternalError("No transcript found in completed transcription".to_string()))
    }

    fn poll_transcription_completion(
        client: &AzureSpeechClient,
        transcription_id: &str,
    ) -> Result<crate::client::BatchTranscriptionStatus, SttError> {
        let max_attempts = 60; // 5 minutes with 5-second intervals
        let poll_interval = Duration::from_secs(5);
        
        for attempt in 1..=max_attempts {
            trace!("Polling Azure transcription {}, attempt {}/{}", transcription_id, attempt, max_attempts);
            
            let status = client.get_batch_transcription(transcription_id)?;
            
            match status.status.as_str() {
                "Succeeded" => {
                    trace!("Azure transcription {} completed", transcription_id);
                    return Ok(status);
                }
                "Failed" => {
                    error!("Azure transcription {} failed", transcription_id);
                    return Err(SttError::TranscriptionFailed(
                        format!("Azure Speech transcription {} failed", transcription_id)
                    ));
                }
                "Running" => {
                    trace!("Azure transcription {} still running", transcription_id);
                    thread::sleep(poll_interval);
                    continue;
                }
                status_str => {
                    warn!("Unknown Azure transcription status: {}", status_str);
                    thread::sleep(poll_interval);
                    continue;
                }
            }
        }
        
        Err(SttError::InternalError(
            format!("Azure transcription {} timed out after {} attempts", transcription_id, max_attempts)
        ))
    }
}

use crate::client::{AzureStreamingSession};

// Azure TranscriptionStream using HTTP chunked transfer encoding
pub struct AzureTranscriptionStream {
    session: Option<AzureStreamingSession>,
    is_finished: RefCell<bool>,
}

impl AzureTranscriptionStream {
    pub fn new(session: AzureStreamingSession) -> Self {
        Self {
            session: Some(session),
            is_finished: RefCell::new(false),
        }
    }
}

impl golem_stt::golem::stt::transcription::GuestTranscriptionStream for AzureTranscriptionStream {
    fn send_audio(&self, chunk: Vec<u8>) -> Result<(), SttError> {
        if *self.is_finished.borrow() {
            return Err(SttError::InternalError("Stream already finished".to_string()));
        }

        if let Some(session) = &self.session {
            session.send_audio(chunk)?;
            trace!("Sent audio chunk to Azure streaming session");
            Ok(())
        } else {
            Err(SttError::InternalError("Azure streaming session not initialized".to_string()))
        }
    }

    fn finish(&self) -> Result<(), SttError> {
        let mut is_finished = self.is_finished.borrow_mut();
        if *is_finished {
            return Err(SttError::InternalError("Stream already finished".to_string()));
        }

        if let Some(session) = &self.session {
            trace!("Finishing Azure real-time streaming session");
            
            // For real-time streaming, we don't need to wait for a final result
            // The session has been processing audio chunks in real-time
            // Just mark as finished so no more audio can be sent
            session.close();
            *is_finished = true;
            trace!("Azure real-time streaming session finished successfully");
            Ok(())
        } else {
            Err(SttError::InternalError("Azure streaming session not initialized".to_string()))
        }
    }

    fn receive_alternative(&self) -> Result<Option<golem_stt::golem::stt::types::TranscriptAlternative>, SttError> {
        // For real-time streaming, check for results even if not finished
        if let Some(session) = &self.session {
            // Get latest streaming results
            let streaming_results = session.get_latest_results()?;
            
            // Process streaming results and convert to alternatives
            for streaming_result in streaming_results {
                if let Some(ref display_text) = streaming_result.display_text {
                    // Convert Azure n_best results if available
                    let words = if let Some(ref n_best) = streaming_result.n_best {
                        if let Some(first_result) = n_best.first() {
                            if let Some(ref azure_words) = first_result.words {
                                azure_words.iter().map(|w| golem_stt::golem::stt::types::WordSegment {
                                    text: w.word.clone(),
                                    start_time: (w.offset as f64 / 10_000_000.0) as f32, // Convert from 100ns ticks to seconds
                                    end_time: ((w.offset + w.duration) as f64 / 10_000_000.0) as f32,
                                    confidence: w.confidence,
                                    speaker_id: None, // Azure doesn't provide speaker ID in this format
                                }).collect()
                            } else {
                                vec![]
                            }
                        } else {
                            vec![]
                        }
                    } else {
                        vec![]
                    };
                    
                    let confidence = streaming_result.n_best
                        .as_ref()
                        .and_then(|n_best| n_best.first())
                        .map(|first| first.confidence)
                        .unwrap_or(0.0);
                    
                    trace!("Returning Azure real-time streaming alternative: {} (final: {})", 
                           display_text, streaming_result.is_final);
                    
                    return Ok(Some(golem_stt::golem::stt::types::TranscriptAlternative {
                        text: display_text.clone(),
                        confidence,
                        words,
                    }));
                }
            }
        }
        
        Ok(None) // No alternatives available
    }

    fn close(&self) {
        if let Some(session) = &self.session {
            session.close();
            trace!("Azure streaming session closed");
        }
    }
}

impl TranscriptionGuest for AzureSTTComponent {
    type TranscriptionStream = AzureTranscriptionStream;

    fn transcribe(
        audio: Vec<u8>,
        config: AudioConfig,
        options: Option<TranscribeOptions>,
    ) -> Result<TranscriptionResult, SttError> {
        golem_stt::init_logging();
        trace!("Starting Azure Speech transcription, audio size: {} bytes", audio.len());

        // Use direct Azure Speech REST API instead of batch or mock
        // This works like Deepgram's direct API
        let use_batch = false; // Use real-time REST API for immediate results

        if use_batch {
            Self::transcribe_batch(audio, config, options)
        } else {
            Self::transcribe_realtime(audio, config, options)
        }
    }

    fn transcribe_stream(
        config: AudioConfig,
        options: Option<TranscribeOptions>,
    ) -> Result<TranscriptionStream, SttError> {
        golem_stt::init_logging();
        trace!("Starting Azure Speech real-time streaming transcription");
        
        let client = Self::get_client()?;
        
        let language = options
            .as_ref()
            .and_then(|opts| opts.language.as_ref())
            .unwrap_or(&"en-US".to_string())
            .clone();
        
        let audio_format = match config.format {
            golem_stt::golem::stt::types::AudioFormat::Wav => "audio/wav",
            golem_stt::golem::stt::types::AudioFormat::Mp3 => "audio/mp3",
            _ => "audio/wav", // Default to WAV
        };
        
        let session = client.start_streaming_session(&language, audio_format)?;
        let stream = AzureTranscriptionStream::new(session);
        
        Ok(TranscriptionStream::new(stream))
    }
}

impl LanguagesGuest for AzureSTTComponent {
    fn list_languages() -> Result<Vec<LanguageInfo>, SttError> {
        Ok(get_supported_languages())
    }
}

// Simple in-memory vocabulary storage for this implementation
thread_local! {
    static VOCABULARIES: RefCell<HashMap<String, Vec<String>>> = RefCell::new(HashMap::new());
}

pub struct AzureVocabulary {
    name: String,
}

impl golem_stt::golem::stt::vocabularies::GuestVocabulary for AzureVocabulary {
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

impl VocabulariesGuest for AzureSTTComponent {
    type Vocabulary = AzureVocabulary;

    fn create_vocabulary(
        name: String,
        phrases: Vec<String>,
    ) -> Result<Vocabulary, SttError> {
        VOCABULARIES.with(|v| {
            v.borrow_mut().insert(name.clone(), phrases);
        });
        
        Ok(Vocabulary::new(AzureVocabulary { name }))
    }
}

impl ExtendedTranscriptionGuest for AzureSTTComponent {}
impl ExtendedVocabulariesGuest for AzureSTTComponent {}
impl ExtendedLanguagesGuest for AzureSTTComponent {}
impl ExtendedGuest for AzureSTTComponent {}

type DurableAzureSTTComponent = DurableSTT<AzureSTTComponent>;

golem_stt::export_stt!(DurableAzureSTTComponent with_types_in golem_stt);