use golem_stt::golem::stt::transcription::{AudioConfig, TranscribeOptions, TranscriptAlternative};
use golem_stt::golem::stt::types::SttError;
use crate::config::GoogleConfig;

#[cfg(not(test))]
pub(crate) fn recognize(
    _audio: &[u8],
    _cfg: &GoogleConfig,
    _conf: &AudioConfig,
    _opts: &Option<TranscribeOptions>,
) -> Result<Vec<TranscriptAlternative>, SttError> {
    Err(SttError::UnsupportedOperation("recognize not yet implemented".into()))
}

#[cfg(test)]
pub(crate) fn recognize(
    _audio: &[u8],
    _cfg: &GoogleConfig,
    _conf: &AudioConfig,
    _opts: &Option<TranscribeOptions>,
) -> Result<Vec<TranscriptAlternative>, SttError> {
    Ok(vec![TranscriptAlternative { text: "hello".into(), confidence: 0.9, words: Vec::new() }])
} 