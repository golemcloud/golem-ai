use golem_stt_azure::AzureTranscriptionComponent;
use golem_stt::golem::stt::transcription::{Guest as TranscriptionGuest, AudioConfig};
use golem_stt::golem::stt::types::{AudioFormat, SttError};
use std::env;

#[test]
fn rate_quota_scaffold_env_guarded() {
    if env::var("AZURE_SPEECH_KEY").is_err() || env::var("AZURE_SPEECH_REGION").is_err() {
        return;
    }
    if env::var("AZURE_STT_SIMULATE_RATE").is_err() { return; }

    let cfg = AudioConfig { format: AudioFormat::Wav, sample_rate: None, channels: None };
    let res = AzureTranscriptionComponent::transcribe(Vec::new(), cfg, None);
    if let Err(e) = res {
        match e {
            SttError::RateLimited(_) | SttError::QuotaExceeded(_) => {}
            _ => {}
        }
    }
}