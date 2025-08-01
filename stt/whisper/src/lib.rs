// Placeholder implementation for OpenAI Whisper
// TODO: Implement actual OpenAI Whisper integration

use golem_stt::durability::{DurableSTT, ExtendedTranscriptionGuest, ExtendedVocabulariesGuest, ExtendedLanguagesGuest, ExtendedGuest};
use golem_stt::golem::stt::languages::{Guest as LanguagesGuest, LanguageInfo};
use golem_stt::golem::stt::transcription::{
    Guest as TranscriptionGuest, TranscribeOptions, TranscriptionStream,
};
use golem_stt::golem::stt::types::{AudioConfig, SttError, TranscriptionResult};
use golem_stt::golem::stt::vocabularies::{Guest as VocabulariesGuest, Vocabulary};
use golem_rust::wasm_rpc::Resource;

struct WhisperSTTComponent;

impl TranscriptionGuest for WhisperSTTComponent {
    fn transcribe(
        _audio: Vec<u8>,
        _config: AudioConfig,
        _options: Option<TranscribeOptions>,
    ) -> Result<TranscriptionResult, SttError> {
        Err(SttError::UnsupportedOperation("Whisper STT not yet implemented".to_string()))
    }

    fn transcribe_stream(
        _config: AudioConfig,
        _options: Option<TranscribeOptions>,
    ) -> Result<Resource<TranscriptionStream>, SttError> {
        // Whisper doesn't support streaming, so this should always return an error
        Err(SttError::UnsupportedOperation("Whisper does not support streaming transcription".to_string()))
    }
}

impl LanguagesGuest for WhisperSTTComponent {
    fn list_languages() -> Result<Vec<LanguageInfo>, SttError> {
        Err(SttError::UnsupportedOperation("Whisper STT not yet implemented".to_string()))
    }
}

pub struct WhisperVocabulary;

impl golem_stt::golem::stt::vocabularies::GuestVocabulary for WhisperVocabulary {
    fn get_name(&self) -> String {
        "placeholder".to_string()
    }

    fn get_phrases(&self) -> Vec<String> {
        vec![]
    }

    fn delete(&self) -> Result<(), SttError> {
        Ok(())
    }
}

impl VocabulariesGuest for WhisperSTTComponent {
    type Vocabulary = WhisperVocabulary;

    fn create_vocabulary(
        _name: String,
        _phrases: Vec<String>,
    ) -> Result<Resource<Vocabulary>, SttError> {
        // Whisper doesn't support custom vocabularies
        Err(SttError::UnsupportedOperation("Whisper does not support custom vocabularies".to_string()))
    }
}

impl ExtendedTranscriptionGuest for WhisperSTTComponent {}
impl ExtendedVocabulariesGuest for WhisperSTTComponent {}
impl ExtendedLanguagesGuest for WhisperSTTComponent {}
impl ExtendedGuest for WhisperSTTComponent {}

type DurableWhisperSTTComponent = DurableSTT<WhisperSTTComponent>;

golem_stt::export_stt!(DurableWhisperSTTComponent with_types_in golem_stt);