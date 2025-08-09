use crate::client::WhisperClient;
use crate::conversions::{
    convert_whisper_response, create_whisper_request, get_supported_languages,
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
use std::collections::HashMap;

mod client;
mod conversions;

struct WhisperSTTComponent;

impl WhisperSTTComponent {
    const API_KEY_ENV_VAR: &'static str = "OPENAI_API_KEY";

    fn get_client() -> Result<WhisperClient, SttError> {
        let api_key = std::env::var(Self::API_KEY_ENV_VAR)
            .map_err(|_| SttError::Unauthorized("OPENAI_API_KEY not set".to_string()))?;
        
        Ok(WhisperClient::new(api_key))
    }
}

// Whisper does not support streaming per bounty requirements
pub struct WhisperTranscriptionStream;

impl golem_stt::golem::stt::transcription::GuestTranscriptionStream for WhisperTranscriptionStream {
    fn send_audio(&self, _chunk: Vec<u8>) -> Result<(), SttError> {
        Err(SttError::UnsupportedOperation("OpenAI Whisper does not support streaming transcription".to_string()))
    }

    fn finish(&self) -> Result<(), SttError> {
        Err(SttError::UnsupportedOperation("OpenAI Whisper does not support streaming transcription".to_string()))
    }

    fn receive_alternative(&self) -> Result<Option<TranscriptAlternative>, SttError> {
        Err(SttError::UnsupportedOperation("OpenAI Whisper does not support streaming transcription".to_string()))
    }

    fn close(&self) {
        // No-op for unsupported streaming
    }
}

impl TranscriptionGuest for WhisperSTTComponent {
    type TranscriptionStream = WhisperTranscriptionStream;

    fn transcribe(
        audio: Vec<u8>,
        config: AudioConfig,
        options: Option<TranscribeOptions>,
    ) -> Result<TranscriptionResult, SttError> {
        golem_stt::init_logging();
        trace!("Starting OpenAI Whisper transcription, audio size: {} bytes", audio.len());

        // Check for unsupported features and warn
        if let Some(ref opts) = options {
            if opts.enable_speaker_diarization.unwrap_or(false) {
                warn!("Speaker diarization is not supported by OpenAI Whisper");
            }
        }

        let client = Self::get_client()?;
        let request = create_whisper_request(&audio, &config, &options)?;
        
        let language = options
            .as_ref()
            .and_then(|opts| opts.language.as_ref())
            .unwrap_or(&"en".to_string())
            .clone();

        let whisper_response = client.transcribe_audio(request)
            .map_err(|e| {
                error!("OpenAI Whisper transcription failed: {:?}", e);
                e
            })?;

        convert_whisper_response(whisper_response, audio.len(), &language)
    }

    fn transcribe_stream(
        _config: AudioConfig,
        _options: Option<TranscribeOptions>,
    ) -> Result<TranscriptionStream, SttError> {
        // Per bounty requirements: "Streaming is not available in Whisper, so transcribe-stream should be omitted"
        Err(SttError::UnsupportedOperation("OpenAI Whisper does not support streaming transcription".to_string()))
    }
}

impl LanguagesGuest for WhisperSTTComponent {
    fn list_languages() -> Result<Vec<LanguageInfo>, SttError> {
        Ok(get_supported_languages())
    }
}

// Simple in-memory vocabulary storage for this implementation
// Note: Whisper doesn't support custom vocabularies, but we can use prompts
thread_local! {
    static VOCABULARIES: RefCell<HashMap<String, Vec<String>>> = RefCell::new(HashMap::new());
}

pub struct WhisperVocabulary {
    name: String,
}

impl golem_stt::golem::stt::vocabularies::GuestVocabulary for WhisperVocabulary {
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

impl VocabulariesGuest for WhisperSTTComponent {
    type Vocabulary = WhisperVocabulary;

    fn create_vocabulary(
        name: String,
        phrases: Vec<String>,
    ) -> Result<Vocabulary, SttError> {
        // Whisper doesn't support custom vocabularies natively, but we can store
        // phrases to use as prompts (context guidance)
        warn!("OpenAI Whisper does not support custom vocabularies. Phrases will be used as prompts for context guidance.");
        
        // Validate vocabulary size (reasonable limit for prompt usage)
        if phrases.len() > 100 {
            return Err(SttError::InvalidAudio(
                "Whisper vocabulary cannot exceed 100 phrases when used as prompts".to_string()
            ));
        }

        // Validate individual phrase length (Whisper prompt limit)
        for phrase in &phrases {
            if phrase.len() > 244 { // Whisper prompt limit is ~244 characters
                return Err(SttError::InvalidAudio(
                    format!("Whisper prompt phrase '{}' exceeds 244 character limit", phrase)
                ));
            }
        }

        VOCABULARIES.with(|v| {
            v.borrow_mut().insert(name.clone(), phrases);
        });
        
        Ok(Vocabulary::new(WhisperVocabulary { name }))
    }
}

impl ExtendedTranscriptionGuest for WhisperSTTComponent {}
impl ExtendedVocabulariesGuest for WhisperSTTComponent {}
impl ExtendedLanguagesGuest for WhisperSTTComponent {}
impl ExtendedGuest for WhisperSTTComponent {}

type DurableWhisperSTTComponent = DurableSTT<WhisperSTTComponent>;

golem_stt::export_stt!(DurableWhisperSTTComponent with_types_in golem_stt);