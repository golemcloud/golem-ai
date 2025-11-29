use golem_rust::{FromValueAndType, IntoValue};
use golem_tts::golem::tts::{
    advanced::{
        GuestLongFormOperation, GuestPronunciationLexicon, LanguageCode, LongFormResult,
        OperationStatus,
    },
    types::TtsError,
};
use serde::{Deserialize, Serialize};

use crate::error::unsupported;

pub struct GoogleLongFormOperation;

impl GuestLongFormOperation for GoogleLongFormOperation {
    fn get_task_id(&self) -> Result<String, TtsError> {
        Ok("".to_string())
    }

    fn get_status(&self) -> Result<OperationStatus, TtsError> {
        Ok(OperationStatus::Failed)
    }

    fn get_progress(&self) -> Result<f32, TtsError> {
        Ok(0.0)
    }

    fn cancel(&self) -> Result<(), TtsError> {
        unsupported(
            "Google TTS long-form synthesis is currently in beta (v1beta1) and not yet supported",
        )
    }

    fn get_result(&self) -> Result<LongFormResult, TtsError> {
        unsupported(
            "Google TTS long-form synthesis is currently in beta (v1beta1) and not yet supported",
        )
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, IntoValue, FromValueAndType)]
pub struct VoiceResponse {
    #[serde(rename = "languageCodes")]
    pub language_codes: Vec<String>,
    pub name: String,
    #[serde(rename = "ssmlGender")]
    pub ssml_gender: String,
    #[serde(rename = "naturalSampleRateHertz")]
    pub natural_sample_rate_hertz: u32,
}
pub struct GooglePronunciationLexicon;

impl GuestPronunciationLexicon for GooglePronunciationLexicon {
    fn get_name(&self) -> String {
        "Unsupported".to_string()
    }

    fn get_language(&self) -> LanguageCode {
        "en".to_string()
    }

    fn get_entry_count(&self) -> u32 {
        0
    }

    #[doc = " Add pronunciation rule"]
    fn add_entry(&self, _word: String, _pronunciation: String) -> Result<(), TtsError> {
        unsupported("Google TTS does not support custom pronunciation lexicons")
    }

    #[doc = " Remove pronunciation rule"]
    fn remove_entry(&self, _word: String) -> Result<(), TtsError> {
        unsupported("Google TTS does not support custom pronunciation lexicons")
    }

    #[doc = " Export lexicon content"]
    fn export_content(&self) -> Result<String, TtsError> {
        unsupported("Google TTS does not support custom pronunciation lexicons")
    }
}
