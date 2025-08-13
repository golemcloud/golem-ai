use golem_stt::golem::stt::transcription::{AudioConfig, TranscribeOptions, TranscriptAlternative};
use golem_stt::golem::stt::types::{SttError, WordSegment};
use serde::Deserialize;

#[derive(Deserialize)]
struct WhisperVerboseWord { word: Option<String>, start: Option<f32>, end: Option<f32> }

#[derive(Deserialize)]
struct WhisperVerbose { text: Option<String>, words: Option<Vec<WhisperVerboseWord>> }

pub(crate) struct RecognizeOut { pub alternatives: Vec<TranscriptAlternative>, pub request_id: Option<String> }

fn map_audio_format_to_mime(format: golem_stt::golem::stt::types::AudioFormat) -> &'static str {
    use golem_stt::golem::stt::types::AudioFormat as F;
    match format { F::Wav | F::Pcm => "audio/wav", F::Mp3 => "audio/mpeg", F::Flac => "audio/flac", F::Ogg => "audio/ogg", F::Aac => "audio/aac" }
}

pub(crate) fn recognize(
    audio: &[u8],
    cfg: &crate::config::WhisperConfig,
    conf: &AudioConfig,
    opts: &Option<TranscribeOptions>,
) -> Result<RecognizeOut, SttError> {
    use reqwest::Client;
    let mut url = cfg.endpoint.clone();
    if !url.ends_with("/audio/transcriptions") { if url.ends_with('/') { url.push_str("audio/transcriptions"); } else { url.push_str("/audio/transcriptions"); } }

    let model = opts.as_ref().and_then(|o| o.model.clone()).or_else(|| cfg.default_model.clone()).unwrap_or_else(|| "whisper-1".to_string());
    let language = opts.as_ref().and_then(|o| o.language.clone());
    let prompt = opts.as_ref().and_then(|o| o.speech_context.clone()).map(|v| v.join(" "));

    let boundary = "XBOUNDARY7b1c19a0";
    let mut body: Vec<u8> = Vec::with_capacity(audio.len() + 1024);
    fn push_text_part(buf: &mut Vec<u8>, boundary: &str, name: &str, value: &str) {
        buf.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
        buf.extend_from_slice(format!("Content-Disposition: form-data; name=\"{}\"\r\n\r\n", name).as_bytes());
        buf.extend_from_slice(value.as_bytes());
        buf.extend_from_slice(b"\r\n");
    }
    push_text_part(&mut body, boundary, "model", &model);
    push_text_part(&mut body, boundary, "response_format", "verbose_json");
    if let Some(lang) = &language { push_text_part(&mut body, boundary, "language", lang); }
    if let Some(p) = &prompt { if !p.is_empty() { push_text_part(&mut body, boundary, "prompt", p); } }
    body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body.extend_from_slice(b"Content-Disposition: form-data; name=\"file\"; filename=\"audio\"\r\n");
    body.extend_from_slice(format!("Content-Type: {}\r\n\r\n", map_audio_format_to_mime(conf.format)).as_bytes());
    body.extend_from_slice(audio);
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());

    let client = Client::builder().timeout(std::time::Duration::from_secs(cfg.timeout_secs)).build().map_err(|e| SttError::InternalError(format!("client build {e}")))?;
    let mut attempt: u32 = 0;
    let max_attempts = cfg.max_retries.max(1);
    let resp = loop {
        let r = client.post(&url)
            .bearer_auth(&cfg.api_key)
            .header("Content-Type", format!("multipart/form-data; boundary={}", boundary))
            .body(body.clone())
            .send();
        match r {
            Ok(resp) => {
                let status = resp.status().as_u16();
                if (200..300).contains(&status) { break resp; }
                if attempt + 1 < max_attempts && (status == 429 || status == 500 || status == 502 || status == 503) { attempt += 1; continue; }
                let text = resp.text().unwrap_or_default();
                return Err(crate::error::map_http_status(status, text));
            }
            Err(e) => {
                if attempt + 1 < max_attempts { attempt += 1; continue; }
                return Err(SttError::NetworkError(format!("{e}")));
            }
        }
    };

    let request_id = resp.headers().get("x-request-id").and_then(|v| v.to_str().ok()).map(|s| s.to_string());
    let body = resp.text().map_err(|e| SttError::InternalError(format!("read body {e}")))?;
    let parsed: WhisperVerbose = serde_json::from_str(&body).map_err(|e| SttError::InternalError(format!("parse body {e}")))?;

    let mut alts_out: Vec<TranscriptAlternative> = Vec::new();
    let mut words_out: Vec<WordSegment> = Vec::new();
    if let Some(ws) = parsed.words { for w in ws.into_iter() { let start = w.start.unwrap_or(0.0); let end = w.end.unwrap_or(start); words_out.push(WordSegment { text: w.word.unwrap_or_default(), start_time: start, end_time: end, confidence: None, speaker_id: None }); } }
    let text = parsed.text.unwrap_or_default();
    alts_out.push(TranscriptAlternative { text, confidence: 0.0, words: words_out });
    Ok(RecognizeOut { alternatives: alts_out, request_id })
}
