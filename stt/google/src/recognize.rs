use golem_stt::golem::stt::transcription::{AudioConfig, TranscribeOptions, TranscriptAlternative};
use golem_stt::golem::stt::types::SttError;
use crate::config::GoogleConfig;

pub(crate) fn recognize(
    _audio: &[u8],
    _cfg: &GoogleConfig,
    _conf: &AudioConfig,
    _opts: &Option<TranscribeOptions>,
) -> Result<Vec<TranscriptAlternative>, SttError> {
    Err(SttError::UnsupportedOperation("recognize not yet implemented".into()))
} 