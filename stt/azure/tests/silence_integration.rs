use golem_stt_azure::AzureTranscriptionComponent;
use golem_stt::golem::stt::transcription::{Guest as TranscriptionGuest, AudioConfig};
use golem_stt::golem::stt::types::AudioFormat;
use std::env;
use std::fs;

#[test]
fn silence_env_guarded() {
    if env::var("AZURE_SPEECH_KEY").is_err() || env::var("AZURE_SPEECH_REGION").is_err() {
        return;
    }
    let audio_path = match env::var("AZURE_STT_TEST_AUDIO_SILENCE") { Ok(p) => p, Err(_) => return };
    let audio = match fs::read(&audio_path) { Ok(b) => b, Err(_) => return };

    let cfg = AudioConfig { format: AudioFormat::Wav, sample_rate: None, channels: None };
    let res = AzureTranscriptionComponent::transcribe(audio, cfg, None);
    let _ = res.is_ok();
}