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
#[cfg(feature = "durability")]
pub mod durability;

pub use crate::stream::GoogleStream;

#[allow(dead_code)]
pub struct GoogleTranscriptionComponent;

impl TranscriptionGuest for GoogleTranscriptionComponent {
    #[cfg(feature = "durability")]
    type TranscriptionStream = crate::durability::DurableTranscriptionStream<GoogleStream>;
    #[cfg(not(feature = "durability"))]
    type TranscriptionStream = GoogleStream;

    fn transcribe(
        _audio: Vec<u8>,
        config: AudioConfig,
        options: Option<TranscribeOptions>,
    ) -> Result<TranscriptionResult, SttError> {
        let cfg = crate::config::GoogleConfig::load()?;
        crate::batch::transcribe_impl(_audio, &cfg, options, config)
    }

    fn transcribe_stream(
        config: AudioConfig,
        options: Option<TranscribeOptions>,
    ) -> Result<TranscriptionStream, SttError> {
        let cfg = crate::config::GoogleConfig::load()?;
        #[cfg(feature = "durability")]
        {
            return Ok(TranscriptionStream::new(crate::durability::DurableTranscriptionStream::new_wrapped_stream(
                cfg, config, options,
            )));
        }
        #[cfg(not(feature = "durability"))]
        {
            Ok(TranscriptionStream::new(GoogleStream::new(cfg, config, options)))
        }
    }
} 