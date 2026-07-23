use golem_ai_http::{Error, ResponseBody};
use log::trace;
use std::string::FromUtf8Error;

pub struct Utf8Stream {
    body: ResponseBody,
    buffer: Vec<u8>,
    terminated: bool,
}

impl Utf8Stream {
    pub fn new(body: ResponseBody) -> Self {
        Self {
            body,
            buffer: Vec::new(),
            terminated: false,
        }
    }

    pub async fn next(&mut self) -> Option<Result<String, Utf8StreamError<Error>>> {
        if self.terminated {
            return None;
        }
        match self.body.chunk().await {
            Ok(Some(bytes)) => {
                trace!("Read {} bytes from response stream", bytes.len());
                self.buffer.extend_from_slice(&bytes);
                let bytes = core::mem::take(&mut self.buffer);
                match String::from_utf8(bytes) {
                    Ok(string) => Some(Ok(string)),
                    Err(err) => {
                        let valid_size = err.utf8_error().valid_up_to();
                        let mut bytes = err.into_bytes();
                        self.buffer = bytes.split_off(valid_size);
                        Some(Ok(unsafe { String::from_utf8_unchecked(bytes) }))
                    }
                }
            }
            Ok(None) => {
                self.terminated = true;
                if self.buffer.is_empty() {
                    None
                } else {
                    Some(
                        String::from_utf8(core::mem::take(&mut self.buffer))
                            .map_err(Utf8StreamError::Utf8),
                    )
                }
            }
            Err(err) => Some(Err(Utf8StreamError::Transport(err))),
        }
    }
}

#[derive(Debug)]
pub enum Utf8StreamError<E> {
    Utf8(FromUtf8Error),
    Transport(E),
}

impl<E> From<FromUtf8Error> for Utf8StreamError<E> {
    fn from(err: FromUtf8Error) -> Self {
        Self::Utf8(err)
    }
}
