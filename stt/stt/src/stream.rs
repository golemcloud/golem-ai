use crate::config::CommonConfig;
use crate::durability::DurableStore;
use crate::errors::InternalSttError;
use crate::http::HttpClient;
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use std::time::UNIX_EPOCH;

/// Provider-agnostic emulated streaming over HTTP:
/// - send: POST {base}/stream/send  with {request_id, chunk_b64}
/// - finish: POST {base}/stream/finish with {request_id}
/// - recv: GET  {base}/stream/recv?request_id=...  -> JSON alternative or null
///
/// Each provider crate should expose base endpoints and auth headers builders.
/// This module handles durability and request orchestration.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmulatedStreamConfig {
    pub base_endpoint: String,
    pub content_type: String,
    pub auth_headers: Vec<(String, String)>,
    pub poll_interval_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamState {
    pub request_id: String,
    pub finished: bool,
}

pub struct EmulatedTranscriptionStream<'a> {
    http: HttpClient,
    durable: &'a mut DurableStore,
    cfg: EmulatedStreamConfig,
    state_key: String,
    pub state: StreamState,
}

#[derive(Debug, Serialize)]
struct SendBody {
    request_id: String,
    chunk_b64: String,
}

#[derive(Debug, Serialize)]
struct FinishBody {
    request_id: String,
}

#[derive(Debug, Deserialize)]
pub struct AltWord {
    pub text: String,
    pub start_time: Option<f32>,
    pub end_time: Option<f32>,
    pub confidence: Option<f32>,
    pub speaker_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Alternative {
    pub text: String,
    pub confidence: Option<f32>,
    pub words: Option<Vec<AltWord>>,
}

/// Provider response used by /stream/recv
#[derive(Debug, Deserialize)]
pub struct RecvResponse {
    pub alternative: Option<Alternative>,
}

impl<'a> EmulatedTranscriptionStream<'a> {
    pub fn new(
        common: &CommonConfig,
        cfg: EmulatedStreamConfig,
        durable: &'a mut DurableStore,
    ) -> Result<Self, InternalSttError> {
        let http = HttpClient::new(common.timeout_secs, common.max_retries)?;
        let request_id = Self::generate_request_id();
        let state = StreamState {
            request_id: request_id.clone(),
            finished: false,
        };
        let state_key = format!("stt:stream:{request_id}");
        durable.put_json(&state_key, &state);
        Ok(Self {
            http,
            durable,
            cfg,
            state_key,
            state,
        })
    }

    pub fn load(
        common: &CommonConfig,
        cfg: EmulatedStreamConfig,
        request_id: String,
        durable: &'a mut DurableStore,
    ) -> Result<Self, InternalSttError> {
        let http = HttpClient::new(common.timeout_secs, common.max_retries)?;
        let state_key = format!("stt:stream:{request_id}");
        let state: StreamState = durable
            .get_json(&state_key)
            .ok_or_else(|| InternalSttError::internal("stream state not found"))?;
        Ok(Self {
            http,
            durable,
            cfg,
            state_key,
            state,
        })
    }

    fn generate_request_id() -> String {
        // Not cryptographically secure; sufficient for correlation ID.
        let ts = std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_else(|_| std::time::Duration::from_secs(0))
            .as_millis();
        format!("stt-req-{ts}")
    }

    fn headers(&self) -> Result<reqwest::header::HeaderMap, InternalSttError> {
        use reqwest::header::{HeaderMap, HeaderName, HeaderValue, CONTENT_TYPE};
        let mut map = HeaderMap::new();
        map.insert(
            CONTENT_TYPE,
            HeaderValue::from_str(&self.cfg.content_type)
                .map_err(|e| InternalSttError::internal(format!("content-type: {e}")))?,
        );
        for (k, v) in &self.cfg.auth_headers {
            // Build owned header name and value to avoid any lifetime tied to &self
            let name = HeaderName::from_bytes(k.as_bytes())
                .map_err(|e| InternalSttError::internal(format!("header name {k}: {e}")))?;
            let value = HeaderValue::from_str(v)
                .map_err(|e| InternalSttError::internal(format!("header {k}: {e}")))?;
            map.insert(name, value);
        }
        Ok(map)
    }

    pub async fn send_audio(&mut self, chunk: Vec<u8>) -> Result<(), InternalSttError> {
        if self.state.finished {
            return Err(InternalSttError::failed("stream already finished"));
        }
        let url = format!(
            "{}/stream/send",
            self.cfg.base_endpoint.trim_end_matches('/')
        );
        let headers = self.headers()?;
        let body = serde_json::to_vec(&SendBody {
            request_id: self.state.request_id.clone(),
            chunk_b64: base64::engine::general_purpose::STANDARD.encode(chunk),
        })
        .map_err(|e| InternalSttError::internal(format!("send serialize: {e}")))?;
        let (status, text, _hdrs) = self
            .http
            .post_bytes(&url, headers, body, "application/json")
            .await?;
        if !status.is_success() {
            return Err(InternalSttError::failed(format!(
                "stream/send failed: {status} {text}"
            )));
        }
        Ok(())
    }

    pub async fn finish(&mut self) -> Result<(), InternalSttError> {
        if self.state.finished {
            return Ok(());
        }
        let url = format!(
            "{}/stream/finish",
            self.cfg.base_endpoint.trim_end_matches('/')
        );
        let headers = self.headers()?;
        let body = serde_json::to_vec(&FinishBody {
            request_id: self.state.request_id.clone(),
        })
        .map_err(|e| InternalSttError::internal(format!("finish serialize: {e}")))?;
        let (status, text, _hdrs) = self
            .http
            .post_bytes(&url, headers, body, "application/json")
            .await?;
        if !status.is_success() {
            return Err(InternalSttError::failed(format!(
                "stream/finish failed: {status} {text}"
            )));
        }
        self.state.finished = true;
        self.durable.put_json(&self.state_key, &self.state);
        Ok(())
    }

    pub async fn receive_alternative(&mut self) -> Result<Option<Alternative>, InternalSttError> {
        // Long-poll
        let url = format!(
            "{}/stream/recv?request_id={}",
            self.cfg.base_endpoint.trim_end_matches('/'),
            urlencoding::encode(&self.state.request_id)
        );
        let headers = self.headers()?;
        let response = self.http.get(&url, headers).await?;
        let status = response.0;
        let text = response.1;
        let _hdrs = response.2;
        if status.as_u16() == 204 {
            return Ok(None);
        }
        if !status.is_success() {
            return Err(InternalSttError::failed(format!(
                "stream/recv failed: {status} {text}"
            )));
        }
        let parsed: RecvResponse = serde_json::from_str(&text)
            .map_err(|e| InternalSttError::internal(format!("recv parse: {e}, body: {text}")))?;
        Ok(parsed.alternative)
    }

    pub fn request_id(&self) -> &str {
        &self.state.request_id
    }

    pub fn is_finished(&self) -> bool {
        self.state.finished
    }

    pub fn poll_interval(&self) -> Duration {
        Duration::from_millis(self.cfg.poll_interval_ms.max(100))
    }
}
