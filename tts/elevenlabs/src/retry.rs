use std::time::{Duration, SystemTime};
use reqwest::{Client, StatusCode};
use reqwest::header::RETRY_AFTER;

/// Map HTTP status + body into a compact provider error message.
pub fn map_http_error(status: StatusCode, body_snippet: &str) -> String {
    match status {
        StatusCode::UNAUTHORIZED => "unauthorized: check ELEVENLABS_API_KEY".to_string(),
        StatusCode::FORBIDDEN => "forbidden/missing permissions (e.g., voices_read)".to_string(),
        StatusCode::NOT_FOUND => "voice not found (404)".to_string(),
        StatusCode::TOO_MANY_REQUESTS => "rate_limited (429)".to_string(),
        s if s.is_server_error() => format!("server_error ({}): {}", s.as_u16(), body_snippet),
        s => format!("http_error ({}): {}", s.as_u16(), body_snippet),
    }
}

/// Parse Retry-After header in either seconds or HTTP-date form.
/// If absent/invalid, fall back to exponential backoff: 1s, 2s, 4s (cap 5s).
fn compute_backoff(h: Option<&str>, attempt: u32) -> Duration {
    if let Some(v) = h {
        if let Ok(secs) = v.trim().parse::<u64>() {
            return Duration::from_secs(secs.min(10));
        }
        if let Ok(dt) = httpdate::parse_http_date(v) {
            let now = SystemTime::now();
            if let Ok(dur) = dt.duration_since(now) {
                return dur.min(Duration::from_secs(10));
            }
        }
    }
    Duration::from_secs((1u64 << attempt).min(5)) // 1,2,4,5…
}

/// POST bytes with backoff on 429. Returns Ok(body bytes) or Err(message).
pub async fn post_with_retry(
    client: &Client,
    url: &str,
    headers: &[(&str, &str)],
    body: Vec<u8>,
) -> Result<Vec<u8>, String> {
    let mut last_err = String::new();
    for attempt in 0..3 {
        let mut req = client.post(url).body(body.clone());
        for (k, v) in headers {
            req = req.header(*k, *v);
        }

        match req.send().await {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() {
                    return resp.bytes().await
                        .map(|b| b.to_vec())
                        .map_err(|e| format!("read_body_failed: {e}"));
                }
                // Non-2xx
                let retry_after = resp.headers().get(RETRY_AFTER)
                    .and_then(|v| v.to_str().ok());
                let snippet = resp.text().await.unwrap_or_default();
                if status == StatusCode::TOO_MANY_REQUESTS && attempt < 2 {
                    std::thread::sleep(compute_backoff(retry_after, attempt));
                    continue;
                }
                return Err(map_http_error(status, snippet.get(0..160).unwrap_or("")));
            }
            Err(e) => { last_err = format!("request_failed: {e}"); }
        }
    }
    Err(last_err)
}
