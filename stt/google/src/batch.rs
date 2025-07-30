use crate::recognize::recognize;
use crate::config::GoogleConfig;
use golem_stt::golem::stt::transcription::{
    AudioConfig, TranscribeOptions, TranscriptionResult,
};
use golem_stt::golem::stt::types::TranscriptionMetadata;
use golem_stt::golem::stt::types::SttError;

pub fn transcribe_impl(
    audio: Vec<u8>,
    cfg: &GoogleConfig,
    opts: Option<TranscribeOptions>,
    conf: AudioConfig,
) -> Result<TranscriptionResult, SttError> {
    match recognize(&audio, cfg, &conf, &opts) {
        Ok(alternatives) => {
            let metadata = TranscriptionMetadata {
                duration_seconds: 0.0,
                audio_size_bytes: audio.len() as u32,
                request_id: String::new(),
                model: None,
                language: opts
                    .as_ref()
                    .and_then(|o| o.language.clone())
                    .unwrap_or_else(|| "unknown".to_string()),
            };
            Ok(TranscriptionResult {
                alternatives,
                metadata,
            })
        }
        Err(e) => Err(e),
    }
} 