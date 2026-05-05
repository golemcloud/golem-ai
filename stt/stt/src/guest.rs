use crate::model::transcription::{MultiTranscriptionResult, TranscribeOptions};
use crate::model::types::{AudioConfig, SttError, TranscriptionResult};
use bytes::Bytes;

pub struct SttTranscriptionRequest {
    pub request_id: String,
    pub audio: Bytes,
    pub config: AudioConfig,
    pub options: Option<TranscribeOptions>,
}

#[allow(async_fn_in_trait)]
pub trait SttTranscriptionProvider {
    async fn transcribe(req: SttTranscriptionRequest) -> Result<TranscriptionResult, SttError>;
    async fn transcribe_many(
        wit_requests: Vec<SttTranscriptionRequest>,
    ) -> Result<MultiTranscriptionResult, SttError>;
}
