use golem_stt_google::GoogleStream;
use golem_stt_google::config::GoogleConfig;
use golem_stt::golem::stt::transcription::AudioConfig;
use golem_stt::golem::stt::transcription::GuestTranscriptionStream;

#[test]
fn stream_basic() {
    static CREDS: &str = include_str!("data/fake_creds.json");
    std::env::set_var("GOOGLE_APPLICATION_CREDENTIALS", CREDS);
    let cfg = GoogleConfig::load().unwrap();
    let conf: AudioConfig = unsafe { std::mem::MaybeUninit::zeroed().assume_init() };
    let stream = GoogleStream::new(cfg, conf, None);
    stream.send_audio(vec![1, 2]).unwrap();
    stream.finish().unwrap();
    assert!(stream.receive_alternative().is_err());
} 