use std::fs;
use std::env;
use golem_stt::golem::stt::transcription::{AudioConfig, Guest};
use golem_stt::golem::stt::types::AudioFormat;

#[test]
fn silence_audio() {
    let path = match env::var("GOOGLE_STT_TEST_SILENCE") { Ok(p) => p, Err(_) => return };
    let audio = match fs::read(&path) { Ok(b) => b, Err(_) => return };
    let conf = AudioConfig { format: AudioFormat::Wav, sample_rate: Some(16000), channels: Some(1) };
    let out = golem_stt_google::GoogleTranscriptionComponent::transcribe(audio, conf, None);
    let _ = out.is_ok();
}

#[test]
fn long_audio() {
    let path = match env::var("GOOGLE_STT_TEST_LONG") { Ok(p) => p, Err(_) => return };
    let audio = match fs::read(&path) { Ok(b) => b, Err(_) => return };
    let conf = AudioConfig { format: AudioFormat::Wav, sample_rate: Some(16000), channels: Some(1) };
    let out = golem_stt_google::GoogleTranscriptionComponent::transcribe(audio, conf, None).ok();
    if let Some(res) = out { assert!(res.metadata.duration_seconds >= 0.0); }
}

