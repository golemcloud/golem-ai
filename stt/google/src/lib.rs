use golem_stt::golem::stt::transcription::{
    Guest as TranscriptionGuest, GuestTranscriptionStream, TranscriptAlternative,
    AudioConfig, TranscribeOptions, TranscriptionResult,
};
use golem_stt::golem::stt::types::SttError;

mod config;
mod auth;
mod constants;
mod error;
mod batch;

fn unsupported() -> SttError {
    SttError::UnsupportedOperation("not implemented".to_string())
}


struct DummyStream;

impl GuestTranscriptionStream for DummyStream {
    fn send_audio(&self, _chunk: Vec<u8>) -> Result<(), SttError> { Err(unsupported()) }
    fn finish(&self) -> Result<(), SttError> { Err(unsupported()) }
    fn receive_alternative(&self) -> Result<Option<TranscriptAlternative>, SttError> { Err(unsupported()) }
    fn close(&self) {}
}

struct TranscriptionStream;

impl GuestTranscriptionStream for TranscriptionStream {
    fn send_audio(&self, _chunk: Vec<u8>) -> Result<(), SttError> { Err(unsupported()) }
    fn finish(&self) -> Result<(), SttError> { Err(unsupported()) }
    fn receive_alternative(&self) -> Result<Option<TranscriptAlternative>, SttError> { Err(unsupported()) }
    fn close(&self) {}
}

struct GoogleTranscriptionComponent;

impl TranscriptionGuest for GoogleTranscriptionComponent {
    type TranscriptionStream = TranscriptionStream;

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
    ) -> Result<golem_stt::golem::stt::transcription::TranscriptionStream, SttError> {
        Err(unsupported())
    }
} 