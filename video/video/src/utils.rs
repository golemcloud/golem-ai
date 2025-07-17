use crate::error::internal_error;
use crate::exports::golem::video_generation::types::{RawBytes, VideoError};
use mime_guess::from_path;

/// Downloads an image from a URL and returns the bytes with mime type
pub fn download_image_from_url(url: &str) -> Result<RawBytes, VideoError> {
    use reqwest::Client;

    let client = Client::builder()
        .build()
        .map_err(|err| internal_error(format!("Failed to create HTTP client: {err}")))?;

    let response = client
        .get(url)
        .send()
        .map_err(|err| internal_error(format!("Failed to download image from {url}: {err}")))?;

    if !response.status().is_success() {
        return Err(internal_error(format!(
            "Failed to download image from {}: HTTP {}",
            url,
            response.status()
        )));
    }

    // Get the mime type from the response headers or guess from URL
    let mime_type = response
        .headers()
        .get("content-type")
        .and_then(|ct| ct.to_str().ok())
        .map(|ct| ct.to_string())
        .unwrap_or_else(|| from_path(url).first_or_octet_stream().to_string());

    let bytes = response
        .bytes()
        .map_err(|err| internal_error(format!("Failed to read image data from {url}: {err}")))?;

    Ok(RawBytes {
        bytes: bytes.to_vec(),
        mime_type,
    })
}

/// Downloads a video from a URL and returns the bytes with mime type
pub fn download_video_from_url(url: &str) -> Result<RawBytes, VideoError> {
    use reqwest::Client;

    let client = Client::builder()
        .build()
        .map_err(|err| internal_error(format!("Failed to create HTTP client: {err}")))?;

    let response = client
        .get(url)
        .send()
        .map_err(|err| internal_error(format!("Failed to download video from {url}: {err}")))?;

    if !response.status().is_success() {
        return Err(internal_error(format!(
            "Failed to download video from {}: HTTP {}",
            url,
            response.status()
        )));
    }

    // Get the mime type from the response headers or guess from URL
    let mime_type = response
        .headers()
        .get("content-type")
        .and_then(|ct| ct.to_str().ok())
        .map(|ct| ct.to_string())
        .unwrap_or_else(|| from_path(url).first_or_octet_stream().to_string());

    let bytes = response
        .bytes()
        .map_err(|err| internal_error(format!("Failed to read video data from {url}: {err}")))?;

    Ok(RawBytes {
        bytes: bytes.to_vec(),
        mime_type,
    })
}
