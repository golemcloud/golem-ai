use crate::config::GoogleConfig;
use crate::recognize::recognize;
use golem_stt::golem::stt::transcription::{AudioConfig, TranscribeOptions, TranscriptAlternative, GuestTranscriptionStream};
use golem_stt::golem::stt::types::SttError;
use std::cell::{Cell, RefCell};
#[cfg(feature = "gateway")]
use tungstenite::WebSocket;
#[cfg(feature = "gateway")]
use tungstenite::stream::MaybeTlsStream;
#[cfg(feature = "gateway")]
use url::Url;

#[cfg(feature = "gateway")]
fn gateway_url() -> Option<String> { std::env::var("STT_STREAM_GATEWAY_URL").ok() }
#[cfg(feature = "gateway")]
fn ws_send_binary(ws: &mut WebSocket<MaybeTlsStream<std::net::TcpStream>>, data: &[u8]) -> Result<(), SttError> {
    ws.write_message(tungstenite::Message::Binary(data.to_vec())).map_err(|e| SttError::NetworkError(format!("{e}")))
}
#[cfg(feature = "gateway")]
fn ws_close_write(ws: &mut WebSocket<MaybeTlsStream<std::net::TcpStream>>) -> Result<(), SttError> {
    ws.close(None).map_err(|e| SttError::NetworkError(format!("{e}")))
}
#[cfg(feature = "gateway")]
fn ws_connect(url: &str) -> Result<WebSocket<MaybeTlsStream<std::net::TcpStream>>, SttError> {
    let (ws, _) = tungstenite::connect(Url::parse(url).map_err(|e| SttError::InternalError(format!("{e}")))?)
        .map_err(|e| SttError::NetworkError(format!("{e}")))?;
    Ok(ws)
}
pub struct GoogleStream {
    buf: RefCell<Vec<u8>>,
    cfg: GoogleConfig,
    conf: AudioConfig,
    opts: Option<TranscribeOptions>,
    results: RefCell<Vec<TranscriptAlternative>>,
    finished: Cell<bool>,
    #[cfg(feature = "gateway")]
    ws: RefCell<Option<WebSocket<MaybeTlsStream<std::net::TcpStream>>>>,
}

impl GoogleStream {
    pub fn new(cfg: GoogleConfig, conf: AudioConfig, opts: Option<TranscribeOptions>) -> Self {
        #[cfg(feature = "gateway")]
        let ws = gateway_url().and_then(|u| ws_connect(&u).ok());
        Self {
            buf: RefCell::new(Vec::new()),
            cfg,
            conf,
            opts,
            results: RefCell::new(Vec::new()),
            finished: Cell::new(false),
            #[cfg(feature = "gateway")]
            ws: RefCell::new(ws),
        }
    }
}

impl GuestTranscriptionStream for GoogleStream {
    fn send_audio(&self, chunk: Vec<u8>) -> Result<(), SttError> {
        if self.finished.get() {
            return Err(SttError::UnsupportedOperation("stream already finished".into()));
        }
        #[cfg(feature = "gateway")]
        if let Some(ws) = self.ws.borrow_mut().as_mut() { return ws_send_binary(ws, &chunk); }
        if self.buf.borrow().len() + chunk.len() > self.cfg.max_buffer_bytes {
            return Err(SttError::InvalidAudio("buffer limit exceeded".into()));
        }
        self.buf.borrow_mut().extend_from_slice(&chunk);
        Ok(())
    }

    fn finish(&self) -> Result<(), SttError> {
        self.finished.set(true);
        #[cfg(feature = "gateway")]
        if let Some(ws) = self.ws.borrow_mut().as_mut() { return ws_close_write(ws); }
        Ok(())
    }

    fn receive_alternative(&self) -> Result<Option<TranscriptAlternative>, SttError> {
        #[cfg(feature = "gateway")]
        if let Some(ws) = self.ws.borrow_mut().as_mut() {
            loop {
                match ws.read_message() {
                    Ok(tungstenite::Message::Text(txt)) => {
                        let v: serde_json::Value = serde_json::from_str(&txt).map_err(|e| SttError::InternalError(format!("{e}")))?;
                        if v.get("event").and_then(|e| e.as_str()) == Some("alt") {
                            let text = v.get("text").and_then(|x| x.as_str()).unwrap_or("").to_string();
                            let confidence = v.get("confidence").and_then(|x| x.as_f64()).unwrap_or(0.0) as f32;
                            return Ok(Some(TranscriptAlternative { text, confidence, words: Vec::new() }));
                        }
                        if v.get("event").and_then(|e| e.as_str()) == Some("eof") { return Ok(None); }
                        continue;
                    }
                    Ok(tungstenite::Message::Binary(_)) => { continue; }
                    Ok(tungstenite::Message::Ping(_)) => { let _ = ws.write_message(tungstenite::Message::Pong(vec![])); continue; }
                    Ok(tungstenite::Message::Close(_)) => { return Ok(None); }
                    Err(e) => { return Err(SttError::NetworkError(format!("{e}"))); }
                }
            }
        }
        if self.results.borrow().is_empty() && self.finished.get() {
            let recognized = recognize(&self.buf.borrow(), &self.cfg, &self.conf, &self.opts)?;
            self.results.borrow_mut().extend(recognized);
        }
        let mut res = self.results.borrow_mut();
        if res.is_empty() {
            Ok(None)
        } else {
            Ok(Some(res.remove(0)))
        }
    }

    fn close(&self) {
        self.buf.borrow_mut().clear();
        self.results.borrow_mut().clear();
        #[cfg(feature = "gateway")]
        if let Some(mut ws) = self.ws.borrow_mut().take() { let _ = ws.close(None); }
    }
} 