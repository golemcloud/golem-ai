use golem_stt::golem::stt::transcription::{AudioConfig, TranscribeOptions, TranscriptAlternative};
use golem_stt::golem::stt::types::{SttError, WordSegment};
use serde::Deserialize;

fn map_audio_format_to_mime(format: golem_stt::golem::stt::types::AudioFormat) -> &'static str {
    use golem_stt::golem::stt::types::AudioFormat as F;
    match format {
        F::Wav => "audio/wav",
        F::Pcm => "audio/L16",
        F::Mp3 => "audio/mpeg",
        F::Flac => "audio/flac",
        F::Ogg => "audio/ogg",
        F::Aac => "audio/aac",
    }
}

#[allow(dead_code)]
fn should_retry_status(status: u16) -> bool { status == 429 || status == 500 || status == 502 || status == 503 }

#[derive(Deserialize)]
struct DgResponse { results: Option<DgResults> }

#[derive(Deserialize)]
struct DgResults { channels: Option<Vec<DgChannel>> }

#[derive(Deserialize)]
struct DgChannel { alternatives: Option<Vec<DgAlternative>> }

#[derive(Deserialize)]
struct DgAlternative { transcript: Option<String>, confidence: Option<f32>, words: Option<Vec<DgWord>> }

#[derive(Deserialize)]
struct DgWord { word: Option<String>, start: Option<f32>, end: Option<f32>, confidence: Option<f32>, speaker: Option<String> }

pub(crate) struct RecognizeOut { pub alternatives: Vec<TranscriptAlternative>, pub request_id: Option<String> }

pub(crate) fn recognize(
    audio: &[u8],
    cfg: &crate::config::DeepgramConfig,
    conf: &AudioConfig,
    opts: &Option<TranscribeOptions>,
) -> Result<RecognizeOut, SttError> {
    use reqwest::Client;
    let mut url = cfg.endpoint.clone();
    if !url.ends_with("/listen") { if url.ends_with('/') { url.push_str("listen"); } else { url.push_str("/listen"); } }

    let language = opts.as_ref().and_then(|o| o.language.clone()).unwrap_or_else(|| "en-US".to_string());
    let model = opts.as_ref().and_then(|o| o.model.clone()).or_else(|| cfg.default_model.clone());
    let diarize = opts.as_ref().and_then(|o| o.enable_speaker_diarization).unwrap_or(false);
    let profanity = opts.as_ref().and_then(|o| o.profanity_filter);
    let keywords = opts.as_ref().and_then(|o| o.speech_context.clone()).unwrap_or_default();

    let mut qp: Vec<(String, String)> = Vec::new();
    qp.push(("language".into(), language));
    if let Some(m) = model { qp.push(("model".into(), m)); }
    if diarize { qp.push(("diarize".into(), "true".into())); }
    if let Some(p) = profanity { qp.push(("profanity_filter".into(), if p { "true" } else { "false" }.into())); }
    qp.push(("punctuate".into(), "true".into()));
    qp.push(("numerals".into(), "true".into()));
    if !keywords.is_empty() { qp.push(("keywords".into(), keywords.join(","))); }

    let client = Client::builder().timeout(std::time::Duration::from_secs(cfg.timeout_secs)).build().map_err(|e| SttError::InternalError(format!("client build {e}")))?;

    let mut full_url = url;
    if !qp.is_empty() {
        let tail = qp.into_iter().map(|(k,v)| format!("{}={}", urlencoding::encode(&k), urlencoding::encode(&v))).collect::<Vec<_>>().join("&");
        full_url.push_str("?");
        full_url.push_str(&tail);
    }

    let mut attempt: u32 = 0;
    let max_attempts = cfg.max_retries.max(1);
    let content_type = map_audio_format_to_mime(conf.format);
    let resp = loop {
        let r = client.post(&full_url)
            .header("Authorization", format!("Token {}", cfg.api_key))
            .header("Accept", "application/json")
            .header("Content-Type", content_type)
            .body(audio.to_vec())
            .send();
        match r {
            Ok(resp) => {
                let status = resp.status().as_u16();
                if status >= 200 && status < 300 { break resp; }
                if attempt + 1 < max_attempts && should_retry_status(status) { attempt += 1; continue; }
                let text = resp.text().unwrap_or_default();
                return Err(crate::error::map_http_status(status, text));
            }
            Err(e) => {
                if attempt + 1 < max_attempts { attempt += 1; continue; }
                return Err(SttError::NetworkError(format!("{e}")));
            }
        }
    };

    let request_id = resp.headers().get("dg-request-id").and_then(|v| v.to_str().ok()).map(|s| s.to_string());
    let body = resp.text().map_err(|e| SttError::InternalError(format!("read body {e}")))?;
    let parsed: DgResponse = serde_json::from_str(&body).map_err(|e| SttError::InternalError(format!("parse body {e}")))?;

    let mut alts_out: Vec<TranscriptAlternative> = Vec::new();
    if let Some(results) = parsed.results {
        if let Some(channels) = results.channels {
            for ch in channels.into_iter() {
                if let Some(alts) = ch.alternatives {
                    for a in alts.into_iter() {
                        let text = a.transcript.unwrap_or_default();
                        let conf = a.confidence.unwrap_or(0.0);
                        let mut words_out: Vec<WordSegment> = Vec::new();
                        if let Some(words) = a.words {
                            for w in words.into_iter() {
                                let start = w.start.unwrap_or(0.0);
                                let end = w.end.unwrap_or(start);
                                let seg = WordSegment { text: w.word.unwrap_or_default(), start_time: start, end_time: end, confidence: w.confidence, speaker_id: w.speaker };
                                words_out.push(seg);
                            }
                        }
                        alts_out.push(TranscriptAlternative { text, confidence: conf, words: words_out });
                    }
                }
            }
        }
    }

    if alts_out.is_empty() { return Err(SttError::TranscriptionFailed("empty result".into())); }
    Ok(RecognizeOut { alternatives: alts_out, request_id })
}
