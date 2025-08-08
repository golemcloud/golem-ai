#![cfg(all(feature = "gateway", feature = "gateway_integration"))]

use golem_stt::golem::stt::transcription::{GuestTranscriptionStream, AudioConfig};
use golem_stt::golem::stt::types::AudioFormat;
use std::net::TcpListener;
use std::thread;

#[test]
fn gateway_minimal_flow() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let _ = stream.unwrap();
            break;
        }
    });
    std::env::set_var("STT_STREAM_GATEWAY_URL", format!("ws://{}", addr));
    std::env::set_var("GOOGLE_APPLICATION_CREDENTIALS", include_str!("data/fake_creds.json"));

    let cfg = golem_stt_google::config::GoogleConfig::load().unwrap();
    let conf = AudioConfig { format: AudioFormat::Wav, sample_rate: Some(16000), channels: Some(1) };
    let s = golem_stt_google::GoogleStream::new(cfg, conf, None);
    let _ = s.send_audio(vec![1,2,3]);
    let _ = s.finish();
    let _ = s.receive_alternative();
}

