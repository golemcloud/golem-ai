use golem_stt::golem::stt::transcription::{Guest as TranscriptionGuest, AudioConfig, TranscribeOptions};
use golem_stt::golem::stt::types::AudioFormat;

#[test]
fn diarization_integration() {
    if std::env::var("DEEPGRAM_API_KEY").is_err() { return; }
    let audio_path = match std::env::var("DEEPGRAM_TEST_AUDIO") { Ok(p) => p, Err(_) => return };
    let audio = std::fs::read(audio_path).unwrap_or_default();
    if audio.is_empty() { return; }
    let config = AudioConfig { format: AudioFormat::Wav, sample_rate: None, channels: None };
    let mut opts = TranscribeOptions { enable_timestamps: None, enable_speaker_diarization: Some(true), language: None, model: None, profanity_filter: None, speech_context: None, enable_word_confidence: None, enable_timing_detail: None };
    let out = golem_stt_deepgram::DeepgramTranscriptionComponent::transcribe(audio, config, Some(opts));
    assert!(out.is_ok());
}
