use golem_stt::golem::stt::transcription::{TranscriptAlternative, GuestTranscriptionStream};
use golem_stt::golem::stt::types::SttError;

pub struct GoogleStream;

impl GuestTranscriptionStream for GoogleStream {
    fn send_audio(&self, _chunk: Vec<u8>) -> Result<(), SttError> {
        Err(SttError::UnsupportedOperation("streaming not yet implemented".into()))
    }
    fn finish(&self) -> Result<(), SttError> {
        Err(SttError::UnsupportedOperation("streaming not yet implemented".into()))
    }
    fn receive_alternative(&self) -> Result<Option<TranscriptAlternative>, SttError> {
        Err(SttError::UnsupportedOperation("streaming not yet implemented".into()))
    }
    fn close(&self) {}
} 