// Placeholder implementation for AWS Transcribe
// TODO: Implement actual AWS Transcribe integration

use golem_stt::durability::{DurableSTT, ExtendedTranscriptionGuest, ExtendedVocabulariesGuest, ExtendedLanguagesGuest, ExtendedGuest};
use golem_stt::golem::stt::languages::{Guest as LanguagesGuest, LanguageInfo};
use golem_stt::golem::stt::transcription::{
    Guest as TranscriptionGuest, TranscribeOptions, TranscriptionStream,
};
use golem_stt::golem::stt::types::{AudioConfig, SttError, TranscriptionResult};
use golem_stt::golem::stt::vocabularies::{Guest as VocabulariesGuest, Vocabulary};
use golem_rust::wasm_rpc::Resource;

struct AwsSTTComponent;

impl TranscriptionGuest for AwsSTTComponent {
    fn transcribe(
        _audio: Vec<u8>,
        _config: AudioConfig,
        _options: Option<TranscribeOptions>,
    ) -> Result<TranscriptionResult, SttError> {
        Err(SttError::UnsupportedOperation("AWS STT not yet implemented".to_string()))
    }

    fn transcribe_stream(
        _config: AudioConfig,
        _options: Option<TranscribeOptions>,
    ) -> Result<Resource<TranscriptionStream>, SttError> {
        Err(SttError::UnsupportedOperation("AWS STT not yet implemented".to_string()))
    }
}

impl LanguagesGuest for AwsSTTComponent {
    fn list_languages() -> Result<Vec<LanguageInfo>, SttError> {
        Err(SttError::UnsupportedOperation("AWS STT not yet implemented".to_string()))
    }
}

pub struct AwsVocabulary;

impl golem_stt::golem::stt::vocabularies::GuestVocabulary for AwsVocabulary {
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

impl VocabulariesGuest for AwsSTTComponent {
    type Vocabulary = AwsVocabulary;

    fn create_vocabulary(
        _name: String,
        _phrases: Vec<String>,
    ) -> Result<Resource<Vocabulary>, SttError> {
        Err(SttError::UnsupportedOperation("AWS STT not yet implemented".to_string()))
    }
}

impl ExtendedTranscriptionGuest for AwsSTTComponent {}
impl ExtendedVocabulariesGuest for AwsSTTComponent {}
impl ExtendedLanguagesGuest for AwsSTTComponent {}
impl ExtendedGuest for AwsSTTComponent {}

type DurableAwsSTTComponent = DurableSTT<AwsSTTComponent>;

golem_stt::export_stt!(DurableAwsSTTComponent with_types_in golem_stt);