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
    /// Provider-specific configuration (API keys, regions, bucket
    /// names, etc.). Each provider crate defines its own concrete
    /// config type and threads it through every top-level call.
    type ProviderConfig: Clone + 'static;

    async fn transcribe(
        provider_config: Self::ProviderConfig,
        req: SttTranscriptionRequest,
    ) -> Result<TranscriptionResult, SttError>;

    async fn transcribe_many(
        provider_config: Self::ProviderConfig,
        wit_requests: Vec<SttTranscriptionRequest>,
    ) -> Result<MultiTranscriptionResult, SttError>;
}
