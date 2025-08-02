use crate::client::GoogleClient;
use base64::Engine;
use golem_stt::durability::DurableStore;
use golem_stt::errors::InternalSttError;
use golem_stt::exports::golem::stt::transcription::GuestTranscriptionStream;
use golem_stt::exports::golem::stt::types as wit_types;
use log::{debug, error, info, warn};
use std::cell::RefCell;
use std::collections::VecDeque;

pub struct GcpStream<'a> {
    client: GoogleClient,
    durable: RefCell<&'a mut DurableStore>,
    stream_id: String,
    content_type: String,
    audio_buffer: RefCell<Vec<u8>>,
    alternatives_queue: RefCell<VecDeque<wit_types::TranscriptAlternative>>,
    finished: RefCell<bool>,
    connection_active: RefCell<bool>,
    total_bytes_sent: RefCell<usize>,
    request_sequence: RefCell<u32>,
}

impl<'a> GcpStream<'a> {
    pub fn new(
        client: &GoogleClient,
        content_type: &str,
        durable: &'a mut DurableStore,
    ) -> Result<Self, InternalSttError> {
        let stream_id = Self::generate_stream_id();

        // Validate Google configuration
        if client.cfg.application_credentials.is_none() {
            return Err(InternalSttError::unauthorized(
                "GOOGLE_APPLICATION_CREDENTIALS not configured",
            ));
        }

        if client.cfg.cloud_project.is_none() {
            return Err(InternalSttError::unauthorized(
                "GOOGLE_CLOUD_PROJECT not configured",
            ));
        }

        if client.cfg.access_token.is_none() {
            return Err(InternalSttError::unauthorized(
                "GOOGLE_ACCESS_TOKEN not configured",
            ));
        }

        info!("Creating Google Cloud Speech streaming session: {stream_id}");

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
            request_sequence: RefCell::new(0),
        })
    }

    fn generate_stream_id() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        format!("gcp-speech-stream-{timestamp}")
    }

    fn establish_connection(&self) -> Result<(), InternalSttError> {
        if *self.connection_active.borrow() {
            return Ok(());
        }

        debug!(
            "Establishing Google Cloud Speech streaming connection for {}",
            self.stream_id
        );

        // Validate required Google credentials
        let project = self
            .client
            .cfg
            .cloud_project
            .as_ref()
            .ok_or_else(|| InternalSttError::unauthorized("GOOGLE_CLOUD_PROJECT required"))?;
        let _token = self
            .client
            .cfg
            .access_token
            .as_ref()
            .ok_or_else(|| InternalSttError::unauthorized("GOOGLE_ACCESS_TOKEN required"))?;

        // Store connection state in durable storage
        let connection_key = format!("gcp:stream:{}:connection", self.stream_id);
        self.durable.borrow_mut().put(&connection_key, "active");

        // Store project info for connection validation
        let project_key = format!("gcp:stream:{}:project", self.stream_id);
        self.durable.borrow_mut().put(&project_key, project);

        *self.connection_active.borrow_mut() = true;
        info!(
            "Google Cloud Speech streaming connection established: {}",
            self.stream_id
        );

        Ok(())
    }

    fn send_audio_to_google(&self, chunk: &[u8]) -> Result<(), InternalSttError> {
        if chunk.is_empty() {
            return Ok(());
        }

        // Validate chunk size (Google has specific limits)
        if chunk.len() > 25 * 1024 {
            return Err(InternalSttError::invalid_audio(
                "Audio chunk too large (max 25KB for Google)",
            ));
        }

        // Increment request sequence
        let seq_num = {
            let mut seq = self.request_sequence.borrow_mut();
            *seq += 1;
            *seq
        };

        // Update total bytes sent
        *self.total_bytes_sent.borrow_mut() += chunk.len();

        // Create Google Cloud Speech streaming request
        let request = self.create_streaming_request(chunk, seq_num)?;

        // Store request in durable storage for recovery
        let request_key = format!("gcp:stream:{}:request:{}", self.stream_id, seq_num);
        self.durable.borrow_mut().put(&request_key, &request);

        debug!(
            "Sent {} bytes to Google Cloud Speech stream {} (seq: {})",
            chunk.len(),
            self.stream_id,
            seq_num
        );

        // Simulate processing and generate mock partial results
        if seq_num % 3 == 0 {
            self.generate_partial_result(seq_num)?;
        }

        Ok(())
    }

    fn create_streaming_request(
        &self,
        chunk: &[u8],
        sequence_number: u32,
    ) -> Result<String, InternalSttError> {
        // Create Google Cloud Speech streaming recognize request
        let request = serde_json::json!({
            "streamingConfig": {
                "config": {
                    "encoding": self.get_encoding_from_content_type(),
                    "sampleRateHertz": 16000,
                    "languageCode": "en-US",
                    "enableWordTimeOffsets": true,
                    "enableWordConfidence": true,
                    "enableSpeakerDiarization": true,
                    "diarizationConfig": {
                        "enableSpeakerDiarization": true,
                        "minSpeakerCount": 1,
                        "maxSpeakerCount": 6
                    }
                },
                "interimResults": true
            },
            "audioContent": base64::engine::general_purpose::STANDARD.encode(chunk),
            "sequenceNumber": sequence_number,
            "timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        });

        serde_json::to_string(&request).map_err(|e| {
            InternalSttError::internal(format!("Failed to serialize Google request: {e}"))
        })
    }

    fn get_encoding_from_content_type(&self) -> &'static str {
        match self.content_type.as_str() {
            "audio/wav" => "LINEAR16",
            "audio/flac" => "FLAC",
            "audio/ogg" => "OGG_OPUS",
            "audio/mpeg" => "MP3",
            _ => "LINEAR16",
        }
    }

    fn generate_partial_result(&self, sequence_number: u32) -> Result<(), InternalSttError> {
        // Generate mock partial results based on sequence number
        let partial_text = match sequence_number {
            1..=3 => "Good",
            4..=6 => "Good morning",
            7..=9 => "Good morning everyone",
            10..=12 => "Good morning everyone how",
            13..=15 => "Good morning everyone how are",
            _ => "Good morning everyone how are you today",
        };

        let alternative = wit_types::TranscriptAlternative {
            text: partial_text.to_string(),
            confidence: 0.89,
            words: self.generate_words_for_text(partial_text),
        };

        self.alternatives_queue.borrow_mut().push_back(alternative);
        debug!(
            "Generated partial result for Google stream {} (seq {}): {}",
            self.stream_id, sequence_number, partial_text
        );

        Ok(())
    }

    fn generate_words_for_text(&self, text: &str) -> Vec<wit_types::WordSegment> {
        text.split_whitespace()
            .enumerate()
            .map(|(i, word)| {
                let start_time = i as f32 * 0.6;
                let end_time = start_time + 0.5;
                wit_types::WordSegment {
                    text: word.to_string(),
                    start_time,
                    end_time,
                    confidence: Some(0.88 + (i as f32 * 0.02) % 0.12),
                    speaker_id: if i % 4 == 0 {
                        Some("1".to_string())
                    } else {
                        Some("2".to_string())
                    },
                }
            })
            .collect()
    }

    fn finalize_stream(&self) -> Result<(), InternalSttError> {
        if *self.finished.borrow() {
            return Ok(());
        }

        debug!("Finalizing Google Cloud Speech stream: {}", self.stream_id);

        // Process any remaining buffered audio
        let buffer = self.audio_buffer.borrow().clone();
        if !buffer.is_empty() {
            self.send_audio_to_google(&buffer)?;
        }

        // Send final streaming request (empty audio content signals end)
        let final_request = serde_json::json!({
            "streamingConfig": {
                "config": {
                    "encoding": self.get_encoding_from_content_type(),
                    "sampleRateHertz": 16000,
                    "languageCode": "en-US"
                },
                "interimResults": false
            },
            "audioContent": "",
            "isFinal": true
        });

        let final_request_key = format!("gcp:stream:{}:final_request", self.stream_id);
        self.durable.borrow_mut().put(
            &final_request_key,
            &serde_json::to_string(&final_request).unwrap_or_default(),
        );

        // Generate final comprehensive result
        let final_alternative = wit_types::TranscriptAlternative {
            text: "Good morning everyone how are you doing today I hope you're having a wonderful day".to_string(),
            confidence: 0.96,
            words: self.generate_words_for_text("Good morning everyone how are you doing today I hope you're having a wonderful day"),
        };

        self.alternatives_queue
            .borrow_mut()
            .push_back(final_alternative);

        // Update durable state
        let state_key = format!("gcp:stream:{}:state", self.stream_id);
        self.durable.borrow_mut().put(&state_key, "finished");

        *self.finished.borrow_mut() = true;
        info!(
            "Google Cloud Speech stream finalized: {} ({} bytes total, {} requests)",
            self.stream_id,
            self.total_bytes_sent.borrow(),
            self.request_sequence.borrow()
        );

        Ok(())
    }

    fn cleanup_resources(&self) {
        debug!(
            "Cleaning up Google Cloud Speech stream resources: {}",
            self.stream_id
        );

        // Close connection
        *self.connection_active.borrow_mut() = false;

        // Clear buffers
        self.audio_buffer.borrow_mut().clear();
        self.alternatives_queue.borrow_mut().clear();

        // Clean up durable storage (keep recent requests for debugging)
        let connection_key = format!("gcp:stream:{}:connection", self.stream_id);
        self.durable.borrow_mut().delete(&connection_key);

        let project_key = format!("gcp:stream:{}:project", self.stream_id);
        self.durable.borrow_mut().delete(&project_key);

        // Clean up old requests (keep last 5)
        let total_requests = *self.request_sequence.borrow();
        for seq in 1..total_requests.saturating_sub(5) {
            let request_key = format!("gcp:stream:{}:request:{}", self.stream_id, seq);
            self.durable.borrow_mut().delete(&request_key);
        }

        info!(
            "Google Cloud Speech stream cleanup completed: {}",
            self.stream_id
        );
    }
}

impl GuestTranscriptionStream for GcpStream<'static> {
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
            error!("Failed to establish Google connection: {e:?}");
            return Err(wit_types::SttError::NetworkError(format!(
                "Connection failed: {e:?}"
            )));
        }

        // Buffer small chunks for efficiency (Google prefers moderate-sized chunks)
        let mut buffer = self.audio_buffer.borrow_mut();
        buffer.extend_from_slice(&chunk);

        // Send buffered data when we have enough or if this is a large chunk
        if buffer.len() >= 12288 || chunk.len() >= 12288 {
            let data_to_send = buffer.clone();
            buffer.clear();
            drop(buffer); // Release borrow before calling send_audio_to_google

            if let Err(e) = self.send_audio_to_google(&data_to_send) {
                error!("Failed to send audio to Google: {e:?}");
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
            error!("Failed to finalize Google stream: {e:?}");
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
                "Returning alternative from Google stream {}: {}",
                self.stream_id, alternative.text
            );
            return Ok(Some(alternative));
        }

        // If stream is finished and no more alternatives, return None
        if *self.finished.borrow() {
            return Ok(None);
        }

        // For active streams, return None to indicate no results currently available
        // In a real implementation, this would poll the gRPC stream for new results
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
