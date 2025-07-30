use crate::auth::fetch_token;
use crate::config::GoogleConfig;
use crate::constants::GOOGLE_SPEECH_ENDPOINT;
use crate::error::map_http_status;
use golem_stt::golem::stt::transcription::{AudioConfig, TranscribeOptions, TranscriptionResult};
use golem_stt::golem::stt::types::SttError;
use serde_json::json;

pub fn transcribe_impl(audio: Vec<u8>, cfg: &GoogleConfig, _opts: Option<TranscribeOptions>, _conf: AudioConfig) -> Result<TranscriptionResult, SttError> {
    // placeholder: return unsupported until streaming implemented
    Err(SttError::UnsupportedOperation("transcribe not yet implemented".into()))
} 