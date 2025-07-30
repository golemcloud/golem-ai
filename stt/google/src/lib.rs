use golem_stt::golem::stt::transcription::{
    Guest as TranscriptionGuest, GuestTranscriptionStream, TranscriptAlternative,
    AudioConfig, TranscribeOptions, TranscriptionResult,
};
use golem_stt::golem::stt::types::SttError;
// Google provider stub only implements transcription interface for now.
// Vocabularies and languages interfaces are not implemented yet.

fn unsupported() -> SttError {
    SttError::UnsupportedOperation("not implemented".to_string())
}

// Minimal stub stream implementing GuestTranscriptionStream
struct DummyStream;

impl GuestTranscriptionStream for DummyStream {
    fn send_audio(&self, _chunk: Vec<u8>) -> Result<(), SttError> { Err(unsupported()) }
    fn finish(&self) -> Result<(), SttError> { Err(unsupported()) }
    fn receive_alternative(&self) -> Result<Option<TranscriptAlternative>, SttError> { Err(unsupported()) }
    fn close(&self) {}
}

// Local stub stream matching expected alias name
struct TranscriptionStream;

impl GuestTranscriptionStream for TranscriptionStream {
    fn send_audio(&self, _chunk: Vec<u8>) -> Result<(), SttError> { Err(unsupported()) }
    fn finish(&self) -> Result<(), SttError> { Err(unsupported()) }
    fn receive_alternative(&self) -> Result<Option<TranscriptAlternative>, SttError> { Err(unsupported()) }
    fn close(&self) {}
}

// -------------------- Transcription Interface --------------------

struct GoogleTranscriptionComponent;

impl TranscriptionGuest for GoogleTranscriptionComponent {
    type TranscriptionStream = TranscriptionStream;

    fn transcribe(
        _audio: Vec<u8>,
        _config: AudioConfig,
        _options: Option<TranscribeOptions>,
    ) -> Result<TranscriptionResult, SttError> {
        Err(unsupported())
    }

    fn transcribe_stream(
        _config: AudioConfig,
        _options: Option<TranscribeOptions>,
    ) -> Result<golem_stt::golem::stt::transcription::TranscriptionStream, SttError> {
        Err(unsupported())
    }
} 