#![cfg(feature = "durability")]

use crate::stream::GoogleStream;
use golem_stt::golem::stt::transcription::{GuestTranscriptionStream, TranscriptAlternative, AudioConfig, TranscribeOptions};
use golem_stt::golem::stt::types::SttError;
use golem_rust::bindings::golem::durability::durability::DurableFunctionType;
use golem_rust::durability::Durability;
use golem_rust::{FromValueAndType, IntoValue};
use std::cell::{Cell, RefCell};
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, FromValueAndType, IntoValue)]
struct NoInput;

#[derive(Debug, FromValueAndType, IntoValue)]
struct UnusedError;

impl Display for UnusedError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { write!(f, "unused") }
}

#[derive(Debug, Clone, FromValueAndType, IntoValue)]
struct NoOutput;

enum DurableState<I: GuestTranscriptionStream> {
    Live { inner: I },
    Replay { buffer: RefCell<Vec<u8>>, finished: Cell<bool> },
}

pub struct DurableTranscriptionStream<I: GuestTranscriptionStream> {
    state: RefCell<DurableState<I>>,
}

impl DurableTranscriptionStream<GoogleStream> {
    pub fn live(inner: GoogleStream) -> Self {
        Self {
            state: RefCell::new(DurableState::Live { inner }),
        }
    }

    pub fn replay() -> Self {
        Self {
            state: RefCell::new(DurableState::Replay { buffer: RefCell::new(Vec::new()), finished: Cell::new(false) }),
        }
    }

    pub fn new_wrapped_stream(cfg: crate::config::GoogleConfig, conf: AudioConfig, opts: Option<TranscribeOptions>) -> Self {
        let durability = Durability::<NoOutput, UnusedError>::new("golem_stt", "stream_start", DurableFunctionType::WriteRemote);
        if durability.is_live() {
            let inner = GoogleStream::new(cfg, conf, opts);
            let s = Self::live(inner);
            let _ = durability.persist_infallible(NoInput, NoOutput);
            s
        } else {
            let _: NoOutput = durability.replay_infallible();
            Self::replay()
        }
    }
}

impl GuestTranscriptionStream for DurableTranscriptionStream<GoogleStream> {
    fn send_audio(&self, mut chunk: Vec<u8>) -> Result<(), SttError> {
        let durability = Durability::<Vec<u8>, UnusedError>::new("golem_stt", "send_audio", DurableFunctionType::WriteRemote);
        match &mut *self.state.borrow_mut() {
            DurableState::Live { inner } => {
                inner.send_audio(chunk.clone())?;
                let _ = durability.persist_infallible(NoInput, chunk);
                Ok(())
            }
            DurableState::Replay { buffer, .. } => {
                let recorded: Vec<u8> = durability.replay_infallible();
                buffer.borrow_mut().extend_from_slice(&recorded);
                Ok(())
            }
        }
    }

    fn finish(&self) -> Result<(), SttError> {
        let durability = Durability::<bool, UnusedError>::new("golem_stt", "finish", DurableFunctionType::WriteRemote);
        match &mut *self.state.borrow_mut() {
            DurableState::Live { inner } => {
                inner.finish()?;
                let _ = durability.persist_infallible(NoInput, true);
                Ok(())
            }
            DurableState::Replay { finished, .. } => {
                let _: bool = durability.replay_infallible();
                finished.set(true);
                Ok(())
            }
        }
    }

    fn receive_alternative(&self) -> Result<Option<TranscriptAlternative>, SttError> {
        let durability = Durability::<Option<TranscriptAlternative>, UnusedError>::new("golem_stt", "receive_alternative", DurableFunctionType::ReadRemote);
        match &mut *self.state.borrow_mut() {
            DurableState::Live { inner } => {
                let out = inner.receive_alternative()?;
                let _ = durability.persist_infallible(NoInput, out.clone());
                Ok(out)
            }
            DurableState::Replay { .. } => {
                let out: Option<TranscriptAlternative> = durability.replay_infallible();
                Ok(out)
            }
        }
    }

    fn close(&self) {
        match &mut *self.state.borrow_mut() {
            DurableState::Live { inner } => inner.close(),
            DurableState::Replay { buffer, .. } => buffer.borrow_mut().clear(),
        }
    }
}


