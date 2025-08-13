use crate::config::AmazonConfig;
use crate::signer::{sigv4_headers, SigV4Params};
use golem_stt::golem::stt::transcription::{AudioConfig, TranscribeOptions, TranscriptionResult, TranscriptAlternative};
use golem_stt::golem::stt::types::WordSegment;
use golem_stt::golem::stt::types::SttError;
use serde::Deserialize;
#[cfg(feature = "durability")]
use golem_stt::durability::saga::{Saga, SttCheckpoint};
#[cfg(feature = "durability")]
use golem_stt::durability::durable_impl;

fn sigv4(
    cfg: &AmazonConfig,
    service: &str,
    method: &str,
    url: &str,
    target: &str,
    payload: &[u8],
) -> Result<(reqwest::Client, String, String, Option<(String, String)>, String), SttError> {
    let client = reqwest::Client::builder().timeout(std::time::Duration::from_secs(cfg.timeout_secs)).build().map_err(|e| SttError::InternalError(format!("client {e}")))?;
    let parsed = reqwest::Url::parse(url).map_err(|e| SttError::InternalError(format!("url {e}")))?;
    let host_hdr = parsed.host_str().ok_or_else(|| SttError::InternalError("no host".into()))?;
    let params = SigV4Params { access_key: cfg.access_key.clone(), secret_key: cfg.secret_key.clone(), session_token: cfg.session_token.clone(), region: cfg.region.clone(), service: service.into() };
    let (amz_date, authorization, security_header, content_type_hdr) = sigv4_headers(&params, method, host_hdr, "/", target, payload);
    Ok((client, amz_date, authorization, security_header, content_type_hdr))
}

fn create_vocabulary(cfg: &AmazonConfig, language: &str, name: &str, phrases: &[String]) -> Result<(), SttError> {
    let host = cfg.endpoint.clone().unwrap_or_else(|| format!("https://transcribe.{}.amazonaws.com", cfg.region));
    let url = format!("{}/", host.trim_end_matches('/'));
    let body = serde_json::json!({ "VocabularyName": name, "LanguageCode": language, "Phrases": phrases });
    let payload = serde_json::to_vec(&body).map_err(|e| SttError::InternalError(format!("json {e}")))?;
    let (client, amz_date, authorization, security_header, content_type_hdr) = sigv4(cfg, "transcribe", "POST", &url, "Transcribe.CreateVocabulary", &payload)?;
    let mut req = client.post(url).header("x-amz-date", amz_date).header("x-amz-target", "Transcribe.CreateVocabulary").header("authorization", authorization).header("content-type", content_type_hdr);
    if let Some((k, v)) = security_header { req = req.header(k, v); }
    let resp = req.body(payload).send().map_err(|e| SttError::NetworkError(format!("{e}")))?;
    if !resp.status().is_success() { return Err(crate::error::map_http_status(resp.status().as_u16())); }
    Ok(())
}

fn get_vocabulary_state(cfg: &AmazonConfig, name: &str) -> Result<String, SttError> {
    let host = cfg.endpoint.clone().unwrap_or_else(|| format!("https://transcribe.{}.amazonaws.com", cfg.region));
    let url = format!("{}/", host.trim_end_matches('/'));
    let body = serde_json::json!({ "VocabularyName": name });
    let payload = serde_json::to_vec(&body).map_err(|e| SttError::InternalError(format!("json {e}")))?;
    let (client, amz_date, authorization, security_header, content_type_hdr) = sigv4(cfg, "transcribe", "POST", &url, "Transcribe.GetVocabulary", &payload)?;
    let mut req = client.post(url).header("x-amz-date", amz_date).header("x-amz-target", "Transcribe.GetVocabulary").header("authorization", authorization).header("content-type", content_type_hdr);
    if let Some((k, v)) = security_header { req = req.header(k, v); }
    let resp = req.body(payload).send().map_err(|e| SttError::NetworkError(format!("{e}")))?;
    if !resp.status().is_success() { return Err(crate::error::map_http_status(resp.status().as_u16())); }
    let v: serde_json::Value = resp.json().map_err(|e| SttError::TranscriptionFailed(format!("resp {e}")))?;
    let state = v.get("VocabularyState").and_then(|s| s.as_str()).unwrap_or("").to_string();
    Ok(state)
}

fn wait_vocabulary_ready(cfg: &AmazonConfig, name: &str, max_attempts: u32) -> Result<(), SttError> {
    let mut attempts = 0u32;
    loop {
        let st = get_vocabulary_state(cfg, name)?;
        if st == "READY" { return Ok(()); }
        if st == "FAILED" { return Err(SttError::TranscriptionFailed("vocabulary failed".into())); }
        attempts += 1;
        if attempts > max_attempts { return Err(SttError::ServiceUnavailable("vocabulary not ready".into())); }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}

fn delete_vocabulary(cfg: &AmazonConfig, name: &str) -> Result<(), SttError> {
    let host = cfg.endpoint.clone().unwrap_or_else(|| format!("https://transcribe.{}.amazonaws.com", cfg.region));
    let url = format!("{}/", host.trim_end_matches('/'));
    let body = serde_json::json!({ "VocabularyName": name });
    let payload = serde_json::to_vec(&body).map_err(|e| SttError::InternalError(format!("json {e}")))?;
    let (client, amz_date, authorization, security_header, content_type_hdr) = sigv4(cfg, "transcribe", "POST", &url, "Transcribe.DeleteVocabulary", &payload)?;
    let mut req = client.post(url).header("x-amz-date", amz_date).header("x-amz-target", "Transcribe.DeleteVocabulary").header("authorization", authorization).header("content-type", content_type_hdr);
    if let Some((k, v)) = security_header { req = req.header(k, v); }
    let resp = req.body(payload).send().map_err(|e| SttError::NetworkError(format!("{e}")))?;
    if !resp.status().is_success() { return Err(crate::error::map_http_status(resp.status().as_u16())); }
    Ok(())
}
#[derive(Deserialize)]
struct TranscriptRoot {
    results: Option<Results>,
}

#[derive(Deserialize)]
struct Results { transcripts: Option<Vec<TranscriptText>>, items: Option<Vec<Item>> }

#[derive(Deserialize, Clone)]
struct TranscriptText { transcript: String }

#[derive(Deserialize)]
struct Item {
    #[serde(rename = "type")]
    item_type: String,
    start_time: Option<String>,
    end_time: Option<String>,
    alternatives: Vec<Alt>,
    speaker_label: Option<String>,
}

#[derive(Deserialize)]
struct Alt { content: String, confidence: Option<String> }


fn parse_f32(s: Option<&String>) -> Option<f32> {
    s.and_then(|v| v.parse::<f32>().ok())
}

pub fn transcribe_once(audio: Vec<u8>, cfg: &AmazonConfig, options: Option<TranscribeOptions>, config: AudioConfig) -> Result<TranscriptionResult, SttError> {
    #[cfg(feature = "durability")]
    let saga: Saga<TranscriptionResult, SttError> = Saga::new("golem_stt_amazon", "transcribe", golem_rust::bindings::golem::durability::durability::DurableFunctionType::WriteRemote);
    let bucket = cfg.s3_bucket.clone().ok_or_else(|| SttError::UnsupportedOperation("missing S3_BUCKET".into()))?;
    let ext = match config.format { golem_stt::golem::stt::types::AudioFormat::Wav => "wav", golem_stt::golem::stt::types::AudioFormat::Mp3 => "mp3", golem_stt::golem::stt::types::AudioFormat::Flac => "flac", golem_stt::golem::stt::types::AudioFormat::Ogg => "ogg", golem_stt::golem::stt::types::AudioFormat::Aac => "aac", golem_stt::golem::stt::types::AudioFormat::Pcm => "pcm" };
    let content_type = mime_guess::from_ext(ext).first_or_octet_stream().essence_str().to_string();
    let key = format!("stt/{}.{}", uuid::Uuid::new_v4(), ext);
    let host = format!("https://{}.s3.{}.amazonaws.com", bucket, cfg.region);
    let url = format!("{}/{}", host, key);
    let client = reqwest::Client::builder().timeout(std::time::Duration::from_secs(cfg.timeout_secs)).build().map_err(|e| SttError::InternalError(format!("client {e}")))?;
    let parsed = reqwest::Url::parse(&url).map_err(|e| SttError::InternalError(format!("url {e}")))?;
    let host_hdr = parsed.host_str().ok_or_else(|| SttError::InternalError("no host".into()))?;
    let params = SigV4Params { access_key: cfg.access_key.clone(), secret_key: cfg.secret_key.clone(), session_token: cfg.session_token.clone(), region: cfg.region.clone(), service: "s3".into() };
    let (amz_date, authorization, security_header, _ct) = sigv4_headers(&params, "PUT", host_hdr, &format!("/{}", key), "", &audio);
    let mut put = client.put(url.clone()).header("x-amz-date", amz_date).header("authorization", authorization).header("content-type", content_type.clone());
    if let Some((k, v)) = security_header { put = put.header(k, v); }
    let resp = put.body(audio.clone()).send().map_err(|e| SttError::NetworkError(format!("{e}")))?;
    let status = resp.status().as_u16();
    if !(200..300).contains(&status) { return Err(crate::error::map_http_status(status)); }

    let media_uri = url;
    #[cfg(feature = "durability")]
    saga.persist_checkpoint(SttCheckpoint { provider: "amazon".into(), state: "uploaded".into(), job_id: None, media_uri: Some(media_uri.clone()), audio_sha256: None, retry_count: 0, backoff_ms: 0, last_ts_ms: golem_rust::time::now_epoch_millis() });
    let transcribe_host = cfg.endpoint.clone().unwrap_or_else(|| format!("https://transcribe.{}.amazonaws.com", cfg.region));
    let transcribe_url = format!("{}/", transcribe_host.trim_end_matches('/'));
    let params = SigV4Params { access_key: cfg.access_key.clone(), secret_key: cfg.secret_key.clone(), session_token: cfg.session_token.clone(), region: cfg.region.clone(), service: "transcribe".into() };
    let job_name = uuid::Uuid::new_v4().to_string();
    let language = options.as_ref().and_then(|o| o.language.clone()).unwrap_or_else(|| "en-US".into());
    let show_speakers = options.as_ref().and_then(|o| o.enable_speaker_diarization).unwrap_or(false);
    let profanity = options.as_ref().and_then(|o| o.profanity_filter).unwrap_or(false);
    let phrase_hints: Vec<String> = options.as_ref().and_then(|o| o.speech_context.clone()).unwrap_or_default();
    let vocab_name_env = std::env::var("AWS_VOCABULARY_NAME").ok();
    let vocab_filter_name_env = std::env::var("AWS_VOCABULARY_FILTER_NAME").ok();
    let profanity_method_env = std::env::var("AWS_PROFANITY_METHOD").ok().unwrap_or_else(|| "mask".into());
    let model = options.as_ref().and_then(|o| o.model.clone());
    let mut body = serde_json::json!({
        "TranscriptionJobName": job_name,
        "LanguageCode": language,
        "MediaFormat": ext,
        "Media": { "MediaFileUri": media_uri },
        "Settings": { "ShowSpeakerLabels": show_speakers }
    });
    let mut temp_vocab: Option<String> = None;
    if let Some(vname) = vocab_name_env {
        body["Settings"]["VocabularyName"] = serde_json::json!(vname);
    } else if !phrase_hints.is_empty() {
        let temp = format!("golem-temp-{}", uuid::Uuid::new_v4());
        create_vocabulary(cfg, &language, &temp, &phrase_hints)?;
        wait_vocabulary_ready(cfg, &temp, cfg.max_retries)?;
        body["Settings"]["VocabularyName"] = serde_json::json!(temp.clone());
        temp_vocab = Some(temp);
    }
    if profanity {
        if let Some(vfname) = vocab_filter_name_env {
            body["Settings"]["VocabularyFilterName"] = serde_json::json!(vfname);
            body["Settings"]["VocabularyFilterMethod"] = serde_json::json!(profanity_method_env);
        }
    }
    if let Some(m) = &model { body["ModelSettings"] = serde_json::json!({ "LanguageModelName": m }); }
    let payload = serde_json::to_vec(&body).map_err(|e| SttError::InternalError(format!("json {e}")))?;
    let parsed = reqwest::Url::parse(&transcribe_url).map_err(|e| SttError::InternalError(format!("url {e}")))?;
    let host_hdr = parsed.host_str().ok_or_else(|| SttError::InternalError("no host".into()))?;
    let amz_target = "Transcribe.StartTranscriptionJob".to_string();
    let (amz_date, authorization, security_header, content_type_hdr) = sigv4_headers(&params, "POST", host_hdr, "/", &amz_target, &payload);
    let mut req = client.post(transcribe_url.clone()).header("x-amz-date", amz_date).header("x-amz-target", amz_target).header("authorization", authorization).header("content-type", content_type_hdr);
    if let Some((k, v)) = security_header { req = req.header(k, v); }
    let resp = req.body(payload).send().map_err(|e| SttError::NetworkError(format!("{e}")))?;
    let status = resp.status().as_u16();
    if !(200..300).contains(&status) { if let Some(name) = temp_vocab.as_ref() { let _ = delete_vocabulary(cfg, name); } return Err(crate::error::map_http_status(status)); }
    #[cfg(feature = "durability")]
    saga.persist_checkpoint(SttCheckpoint { provider: "amazon".into(), state: "started".into(), job_id: Some(job_name.clone()), media_uri: Some(media_uri.clone()), audio_sha256: None, retry_count: 0, backoff_ms: 0, last_ts_ms: golem_rust::time::now_epoch_millis() });

    let mut attempts = 0u32;
    let max = cfg.max_retries;
    let transcript_uri: String = loop {
        let get_body = serde_json::json!({ "TranscriptionJobName": job_name });
        let payload = serde_json::to_vec(&get_body).map_err(|e| SttError::InternalError(format!("json {e}")))?;
        let amz_target = "Transcribe.GetTranscriptionJob".to_string();
        let (amz_date, authorization, security_header, content_type_hdr) = sigv4_headers(&params, "POST", host_hdr, "/", &amz_target, &payload);
        let mut req = client.post(transcribe_url.clone()).header("x-amz-date", amz_date).header("x-amz-target", amz_target).header("authorization", authorization).header("content-type", content_type_hdr);
        if let Some((k, v)) = security_header { req = req.header(k, v); }
        let resp = req.body(payload).send().map_err(|e| SttError::NetworkError(format!("{e}")))?;
        let status = resp.status().as_u16();
        if !(200..300).contains(&status) { if let Some(name) = temp_vocab.as_ref() { let _ = delete_vocabulary(cfg, name); } return Err(crate::error::map_http_status(status)); }
        let v: serde_json::Value = resp.json().map_err(|e| SttError::TranscriptionFailed(format!("resp {e}")))?;
        if let Some(state) = v.get("TranscriptionJob").and_then(|j| j.get("TranscriptionJobStatus")).and_then(|s| s.as_str()) {
            if state == "COMPLETED" {
                if let Some(u) = v.get("TranscriptionJob").and_then(|j| j.get("Transcript")).and_then(|t| t.get("TranscriptFileUri")).and_then(|u| u.as_str()).map(|s| s.to_string()) {
                    #[cfg(feature = "durability")]
                    saga.persist_checkpoint(SttCheckpoint { provider: "amazon".into(), state: "completed".into(), job_id: Some(job_name.clone()), media_uri: Some(media_uri.clone()), audio_sha256: None, retry_count: 0, backoff_ms: 0, last_ts_ms: golem_rust::time::now_epoch_millis() });
                    break u;
                } else {
                    if let Some(name) = temp_vocab.as_ref() { let _ = delete_vocabulary(cfg, name); }
                    return Err(SttError::TranscriptionFailed("no transcript uri".into()));
                }
            } else if state == "FAILED" { if let Some(name) = temp_vocab.as_ref() { let _ = delete_vocabulary(cfg, name); } return Err(SttError::TranscriptionFailed("job failed".into())); }
        }
        attempts += 1;
        if attempts > max { if let Some(name) = temp_vocab.as_ref() { let _ = delete_vocabulary(cfg, name); } return Err(SttError::ServiceUnavailable("too many retries".into())); }
        std::thread::sleep(std::time::Duration::from_millis(500));
    };
    let text_resp = client.get(&transcript_uri).send().map_err(|e| SttError::NetworkError(format!("{e}")))?;
    let status = text_resp.status().as_u16();
    if !(200..300).contains(&status) { if let Some(name) = temp_vocab.as_ref() { let _ = delete_vocabulary(cfg, name); } return Err(crate::error::map_http_status(status)); }
    let body = text_resp.text().map_err(|e| SttError::NetworkError(format!("{e}")))?;
    let duration_seconds = if let (Some(sr), Some(ch)) = (config.sample_rate, config.channels) {
        let bytes_per_sample = 2u32;
        let samples = audio.len() as f32 / (bytes_per_sample as f32 * ch as f32);
        samples / sr as f32
    } else { 0.0 };
    let result = map_transcript(&body, language, model, audio.len(), job_name, duration_seconds);
    if let Some(name) = temp_vocab.as_ref() { let _ = delete_vocabulary(cfg, name); }
    #[cfg(feature = "durability")]
    {
        #[derive(golem_rust::FromValueAndType, golem_rust::IntoValue, Clone, Debug)]
        struct TranscribeInputMeta { provider: String, audio_size_bytes: u32 }
        let meta = TranscribeInputMeta { provider: "amazon".into(), audio_size_bytes: audio.len() as u32 };
        return durable_impl::persist_transcribe("golem_stt_amazon", meta, result);
    }
    #[cfg(not(feature = "durability"))]
    { result }
}

pub fn map_transcript(json: &str, language: String, model: Option<String>, audio_size: usize, request_id: String, duration_seconds: f32) -> Result<TranscriptionResult, SttError> {
    let root: TranscriptRoot = serde_json::from_str(json).map_err(|e| SttError::TranscriptionFailed(format!("parse transcript {e}")))?;
    let mut words: Vec<WordSegment> = Vec::new();
    if let Some(results) = root.results {
        if let Some(items) = results.items {
            for it in items.into_iter() {
                if it.item_type == "pronunciation" {
                    let text = it.alternatives.get(0).map(|a| a.content.clone()).unwrap_or_default();
                    let confidence = it.alternatives.get(0).and_then(|a| a.confidence.as_ref()).and_then(|c| c.parse::<f32>().ok());
                    let start = parse_f32(it.start_time.as_ref()).unwrap_or(0.0);
                    let end = parse_f32(it.end_time.as_ref()).unwrap_or(start);
                    let speaker = it.speaker_label.clone();
                    words.push(WordSegment { text, start_time: start, end_time: end, confidence, speaker_id: speaker });
                }
            }
        }
        let full_text = results.transcripts.and_then(|t| t.get(0).cloned()).map(|t| t.transcript).unwrap_or_default();
        let alt = TranscriptAlternative { text: full_text, confidence: 1.0, words };
        let meta = golem_stt::golem::stt::types::TranscriptionMetadata {
            duration_seconds,
            audio_size_bytes: audio_size as u32,
            request_id,
            model,
            language,
        };
        return Ok(TranscriptionResult { alternatives: vec![alt], metadata: meta });
    }
    Err(SttError::TranscriptionFailed("empty transcript".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_simple_transcript() {
        let sample = r#"{
            "results": {
                "transcripts": [{"transcript": "hello world"}],
                "items": [
                    {"type": "pronunciation", "start_time": "0.0", "end_time": "0.5", "alternatives": [{"content": "hello", "confidence": "0.9"}], "speaker_label": "spk_0"},
                    {"type": "pronunciation", "start_time": "0.5", "end_time": "1.0", "alternatives": [{"content": "world", "confidence": "0.8"}]}
                ]
            }
        }"#;
        let out = map_transcript(sample, "en-US".into(), None, 1000, "rid".into(), 1.0).unwrap();
        assert_eq!(out.alternatives.len(), 1);
        assert_eq!(out.alternatives[0].text, "hello world");
        assert_eq!(out.alternatives[0].words.len(), 2);
        assert_eq!(out.metadata.language, "en-US");
        assert_eq!(out.metadata.duration_seconds, 1.0);
    }
}

