#![cfg(all(feature = "durability", feature = "durability_integration"))]

use golem_stt::golem::stt::transcription::{GuestTranscriptionStream, AudioConfig};
use golem_stt::golem::stt::types::AudioFormat;
use golem_stt_google::config::GoogleConfig;
use golem_stt_google::{GoogleStream};

#[test]
fn durable_stream_replay_flow() {
    std::env::set_var("GOOGLE_APPLICATION_CREDENTIALS", include_str!("data/fake_creds.json"));
    std::env::remove_var("STT_PROVIDER_ENDPOINT");

    let cfg = GoogleConfig::load().unwrap();
    let conf = AudioConfig { format: AudioFormat::Wav, sample_rate: Some(16000), channels: Some(1) };

    let live = GoogleStream::new(cfg, conf.clone(), None);
    let durable = golem_stt_google::durability::DurableTranscriptionStream::live(live);

    durable.send_audio(vec![1,2,3]).unwrap();
    durable.send_audio(vec![4,5]).unwrap();
    durable.finish().unwrap();

    let replay = golem_stt_google::durability::DurableTranscriptionStream::replay();
    let _ = replay.receive_alternative().unwrap();
}

