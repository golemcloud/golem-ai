use crate::errors::InternalSttError;
use log::{debug, trace};
use reqwest::header::HeaderMap;
use reqwest::StatusCode;
use std::time::Duration;

#[derive(Clone)]
pub struct HttpClient {
    #[allow(dead_code)]
    client: reqwest::Client,
    #[allow(dead_code)]
    timeout: Duration,
    max_retries: u32,
}

impl HttpClient {
    pub async fn get(
        &self,
        url: &str,
        headers: HeaderMap,
    ) -> Result<(StatusCode, String, HeaderMap), InternalSttError> {
        // Build the future without capturing `&self` inside the async block to satisfy borrow checker,
        // but do not wrap a ready Result in `.await`.
        self.retrying(|attempt| {
            let url = url.to_string();
            let headers = headers.clone();
            let client = self.client.clone();
            async move {
                trace!("GET {url} attempt {attempt}");
                let resp = client
                    .get(&url)
                    .headers(headers.clone())
                    .send()
                    .map_err(|e| InternalSttError::network(format!("network send error: {e}")))?;
                let status = resp.status();
                let headers = resp.headers().clone();
                let text = resp
                    .text()
                    .map_err(|e| InternalSttError::network(format!("read body error: {e}")))?;
                Ok((status, text, headers))
            }
        })
        .await
    }
    pub fn new(timeout_secs: u64, max_retries: u32) -> Result<Self, InternalSttError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .map_err(|e| InternalSttError::internal(format!("failed to build http client: {e}")))?;
        Ok(Self {
            client,
            timeout: Duration::from_secs(timeout_secs),
            max_retries,
        })
    }

    pub async fn post_bytes(
        &self,
        url: &str,
        headers: HeaderMap,
        body: Vec<u8>,
        content_type: &str,
    ) -> Result<(StatusCode, String, HeaderMap), InternalSttError> {
        let url = url.to_string();
        let content_type = content_type.to_string();
        // Use shared Bytes to avoid reallocating/copying the whole body on each retry
        let body_bytes = bytes::Bytes::from(body);

        self.retrying(|attempt| {
            let url = url.clone();
            let headers = headers.clone();
            let content_type = content_type.clone();
            let client = self.client.clone();
            let body_bytes = body_bytes.clone();
            async move {
                let req = client
                    .post(&url)
                    .headers(headers)
                    .header("Content-Type", content_type)
                    .body(body_bytes);
                trace!("POST {url} attempt {attempt}");
                let resp = req
                    .send()
                    .map_err(|e| InternalSttError::network(format!("network send error: {e}")))?;
                let status = resp.status();
                let headers = resp.headers().clone();
                let text = resp
                    .text()
                    .map_err(|e| InternalSttError::network(format!("read body error: {e}")))?;
                Ok((status, text, headers))
            }
        })
        .await
    }

    async fn retrying<F, Fut, T>(&self, mut f: F) -> Result<T, InternalSttError>
    where
        F: FnMut(u32) -> Fut,
        Fut: std::future::Future<Output = Result<T, InternalSttError>>,
    {
        let mut attempt = 0;
        loop {
            attempt += 1;
            match f(attempt).await {
                Ok(v) => return Ok(v),
                Err(e) => {
                    if attempt > self.max_retries || !is_retryable(&e) {
                        return Err(e);
                    }
                    let backoff = backoff_delay(attempt);
                    debug!("retryable error on attempt {attempt}: {e:?}, backing off {backoff:?}");
                    // Use proper async sleep in WASI via wstd
                    wstd::task::sleep(backoff).await;
                }
            }
        }
    }

    pub async fn put_bytes(
        &self,
        url: &str,
        headers: HeaderMap,
        body: Vec<u8>,
        content_type: &str,
    ) -> Result<(StatusCode, String, HeaderMap), InternalSttError> {
        let url = url.to_string();
        let content_type = content_type.to_string();
        let body_bytes = bytes::Bytes::from(body);

        self.retrying(|attempt| {
            let url = url.clone();
            let headers = headers.clone();
            let content_type = content_type.clone();
            let client = self.client.clone();
            let body_bytes = body_bytes.clone();
            async move {
                trace!("PUT {url} attempt {attempt}");
                let resp = client
                    .put(&url)
                    .headers(headers.clone())
                    .header("Content-Type", content_type)
                    .body(body_bytes)
                    .send()
                    .map_err(|e| InternalSttError::network(format!("network send error: {e}")))?;
                let status = resp.status();
                let headers = resp.headers().clone();
                let text = resp
                    .text()
                    .map_err(|e| InternalSttError::network(format!("read body error: {e}")))?;
                Ok((status, text, headers))
            }
        })
        .await
    }
}

fn is_retryable(e: &InternalSttError) -> bool {
    matches!(
        e,
        InternalSttError::NetworkError(_)
            | InternalSttError::ServiceUnavailable(_)
            | InternalSttError::InternalError(_)
            | InternalSttError::RateLimited(_)
    )
}

fn backoff_delay(attempt: u32) -> Duration {
    // Exponential backoff with deterministic jitter
    let base = 100u64; // ms
    let exp = base.saturating_mul(1u64 << (attempt.min(6).saturating_sub(1)));
    // xorshift-like simple hash for jitter
    let mut x = exp.wrapping_mul(0x9E3779B97F4A7C15);
    x ^= x >> 33;
    x = x.wrapping_mul(0xC2B2AE3D27D4EB4F);
    let jitter = x % 100;
    Duration::from_millis(exp + jitter)
}
