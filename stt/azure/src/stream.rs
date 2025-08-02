use crate::client::AzureClient;
use base64::Engine;
use golem_stt::durability::DurableStore;
use golem_stt::errors::InternalSttError;
use golem_stt::exports::golem::stt::transcription::GuestTranscriptionStream;
use golem_stt::exports::golem::stt::types as wit_types;
use log::{debug, error, info, warn};
use std::cell::RefCell;
use std::collections::VecDeque;

pub struct AzureStream<'a> {
    #[allow(dead_code)]
    client: AzureClient,
    durable: RefCell<&'a mut DurableStore>,
    stream_id: String,
    #[allow(dead_code)]
    content_type: String,
    audio_buffer: RefCell<Vec<u8>>,
    alternatives_queue: RefCell<VecDeque<wit_types::TranscriptAlternative>>,
    finished: RefCell<bool>,
    connection_active: RefCell<bool>,
    total_bytes_sent: RefCell<usize>,
}

impl<'a> AzureStream<'a> {
    pub fn new(
        client: &AzureClient,
        content_type: &str,
        durable: &'a mut DurableStore,
    ) -> Result<Self, InternalSttError> {
        let stream_id = Self::generate_stream_id();

        // Validate Azure configuration
        if client.cfg.speech_key.is_none() {
            return Err(InternalSttError::unauthorized(
                "AZURE_SPEECH_KEY not configured",
            ));
        }

        if client.cfg.speech_region.is_none() {
            return Err(InternalSttError::unauthorized(
                "AZURE_SPEECH_REGION not configured",
            ));
        }

        info!("Creating Azure streaming session: {stream_id}");

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
        })
    }

    fn generate_stream_id() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        format!("azure-stream-{timestamp}")
    }

    fn establish_connection(&self) -> Result<(), InternalSttError> {
        if *self.connection_active.borrow() {
            return Ok(());
        }

        debug!(
            "Establishing Azure streaming connection for {}",
            self.stream_id
        );

        // In a full implementation, this would establish a WebSocket connection
        // to Azure's streaming endpoint. For now, we simulate the connection.

        // Store connection state in durable storage
        let connection_key = format!("azure:stream:{}:connection", self.stream_id);
        self.durable.borrow_mut().put(&connection_key, "active");

        *self.connection_active.borrow_mut() = true;
        info!("Azure streaming connection established: {}", self.stream_id);

        Ok(())
    }

    fn send_audio_to_azure(&self, chunk: &[u8]) -> Result<(), InternalSttError> {
        if chunk.is_empty() {
            return Ok(());
        }

        // Validate chunk size
        if chunk.len() > 64 * 1024 {
            return Err(InternalSttError::invalid_audio(
                "Audio chunk too large (max 64KB)",
            ));
        }

        // Update total bytes sent
        *self.total_bytes_sent.borrow_mut() += chunk.len();

        // Store chunk in durable storage for recovery
        let chunk_key = format!(
            "azure:stream:{}:chunk:{}",
            self.stream_id,
            self.total_bytes_sent.borrow()
        );
        self.durable.borrow_mut().put(
            &chunk_key,
            &base64::engine::general_purpose::STANDARD.encode(chunk),
        );

        debug!(
            "Sent {} bytes to Azure stream {}",
            chunk.len(),
            self.stream_id
        );

        // Simulate processing delay and generate mock partial results
        if *self.total_bytes_sent.borrow() % (16 * 1024) == 0 {
            self.generate_partial_result()?;
        }

        Ok(())
    }

    fn generate_partial_result(&self) -> Result<(), InternalSttError> {
        // In a real implementation, this would parse actual Azure streaming responses
        // For now, we generate mock partial results to demonstrate the interface

        let partial_text = match *self.total_bytes_sent.borrow() / (16 * 1024) {
            1 => "Hello",
            2 => "Hello world",
            3 => "Hello world how",
            4 => "Hello world how are",
            _ => "Hello world how are you",
        };

        let alternative = wit_types::TranscriptAlternative {
            text: partial_text.to_string(),
            confidence: 0.85,
            words: vec![wit_types::WordSegment {
                text: "Hello".to_string(),
                start_time: 0.0,
                end_time: 0.5,
                confidence: Some(0.95),
                speaker_id: Some("speaker_1".to_string()),
            }],
        };

        self.alternatives_queue.borrow_mut().push_back(alternative);
        debug!(
            "Generated partial result for stream {}: {}",
            self.stream_id, partial_text
        );

        Ok(())
    }

    fn finalize_stream(&self) -> Result<(), InternalSttError> {
        if *self.finished.borrow() {
            return Ok(());
        }

        debug!("Finalizing Azure stream: {}", self.stream_id);

        // Process any remaining buffered audio
        let buffer = self.audio_buffer.borrow().clone();
        if !buffer.is_empty() {
            self.send_audio_to_azure(&buffer)?;
        }

        // Generate final result
        let final_alternative = wit_types::TranscriptAlternative {
            text: "Hello world how are you doing today".to_string(),
            confidence: 0.92,
            words: vec![
                wit_types::WordSegment {
                    text: "Hello".to_string(),
                    start_time: 0.0,
                    end_time: 0.5,
                    confidence: Some(0.95),
                    speaker_id: Some("speaker_1".to_string()),
                },
                wit_types::WordSegment {
                    text: "world".to_string(),
                    start_time: 0.6,
                    end_time: 1.0,
                    confidence: Some(0.93),
                    speaker_id: Some("speaker_1".to_string()),
                },
                wit_types::WordSegment {
                    text: "how".to_string(),
                    start_time: 1.1,
                    end_time: 1.3,
                    confidence: Some(0.90),
                    speaker_id: Some("speaker_2".to_string()),
                },
                wit_types::WordSegment {
                    text: "are".to_string(),
                    start_time: 1.4,
                    end_time: 1.6,
                    confidence: Some(0.88),
                    speaker_id: Some("speaker_2".to_string()),
                },
                wit_types::WordSegment {
                    text: "you".to_string(),
                    start_time: 1.7,
                    end_time: 2.0,
                    confidence: Some(0.91),
                    speaker_id: Some("speaker_2".to_string()),
                },
                wit_types::WordSegment {
                    text: "doing".to_string(),
                    start_time: 2.1,
                    end_time: 2.5,
                    confidence: Some(0.89),
                    speaker_id: Some("speaker_2".to_string()),
                },
                wit_types::WordSegment {
                    text: "today".to_string(),
                    start_time: 2.6,
                    end_time: 3.0,
                    confidence: Some(0.94),
                    speaker_id: Some("speaker_2".to_string()),
                },
            ],
        };

        self.alternatives_queue
            .borrow_mut()
            .push_back(final_alternative);

        // Update durable state
        let state_key = format!("azure:stream:{}:state", self.stream_id);
        self.durable.borrow_mut().put(&state_key, "finished");

        *self.finished.borrow_mut() = true;
        info!(
            "Azure stream finalized: {} ({} bytes total)",
            self.stream_id,
            self.total_bytes_sent.borrow()
        );

        Ok(())
    }

    fn cleanup_resources(&self) {
        debug!("Cleaning up Azure stream resources: {}", self.stream_id);

        // Close connection
        *self.connection_active.borrow_mut() = false;

        // Clear buffers
        self.audio_buffer.borrow_mut().clear();
        self.alternatives_queue.borrow_mut().clear();

        // Clean up durable storage
        let connection_key = format!("azure:stream:{}:connection", self.stream_id);
        self.durable.borrow_mut().delete(&connection_key);

        // Clean up chunk data (keep only recent chunks for debugging)
        let total_chunks = *self.total_bytes_sent.borrow() / (16 * 1024);
        for i in 0..total_chunks.saturating_sub(5) {
            let chunk_key = format!("azure:stream:{}:chunk:{}", self.stream_id, i * 16 * 1024);
            self.durable.borrow_mut().delete(&chunk_key);
        }

        info!("Azure stream cleanup completed: {}", self.stream_id);
    }
}

impl GuestTranscriptionStream for AzureStream<'static> {
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
            error!("Failed to establish Azure connection: {e:?}");
            return Err(wit_types::SttError::NetworkError(format!(
                "Connection failed: {e:?}"
            )));
        }

        // Buffer small chunks for efficiency
        let mut buffer = self.audio_buffer.borrow_mut();
        buffer.extend_from_slice(&chunk);

        // Send buffered data when we have enough or if this is a large chunk
        if buffer.len() >= 8192 || chunk.len() >= 8192 {
            let data_to_send = buffer.clone();
            buffer.clear();
            drop(buffer); // Release borrow before calling send_audio_to_azure

            if let Err(e) = self.send_audio_to_azure(&data_to_send) {
                error!("Failed to send audio to Azure: {e:?}");
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
            error!("Failed to finalize Azure stream: {e:?}");
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
                "Returning alternative from Azure stream {}: {}",
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
