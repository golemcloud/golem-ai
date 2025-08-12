use crate::recognize::RecognizeOut;
use crate::recognize::recognize;
use crate::config::AzureConfig;
use golem_stt::golem::stt::transcription::{
    AudioConfig, TranscribeOptions, TranscriptionResult,
};
use golem_stt::golem::stt::types::TranscriptionMetadata;
use golem_stt::golem::stt::types::SttError;

pub fn transcribe_impl(
    audio: Vec<u8>,
    cfg: &AzureConfig,
    opts: Option<TranscribeOptions>,
    conf: AudioConfig,
) -> Result<TranscriptionResult, SttError> {
    match recognize(&audio, cfg, &conf, &opts) {
        Ok(RecognizeOut { alternatives, request_id, elapsed_secs }) => {
            let metadata = TranscriptionMetadata {
                duration_seconds: elapsed_secs,
                audio_size_bytes: audio.len() as u32,
                request_id: request_id.unwrap_or_default(),
                model: None,
                language: opts
                    .as_ref()
                    .and_then(|o| o.language.clone())
                    .unwrap_or_else(|| "en-US".to_string()),
            };
            Ok(TranscriptionResult { alternatives, metadata })
        }
        Err(e) => Err(e),
    }
}
