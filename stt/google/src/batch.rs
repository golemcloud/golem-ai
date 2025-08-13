use crate::recognize::recognize;
use crate::config::GoogleConfig;
use golem_stt::golem::stt::transcription::{
    AudioConfig, TranscribeOptions, TranscriptionResult,
};
use golem_stt::golem::stt::types::TranscriptionMetadata;
use golem_stt::golem::stt::types::SttError;
#[cfg(feature = "durability")]
use golem_rust::durability::Durability;
#[cfg(feature = "durability")]
use golem_rust::{FromValueAndType, IntoValue};
#[cfg(feature = "durability")]
use golem_rust::bindings::golem::durability::durability::DurableFunctionType;

fn compute_pcm_duration_seconds(bytes: usize, sample_rate: u32, channels: u8) -> f32 {
    let bytes_per_sample = 2u32;
    let samples = bytes as f32 / (bytes_per_sample as f32 * channels as f32);
    samples / sample_rate as f32
}

pub fn transcribe_impl(
    audio: Vec<u8>,
    cfg: &GoogleConfig,
    opts: Option<TranscribeOptions>,
    conf: AudioConfig,
) -> Result<TranscriptionResult, SttError> {
    let result = match recognize(&audio, cfg, &conf, &opts) {
        Ok(alternatives) => {
            let duration_seconds = if let (Some(sr), Some(ch)) = (conf.sample_rate, conf.channels) { compute_pcm_duration_seconds(audio.len(), sr, ch) } else { 0.0 };
            let metadata = TranscriptionMetadata {
                duration_seconds,
                audio_size_bytes: audio.len() as u32,
                request_id: String::new(),
                model: opts.as_ref().and_then(|o| o.model.clone()),
                language: opts
                    .as_ref()
                    .and_then(|o| o.language.clone())
                    .unwrap_or_else(|| "en-US".to_string()),
            };
            Ok(TranscriptionResult {
                alternatives,
                metadata,
            })
        }
        Err(e) => Err(e),
    };
    #[cfg(feature = "durability")]
    {
        use crate::durability::{persist_transcribe, TranscribeInputMeta};
        let input = TranscribeInputMeta { language: opts.as_ref().and_then(|o| o.language.clone()), model: opts.as_ref().and_then(|o| o.model.clone()), enable_timestamps: opts.as_ref().and_then(|o| o.enable_timestamps).unwrap_or(false), enable_diarization: opts.as_ref().and_then(|o| o.enable_speaker_diarization).unwrap_or(false), enable_word_confidence: opts.as_ref().and_then(|o| o.enable_word_confidence).unwrap_or(false), audio_size_bytes: audio.len() as u32 };
        return persist_transcribe(input, result);
    }
    #[cfg(not(feature = "durability"))]
    { result }
} 