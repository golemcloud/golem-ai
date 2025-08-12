use crate::client::AzureSpeechClient;
use crate::conversions::{
    convert_realtime_response, create_realtime_transcription_request,
    get_supported_languages,
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



    fn estimate_audio_duration(audio: &[u8], config: &AudioConfig) -> f32 {
        // Conservative estimation based on audio format and size
        let sample_rate = config.sample_rate.unwrap_or(16000) as f32;
        let channels = config.channels.unwrap_or(1) as f32;
        
        let bytes_per_sample = match config.format {
            golem_stt::golem::stt::types::AudioFormat::Pcm => 2, // 16-bit PCM
            golem_stt::golem::stt::types::AudioFormat::Wav => 2, // Typically 16-bit
            _ => 1, // Compressed formats - conservative estimate
        };
        
        let header_size = match config.format {
            golem_stt::golem::stt::types::AudioFormat::Wav => 44, // WAV header
            _ => 0,
        };
        
        let audio_data_size = (audio.len() as i32 - header_size).max(0) as f32;
        let bytes_per_second = sample_rate * channels * bytes_per_sample as f32;
        
        audio_data_size / bytes_per_second
    }
    
    fn transcribe_fast_api(
        audio: Vec<u8>,
        config: AudioConfig,
        options: Option<TranscribeOptions>,
    ) -> Result<TranscriptionResult, SttError> {
        let client = Self::get_client()?;
        let audio_len = audio.len(); // Capture length before moving
        
        let language = options
            .as_ref()
            .and_then(|opts| opts.language.as_ref())
            .unwrap_or(&"en-US".to_string())
            .clone();
            
        trace!("Using Azure Fast Transcription API for large audio file");
        
        // Use Azure Fast Transcription API which supports direct file upload
        let azure_response = client.transcribe_fast_api(audio, &config, &options)?;

        convert_realtime_response(azure_response, audio_len, &language)
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

        // Estimate audio duration based on size and format to choose appropriate API
        // Azure Speech REST API has a 60-second limit, use Fast Transcription for longer audio
        let estimated_duration = Self::estimate_audio_duration(&audio, &config);
        
        if estimated_duration > 58.0 { // Leave some buffer for 60s limit
            trace!("Audio estimated at {:.1}s - attempting Azure Fast Transcription for longer audio", estimated_duration);
            // Try Fast Transcription first, fall back to real-time if not available
            match Self::transcribe_fast_api(audio.clone(), config.clone(), options.clone()) {
                Ok(result) => {
                    trace!("Azure Fast Transcription completed successfully");
                    Ok(result)
                },
                Err(SttError::NetworkError(ref e)) if e.contains("404") || e.contains("Not Found") => {
                    trace!("Azure Fast Transcription not available in region (404), falling back to real-time API");
                    warn!("Audio >60s detected but Fast Transcription unavailable in region - using real-time API (will truncate)");
                    Self::transcribe_realtime(audio, config, options)
                },
                Err(e) => {
                    trace!("Azure Fast Transcription failed: {:?}, falling back to real-time API", e);
                    warn!("Fast Transcription failed, using real-time API fallback (may truncate >60s audio)");
                    Self::transcribe_realtime(audio, config, options)
                },
            }
        } else {
            trace!("Audio estimated at {:.1}s - using Azure real-time API", estimated_duration);
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
            golem_stt::golem::stt::types::AudioFormat::Wav => "wav",
            golem_stt::golem::stt::types::AudioFormat::Mp3 => "mp3",
            golem_stt::golem::stt::types::AudioFormat::Flac => "flac",
            golem_stt::golem::stt::types::AudioFormat::Ogg => "ogg",
            golem_stt::golem::stt::types::AudioFormat::Aac => "aac",
            golem_stt::golem::stt::types::AudioFormat::Pcm => "wav", // PCM in WAV container
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