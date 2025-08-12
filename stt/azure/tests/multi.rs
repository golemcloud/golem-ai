use golem_stt_azure::AzureTranscriptionComponent;
use golem_stt::golem::stt::transcription::{AudioConfig, TranscribeOptions, TranscribeRequest, Guest as TranscriptionGuest};
use golem_stt::golem::stt::types::AudioFormat;

#[test]
fn test_multi_transcribe_empty_audio() {
    let cfg = AudioConfig { format: AudioFormat::Wav, sample_rate: None, channels: None };
    let req1 = TranscribeRequest { audio: Vec::new(), config: cfg.clone(), options: None };
    let req2 = TranscribeRequest { audio: Vec::new(), config: cfg, options: None };

    let res = AzureTranscriptionComponent::multi_transcribe(vec![req1, req2]);
    assert!(res.is_err());
}