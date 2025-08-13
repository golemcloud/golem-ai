use golem_stt_google::GoogleTranscriptionComponent;
use golem_stt::golem::stt::transcription::{Guest as TranscriptionGuest, AudioConfig};
use golem_stt::golem::stt::types::AudioFormat;
use std::env;
use std::fs;

#[test]
fn formats_env_guarded() {
    if env::var("GOOGLE_APPLICATION_CREDENTIALS").is_err() && env::var("GOOGLE_ACCESS_TOKEN").is_err() {
        return;
    }
    let mut ran = false;
    if let Ok(p) = env::var("GOOGLE_STT_TEST_AUDIO_MP3") {
        if let Ok(audio) = fs::read(&p) {
            let cfg = AudioConfig { format: AudioFormat::Mp3, sample_rate: None, channels: None };
            let res = GoogleTranscriptionComponent::transcribe(audio, cfg, None);
            assert!(res.is_ok());
            ran = true;
        }
    }
    if let Ok(p) = env::var("GOOGLE_STT_TEST_AUDIO_FLAC") {
        if let Ok(audio) = fs::read(&p) {
            let cfg = AudioConfig { format: AudioFormat::Flac, sample_rate: None, channels: None };
            let res = GoogleTranscriptionComponent::transcribe(audio, cfg, None);
            assert!(res.is_ok());
            ran = true;
        }
    }
    if !ran { return; }
}