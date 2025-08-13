use golem_stt::golem::stt::transcription::{Guest as TranscriptionGuest, AudioConfig};
use golem_stt::golem::stt::types::AudioFormat;

#[test]
fn wav_basic() {
    if std::env::var("OPENAI_API_KEY").is_err() { return; }
    let audio_path = match std::env::var("WHISPER_TEST_AUDIO") { Ok(p) => p, Err(_) => return };
    let audio = std::fs::read(audio_path).unwrap_or_default();
    if audio.is_empty() { return; }
    let config = AudioConfig { format: AudioFormat::Wav, sample_rate: None, channels: None };
    let out = golem_stt_whisper::WhisperTranscriptionComponent::transcribe(audio, config, None);
    assert!(out.is_ok());
}
