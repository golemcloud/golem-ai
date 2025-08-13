use golem_stt_azure::AzureTranscriptionComponent;
use golem_stt::golem::stt::transcription::{Guest as TranscriptionGuest, AudioConfig};
use golem_stt::golem::stt::types::AudioFormat;
use std::env;
use std::fs;

#[test]
fn formats_env_guarded() {
    if env::var("AZURE_SPEECH_KEY").is_err() || env::var("AZURE_SPEECH_REGION").is_err() {
        return;
    }
    let mut ran = false;
    if let Ok(p) = env::var("AZURE_STT_TEST_AUDIO_MP3") {
        if let Ok(audio) = fs::read(&p) {
            let cfg = AudioConfig { format: AudioFormat::Mp3, sample_rate: None, channels: None };
            let res = AzureTranscriptionComponent::transcribe(audio, cfg, None);
            assert!(res.is_ok());
            ran = true;
        }
    }
    if let Ok(p) = env::var("AZURE_STT_TEST_AUDIO_FLAC") {
        if let Ok(audio) = fs::read(&p) {
            let cfg = AudioConfig { format: AudioFormat::Flac, sample_rate: None, channels: None };
            let res = AzureTranscriptionComponent::transcribe(audio, cfg, None);
            assert!(res.is_ok());
            ran = true;
        }
    }
    if !ran { return; }
}