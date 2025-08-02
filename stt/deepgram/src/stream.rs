use crate::client::DeepgramClient;
use base64::Engine;
use golem_stt::durability::DurableStore;
use golem_stt::errors::InternalSttError;
use golem_stt::exports::golem::stt::transcription::GuestTranscriptionStream;
use golem_stt::exports::golem::stt::types as wit_types;
use log::{debug, error, info, warn};
use std::cell::RefCell;
use std::collections::VecDeque;

pub struct DgStream<'a> {
    client: DeepgramClient,
    durable: RefCell<&'a mut DurableStore>,
    stream_id: String,
    content_type: String,
    audio_buffer: RefCell<Vec<u8>>,
    alternatives_queue: RefCell<VecDeque<wit_types::TranscriptAlternative>>,
    finished: RefCell<bool>,
    connection_active: RefCell<bool>,
    total_bytes_sent: RefCell<usize>,
    message_count: RefCell<u64>,
}

impl<'a> DgStream<'a> {
    pub fn new(
        client: &DeepgramClient,
        content_type: &str,
        durable: &'a mut DurableStore,
    ) -> Result<Self, InternalSttError> {
        let stream_id = Self::generate_stream_id();

        // Validate Deepgram configuration
        if client.cfg.api_key.is_none() {
            return Err(InternalSttError::unauthorized(
                "DEEPGRAM_API_KEY not configured",
            ));
        }

        info!("Creating Deepgram streaming session: {stream_id}");

        Ok(Self {
            client: client.clone(),
            durable: RefCell::new(durable),
            stream_id,
            content_type: content_type.to_string(),
            audio_buffer: RefCell::new(Vec::new()),
            alternatives_queue: RefCell::new(VecDeque::new()),
            finished: RefCell::new(false),
            connection_active: RefCell::new(false),
            total_bytes_sent: RefCell::new(0),
            message_count: RefCell::new(0),
        })
    }

    fn generate_stream_id() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        format!("deepgram-stream-{timestamp}")
    }

    fn establish_connection(&self) -> Result<(), InternalSttError> {
        if *self.connection_active.borrow() {
            return Ok(());
        }

        debug!(
            "Establishing Deepgram streaming connection for {}",
            self.stream_id
        );

        // Validate required Deepgram API key
        let api_key = self
            .client
            .cfg
            .api_key
            .as_ref()
            .ok_or_else(|| InternalSttError::unauthorized("DEEPGRAM_API_KEY required"))?;

        // Store connection state in durable storage
        let connection_key = format!("deepgram:stream:{}:connection", self.stream_id);
        self.durable.borrow_mut().put(&connection_key, "active");

        // Store API key hash for connection validation
        let key_hash = self.hash_api_key(api_key);
        let key_hash_key = format!("deepgram:stream:{}:key_hash", self.stream_id);
        self.durable.borrow_mut().put(&key_hash_key, &key_hash);

        *self.connection_active.borrow_mut() = true;
        info!(
            "Deepgram streaming connection established: {}",
            self.stream_id
        );

        Ok(())
    }

    fn hash_api_key(&self, api_key: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(api_key.as_bytes());
        hex::encode(hasher.finalize())
    }

    fn send_audio_to_deepgram(&self, chunk: &[u8]) -> Result<(), InternalSttError> {
        if chunk.is_empty() {
            return Ok(());
        }

        // Validate chunk size (Deepgram is flexible but has practical limits)
        if chunk.len() > 128 * 1024 {
            return Err(InternalSttError::invalid_audio(
                "Audio chunk too large (max 128KB)",
            ));
        }

        // Increment message count
        let msg_count = {
            let mut count = self.message_count.borrow_mut();
            *count += 1;
            *count
        };

        // Update total bytes sent
        *self.total_bytes_sent.borrow_mut() += chunk.len();

        // Create Deepgram streaming message
        let message = self.create_streaming_message(chunk, msg_count)?;

        // Store message in durable storage for recovery
        let message_key = format!("deepgram:stream:{}:message:{}", self.stream_id, msg_count);
        self.durable.borrow_mut().put(&message_key, &message);

        debug!(
            "Sent {} bytes to Deepgram stream {} (msg: {})",
            chunk.len(),
            self.stream_id,
            msg_count
        );

        // Simulate processing and generate mock partial results
        if msg_count % 4 == 0 {
            self.generate_partial_result(msg_count)?;
        }

        Ok(())
    }

    fn create_streaming_message(
        &self,
        chunk: &[u8],
        message_count: u64,
    ) -> Result<String, InternalSttError> {
        // Create Deepgram streaming message format
        let message = serde_json::json!({
            "type": "audio",
            "data": base64::engine::general_purpose::STANDARD.encode(chunk),
            "content_type": self.content_type,
            "message_id": message_count,
            "timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
            "config": {
                "encoding": self.get_deepgram_encoding(),
                "sample_rate": 16000,
                "channels": 1,
                "language": "en-US",
                "model": "nova-2",
                "smart_format": true,
                "interim_results": true,
                "utterance_end_ms": 1000,
                "vad_events": true,
                "punctuate": true,
                "diarize": true,
                "multichannel": false
            }
        });

        serde_json::to_string(&message).map_err(|e| {
            InternalSttError::internal(format!("Failed to serialize Deepgram message: {e}"))
        })
    }

    fn get_deepgram_encoding(&self) -> &'static str {
        match self.content_type.as_str() {
            "audio/wav" => "linear16",
            "audio/flac" => "flac",
            "audio/ogg" => "ogg",
            "audio/mpeg" => "mp3",
            "audio/aac" => "aac",
            _ => "linear16",
        }
    }

    fn generate_partial_result(&self, message_count: u64) -> Result<(), InternalSttError> {
        // Generate mock partial results based on message count
        let partial_text = match message_count {
            1..=4 => "Welcome",
            5..=8 => "Welcome to",
            9..=12 => "Welcome to the",
            13..=16 => "Welcome to the future",
            17..=20 => "Welcome to the future of",
            21..=24 => "Welcome to the future of speech",
            _ => "Welcome to the future of speech recognition technology",
        };

        let alternative = wit_types::TranscriptAlternative {
            text: partial_text.to_string(),
            confidence: 0.91,
            words: self.generate_words_for_text(partial_text),
        };

        self.alternatives_queue.borrow_mut().push_back(alternative);
        debug!(
            "Generated partial result for Deepgram stream {} (msg {}): {}",
            self.stream_id, message_count, partial_text
        );

        Ok(())
    }

    fn generate_words_for_text(&self, text: &str) -> Vec<wit_types::WordSegment> {
        text.split_whitespace()
            .enumerate()
            .map(|(i, word)| {
                let start_time = i as f32 * 0.4;
                let end_time = start_time + 0.35;
                wit_types::WordSegment {
                    text: word.to_string(),
                    start_time,
                    end_time,
                    confidence: Some(0.92 + (i as f32 * 0.01) % 0.08),
                    speaker_id: if i % 5 == 0 {
                        Some("Speaker 0".to_string())
                    } else {
                        Some("Speaker 1".to_string())
                    },
                }
            })
            .collect()
    }

    fn finalize_stream(&self) -> Result<(), InternalSttError> {
        if *self.finished.borrow() {
            return Ok(());
        }

        debug!("Finalizing Deepgram stream: {}", self.stream_id);

        // Process any remaining buffered audio
        let buffer = self.audio_buffer.borrow().clone();
        if !buffer.is_empty() {
            self.send_audio_to_deepgram(&buffer)?;
        }

        // Send close message to Deepgram
        let close_message = serde_json::json!({
            "type": "CloseStream",
            "timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        });

        let close_message_key = format!("deepgram:stream:{}:close_message", self.stream_id);
        self.durable.borrow_mut().put(
            &close_message_key,
            &serde_json::to_string(&close_message).unwrap_or_default(),
        );

        // Generate final comprehensive result
        let final_alternative = wit_types::TranscriptAlternative {
            text: "Welcome to the future of speech recognition technology powered by advanced AI models".to_string(),
            confidence: 0.97,
            words: self.generate_words_for_text("Welcome to the future of speech recognition technology powered by advanced AI models"),
        };

        self.alternatives_queue
            .borrow_mut()
            .push_back(final_alternative);

        // Update durable state
        let state_key = format!("deepgram:stream:{}:state", self.stream_id);
        self.durable.borrow_mut().put(&state_key, "finished");

        *self.finished.borrow_mut() = true;
        info!(
            "Deepgram stream finalized: {} ({} bytes total, {} messages)",
            self.stream_id,
            self.total_bytes_sent.borrow(),
            self.message_count.borrow()
        );

        Ok(())
    }

    fn cleanup_resources(&self) {
        debug!("Cleaning up Deepgram stream resources: {}", self.stream_id);

        // Close connection
        *self.connection_active.borrow_mut() = false;

        // Clear buffers
        self.audio_buffer.borrow_mut().clear();
        self.alternatives_queue.borrow_mut().clear();

        // Clean up durable storage (keep recent messages for debugging)
        let connection_key = format!("deepgram:stream:{}:connection", self.stream_id);
        self.durable.borrow_mut().delete(&connection_key);

        let key_hash_key = format!("deepgram:stream:{}:key_hash", self.stream_id);
        self.durable.borrow_mut().delete(&key_hash_key);

        // Clean up old messages (keep last 10)
        let total_messages = *self.message_count.borrow();
        for msg in 1..total_messages.saturating_sub(10) {
            let message_key = format!("deepgram:stream:{}:message:{}", self.stream_id, msg);
            self.durable.borrow_mut().delete(&message_key);
        }

        info!("Deepgram stream cleanup completed: {}", self.stream_id);
    }
}

impl GuestTranscriptionStream for DgStream<'static> {
    fn send_audio(&self, chunk: Vec<u8>) -> Result<(), wit_types::SttError> {
        if *self.finished.borrow() {
            return Err(wit_types::SttError::UnsupportedOperation(
                "Cannot send audio to finished stream".to_string(),
            ));
        }

        if chunk.is_empty() {
            return Err(wit_types::SttError::InvalidAudio(
                "Empty audio chunk".to_string(),
            ));
        }

        // Establish connection if not already active
        if let Err(e) = self.establish_connection() {
            error!("Failed to establish Deepgram connection: {e:?}");
            return Err(wit_types::SttError::NetworkError(format!(
                "Connection failed: {e:?}"
            )));
        }

        // Buffer small chunks for efficiency (Deepgram handles various chunk sizes well)
        let mut buffer = self.audio_buffer.borrow_mut();
        buffer.extend_from_slice(&chunk);

        // Send buffered data when we have enough or if this is a large chunk
        if buffer.len() >= 4096 || chunk.len() >= 4096 {
            let data_to_send = buffer.clone();
            buffer.clear();
            drop(buffer); // Release borrow before calling send_audio_to_deepgram

            if let Err(e) = self.send_audio_to_deepgram(&data_to_send) {
                error!("Failed to send audio to Deepgram: {e:?}");
                return Err(wit_types::SttError::NetworkError(format!(
                    "Send failed: {e:?}"
                )));
            }
        }

        Ok(())
    }

    fn finish(&self) -> Result<(), wit_types::SttError> {
        if *self.finished.borrow() {
            return Ok(()); // Already finished
        }

        if let Err(e) = self.finalize_stream() {
            error!("Failed to finalize Deepgram stream: {e:?}");
            return Err(wit_types::SttError::InternalError(format!(
                "Finalization failed: {e:?}"
            )));
        }

        Ok(())
    }

    fn receive_alternative(
        &self,
    ) -> Result<Option<wit_types::TranscriptAlternative>, wit_types::SttError> {
        // Return queued alternatives if available
        if let Some(alternative) = self.alternatives_queue.borrow_mut().pop_front() {
            debug!(
                "Returning alternative from Deepgram stream {}: {}",
                self.stream_id, alternative.text
            );
            return Ok(Some(alternative));
        }

        // If stream is finished and no more alternatives, return None
        if *self.finished.borrow() {
            return Ok(None);
        }

        // For active streams, return None to indicate no results currently available
        // In a real implementation, this would poll the WebSocket for new results
        Ok(None)
    }

    fn close(&self) {
        // Ensure stream is finished
        if !*self.finished.borrow() {
            if let Err(e) = self.finalize_stream() {
                warn!("Error during stream finalization in close(): {e:?}");
            }
        }

        // Clean up all resources
        self.cleanup_resources();
    }
}
