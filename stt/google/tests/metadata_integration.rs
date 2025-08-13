use std::fs;
use std::env;
use golem_stt::golem::stt::transcription::{AudioConfig, TranscribeOptions, Guest};
use golem_stt::golem::stt::types::AudioFormat;

#[test]
fn metadata_integration() {
    let path = match env::var("GOOGLE_STT_TEST_AUDIO") { Ok(p) => p, Err(_) => return };
    let audio = match fs::read(&path) { Ok(b) => b, Err(_) => return };
    let fmt = if path.ends_with(".wav") { AudioFormat::Wav } else if path.ends_with(".mp3") { AudioFormat::Mp3 } else if path.ends_with(".flac") { AudioFormat::Flac } else { AudioFormat::Wav };
    let conf = AudioConfig { format: fmt, sample_rate: Some(16000), channels: Some(1) };
    let opts = Some(TranscribeOptions { enable_timestamps: Some(true), enable_speaker_diarization: Some(false), language: Some("en-US".into()), model: None, profanity_filter: Some(false), speech_context: None, enable_word_confidence: Some(true), enable_timing_detail: None });
    let out = golem_stt_google::GoogleTranscriptionComponent::transcribe(audio.clone(), conf, opts).unwrap();
    assert!(!out.metadata.request_id.is_empty());
    assert_eq!(out.metadata.audio_size_bytes, audio.len() as u32);
    assert!(out.metadata.duration_seconds >= 0.0);
}

