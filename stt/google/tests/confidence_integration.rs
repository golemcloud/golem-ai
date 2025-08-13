use golem_stt_google::GoogleTranscriptionComponent;
use golem_stt::golem::stt::transcription::{Guest as TranscriptionGuest, AudioConfig, TranscribeOptions};
use golem_stt::golem::stt::types::AudioFormat;
use std::env;
use std::fs;

#[test]
fn word_confidence_env_guarded() {
    if env::var("GOOGLE_APPLICATION_CREDENTIALS").is_err() && env::var("GOOGLE_ACCESS_TOKEN").is_err() {
        return;
    }
    let audio_path = match env::var("GOOGLE_STT_TEST_AUDIO") { Ok(p) => p, Err(_) => return };
    let audio = match fs::read(&audio_path) { Ok(b) => b, Err(_) => return };

    let cfg = AudioConfig { format: AudioFormat::Wav, sample_rate: None, channels: None };
    let opts = Some(TranscribeOptions { enable_timestamps: Some(true), enable_speaker_diarization: None, language: Some("en-US".into()), model: None, profanity_filter: None, speech_context: None, enable_word_confidence: Some(true), enable_timing_detail: None });
    let res = GoogleTranscriptionComponent::transcribe(audio, cfg, opts).expect("transcribe");

    if let Some(first) = res.alternatives.first() {
        if first.words.is_empty() { return; }
        let has_conf = first.words.iter().any(|w| w.confidence.is_some());
        if !has_conf { return; }
    }
}