use std::marker::PhantomData;

use crate::golem::tts::{
    advanced::{Guest as AdvancedTrait, LanguageCode, PronunciationEntry, Voice},
    types::TtsError,
};

pub struct DurableTTS<Impl> {
    pub(crate) phantom: PhantomData<Impl>,
}

impl<Impl> Default for DurableTTS<Impl> {
    fn default() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}

impl From<&TtsError> for TtsError {
    fn from(error: &TtsError) -> Self {
        error.clone()
    }
}

pub trait ExtendedAdvancedTrait: AdvancedTrait + 'static {
    fn unwrappered_created_lexicon(
        name: String,
        language: LanguageCode,
        entries: Option<Vec<PronunciationEntry>>,
    ) -> Result<Self::PronunciationLexicon, TtsError>;
    fn unwrappered_synthesize_long_form(
        content: String,
        voice: Voice,
        chapter_breaks: Option<Vec<u32>>,
        task_id: Option<String>,
    ) -> Result<Self::LongFormOperation, TtsError>;
}

pub mod advanced;
pub mod synthesis;
pub mod voices;
