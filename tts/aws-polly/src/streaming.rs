//! Streaming synthesis implementation for AWS Polly

use crate::client;
use crate::types;
use crate::voices::VoiceImpl;
use crate::wit_streaming;
use crate::wit_synthesis;
use crate::wit_types;
use std::cell::RefCell;

struct SynthesisStreamState {
    voice_id: String,
    engine: String,
    format: String,
    status: wit_streaming::StreamStatus,
    pending_chunks: Vec<wit_types::AudioChunk>,
    text_buffer: String,
}

pub struct SynthesisStreamImpl {
    state: RefCell<SynthesisStreamState>,
}

impl SynthesisStreamImpl {
    pub fn new(voice_id: String, engine: String, format: String) -> Self {
        return SynthesisStreamImpl {
            state: RefCell::new(SynthesisStreamState {
                voice_id,
                engine,
                format,
                status: wit_streaming::StreamStatus::Ready,
                pending_chunks: Vec::new(),
                text_buffer: String::new(),
            }),
        };
    }
}

impl wit_streaming::GuestSynthesisStream for SynthesisStreamImpl {
    fn send_text(&self, input: wit_types::TextInput) -> Result<(), wit_types::TtsError> {
        let mut state = self.state.borrow_mut();
        state.text_buffer.push_str(&input.content);
        return Ok(());
    }

    fn finish(&self) -> Result<(), wit_types::TtsError> {
        let mut state = self.state.borrow_mut();
        let audio_data = client::synthesize_api(
            &state.voice_id,
            &state.text_buffer,
            &state.engine,
            &state.format,
        )?;

        let chunk = wit_types::AudioChunk {
            data: audio_data,
            sequence_number: 0,
            is_final: true,
            timing_info: None,
        };

        state.pending_chunks.push(chunk);
        state.status = wit_streaming::StreamStatus::Finished;
        return Ok(());
    }

    fn receive_chunk(&self) -> Result<Option<wit_types::AudioChunk>, wit_types::TtsError> {
        let mut state = self.state.borrow_mut();
        if state.pending_chunks.is_empty() {
            return Ok(None);
        }
        return Ok(Some(state.pending_chunks.remove(0)));
    }

    fn has_pending_audio(&self) -> bool {
        return !self.state.borrow().pending_chunks.is_empty();
    }

    fn get_status(&self) -> wit_streaming::StreamStatus {
        return self.state.borrow().status.clone();
    }

    fn close(&self) {
        let mut state = self.state.borrow_mut();
        state.status = wit_streaming::StreamStatus::Closed;
        state.pending_chunks.clear();
    }
}

pub struct VoiceConversionStreamImpl;
impl wit_streaming::GuestVoiceConversionStream for VoiceConversionStreamImpl {
    fn send_audio(&self, _audio: Vec<u8>) -> Result<(), wit_types::TtsError> {
        return Err(types::internal_error(
            "Polly does not support voice conversion streaming",
        ));
    }
    fn receive_converted(&self) -> Result<Option<wit_types::AudioChunk>, wit_types::TtsError> {
        return Ok(None);
    }
    fn finish(&self) -> Result<(), wit_types::TtsError> {
        Ok(())
    }
    fn close(&self) {}
}

pub fn create_stream(
    voice: &VoiceImpl,
    options: Option<wit_synthesis::SynthesisOptions>,
) -> Result<wit_streaming::SynthesisStream, wit_types::TtsError> {
    let engine = std::env::var("POLLY_ENGINE").unwrap_or_else(|_| voice.best_engine().to_string());
    let format = options
        .as_ref()
        .and_then(|o| o.audio_config.as_ref())
        .map(|c| types::map_audio_format(c.format.clone()))
        .unwrap_or("mp3");

    let stream = SynthesisStreamImpl::new(voice.voice_id().to_string(), engine, format.to_string());
    return Ok(wit_streaming::SynthesisStream::new(stream));
}

pub fn create_voice_conversion_stream(
    _voice: &VoiceImpl,
    _options: Option<wit_synthesis::SynthesisOptions>,
) -> Result<wit_streaming::VoiceConversionStream, wit_types::TtsError> {
    return Err(types::internal_error(
        "Polly does not support voice conversion",
    ));
}
