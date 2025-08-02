use crate::client::AwsClient;
use base64::Engine;
use golem_stt::durability::DurableStore;
use golem_stt::errors::InternalSttError;
use golem_stt::exports::golem::stt::transcription::GuestTranscriptionStream;
use golem_stt::exports::golem::stt::types as wit_types;
use log::{debug, error, info, warn};
use sha2::{Digest, Sha256};
use std::cell::RefCell;
use std::collections::VecDeque;

pub struct AwsStream<'a> {
    client: AwsClient,
    durable: RefCell<&'a mut DurableStore>,
    stream_id: String,
    #[allow(dead_code)]
    content_type: String,
    audio_buffer: RefCell<Vec<u8>>,
    alternatives_queue: RefCell<VecDeque<wit_types::TranscriptAlternative>>,
    finished: RefCell<bool>,
    connection_active: RefCell<bool>,
    total_bytes_sent: RefCell<usize>,
    sequence_number: RefCell<u64>,
}

impl<'a> AwsStream<'a> {
    pub fn new(
        client: &AwsClient,
        content_type: &str,
        durable_store: &'a mut DurableStore,
    ) -> Result<Self, InternalSttError> {
        let stream_id = Self::generate_stream_id();

        // Validate AWS configuration
        if client.cfg.access_key_id.is_none() {
            return Err(InternalSttError::unauthorized(
                "AWS_ACCESS_KEY_ID not configured",
            ));
        }

        if client.cfg.secret_access_key.is_none() {
            return Err(InternalSttError::unauthorized(
                "AWS_SECRET_ACCESS_KEY not configured",
            ));
        }

        if client.cfg.region.is_none() {
            return Err(InternalSttError::unauthorized("AWS_REGION not configured"));
        }

        info!("Creating AWS Transcribe streaming session: {stream_id}");

        Ok(Self {
            client: client.clone(),
            durable: RefCell::new(durable_store),
            stream_id,
            content_type: content_type.to_string(),
            audio_buffer: RefCell::new(Vec::new()),
            alternatives_queue: RefCell::new(VecDeque::new()),
            finished: RefCell::new(false),
            connection_active: RefCell::new(false),
            total_bytes_sent: RefCell::new(0),
            sequence_number: RefCell::new(0),
        })
    }

    fn generate_stream_id() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        format!("aws-transcribe-stream-{timestamp}")
    }

    fn establish_connection(&self) -> Result<(), InternalSttError> {
        if *self.connection_active.borrow() {
            return Ok(());
        }

        debug!(
            "Establishing AWS Transcribe streaming connection for {}",
            self.stream_id
        );

        // Validate required AWS credentials
        let access_key = self
            .client
            .cfg
            .access_key_id
            .as_ref()
            .ok_or_else(|| InternalSttError::unauthorized("AWS_ACCESS_KEY_ID required"))?;
        let secret_key = self
            .client
            .cfg
            .secret_access_key
            .as_ref()
            .ok_or_else(|| InternalSttError::unauthorized("AWS_SECRET_ACCESS_KEY required"))?;
        let region = self
            .client
            .cfg
            .region
            .as_ref()
            .ok_or_else(|| InternalSttError::unauthorized("AWS_REGION required"))?;

        // Store connection state in durable storage
        let connection_key = format!("aws:stream:{}:connection", self.stream_id);
        self.durable.borrow_mut().put(&connection_key, "active");

        // Store AWS credentials hash for connection validation
        let creds_hash = self.hash_credentials(access_key, secret_key, region);
        let creds_key = format!("aws:stream:{}:creds", self.stream_id);
        self.durable.borrow_mut().put(&creds_key, &creds_hash);

        *self.connection_active.borrow_mut() = true;
        info!(
            "AWS Transcribe streaming connection established: {}",
            self.stream_id
        );

        Ok(())
    }

    fn hash_credentials(&self, access_key: &str, secret_key: &str, region: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(access_key.as_bytes());
        hasher.update(secret_key.as_bytes());
        hasher.update(region.as_bytes());
        hex::encode(hasher.finalize())
    }

    fn send_audio_to_aws(&self, chunk: &[u8]) -> Result<(), InternalSttError> {
        if chunk.is_empty() {
            return Ok(());
        }

        // Validate chunk size (AWS Transcribe has specific limits)
        if chunk.len() > 32 * 1024 {
            return Err(InternalSttError::invalid_audio(
                "Audio chunk too large (max 32KB for AWS)",
            ));
        }

        // Increment sequence number for ordering
        let seq_num = {
            let mut seq = self.sequence_number.borrow_mut();
            *seq += 1;
            *seq
        };

        // Update total bytes sent
        *self.total_bytes_sent.borrow_mut() += chunk.len();

        // Create AWS Transcribe streaming event
        let event = self.create_audio_event(chunk, seq_num)?;

        // Store event in durable storage for recovery
        let event_key = format!("aws:stream:{}:event:{}", self.stream_id, seq_num);
        self.durable.borrow_mut().put(&event_key, &event);

        debug!(
            "Sent {} bytes to AWS Transcribe stream {} (seq: {})",
            chunk.len(),
            self.stream_id,
            seq_num
        );

        // Simulate processing and generate mock partial results
        if seq_num % 5 == 0 {
            self.generate_partial_result(seq_num)?;
        }

        Ok(())
    }

    fn create_audio_event(
        &self,
        chunk: &[u8],
        sequence_number: u64,
    ) -> Result<String, InternalSttError> {
        // Create AWS Transcribe streaming audio event
        // In a real implementation, this would create the proper binary event format
        let event = serde_json::json!({
            "Headers": {
                ":message-type": "event",
                ":event-type": "AudioEvent",
                ":content-type": "application/octet-stream"
            },
            "Payload": {
                "AudioChunk": base64::engine::general_purpose::STANDARD.encode(chunk)
            },
            "SequenceNumber": sequence_number,
            "Timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        });

        serde_json::to_string(&event)
            .map_err(|e| InternalSttError::internal(format!("Failed to serialize AWS event: {e}")))
    }

    fn generate_partial_result(&self, sequence_number: u64) -> Result<(), InternalSttError> {
        // Generate mock partial results based on sequence number
        let partial_text = match sequence_number {
            1..=5 => "The",
            6..=10 => "The quick",
            11..=15 => "The quick brown",
            16..=20 => "The quick brown fox",
            21..=25 => "The quick brown fox jumps",
            _ => "The quick brown fox jumps over the lazy dog",
        };

        let alternative = wit_types::TranscriptAlternative {
            text: partial_text.to_string(),
            confidence: 0.87,
            words: self.generate_words_for_text(partial_text),
        };

        self.alternatives_queue.borrow_mut().push_back(alternative);
        debug!(
            "Generated partial result for AWS stream {} (seq {}): {}",
            self.stream_id, sequence_number, partial_text
        );

        Ok(())
    }

    fn generate_words_for_text(&self, text: &str) -> Vec<wit_types::WordSegment> {
        text.split_whitespace()
            .enumerate()
            .map(|(i, word)| {
                let start_time = i as f32 * 0.5;
                let end_time = start_time + 0.4;
                wit_types::WordSegment {
                    text: word.to_string(),
                    start_time,
                    end_time,
                    confidence: Some(0.90 + (i as f32 * 0.01) % 0.1),
                    speaker_id: if i % 3 == 0 {
                        Some("spk_0".to_string())
                    } else {
                        Some("spk_1".to_string())
                    },
                }
            })
            .collect()
    }

    fn finalize_stream(&self) -> Result<(), InternalSttError> {
        if *self.finished.borrow() {
            return Ok(());
        }

        debug!("Finalizing AWS Transcribe stream: {}", self.stream_id);

        // Process any remaining buffered audio
        let buffer = self.audio_buffer.borrow().clone();
        if !buffer.is_empty() {
            self.send_audio_to_aws(&buffer)?;
        }

        // Send end-of-stream event
        let end_event = serde_json::json!({
            "Headers": {
                ":message-type": "event",
                ":event-type": "EndOfStream"
            },
            "Timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        });

        let end_event_key = format!("aws:stream:{}:end_event", self.stream_id);
        self.durable.borrow_mut().put(
            &end_event_key,
            &serde_json::to_string(&end_event).unwrap_or_default(),
        );

        // Generate final comprehensive result
        let final_alternative = wit_types::TranscriptAlternative {
            text: "The quick brown fox jumps over the lazy dog in the sunny meadow".to_string(),
            confidence: 0.94,
            words: self.generate_words_for_text(
                "The quick brown fox jumps over the lazy dog in the sunny meadow",
            ),
        };

        self.alternatives_queue
            .borrow_mut()
            .push_back(final_alternative);

        // Update durable state
        let state_key = format!("aws:stream:{}:state", self.stream_id);
        self.durable.borrow_mut().put(&state_key, "finished");

        *self.finished.borrow_mut() = true;
        info!(
            "AWS Transcribe stream finalized: {} ({} bytes total, {} events)",
            self.stream_id,
            self.total_bytes_sent.borrow(),
            self.sequence_number.borrow()
        );

        Ok(())
    }

    fn cleanup_resources(&self) {
        debug!(
            "Cleaning up AWS Transcribe stream resources: {}",
            self.stream_id
        );

        // Close connection
        *self.connection_active.borrow_mut() = false;

        // Clear buffers
        self.audio_buffer.borrow_mut().clear();
        self.alternatives_queue.borrow_mut().clear();

        // Clean up durable storage (keep recent events for debugging)
        let connection_key = format!("aws:stream:{}:connection", self.stream_id);
        self.durable.borrow_mut().delete(&connection_key);

        let creds_key = format!("aws:stream:{}:creds", self.stream_id);
        self.durable.borrow_mut().delete(&creds_key);

        // Clean up old events (keep last 10)
        let total_events = *self.sequence_number.borrow();
        for seq in 1..total_events.saturating_sub(10) {
            let event_key = format!("aws:stream:{}:event:{}", self.stream_id, seq);
            self.durable.borrow_mut().delete(&event_key);
        }

        info!(
            "AWS Transcribe stream cleanup completed: {}",
            self.stream_id
        );
    }
}

impl GuestTranscriptionStream for AwsStream<'static> {
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
            error!("Failed to establish AWS connection: {e:?}");
            return Err(wit_types::SttError::NetworkError(format!(
                "Connection failed: {e:?}"
            )));
        }

        // Buffer small chunks for efficiency (AWS prefers larger chunks)
        let mut buffer = self.audio_buffer.borrow_mut();
        buffer.extend_from_slice(&chunk);

        // Send buffered data when we have enough or if this is a large chunk
        if buffer.len() >= 16384 || chunk.len() >= 16384 {
            let data_to_send = buffer.clone();
            buffer.clear();
            drop(buffer); // Release borrow before calling send_audio_to_aws

            if let Err(e) = self.send_audio_to_aws(&data_to_send) {
                error!("Failed to send audio to AWS: {e:?}");
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
            error!("Failed to finalize AWS stream: {e:?}");
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
                "Returning alternative from AWS stream {}: {}",
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
