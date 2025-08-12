use golem_stt::golem::stt::transcription::{
    Guest as TranscriptionGuest,
    AudioConfig, TranscribeOptions, TranscriptionResult,
    TranscribeRequest,
};
use golem_stt::golem::stt::types::SttError;

pub mod config;
mod auth;
mod constants;
mod recognize;
pub mod error;
mod batch;
pub mod languages;
#[cfg(feature = "durability")]
pub mod durability;

#[allow(dead_code)]
pub struct GoogleTranscriptionComponent;

impl TranscriptionGuest for GoogleTranscriptionComponent {
    fn transcribe(
        _audio: Vec<u8>,
        config: AudioConfig,
        options: Option<TranscribeOptions>,
    ) -> Result<TranscriptionResult, SttError> {
        let cfg = crate::config::GoogleConfig::load()?;
        crate::batch::transcribe_impl(_audio, &cfg, options, config)
    }

    fn multi_transcribe(requests: Vec<TranscribeRequest>) -> Result<Vec<TranscriptionResult>, SttError> {
        let mut results = Vec::with_capacity(requests.len());
        for req in requests {
            results.push(Self::transcribe(req.audio, req.config, req.options)?);
        }
        Ok(results)
    }
}

pub struct GoogleLanguagesComponent;

impl golem_stt::golem::stt::languages::Guest for GoogleLanguagesComponent {
    fn list_languages() -> Result<Vec<golem_stt::golem::stt::languages::LanguageInfo>, SttError> {
        crate::languages::GoogleLanguagesComponent::list_languages()
    }
} 