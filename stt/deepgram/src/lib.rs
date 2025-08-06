use crate::client::DeepgramClient;
use crate::conversions::{
    convert_deepgram_response, create_prerecorded_request, get_supported_languages,
    get_recommended_model,
};
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

mod client;
mod conversions;

struct DeepgramSTTComponent;

impl DeepgramSTTComponent {
    const API_KEY_ENV_VAR: &'static str = "DEEPGRAM_API_KEY";

    fn get_client() -> Result<DeepgramClient, SttError> {
        let api_key = std::env::var(Self::API_KEY_ENV_VAR)
            .map_err(|_| SttError::Unauthorized("DEEPGRAM_API_KEY not set".to_string()))?;
        
        Ok(DeepgramClient::new(api_key))
    }
}

// Placeholder for TranscriptionStream - Deepgram streaming would require WebSocket
pub struct DeepgramTranscriptionStream;

impl golem_stt::golem::stt::transcription::GuestTranscriptionStream for DeepgramTranscriptionStream {
    fn send_audio(&self, _chunk: Vec<u8>) -> Result<(), SttError> {
        Err(SttError::UnsupportedOperation("Deepgram streaming not yet implemented - requires WebSocket".to_string()))
    }

    fn finish(&self) -> Result<(), SttError> {
        Err(SttError::UnsupportedOperation("Deepgram streaming not yet implemented - requires WebSocket".to_string()))
    }

    fn receive_alternative(&self) -> Result<Option<golem_stt::golem::stt::types::TranscriptAlternative>, SttError> {
        Err(SttError::UnsupportedOperation("Deepgram streaming not yet implemented - requires WebSocket".to_string()))
    }

    fn close(&self) {
        // No-op for now
    }
}

impl TranscriptionGuest for DeepgramSTTComponent {
    type TranscriptionStream = DeepgramTranscriptionStream;

    fn transcribe(
        audio: Vec<u8>,
        config: AudioConfig,
        options: Option<TranscribeOptions>,
    ) -> Result<TranscriptionResult, SttError> {
        golem_stt::init_logging();
        trace!("Starting Deepgram transcription, audio size: {} bytes", audio.len());

        let client = Self::get_client()?;
        let mut request = create_prerecorded_request(&audio, &config, &options)?;
        
        let language = options
            .as_ref()
            .and_then(|opts| opts.language.as_ref())
            .unwrap_or(&"en-US".to_string())
            .clone();

        // Auto-select model based on language and use case
        if request.model.is_none() {
            let use_case = if request.diarize { "meeting" } else { "general" };
            request.model = Some(get_recommended_model(&language, use_case));
            trace!("Auto-selected Deepgram model: {:?}", request.model);
        }

        let deepgram_response = client.transcribe_prerecorded(request)
            .map_err(|e| {
                error!("Deepgram transcription failed: {:?}", e);
                e
            })?;

        convert_deepgram_response(deepgram_response, audio.len(), &language)
    }

    fn transcribe_stream(
        _config: AudioConfig,
        _options: Option<TranscribeOptions>,
    ) -> Result<TranscriptionStream, SttError> {
        // Deepgram streaming would require WebSocket connection
        Err(SttError::UnsupportedOperation(
            "Deepgram streaming not yet implemented - requires WebSocket".to_string(),
        ))
    }
}

impl LanguagesGuest for DeepgramSTTComponent {
    fn list_languages() -> Result<Vec<LanguageInfo>, SttError> {
        Ok(get_supported_languages())
    }
}

// Simple in-memory vocabulary storage for this implementation
// Deepgram uses "keywords" for vocabulary boosting
thread_local! {
    static VOCABULARIES: RefCell<HashMap<String, Vec<String>>> = RefCell::new(HashMap::new());
}

pub struct DeepgramVocabulary {
    name: String,
}

impl golem_stt::golem::stt::vocabularies::GuestVocabulary for DeepgramVocabulary {
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

impl VocabulariesGuest for DeepgramSTTComponent {
    type Vocabulary = DeepgramVocabulary;

    fn create_vocabulary(
        name: String,
        phrases: Vec<String>,
    ) -> Result<Vocabulary, SttError> {
        // Validate vocabulary size (Deepgram has limits)
        if phrases.len() > 1000 {
            return Err(SttError::InvalidAudio(
                "Deepgram vocabulary cannot exceed 1000 keywords".to_string()
            ));
        }

        // Validate individual phrase length
        for phrase in &phrases {
            if phrase.len() > 100 {
                return Err(SttError::InvalidAudio(
                    format!("Deepgram keyword '{}' exceeds 100 character limit", phrase)
                ));
            }
        }

        VOCABULARIES.with(|v| {
            v.borrow_mut().insert(name.clone(), phrases);
        });
        
        Ok(Vocabulary::new(DeepgramVocabulary { name }))
    }
}

impl ExtendedTranscriptionGuest for DeepgramSTTComponent {}
impl ExtendedVocabulariesGuest for DeepgramSTTComponent {}
impl ExtendedLanguagesGuest for DeepgramSTTComponent {}
impl ExtendedGuest for DeepgramSTTComponent {}

type DurableDeepgramSTTComponent = DurableSTT<DeepgramSTTComponent>;

golem_stt::export_stt!(DurableDeepgramSTTComponent with_types_in golem_stt);