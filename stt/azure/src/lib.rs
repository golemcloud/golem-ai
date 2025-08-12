use golem_stt::golem::stt::transcription::{Guest as TranscriptionGuest, AudioConfig, TranscribeOptions, TranscriptionResult, TranscriptionStream};
use golem_stt::golem::stt::types::SttError;

pub mod config;
pub mod error;
mod recognize;
mod batch;

pub struct AzureTranscriptionComponent;

impl TranscriptionGuest for AzureTranscriptionComponent {
    type TranscriptionStream = DummyStream;

    fn transcribe(
        audio: Vec<u8>,
        config: AudioConfig,
        options: Option<TranscribeOptions>,
    ) -> Result<TranscriptionResult, SttError> {
        let cfg = crate::config::AzureConfig::load()?;
        crate::batch::transcribe_impl(audio, &cfg, options, config)
    }

    fn transcribe_stream(
        _config: AudioConfig,
        _options: Option<TranscribeOptions>,
    ) -> Result<TranscriptionStream, SttError> {
        Err(SttError::UnsupportedOperation("streaming not supported".into()))
    }
}

pub struct DummyStream;

impl golem_stt::golem::stt::transcription::GuestTranscriptionStream for DummyStream {
    fn send_audio(&self, _chunk: Vec<u8>) -> Result<(), SttError> { Err(SttError::UnsupportedOperation("not implemented".into())) }
    fn finish(&self) -> Result<(), SttError> { Err(SttError::UnsupportedOperation("not implemented".into())) }
    fn receive_alternative(&self) -> Result<Option<golem_stt::golem::stt::transcription::TranscriptAlternative>, SttError> { Err(SttError::UnsupportedOperation("not implemented".into())) }
    fn close(&self) {}
}



