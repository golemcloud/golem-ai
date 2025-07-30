use golem_stt::golem::stt::transcription::{
    Guest as TranscriptionGuest,
    AudioConfig, TranscribeOptions, TranscriptionResult,
    TranscriptionStream,
};
use golem_stt::golem::stt::types::SttError;

pub mod config;
mod auth;
mod constants;
mod recognize;
pub mod error;
mod batch;
mod stream;

pub use crate::stream::GoogleStream;

struct GoogleTranscriptionComponent;

impl TranscriptionGuest for GoogleTranscriptionComponent {
    type TranscriptionStream = GoogleStream;

    fn transcribe(
        _audio: Vec<u8>,
        config: AudioConfig,
        options: Option<TranscribeOptions>,
    ) -> Result<TranscriptionResult, SttError> {
        let cfg = crate::config::GoogleConfig::load().map_err(|e| e)?;
        crate::batch::transcribe_impl(_audio, &cfg, options, config)
    }

    fn transcribe_stream(
        config: AudioConfig,
        options: Option<TranscribeOptions>,
    ) -> Result<TranscriptionStream, SttError> {
        let cfg = crate::config::GoogleConfig::load().map_err(|e| e)?;
        Ok(TranscriptionStream::new(GoogleStream::new(cfg, config, options)))
    }
} 