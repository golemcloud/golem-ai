use golem_stt::golem::stt::transcription::{AudioConfig, TranscribeOptions, TranscriptAlternative};
use golem_stt::golem::stt::types::SttError;
#[cfg(not(test))]
use golem_stt::golem::stt::types::WordSegment;
use crate::config::GoogleConfig;
use serde_json::Value;

fn map_encoding(conf: &AudioConfig) -> Option<&'static str> {
    use golem_stt::golem::stt::types::AudioFormat as F;
    match conf.format {
        F::Pcm | F::Wav => Some("LINEAR16"),
        F::Flac => Some("FLAC"),
        F::Mp3 => Some("MP3"),
        F::Ogg => Some("OGG_OPUS"),
        F::Aac => None,
    }
}

fn should_retry_status(status: u16) -> bool {
    status == 429 || status == 500 || status == 502 || status == 503
}

fn build_request_config(conf: &AudioConfig, opts: &Option<TranscribeOptions>) -> Value {
    use serde_json::json;
    let mut cfg = serde_json::Map::new();
    if let Some(enc) = map_encoding(conf) { cfg.insert("encoding".into(), json!(enc)); }
    if let Some(sr) = conf.sample_rate { cfg.insert("sampleRateHertz".into(), json!(sr)); }
    if let Some(ch) = conf.channels { cfg.insert("audioChannelCount".into(), json!(ch as u32)); }
    if let Some(o) = opts {
        if let Some(lang) = &o.language { cfg.insert("languageCode".into(), json!(lang)); }
        if let Some(model) = &o.model { cfg.insert("model".into(), json!(model)); }
        if let Some(pf) = o.profanity_filter { cfg.insert("profanityFilter".into(), json!(pf)); }
        if let Some(t) = o.enable_timestamps { cfg.insert("enableWordTimeOffsets".into(), json!(t)); }
        if let Some(wc) = o.enable_word_confidence { cfg.insert("enableWordConfidence".into(), json!(wc)); }
        if let Some(d) = o.enable_speaker_diarization { cfg.insert("diarizationConfig".into(), json!({"enableSpeakerDiarization": d})); }
        if let Some(ctx) = &o.speech_context {
            if !ctx.is_empty() { cfg.insert("speechContexts".into(), json!([{"phrases": ctx}])); }
        }
    }
    Value::Object(cfg)
}

#[cfg(not(test))]
pub(crate) fn recognize(
    audio: &[u8],
    cfg: &GoogleConfig,
    conf: &AudioConfig,
    opts: &Option<TranscribeOptions>,
) -> Result<Vec<TranscriptAlternative>, SttError> {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    use reqwest::Client;
    use serde::Deserialize;
    use serde_json::json;

    let token = crate::auth::fetch_token(cfg)?.access_token;
    let mut config_obj = build_request_config(conf, opts);
    if config_obj.get("languageCode").is_none() {
        config_obj.as_object_mut().unwrap().insert("languageCode".into(), json!("en-US"));
    }
    let payload = json!({"config": config_obj, "audio": {"content": STANDARD.encode(audio)}});

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(cfg.timeout_secs))
        .build()
        .map_err(|e| SttError::InternalError(format!("client build {e}")))?;

    let endpoint = cfg
        .endpoint
        .clone()
        .unwrap_or_else(|| crate::constants::GOOGLE_SPEECH_ENDPOINT.to_string());

    let mut attempt: u32 = 0;
    let max_attempts = cfg.max_retries.max(1);
    let resp = loop {
        match client
            .post(&endpoint)
            .bearer_auth(&token)
            .json(&payload)
            .send()
        {
            Ok(r) => {
                let status = r.status().as_u16();
                if status == 200 { break r; }
                attempt += 1;
                if attempt >= max_attempts || !should_retry_status(status) {
                    return Err(crate::error::map_http_status(status));
                }
                let delay_ms = 200u64.saturating_mul(1u64 << (attempt - 1));
                let jitter_ms = (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().subsec_nanos() as u64) % 100;
                std::thread::sleep(std::time::Duration::from_millis(delay_ms + jitter_ms));
                continue;
            }
            Err(e) => {
                attempt += 1;
                if attempt >= max_attempts {
                    return Err(SttError::NetworkError(format!("{e}")));
                }
                let delay_ms = 200u64.saturating_mul(1u64 << (attempt - 1));
                let jitter_ms = (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().subsec_nanos() as u64) % 100;
                std::thread::sleep(std::time::Duration::from_millis(delay_ms + jitter_ms));
            }
        }
    };

    let status = resp.status().as_u16();
    if !(200..300).contains(&status) { return Err(crate::error::map_http_status(status)); }

    #[derive(Deserialize)]
    #[allow(non_snake_case)]
    struct WordInfo { startTime: Option<String>, endTime: Option<String>, word: Option<String>, confidence: Option<f32>, speakerTag: Option<i32> }
    #[derive(Deserialize)]
    struct AltObj { transcript: String, confidence: Option<f32>, words: Option<Vec<WordInfo>> }
    #[derive(Deserialize)]
    struct ResultObj { alternatives: Vec<AltObj> }
    #[derive(Deserialize)]
    struct ApiResp { results: Vec<ResultObj> }

    let api_resp: ApiResp = resp
        .json()
        .map_err(|e| SttError::InternalError(format!("json parse {e}")))?;

    let timestamps_enabled = opts.as_ref().and_then(|o| o.enable_timestamps).unwrap_or(false);
    let diarization_enabled = opts.as_ref().and_then(|o| o.enable_speaker_diarization).unwrap_or(false);
    let mut collected = Vec::new();
    for res in api_resp.results {
        for alt in res.alternatives {
            let mut words_out = Vec::new();
            if timestamps_enabled {
                if let Some(words) = alt.words.as_ref() {
                    for w in words {
                        let start = parse_google_duration(&w.startTime).unwrap_or(0.0);
                        let end = parse_google_duration(&w.endTime).unwrap_or(start);
                        let speaker = if diarization_enabled { w.speakerTag.map(|t| t.to_string()) } else { None };
                        words_out.push(WordSegment { text: w.word.clone().unwrap_or_default(), start_time: start, end_time: end, confidence: w.confidence, speaker_id: speaker });
                    }
                }
            }
            collected.push(TranscriptAlternative { text: alt.transcript, confidence: alt.confidence.unwrap_or(0.0), words: words_out });
        }
    }

    Ok(collected)
}

#[cfg_attr(test, allow(dead_code))]
fn parse_google_duration(value: &Option<String>) -> Option<f32> {
    if let Some(v) = value {
        // Formats like "1.234s" or "2s"
        let s = v.trim_end_matches('s');
        if let Ok(f) = s.parse::<f32>() { return Some(f); }
    }
    None
}

#[cfg(test)]
pub(crate) fn recognize(
    _audio: &[u8],
    _cfg: &GoogleConfig,
    _conf: &AudioConfig,
    _opts: &Option<TranscribeOptions>,
) -> Result<Vec<TranscriptAlternative>, SttError> {
    Ok(vec![TranscriptAlternative { text: "hello".into(), confidence: 0.9, words: Vec::new() }])
} 

#[cfg(test)]
mod tests {
    use super::*;
    use golem_stt::golem::stt::transcription::AudioConfig as AC;
    use golem_stt::golem::stt::types::AudioFormat;
    use serde_json::json;

    #[test]
    fn config_mapping_basic() {
        let conf = AC { format: AudioFormat::Wav, sample_rate: Some(16000), channels: Some(1) };
        let val = build_request_config(&conf, &None);
        assert_eq!(val["encoding"], json!("LINEAR16"));
        assert_eq!(val["sampleRateHertz"], json!(16000));
        assert_eq!(val["audioChannelCount"], json!(1));
    }

    #[test]
    fn config_mapping_options() {
        let conf = AC { format: AudioFormat::Mp3, sample_rate: None, channels: None };
        let opts = TranscribeOptions {
            enable_timestamps: Some(true),
            enable_speaker_diarization: Some(true),
            language: Some("fr-FR".into()),
            model: Some("latest_long".into()),
            profanity_filter: Some(true),
            speech_context: Some(vec!["foo".into(), "bar".into()]),
            enable_word_confidence: Some(true),
            enable_timing_detail: None,
        };
        let val = build_request_config(&conf, &Some(opts));
        assert_eq!(val["encoding"], json!("MP3"));
        assert_eq!(val["languageCode"], json!("fr-FR"));
        assert_eq!(val["model"], json!("latest_long"));
        assert_eq!(val["profanityFilter"], json!(true));
        assert_eq!(val["enableWordTimeOffsets"], json!(true));
        assert_eq!(val["enableWordConfidence"], json!(true));
        assert_eq!(val["diarizationConfig"]["enableSpeakerDiarization"], json!(true));
        assert_eq!(val["speechContexts"][0]["phrases"], json!(["foo","bar"]));
    }

    #[test]
    fn retry_classification() {
        assert!(should_retry_status(429));
        assert!(should_retry_status(500));
        assert!(should_retry_status(502));
        assert!(should_retry_status(503));
        assert!(!should_retry_status(404));
    }
}