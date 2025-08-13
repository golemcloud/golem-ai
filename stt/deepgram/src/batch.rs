use golem_stt::golem::stt::transcription::{AudioConfig, TranscribeOptions, TranscriptionResult, TranscriptAlternative};
use golem_stt::golem::stt::types::SttError;
#[cfg(feature = "durability")]
use golem_rust::{FromValueAndType, IntoValue};
#[cfg(feature = "durability")]
use golem_stt::durability::durable_impl;

pub fn transcribe_impl(
    audio: Vec<u8>,
    cfg: &crate::config::DeepgramConfig,
    opts: Option<TranscribeOptions>,
    conf: AudioConfig,
) -> Result<TranscriptionResult, SttError> {
    let alternatives: Vec<TranscriptAlternative> = crate::recognize::recognize(&audio, cfg, &conf, &opts)?;
    let language = opts.as_ref().and_then(|o| o.language.clone()).unwrap_or_else(|| "en-US".to_string());
    let model = opts.as_ref().and_then(|o| o.model.clone()).or_else(|| cfg.default_model.clone());
    let metadata = golem_stt::golem::stt::types::TranscriptionMetadata {
        duration_seconds: 0.0,
        audio_size_bytes: audio.len() as u32,
        request_id: uuid::Uuid::new_v4().to_string(),
        model,
        language,
    };
    let result = TranscriptionResult { alternatives, metadata };

    #[cfg(feature = "durability")]
    {
        #[derive(Clone, Debug, FromValueAndType, IntoValue)]
        struct InputMeta {
            language: Option<String>,
            model: Option<String>,
            enable_timestamps: bool,
            enable_diarization: bool,
            enable_word_confidence: bool,
            audio_size_bytes: u32,
        }
        let input = InputMeta {
            language: opts.as_ref().and_then(|o| o.language.clone()),
            model: opts.as_ref().and_then(|o| o.model.clone()),
            enable_timestamps: opts.as_ref().and_then(|o| o.enable_timestamps).unwrap_or(false),
            enable_diarization: opts.as_ref().and_then(|o| o.enable_speaker_diarization).unwrap_or(false),
            enable_word_confidence: opts.as_ref().and_then(|o| o.enable_word_confidence).unwrap_or(false),
            audio_size_bytes: audio.len() as u32,
        };
        return durable_impl::persist_transcribe("golem_stt_deepgram", input, Ok(result));
    }

    #[cfg(not(feature = "durability"))]
    { Ok(result) }
}
