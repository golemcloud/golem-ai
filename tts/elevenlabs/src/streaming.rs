//! Streaming synthesis implementation for ElevenLabs
//!
//! Implements real-time streaming TTS using ElevenLabs streaming API.

use crate::client;
use crate::types;
use crate::voices::VoiceImpl;
use crate::wit_streaming;
use crate::wit_synthesis;
use crate::wit_types;
use std::cell::RefCell;

// ============================================================
// SYNTHESIS STREAM IMPLEMENTATION
// ============================================================

struct SynthesisStreamState {
    voice_id: String,
    model_id: String,
    output_format: String,
    status: wit_streaming::StreamStatus,
    pending_chunks: Vec<wit_types::AudioChunk>,
    sequence_counter: u32,
    text_buffer: String,
}

/// Streaming synthesis session with interior mutability
pub struct SynthesisStreamImpl {
    state: RefCell<SynthesisStreamState>,
}

impl SynthesisStreamImpl {
    /// Create new synthesis stream
    pub fn new(voice_id: String, model_id: String, output_format: String) -> Self {
        return SynthesisStreamImpl {
            state: RefCell::new(SynthesisStreamState {
                voice_id,
                model_id,
                output_format,
                status: wit_streaming::StreamStatus::Ready,
                pending_chunks: Vec::new(),
                sequence_counter: 0,
                text_buffer: String::new(),
            }),
        };
    }

    /// Process buffered text and generate audio chunks
    fn process_buffer(&self) -> Result<(), wit_types::TtsError> {
        let mut state = self.state.borrow_mut();
        if state.text_buffer.is_empty() {
            return Ok(());
        }

        state.status = wit_streaming::StreamStatus::Processing;

        // Synthesize the buffered text
        let audio_data = client::synthesize_api(
            &state.voice_id,
            &state.text_buffer,
            &state.model_id,
            &state.output_format,
        )?;

        // Split into chunks for streaming semantics
        let chunk_size: usize = 8192; // 8KB chunks
        let total_chunks = (audio_data.len() + chunk_size - 1) / chunk_size;

        let mut offset: usize = 0;
        let mut chunk_index: usize = 0;

        while offset < audio_data.len() {
            let end = core::cmp::min(offset + chunk_size, audio_data.len());
            let chunk_data = audio_data[offset..end].to_vec();

            chunk_index = chunk_index + 1;
            let is_final = chunk_index >= total_chunks;

            let chunk = wit_types::AudioChunk {
                data: chunk_data,
                sequence_number: state.sequence_counter,
                is_final,
                timing_info: None,
            };

            state.sequence_counter = state.sequence_counter + 1;
            state.pending_chunks.push(chunk);
            offset = end;
        }

        // Clear the buffer
        state.text_buffer.clear();

        return Ok(());
    }
}

impl wit_streaming::GuestSynthesisStream for SynthesisStreamImpl {
    fn send_text(&self, input: wit_types::TextInput) -> Result<(), wit_types::TtsError> {
        {
            let state = self.state.borrow();
            if state.status == wit_streaming::StreamStatus::Closed {
                return Err(types::internal_error("Stream is closed"));
            }
            if state.status == wit_streaming::StreamStatus::Error {
                return Err(types::internal_error("Stream is in error state"));
            }
        }

        // Append to buffer
        {
            let mut state = self.state.borrow_mut();
            state.text_buffer.push_str(&input.content);
        }

        // For ElevenLabs, we'll process in larger chunks
        // Process if buffer is large enough
        let should_process = {
            let state = self.state.borrow();
            state.text_buffer.len() >= 500
        };

        if should_process {
            self.process_buffer()?;
        }

        return Ok(());
    }

    fn finish(&self) -> Result<(), wit_types::TtsError> {
        // Process any remaining text
        self.process_buffer()?;

        let mut state = self.state.borrow_mut();
        state.status = wit_streaming::StreamStatus::Finished;
        return Ok(());
    }

    fn receive_chunk(&self) -> Result<Option<wit_types::AudioChunk>, wit_types::TtsError> {
        let mut state = self.state.borrow_mut();
        if state.pending_chunks.is_empty() {
            return Ok(None);
        }

        let chunk = state.pending_chunks.remove(0);
        return Ok(Some(chunk));
    }

    fn has_pending_audio(&self) -> bool {
        let state = self.state.borrow();
        return !state.pending_chunks.is_empty();
    }

    fn get_status(&self) -> wit_streaming::StreamStatus {
        let state = self.state.borrow();
        return state.status.clone();
    }

    fn close(&self) {
        let mut state = self.state.borrow_mut();
        state.status = wit_streaming::StreamStatus::Closed;
        state.pending_chunks.clear();
        state.text_buffer.clear();
    }
}

// ============================================================
// VOICE CONVERSION STREAM IMPLEMENTATION
// ============================================================

struct VoiceConversionStreamState {
    target_voice_id: String,
    status: wit_streaming::StreamStatus,
    pending_chunks: Vec<wit_types::AudioChunk>,
    sequence_counter: u32,
}

/// Voice conversion streaming session with interior mutability
pub struct VoiceConversionStreamImpl {
    state: RefCell<VoiceConversionStreamState>,
}

impl VoiceConversionStreamImpl {
    pub fn new(target_voice_id: String) -> Self {
        return VoiceConversionStreamImpl {
            state: RefCell::new(VoiceConversionStreamState {
                target_voice_id,
                status: wit_streaming::StreamStatus::Ready,
                pending_chunks: Vec::new(),
                sequence_counter: 0,
            }),
        };
    }
}

impl wit_streaming::GuestVoiceConversionStream for VoiceConversionStreamImpl {
    fn send_audio(&self, _audio_data: Vec<u8>) -> Result<(), wit_types::TtsError> {
        // ElevenLabs voice conversion is not real-time streaming
        // This would require their speech-to-speech API
        return Err(types::unsupported_operation_error(
            "Real-time voice conversion streaming not supported by ElevenLabs",
        ));
    }

    fn receive_converted(&self) -> Result<Option<wit_types::AudioChunk>, wit_types::TtsError> {
        let mut state = self.state.borrow_mut();
        if state.pending_chunks.is_empty() {
            return Ok(None);
        }

        let chunk = state.pending_chunks.remove(0);
        return Ok(Some(chunk));
    }

    fn finish(&self) -> Result<(), wit_types::TtsError> {
        let mut state = self.state.borrow_mut();
        state.status = wit_streaming::StreamStatus::Finished;
        return Ok(());
    }

    fn close(&self) {
        let mut state = self.state.borrow_mut();
        state.status = wit_streaming::StreamStatus::Closed;
        state.pending_chunks.clear();
    }
}

// ============================================================
// STREAMING INTERFACE FUNCTIONS
// ============================================================

/// Create a new streaming synthesis session
pub fn create_stream(
    voice: &VoiceImpl,
    options: Option<wit_synthesis::SynthesisOptions>,
) -> Result<wit_streaming::SynthesisStream, wit_types::TtsError> {
    let model_id = options
        .as_ref()
        .and_then(|o| o.model_version.clone())
        .unwrap_or_else(|| client::get_model_version());

    let output_format = options
        .as_ref()
        .and_then(|o| o.audio_config.as_ref())
        .map(|c| types::map_audio_format(c.format.clone()).to_string())
        .unwrap_or_else(|| "mp3_44100_128".to_string());

    let stream_impl =
        SynthesisStreamImpl::new(voice.voice_id().to_string(), model_id, output_format);

    return Ok(wit_streaming::SynthesisStream::new(stream_impl));
}

/// Create a voice conversion stream
pub fn create_voice_conversion_stream(
    target_voice: &VoiceImpl,
    _options: Option<wit_synthesis::SynthesisOptions>,
) -> Result<wit_streaming::VoiceConversionStream, wit_types::TtsError> {
    let stream_impl = VoiceConversionStreamImpl::new(target_voice.voice_id().to_string());
    return Ok(wit_streaming::VoiceConversionStream::new(stream_impl));
}
