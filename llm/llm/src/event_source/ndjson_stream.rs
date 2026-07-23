use super::stream::{LlmStream, StreamError as NdJsonStreamError};
use crate::event_source::utf8_stream::Utf8Stream;
use crate::event_source::MessageEvent;
use log::trace;

#[derive(Debug, Clone, Copy)]
pub enum NdJsonStreamState {
    NotStarted,
    Started,
    Terminated,
}

impl NdJsonStreamState {
    fn is_terminated(self) -> bool {
        matches!(self, Self::Terminated)
    }
}

/// A Stream of NDJSON events (newline-delimited JSON)
pub struct NdJsonStream {
    stream: Utf8Stream,
    buffer: String,
    state: NdJsonStreamState,
    last_event_id: String,
}

impl LlmStream for NdJsonStream {
    /// Initialize the NdJsonStream with a Stream
    fn new(body: golem_ai_http::ResponseBody) -> Self {
        Self {
            stream: Utf8Stream::new(body),
            buffer: String::new(),
            state: NdJsonStreamState::NotStarted,
            last_event_id: String::new(),
        }
    }

    /// Set the last event ID of the stream
    fn set_last_event_id(&mut self, id: impl Into<String>) {
        self.last_event_id = id.into();
    }

    /// Get the last event ID of the stream
    fn last_event_id(&self) -> &str {
        &self.last_event_id
    }

    async fn next(
        &mut self,
    ) -> Option<Result<MessageEvent, NdJsonStreamError<golem_ai_http::Error>>> {
        trace!("Polling for next NDJSON event");

        // Try to parse a complete line from the current buffer
        match try_parse_line(self) {
            Ok(Some(event)) => return Some(Ok(event)),
            Ok(None) => {}
            Err(error) => return Some(Err(error)),
        }

        if self.state.is_terminated() {
            return None;
        }

        loop {
            match self.stream.next().await {
                Some(Ok(string)) => {
                    if string.is_empty() {
                        continue;
                    }

                    if !self.state.is_terminated() {
                        self.state = NdJsonStreamState::Started;
                    }

                    self.buffer.push_str(&string);

                    // Try to parse complete lines from the updated buffer
                    match try_parse_line(self) {
                        Ok(Some(event)) => return Some(Ok(event)),
                        Ok(None) => {}
                        Err(error) => return Some(Err(error)),
                    }
                }
                Some(Err(err)) => return Some(Err(err.into())),
                None => {
                    self.state = NdJsonStreamState::Terminated;

                    // Process any remaining content in buffer before terminating
                    if !self.buffer.trim().is_empty() {
                        let remaining = std::mem::take(&mut self.buffer);
                        let event = MessageEvent {
                            event: "message".to_string(),
                            data: remaining.trim().to_string(),
                            id: self.last_event_id.clone(),
                            retry: None,
                        };
                        return Some(Ok(event));
                    }

                    return None;
                }
            }
        }
    }
}

/// Try to parse a complete line from the buffer
/// Returns Ok(Some(event)) if a complete line was found and parsed
/// Returns Ok(None) if no complete line is available
/// Returns Err if there was a parsing error
fn try_parse_line(
    stream: &mut NdJsonStream,
) -> Result<Option<MessageEvent>, NdJsonStreamError<golem_ai_http::Error>> {
    // Consume empty lines without waiting for another body chunk.
    while let Some(newline_pos) = stream.buffer.find('\n') {
        // Extract the line (without the newline)
        let line = stream.buffer[..newline_pos].trim().to_string();

        // Remove the processed line from the buffer (including the newline)
        stream.buffer.drain(..=newline_pos);

        // Skip empty lines
        if line.is_empty() {
            continue;
        }

        trace!("Parsed NDJSON line: {line}");

        // Create a MessageEvent with the JSON line as data
        let event = MessageEvent {
            event: "message".to_string(),
            data: line,
            id: stream.last_event_id.clone(),
            retry: None,
        };

        return Ok(Some(event));
    }

    Ok(None)
}
