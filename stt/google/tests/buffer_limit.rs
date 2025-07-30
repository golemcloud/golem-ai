use golem_stt_google::GoogleStream;
use golem_stt_google::config::GoogleConfig;
use golem_stt::golem::stt::transcription::{AudioConfig, GuestTranscriptionStream};
use golem_stt::golem::stt::types::SttError;

#[test]
fn buffer_limit_exceeded() {
    static CREDS: &str = include_str!("data/fake_creds.json");
    std::env::set_var("GOOGLE_APPLICATION_CREDENTIALS", CREDS);
    std::env::set_var("STT_BUFFER_LIMIT_BYTES", "4");

    let cfg = GoogleConfig::load().unwrap();
    let conf: AudioConfig = unsafe { std::mem::MaybeUninit::zeroed().assume_init() };
    let stream = GoogleStream::new(cfg, conf, None);

    let err = stream.send_audio(vec![0, 1, 2, 3, 4]).unwrap_err();
    matches!(err, SttError::InvalidAudio(_));
} 