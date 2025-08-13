use golem_stt::golem::stt::transcription::{Guest as TranscriptionGuest, AudioConfig};
use golem_stt::golem::stt::types::AudioFormat;

#[test]
fn metadata_integration() {
    if std::env::var("DEEPGRAM_API_KEY").is_err() { return; }
    let audio_path = match std::env::var("DEEPGRAM_TEST_AUDIO") { Ok(p) => p, Err(_) => return };
    let audio = std::fs::read(audio_path).unwrap_or_default();
    if audio.is_empty() { return; }
    let config = AudioConfig { format: AudioFormat::Wav, sample_rate: None, channels: None };
    let out = golem_stt_deepgram::DeepgramTranscriptionComponent::transcribe(audio, config, None);
    let res = match out { Ok(r) => r, Err(_) => return };
    assert!(res.metadata.audio_size_bytes > 0);
    assert!(!res.metadata.request_id.is_empty());
}
