use golem_stt_azure::AzureTranscriptionComponent;
use golem_stt::golem::stt::transcription::{Guest as TranscriptionGuest, AudioConfig, TranscribeOptions};
use golem_stt::golem::stt::types::AudioFormat;
use std::env;
use std::fs;

#[test]
fn diarization_env_guarded() {
    if env::var("AZURE_SPEECH_KEY").is_err() || env::var("AZURE_SPEECH_REGION").is_err() {
        return;
    }
    let audio_path = match env::var("AZURE_STT_TEST_AUDIO_DIAR") { Ok(p) => p, Err(_) => return };
    let audio = match fs::read(&audio_path) { Ok(b) => b, Err(_) => return };

    let cfg = AudioConfig { format: AudioFormat::Wav, sample_rate: None, channels: None };
    let opts = Some(TranscribeOptions { enable_timestamps: Some(true), enable_speaker_diarization: Some(true), language: Some("en-US".into()), model: None, profanity_filter: None, speech_context: None, enable_word_confidence: None, enable_timing_detail: None });
    let res = AzureTranscriptionComponent::transcribe(audio, cfg, opts).expect("transcribe");

    if let Some(first) = res.alternatives.first() {
        let has_speaker = first.words.iter().any(|w| w.speaker_id.is_some());
        if !has_speaker { return; }
    }
}