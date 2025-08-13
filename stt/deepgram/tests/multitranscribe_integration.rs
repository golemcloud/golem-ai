use golem_stt::golem::stt::transcription::{Guest as TranscriptionGuest, AudioConfig, TranscribeRequest};
use golem_stt::golem::stt::types::AudioFormat;

#[test]
fn multi_transcribe_concurrent() {
    if std::env::var("DEEPGRAM_API_KEY").is_err() { return; }
    let audio_path = match std::env::var("DEEPGRAM_TEST_AUDIO") { Ok(p) => p, Err(_) => return };
    let audio = std::fs::read(audio_path).unwrap_or_default();
    if audio.is_empty() { return; }
    let cfg = AudioConfig { format: AudioFormat::Wav, sample_rate: None, channels: None };
    let reqs = vec![
        TranscribeRequest { audio: audio.clone(), config: cfg.clone(), options: None },
        TranscribeRequest { audio: audio.clone(), config: cfg.clone(), options: None },
    ];
    let out = golem_stt_deepgram::DeepgramTranscriptionComponent::multi_transcribe(reqs);
    assert!(out.is_ok());
}
