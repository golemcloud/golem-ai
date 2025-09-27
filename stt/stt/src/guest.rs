use bytes::Bytes;

use crate::exports::golem::stt::transcription::{
    AudioConfig as WitAudioConfig, MultiTranscriptionResult as WitMultiTranscriptionResult,
    TranscribeOptions as WitTranscribeOptions,
};

use crate::exports::golem::stt::types::{
    SttError as WitSttError, TranscriptionResult as WitTranscriptionResult,
};

pub struct SttTranscriptionRequest {
    pub request_id: String,
    pub audio: Bytes,
    pub config: WitAudioConfig,
    pub options: Option<WitTranscribeOptions>,
}

pub trait SttTranscriptionGuest {
    fn transcribe(req: SttTranscriptionRequest) -> Result<WitTranscriptionResult, WitSttError>;
    fn transcribe_many(
        wit_requests: Vec<SttTranscriptionRequest>,
    ) -> Result<WitMultiTranscriptionResult, WitSttError>;
}
