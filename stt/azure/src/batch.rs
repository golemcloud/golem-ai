use crate::recognize::RecognizeOut;
use crate::recognize::recognize;
use crate::config::AzureConfig;
use golem_stt::golem::stt::transcription::{
    AudioConfig, TranscribeOptions, TranscriptionResult,
};
use golem_stt::golem::stt::types::TranscriptionMetadata;
use golem_stt::golem::stt::types::SttError;
#[cfg(feature = "durability")]
use golem_stt::durability::saga::{Saga, SttCheckpoint};
#[cfg(feature = "durability")]
use golem_rust::bindings::golem::durability::durability::DurableFunctionType;
#[cfg(feature = "durability")]
use golem_stt::durability::durable_impl;
#[cfg(feature = "durability")]
use golem_rust::{FromValueAndType, IntoValue};

fn compute_pcm_duration_seconds(bytes: usize, sample_rate: u32, channels: u8) -> f32 {
    let bytes_per_sample = 2u32;
    let samples = bytes as f32 / (bytes_per_sample as f32 * channels as f32);
    samples / sample_rate as f32
}

pub fn transcribe_impl(
    audio: Vec<u8>,
    cfg: &AzureConfig,
    opts: Option<TranscribeOptions>,
    conf: AudioConfig,
) -> Result<TranscriptionResult, SttError> {
    #[cfg(feature = "durability")]
    let saga: Saga<TranscriptionResult, SttError> = Saga::new("golem_stt_azure", "transcribe", DurableFunctionType::WriteRemote);
    let result = match recognize(&audio, cfg, &conf, &opts) {
        Ok(RecognizeOut { alternatives, request_id, elapsed_secs, server_duration_secs }) => {
            let model = opts.as_ref().and_then(|o| o.model.clone());
            let duration_seconds = if let Some(srv) = server_duration_secs { srv } else {
                if let (Some(sr), Some(ch)) = (conf.sample_rate, conf.channels) {
                    compute_pcm_duration_seconds(audio.len(), sr, ch)
                } else {
                    elapsed_secs
                }
            };
            let request_id_ref = request_id.as_deref();
            let request_id_string = request_id_ref.unwrap_or("").to_string();
            let metadata = TranscriptionMetadata {
                duration_seconds,
                audio_size_bytes: audio.len() as u32,
                request_id: request_id_string.clone(),
                model,
                language: opts
                    .as_ref()
                    .and_then(|o| o.language.clone())
                    .unwrap_or_else(|| "en-US".to_string()),
            };
                Ok(TranscriptionResult { alternatives, metadata })
        }
        Err(e) => {
            Err(e)
        }
    };

    #[cfg(feature = "durability")]
    {
        #[derive(Clone, Debug, FromValueAndType, IntoValue)]
        struct InputMeta {
            provider: String,
            language: Option<String>,
            model: Option<String>,
            enable_timestamps: bool,
            enable_diarization: bool,
            enable_word_confidence: bool,
            audio_size_bytes: u32,
        }
        let input = InputMeta {
            provider: "azure".to_string(),
            language: opts.as_ref().and_then(|o| o.language.clone()),
            model: opts.as_ref().and_then(|o| o.model.clone()),
            enable_timestamps: opts.as_ref().and_then(|o| o.enable_timestamps).unwrap_or(false),
            enable_diarization: opts.as_ref().and_then(|o| o.enable_speaker_diarization).unwrap_or(false),
            enable_word_confidence: opts.as_ref().and_then(|o| o.enable_word_confidence).unwrap_or(false),
            audio_size_bytes: audio.len() as u32,
        };
                    let out = durable_impl::persist_transcribe("golem_stt_azure", input, result);
                    if out.is_ok() { saga.persist_checkpoint(SttCheckpoint { provider: "azure".into(), state: "completed".into(), job_id: None, media_uri: None, audio_sha256: None, retry_count: 0, backoff_ms: 0, last_ts_ms: 0 }); }
                    return out;
    }

    #[cfg(not(feature = "durability"))]
    { result }
}

#[cfg(test)]
mod tests {
    use super::compute_pcm_duration_seconds;

    #[test]
    fn pcm_duration_helper() {
        let sr = 16000u32;
        let ch = 1u8;
        let secs = 2.0f32;
        let bytes = (sr as f32 * secs * 2.0) as usize;
        let d = compute_pcm_duration_seconds(bytes, sr, ch);
        assert!((d - secs).abs() < 0.001);
    }
}
