use crate::model::transcription::{MultiTranscriptionResult, TranscribeOptions};
use crate::model::types::{AudioConfig, SttError, TranscriptionResult};
use bytes::Bytes;

pub struct SttTranscriptionRequest {
    pub request_id: String,
    pub audio: Bytes,
    pub config: AudioConfig,
    pub options: Option<TranscribeOptions>,
}

pub trait SttTranscriptionProvider {
    fn transcribe(req: SttTranscriptionRequest) -> Result<TranscriptionResult, SttError>;
    fn transcribe_many(
        wit_requests: Vec<SttTranscriptionRequest>,
    ) -> Result<MultiTranscriptionResult, SttError>;
}
