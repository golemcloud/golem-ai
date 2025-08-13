use golem_stt_azure::AzureTranscriptionComponent;
use golem_stt::golem::stt::transcription::{Guest as TranscriptionGuest, AudioConfig, TranscribeOptions};
use golem_stt::golem::stt::types::AudioFormat;
use std::env;
use std::fs;

#[test]
fn transcribe_metadata_env_guarded() {
    if env::var("AZURE_SPEECH_KEY").is_err() || env::var("AZURE_SPEECH_REGION").is_err() {
        return;
    }
    let audio_path = match env::var("AZURE_STT_TEST_AUDIO") {
        Ok(p) => p,
        Err(_) => return,
    };

    let audio = match fs::read(&audio_path) { Ok(b) => b, Err(_) => return };

    let cfg = AudioConfig { format: AudioFormat::Wav, sample_rate: None, channels: None };
    let opts = Some(TranscribeOptions { language: Some("en-US".into()), enable_timestamps: Some(false), enable_speaker_diarization: None, model: None, profanity_filter: None, speech_context: None, enable_word_confidence: None, enable_timing_detail: None });

    let res = AzureTranscriptionComponent::transcribe(audio, cfg, opts).expect("transcribe");
    assert!(!res.metadata.request_id.is_empty());
    assert!(res.metadata.audio_size_bytes > 0);
    assert!(res.metadata.duration_seconds >= 0.0);
}