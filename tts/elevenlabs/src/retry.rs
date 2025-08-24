use httpdate::parse_http_date;
use reqwest::{header::RETRY_AFTER, Client, RequestBuilder, Response, StatusCode};
use std::time::{Duration, SystemTime};

/// Execute a POST with bounded exponential backoff, honoring `Retry-After` on 429/503.
/// NOTE: this is synchronous (`.send()`), matching the reqwest fork used under WASI.
pub fn post_with_retry(client: &Client, rb: RequestBuilder) -> Result<Response, reqwest::Error> {
    let base_req = rb.build()?; // build once
    let max_tries = 5;
    let mut backoff = Duration::from_millis(250);
    let max_backoff = Duration::from_secs(5);

    for attempt in 1..=max_tries {
        // Try to clone the built Request for this attempt; if we can't, execute once and return.
        let to_send = match base_req.try_clone() {
            Some(r) => r,
            None => {
                // Not clonable — do a single attempt without retries.
                return client.execute(base_req);
            }
        };

        match client.execute(to_send) {
            Ok(resp) => {
                let status = resp.status();
                if (status == StatusCode::TOO_MANY_REQUESTS || status == StatusCode::SERVICE_UNAVAILABLE)
                    && attempt < max_tries
                {
                    // Respect Retry-After seconds or HTTP-date per RFC 9110 §10.2.3
                    let wait = retry_after_delay(&resp).unwrap_or(backoff);
                    std::thread::sleep(wait);
                    backoff = (backoff * 2).min(max_backoff);
                    continue;
                }
                return Ok(resp);
            }
            Err(err) => {
                if attempt >= max_tries {
                    return Err(err);
                }
                std::thread::sleep(backoff);
                backoff = (backoff * 2).min(max_backoff);
                continue;
            }
        }
    }

    unreachable!("loop exits via return");
}

fn retry_after_delay(resp: &Response) -> Option<Duration> {
    let val = resp.headers().get(RETRY_AFTER)?;
    let s = val.to_str().ok()?;

    // Retry-After can be delta-seconds or an HTTP-date
    if let Ok(secs) = s.parse::<u64>() {
        return Some(Duration::from_secs(secs));
    }
    if let Ok(when) = parse_http_date(s) {
        if let Ok(diff) = when.duration_since(SystemTime::now()) {
            return Some(diff);
        }
    }
    None
}
