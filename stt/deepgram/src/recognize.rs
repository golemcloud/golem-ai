use golem_stt::golem::stt::transcription::{AudioConfig, TranscribeOptions, TranscriptAlternative};
use golem_stt::golem::stt::types::{SttError, WordSegment};

pub(crate) fn recognize(
    _audio: &[u8],
    _cfg: &crate::config::DeepgramConfig,
    _conf: &AudioConfig,
    _opts: &Option<TranscribeOptions>,
) -> Result<Vec<TranscriptAlternative>, SttError> {
    Err(SttError::UnsupportedOperation("not implemented".into()))
}
