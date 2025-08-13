use golem_stt::golem::stt::transcription::{Guest as TranscriptionGuest, AudioConfig, TranscribeOptions};
use golem_stt::golem::stt::types::AudioFormat;

#[test]
fn basic_transcription_skipped_without_key() {
    if std::env::var("DEEPGRAM_API_KEY").is_err() { return; }
    let audio = vec![];
    let config = AudioConfig { format: AudioFormat::Wav, sample_rate: None, channels: None };
    let options: Option<TranscribeOptions> = None;
    let _ = golem_stt_deepgram::DeepgramTranscriptionComponent::transcribe(audio, config, options);
}
