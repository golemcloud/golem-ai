use crate::client::GoogleSpeechClient;
use crate::conversions::{convert_response, create_recognize_request, get_supported_languages};
use golem_stt::config::with_config_key;
use golem_stt::durability::{DurableSTT, ExtendedTranscriptionGuest, ExtendedVocabulariesGuest, ExtendedLanguagesGuest, ExtendedGuest};
use golem_stt::golem::stt::languages::{Guest as LanguagesGuest, LanguageInfo};
use golem_stt::golem::stt::transcription::{
    Guest as TranscriptionGuest, TranscribeOptions, TranscriptionStream,
};
use golem_stt::golem::stt::types::{AudioConfig, SttError, TranscriptionResult};
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
    const PROJECT_ID_ENV_VAR: &'static str = "GOOGLE_CLOUD_PROJECT";

    fn get_client() -> Result<GoogleSpeechClient, SttError> {
        let api_key = std::env::var(Self::API_KEY_ENV_VAR)
            .map_err(|_| SttError::Unauthorized("GOOGLE_API_KEY not set".to_string()))?;
        
        let project_id = std::env::var(Self::PROJECT_ID_ENV_VAR)
            .map_err(|_| SttError::Unauthorized("GOOGLE_CLOUD_PROJECT not set".to_string()))?;
        
        Ok(GoogleSpeechClient::new(api_key, project_id))
    }
}

// Placeholder for TranscriptionStream - we'll implement this later
pub struct GoogleTranscriptionStream;

impl golem_stt::golem::stt::transcription::GuestTranscriptionStream for GoogleTranscriptionStream {
    fn send_audio(&self, _chunk: Vec<u8>) -> Result<(), SttError> {
        Err(SttError::UnsupportedOperation("Streaming not yet implemented".to_string()))
    }

    fn finish(&self) -> Result<(), SttError> {
        Err(SttError::UnsupportedOperation("Streaming not yet implemented".to_string()))
    }

    fn receive_alternative(&self) -> Result<Option<golem_stt::golem::stt::types::TranscriptAlternative>, SttError> {
        Err(SttError::UnsupportedOperation("Streaming not yet implemented".to_string()))
    }

    fn close(&self) {
        // No-op for now
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
        _config: AudioConfig,
        _options: Option<TranscribeOptions>,
    ) -> Result<TranscriptionStream, SttError> {
        // Google Cloud Speech streaming would require WebSocket or gRPC
        // For now, return an error indicating streaming is not supported in this implementation
        Err(SttError::UnsupportedOperation(
            "Streaming transcription not yet implemented for Google Speech".to_string(),
        ))
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