// Placeholder implementation for Deepgram
// TODO: Implement actual Deepgram integration

use golem_stt::durability::{DurableSTT, ExtendedTranscriptionGuest, ExtendedVocabulariesGuest, ExtendedLanguagesGuest, ExtendedGuest};
use golem_stt::golem::stt::languages::{Guest as LanguagesGuest, LanguageInfo};
use golem_stt::golem::stt::transcription::{
    Guest as TranscriptionGuest, TranscribeOptions, TranscriptionStream,
};
use golem_stt::golem::stt::types::{AudioConfig, SttError, TranscriptionResult};
use golem_stt::golem::stt::vocabularies::{Guest as VocabulariesGuest, Vocabulary};
use golem_rust::wasm_rpc::Resource;

struct DeepgramSTTComponent;

impl TranscriptionGuest for DeepgramSTTComponent {
    fn transcribe(
        _audio: Vec<u8>,
        _config: AudioConfig,
        _options: Option<TranscribeOptions>,
    ) -> Result<TranscriptionResult, SttError> {
        Err(SttError::UnsupportedOperation("Deepgram STT not yet implemented".to_string()))
    }

    fn transcribe_stream(
        _config: AudioConfig,
        _options: Option<TranscribeOptions>,
    ) -> Result<Resource<TranscriptionStream>, SttError> {
        Err(SttError::UnsupportedOperation("Deepgram STT not yet implemented".to_string()))
    }
}

impl LanguagesGuest for DeepgramSTTComponent {
    fn list_languages() -> Result<Vec<LanguageInfo>, SttError> {
        Err(SttError::UnsupportedOperation("Deepgram STT not yet implemented".to_string()))
    }
}

pub struct DeepgramVocabulary;

impl golem_stt::golem::stt::vocabularies::GuestVocabulary for DeepgramVocabulary {
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

impl VocabulariesGuest for DeepgramSTTComponent {
    type Vocabulary = DeepgramVocabulary;

    fn create_vocabulary(
        _name: String,
        _phrases: Vec<String>,
    ) -> Result<Resource<Vocabulary>, SttError> {
        Err(SttError::UnsupportedOperation("Deepgram STT not yet implemented".to_string()))
    }
}

impl ExtendedTranscriptionGuest for DeepgramSTTComponent {}
impl ExtendedVocabulariesGuest for DeepgramSTTComponent {}
impl ExtendedLanguagesGuest for DeepgramSTTComponent {}
impl ExtendedGuest for DeepgramSTTComponent {}

type DurableDeepgramSTTComponent = DurableSTT<DeepgramSTTComponent>;

golem_stt::export_stt!(DurableDeepgramSTTComponent with_types_in golem_stt);