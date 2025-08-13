use golem_stt::golem::stt::transcription::{AudioConfig, TranscribeOptions, TranscriptionResult};
use golem_stt::golem::stt::types::SttError;
#[cfg(feature = "durability")]
use golem_stt::durability::saga::{Saga, SttCheckpoint};
#[cfg(feature = "durability")]
use golem_rust::bindings::golem::durability::durability::DurableFunctionType;
#[cfg(feature = "durability")]
use golem_rust::{FromValueAndType, IntoValue};
#[cfg(feature = "durability")]
use golem_stt::durability::durable_impl;

pub(crate) fn compute_pcm_duration_seconds(bytes: usize, sample_rate: u32, channels: u8) -> f32 {
    let bytes_per_sample = 2u32;
    let samples = bytes as f32 / (bytes_per_sample as f32 * channels as f32);
    samples / sample_rate as f32
}

pub fn transcribe_impl(
    audio: Vec<u8>,
    cfg: &crate::config::WhisperConfig,
    opts: Option<TranscribeOptions>,
    conf: AudioConfig,
) -> Result<TranscriptionResult, SttError> {
    #[cfg(feature = "durability")]
    let saga: Saga<TranscriptionResult, SttError> = Saga::new("golem_stt_whisper", "transcribe", DurableFunctionType::WriteRemote);
    let rec = crate::recognize::recognize(&audio, cfg, &conf, &opts)?;
    let alternatives = rec.alternatives;
    let language = opts.as_ref().and_then(|o| o.language.clone()).unwrap_or_else(|| "en".to_string());
    let model = opts.as_ref().and_then(|o| o.model.clone()).or_else(|| cfg.default_model.clone());
    let duration_seconds = if let (Some(sr), Some(ch)) = (conf.sample_rate, conf.channels) { compute_pcm_duration_seconds(audio.len(), sr, ch) } else { 0.0 };
    let metadata = golem_stt::golem::stt::types::TranscriptionMetadata {
        duration_seconds,
        audio_size_bytes: audio.len() as u32,
        request_id: rec.request_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
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
        let input = InputMeta { language: opts.as_ref().and_then(|o| o.language.clone()), model: opts.as_ref().and_then(|o| o.model.clone()), enable_timestamps: opts.as_ref().and_then(|o| o.enable_timestamps).unwrap_or(false), enable_diarization: opts.as_ref().and_then(|o| o.enable_speaker_diarization).unwrap_or(false), enable_word_confidence: opts.as_ref().and_then(|o| o.enable_word_confidence).unwrap_or(false), audio_size_bytes: audio.len() as u32 };
        let out = durable_impl::persist_transcribe("golem_stt_whisper", input, Ok(result));
        if out.is_ok() { saga.persist_checkpoint(SttCheckpoint { provider: "whisper".into(), state: "completed".into(), job_id: None, media_uri: None, audio_sha256: None, retry_count: 0, backoff_ms: 0, last_ts_ms: 0 }); }
        return out;
    }
    #[cfg(not(feature = "durability"))]
    { Ok(result) }
}
