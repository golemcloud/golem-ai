use golem_rust::{FromValueAndType, IntoValue};
use golem_tts::golem::tts::advanced::{
    GuestLongFormOperation, GuestPronunciationLexicon, LanguageCode, LongFormResult,
    OperationStatus, TtsError,
};
use serde::{Deserialize, Serialize};

use crate::error::unsupported;

#[derive(Debug, Clone, Serialize, Deserialize, IntoValue, FromValueAndType)]
pub struct VoiceResponse {
    pub name: String,
    pub canonical_name: String,
    pub architecture: String,
    pub languages: Vec<String>,
    pub version: String,
    pub uuid: String,
    pub metadata: DeepgramVoiceMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize, IntoValue, FromValueAndType)]
pub struct DeepgramVoiceMetadata {
    pub accent: String,
    pub age: String,
    pub color: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub image: String,
    pub sample: String,
    pub tags: Vec<String>,
    pub use_cases: Vec<String>,
}

pub struct DeepgramPronunciationLexicon;

impl GuestPronunciationLexicon for DeepgramPronunciationLexicon {
    fn get_name(&self) -> String {
        "Deepgram does not support pronunciation lexicon".to_string()
    }

    fn get_language(&self) -> LanguageCode {
        "Deepgram does not support pronunciation lexicon".to_string()
    }

    fn get_entry_count(&self) -> u32 {
        0
    }

    #[doc = " Add pronunciation rule"]
    fn add_entry(&self, _word: String, _pronunciation: String) -> Result<(), TtsError> {
        unsupported("Deepgram does not support pronunciation lexicon")
    }

    #[doc = " Remove pronunciation rule"]
    fn remove_entry(&self, _word: String) -> Result<(), TtsError> {
        unsupported("Deepgram does not support pronunciation lexicon")
    }

    #[doc = " Export lexicon content"]
    fn export_content(&self) -> Result<String, TtsError> {
        unsupported("Deepgram does not support pronunciation lexicon")
    }
}

pub struct DeepgramLongFormOperation;

impl GuestLongFormOperation for DeepgramLongFormOperation {
    fn get_task_id(&self) -> Result<String, TtsError> {
        Ok("".to_string())
    }

    fn get_status(&self) -> Result<OperationStatus, TtsError> {
        Ok(OperationStatus::Failed)
    }

    fn get_progress(&self) -> Result<f32, TtsError> {
        Ok(100.0)
    }

    fn cancel(&self) -> Result<(), TtsError> {
        unsupported("Deepgram does not support long form operations")
    }

    fn get_result(&self) -> Result<LongFormResult, TtsError> {
        unsupported("Deepgram does not support long form operations")
    }
}
