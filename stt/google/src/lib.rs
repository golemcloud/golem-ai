use crate::client::{GoogleSpeechClient, GoogleStreamingSession};
use crate::conversions::{convert_response, create_recognize_request, get_supported_languages};
use golem_stt::durability::{DurableSTT, ExtendedTranscriptionGuest, ExtendedVocabulariesGuest, ExtendedLanguagesGuest, ExtendedGuest};
use golem_stt::golem::stt::languages::{Guest as LanguagesGuest, LanguageInfo};
use golem_stt::golem::stt::transcription::{
    Guest as TranscriptionGuest, TranscribeOptions, TranscriptionStream,
};
use golem_stt::golem::stt::types::{AudioConfig, SttError, TranscriptionResult, TranscriptAlternative};
use golem_stt::golem::stt::vocabularies::{Guest as VocabulariesGuest, Vocabulary};
use log::{error, trace};
use std::cell::RefCell;
use std::collections::HashMap;

mod bindings;
mod client;
mod conversions;

struct GoogleSTTComponent;

impl GoogleSTTComponent {
    const API_KEY_ENV_VAR: &'static str = "GOOGLE_API_KEY";

    fn get_client() -> Result<GoogleSpeechClient, SttError> {
        let api_key = std::env::var(Self::API_KEY_ENV_VAR)
            .map_err(|_| SttError::Unauthorized("GOOGLE_API_KEY not set".to_string()))?;
        
        Ok(GoogleSpeechClient::new(api_key))
    }
}

pub struct GoogleTranscriptionStream {
    session: Option<GoogleStreamingSession>,
    is_finished: RefCell<bool>,
    result: RefCell<Option<TranscriptionResult>>,
}

impl GoogleTranscriptionStream {
    pub fn new(session: GoogleStreamingSession) -> Self {
        Self {
            session: Some(session),
            is_finished: RefCell::new(false),
            result: RefCell::new(None),
        }
    }
}

impl golem_stt::golem::stt::transcription::GuestTranscriptionStream for GoogleTranscriptionStream {
    fn send_audio(&self, chunk: Vec<u8>) -> Result<(), SttError> {
        if *self.is_finished.borrow() {
            return Err(SttError::InternalError("Stream already finished".to_string()));
        }

        if let Some(session) = &self.session {
            session.send_audio(chunk)?;
            trace!("Sent audio chunk to Google streaming session");
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
            trace!("Finishing Google real-time streaming session");
            
            // For real-time streaming, we don't need to wait for a final result
            // The session has been processing audio chunks in real-time
            // Just mark as finished so no more audio can be sent
            session.close();
            *is_finished = true;
            trace!("Google real-time streaming session finished successfully");
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
                for alternative in streaming_result.alternatives {
                    // Convert words if available
                    let words = if let Some(ref words) = alternative.words {
                        words.iter().map(|w| golem_stt::golem::stt::types::WordSegment {
                            text: w.word.clone().unwrap_or_default(),
                            start_time: w.start_time.as_ref().and_then(|s| s.parse().ok()).unwrap_or(0.0),
                            end_time: w.end_time.as_ref().and_then(|s| s.parse().ok()).unwrap_or(0.0),
                            confidence: w.confidence,
                            speaker_id: w.speaker_tag.map(|tag| tag.to_string()),
                        }).collect()
                    } else {
                        vec![]
                    };
                    
                    let transcript_text = alternative.transcript.clone().unwrap_or_default();
                    trace!("Returning Google real-time streaming alternative: {} (final: {})", 
                           transcript_text, streaming_result.is_final);
                    
                    return Ok(Some(TranscriptAlternative {
                        text: transcript_text,
                        confidence: alternative.confidence.unwrap_or(0.0),
                        words,
                    }));
                }
            }
        }
        
        // If finished and no session, check buffered results
        if *self.is_finished.borrow() {
            let mut result = self.result.borrow_mut();
            if let Some(transcription_result) = result.take() {
                if let Some(alternative) = transcription_result.alternatives.into_iter().next() {
                    trace!("Returning Google buffered streaming alternative: {}", alternative.text);
                    return Ok(Some(alternative));
                }
            }
        }
        
        Ok(None) // No alternatives available
    }

    fn close(&self) {
        if let Some(session) = &self.session {
            session.close();
        }
        trace!("Google streaming session closed");
    }
}

impl TranscriptionGuest for GoogleSTTComponent {
    type TranscriptionStream = GoogleTranscriptionStream;
    fn transcribe(
        audio: Vec<u8>,
        config: AudioConfig,
        options: Option<TranscribeOptions>,
    ) -> Result<TranscriptionResult, SttError> {
        golem_stt::init_logging();
        trace!("Starting Google Speech transcription, audio size: {} bytes", audio.len());

        let client = Self::get_client()?;
        let request = create_recognize_request(&audio, &config, &options)?;
        
        let language = options
            .as_ref()
            .and_then(|opts| opts.language.as_ref())
            .unwrap_or(&"en-US".to_string())
            .clone();

        let response = client.transcribe(request)
            .map_err(|e| {
                error!("Google Speech API call failed: {:?}", e);
                e
            })?;

        convert_response(response, audio.len(), &language)
    }

    fn transcribe_stream(
        config: AudioConfig,
        options: Option<TranscribeOptions>,
    ) -> Result<TranscriptionStream, SttError> {
        golem_stt::init_logging();
        trace!("Starting Google Speech streaming transcription");

        let client = Self::get_client()?;
        let recognition_config = create_recognize_request(&vec![], &config, &options)?.config;
        
        let session = client.start_streaming_session(recognition_config)?;
        let stream = GoogleTranscriptionStream::new(session);
        
        Ok(TranscriptionStream::new(stream))
    }
}

impl LanguagesGuest for GoogleSTTComponent {
    fn list_languages() -> Result<Vec<LanguageInfo>, SttError> {
        Ok(get_supported_languages())
    }
}

// Simple in-memory vocabulary storage for this implementation
thread_local! {
    static VOCABULARIES: RefCell<HashMap<String, Vec<String>>> = RefCell::new(HashMap::new());
}

pub struct GoogleVocabulary {
    name: String,
}

impl golem_stt::golem::stt::vocabularies::GuestVocabulary for GoogleVocabulary {
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

impl VocabulariesGuest for GoogleSTTComponent {
    type Vocabulary = GoogleVocabulary;

    fn create_vocabulary(
        name: String,
        phrases: Vec<String>,
    ) -> Result<Vocabulary, SttError> {
        VOCABULARIES.with(|v| {
            v.borrow_mut().insert(name.clone(), phrases);
        });
        
        Ok(Vocabulary::new(GoogleVocabulary { name }))
    }
}

impl ExtendedTranscriptionGuest for GoogleSTTComponent {}
impl ExtendedVocabulariesGuest for GoogleSTTComponent {}
impl ExtendedLanguagesGuest for GoogleSTTComponent {}
impl ExtendedGuest for GoogleSTTComponent {}

type DurableGoogleSTTComponent = DurableSTT<GoogleSTTComponent>;

golem_stt::export_stt!(DurableGoogleSTTComponent with_types_in golem_stt);