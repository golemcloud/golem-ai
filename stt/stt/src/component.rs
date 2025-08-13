use crate::exports::golem::stt::{languages, transcription, types as wit_types, vocabularies};

pub struct Component;

// No static variables needed - durability handled by DurableStt wrapper

pub struct VocabularyResource {
    name: String,
    phrases: Vec<String>,
}

impl vocabularies::GuestVocabulary for VocabularyResource {
    fn get_name(&self) -> String {
        self.name.clone()
    }

    fn get_phrases(&self) -> Vec<String> {
        self.phrases.clone()
    }

    fn delete(&self) -> Result<(), wit_types::SttError> {
        // Vocabulary deletion handled by durability layer
        Ok(())
    }
}

impl vocabularies::Guest for Component {
    type Vocabulary = VocabularyResource;

    fn create_vocabulary(
        name: String,
        phrases: Vec<String>,
    ) -> Result<vocabularies::Vocabulary, wit_types::SttError> {
        let vocab = VocabularyResource { name, phrases };
        Ok(vocabularies::Vocabulary::new(vocab))
    }
}

impl languages::Guest for Component {
    fn list_languages() -> Result<Vec<languages::LanguageInfo>, wit_types::SttError> {
        // Return empty list for base implementation
        Ok(vec![])
    }
}

impl transcription::Guest for Component {
    type TranscriptionStream = TranscriptionStreamResource;

    fn transcribe(
        _audio: Vec<u8>,
        _config: wit_types::AudioConfig,
        _options: Option<transcription::TranscribeOptions>,
    ) -> Result<wit_types::TranscriptionResult, wit_types::SttError> {
        Err(wit_types::SttError::UnsupportedOperation(
            "Base STT library does not provide transcription implementation".to_string(),
        ))
    }

    fn transcribe_stream(
        _config: wit_types::AudioConfig,
        _options: Option<transcription::TranscribeOptions>,
    ) -> Result<transcription::TranscriptionStream, wit_types::SttError> {
        Err(wit_types::SttError::UnsupportedOperation(
            "Base STT library does not provide streaming transcription implementation".to_string(),
        ))
    }
}

pub struct TranscriptionStreamResource;

impl transcription::GuestTranscriptionStream for TranscriptionStreamResource {
    fn send_audio(&self, _chunk: Vec<u8>) -> Result<(), wit_types::SttError> {
        Err(wit_types::SttError::UnsupportedOperation(
            "Base STT library does not provide streaming transcription implementation".to_string(),
        ))
    }

    fn finish(&self) -> Result<(), wit_types::SttError> {
        Err(wit_types::SttError::UnsupportedOperation(
            "Base STT library does not provide streaming transcription implementation".to_string(),
        ))
    }

    fn receive_alternative(
        &self,
    ) -> Result<Option<wit_types::TranscriptAlternative>, wit_types::SttError> {
        Err(wit_types::SttError::UnsupportedOperation(
            "Base STT library does not provide streaming transcription implementation".to_string(),
        ))
    }

    fn close(&self) {
        // No-op for base implementation
    }
}

// Component is exported via lib.rs
