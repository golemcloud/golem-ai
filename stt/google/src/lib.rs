use golem_stt::golem::stt::transcription::{
    Guest as TranscriptionGuest, GuestTranscriptionStream, TranscriptAlternative,
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

#[allow(dead_code)]
struct DummyStream;

impl GuestTranscriptionStream for DummyStream {
    fn send_audio(&self, _chunk: Vec<u8>) -> Result<(), SttError> { Err(unsupported()) }
    fn finish(&self) -> Result<(), SttError> { Err(unsupported()) }
    fn receive_alternative(&self) -> Result<Option<TranscriptAlternative>, SttError> { Err(unsupported()) }
    fn close(&self) {}
}

fn unsupported() -> SttError {
    SttError::UnsupportedOperation("not implemented".to_string())
}

struct GoogleTranscriptionComponent;

impl TranscriptionGuest for GoogleTranscriptionComponent {
    type TranscriptionStream = DummyStream;

    fn transcribe(
        _audio: Vec<u8>,
        config: AudioConfig,
        options: Option<TranscribeOptions>,
    ) -> Result<TranscriptionResult, SttError> {
        let cfg = crate::config::GoogleConfig::load().map_err(|e| e)?;
        crate::batch::transcribe_impl(_audio, &cfg, options, config)
    }

    fn transcribe_stream(
        _config: AudioConfig,
        _options: Option<TranscribeOptions>,
    ) -> Result<TranscriptionStream, SttError> {
        Err(SttError::UnsupportedOperation("streaming not yet implemented".into()))
    }
} 