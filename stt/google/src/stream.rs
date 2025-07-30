use crate::config::GoogleConfig;
use crate::recognize::recognize;
use golem_stt::golem::stt::transcription::{AudioConfig, TranscribeOptions, TranscriptAlternative, GuestTranscriptionStream};
use golem_stt::golem::stt::types::SttError;
use std::cell::{Cell, RefCell};

pub struct GoogleStream {
    buf: RefCell<Vec<u8>>,
    cfg: GoogleConfig,
    conf: AudioConfig,
    opts: Option<TranscribeOptions>,
    results: RefCell<Vec<TranscriptAlternative>>,
    finished: Cell<bool>,
}

impl GoogleStream {
    pub fn new(cfg: GoogleConfig, conf: AudioConfig, opts: Option<TranscribeOptions>) -> Self {
        Self {
            buf: RefCell::new(Vec::new()),
            cfg,
            conf,
            opts,
            results: RefCell::new(Vec::new()),
            finished: Cell::new(false),
        }
    }
}

impl GuestTranscriptionStream for GoogleStream {
    fn send_audio(&self, chunk: Vec<u8>) -> Result<(), SttError> {
        if self.finished.get() {
            return Err(SttError::UnsupportedOperation("stream already finished".into()));
        }
        if self.buf.borrow().len() + chunk.len() > self.cfg.max_buffer_bytes {
            return Err(SttError::InvalidAudio("buffer limit exceeded".into()));
        }
        self.buf.borrow_mut().extend_from_slice(&chunk);
        Ok(())
    }

    fn finish(&self) -> Result<(), SttError> {
        self.finished.set(true);
        Ok(())
    }

    fn receive_alternative(&self) -> Result<Option<TranscriptAlternative>, SttError> {
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
    }
} 