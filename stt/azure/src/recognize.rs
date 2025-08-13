use golem_stt::golem::stt::transcription::{AudioConfig, TranscribeOptions, TranscriptAlternative};
use golem_stt::golem::stt::types::SttError;
#[cfg(not(test))]
use golem_stt::golem::stt::types::WordSegment;
use crate::config::AzureConfig;

fn map_audio_format_to_mime(format: golem_stt::golem::stt::types::AudioFormat) -> &'static str {
    use golem_stt::golem::stt::types::AudioFormat as F;
    match format { F::Wav | F::Pcm => "audio/wav", F::Mp3 => "audio/mpeg", F::Flac => "audio/flac", F::Ogg => "audio/ogg", F::Aac => "audio/aac" }
}

#[allow(dead_code)]
fn should_retry_status(status: u16) -> bool { status == 429 || status == 500 || status == 502 || status == 503 }

pub struct RecognizeOut { pub alternatives: Vec<TranscriptAlternative>, pub request_id: Option<String>, pub elapsed_secs: f32, pub server_duration_secs: Option<f32> }

fn build_azure_query_params(language: &str, opts: &Option<TranscribeOptions>) -> String {
    let mut params = vec!["format=detailed".to_string(), format!("language={}", language)];
    if let Some(o) = opts {
        if o.enable_timestamps.unwrap_or(false) { params.push("wordLevelTimestamps=true".to_string()); }
        if o.enable_speaker_diarization.unwrap_or(false) { params.push("diarizationEnabled=true".to_string()); }
        if o.enable_word_confidence.unwrap_or(false) { params.push("wordLevelConfidence=true".to_string()); }
        if let Some(p) = o.profanity_filter { params.push(format!("profanityFilter={}", if p { "true" } else { "false" })); }
        if let Some(ctx) = &o.speech_context {
            if !ctx.is_empty() {
                use std::collections::BTreeSet;
                let set: BTreeSet<_> = ctx.iter().map(|s| s.as_str()).collect();
                let merged = set.into_iter().collect::<Vec<_>>().join(",");
                params.push(format!("phraseList={}", merged));
            }
        }
        if let Some(model) = &o.model { if !model.is_empty() { params.push(format!("deploymentId={}", model)); } }
    }
    params.join("&")
}

#[cfg(not(test))]
pub(crate) fn recognize(
    audio: &[u8], cfg: &AzureConfig, conf: &AudioConfig, opts: &Option<TranscribeOptions>,
) -> Result<RecognizeOut, SttError> {
    use reqwest::Client;

    let endpoint = cfg.endpoint.clone().unwrap_or_else(|| format!("https://{}.stt.speech.microsoft.com/speech/recognition/conversation/cognitiveservices/v1", cfg.region));
    let language = opts.as_ref().and_then(|o| o.language.clone()).unwrap_or_else(|| "en-US".to_string());
    let mut url = endpoint;
    let qp = build_azure_query_params(&language, opts);
    url.push_str(&format!("?{}", qp));

    let client = Client::builder().timeout(std::time::Duration::from_secs(cfg.timeout_secs)).build().map_err(|e| SttError::InternalError(format!("client build {e}")))?;

    let started = std::time::Instant::now();
    let mut attempt: u32 = 0;
    let max_attempts = cfg.max_retries.max(1);
    let resp = loop {
        match client.post(&url)
            .header("Ocp-Apim-Subscription-Key", &cfg.subscription_key)
            .header("Accept", "application/json")
            .header("Content-Type", map_audio_format_to_mime(conf.format))
            .body(audio.to_vec())
            .send() {
            Ok(r) => break r,
            Err(e) => { if attempt + 1 >= max_attempts { return Err(SttError::NetworkError(format!("{e}"))); } attempt += 1; continue; }
        }
    };
    let elapsed_secs = started.elapsed().as_secs_f32();

    let status = resp.status().as_u16();
    if !(200..300).contains(&status) { return Err(crate::error::map_http_status(status)); }

    let req_id = resp.headers().get("x-requestid").or_else(|| resp.headers().get("X-RequestId")).and_then(|v| v.to_str().ok()).map(|s| s.to_string());
    let server_duration_secs = resp.headers().get("Duration").and_then(|v| v.to_str().ok()).and_then(|s| s.parse::<f32>().ok());

    #[derive(serde::Deserialize)]
    struct WordItem { #[serde(rename = "Word")] word: Option<String>, #[serde(rename = "Text")] text: Option<String>, #[serde(rename = "Offset")] offset: Option<u64>, #[serde(rename = "Duration")] duration: Option<u64>, #[serde(rename = "Confidence")] confidence: Option<f32>, #[serde(rename = "SpeakerId")] speaker_id: Option<u32> }
    #[derive(serde::Deserialize)]
    struct NBest { #[serde(rename = "Display")] display: Option<String>, #[serde(rename = "Lexical")] lexical: Option<String>, #[serde(rename = "Confidence")] confidence: Option<f32>, #[serde(rename = "Words")] words: Option<Vec<WordItem>> }
    #[derive(serde::Deserialize)]
    struct ResultObj { #[serde(rename = "DisplayText")] display_text: Option<String>, #[serde(rename = "NBest")] nbest: Option<Vec<NBest>> }
    #[derive(serde::Deserialize)]
    struct ApiResp { results: Vec<ResultObj> }

    let api_resp: ApiResp = resp.json().map_err(|e| SttError::InternalError(format!("json parse {e}")))?;

    let timestamps_enabled = opts.as_ref().and_then(|o| o.enable_timestamps).unwrap_or(false);
    let diarization_enabled = opts.as_ref().and_then(|o| o.enable_speaker_diarization).unwrap_or(false);

    let mut alternatives_out = Vec::new();
    for res in api_resp.results {
        if let Some(nbest) = res.nbest {
            for alt in nbest {
                let text = alt.display.or(alt.lexical).unwrap_or_default();
                let confidence = alt.confidence.unwrap_or(0.0);
                let mut words_out = Vec::new();
                if timestamps_enabled {
                    if let Some(items) = alt.words.as_ref() {
                        for w in items {
                            let wtext = w.word.as_ref().or(w.text.as_ref()).cloned().unwrap_or_default();
                            let start = w.offset.unwrap_or(0) as f32 / 10_000_000.0;
                            let end = w.duration.map(|d| (w.offset.unwrap_or(0) + d) as f32 / 10_000_000.0);
                            let speaker = if diarization_enabled { w.speaker_id.map(|id| id.to_string()) } else { None };
                            words_out.push(WordSegment { text: wtext, start_time: start, end_time: end.unwrap_or(start), confidence: w.confidence, speaker_id: speaker });
                        }
                    }
                }
                alternatives_out.push(TranscriptAlternative { text, confidence, words: words_out });
            }
        } else if let Some(display_text) = res.display_text {
            alternatives_out.push(TranscriptAlternative { text: display_text, confidence: 0.0, words: Vec::new() });
        }
    }

    Ok(RecognizeOut { alternatives: alternatives_out, request_id: req_id, elapsed_secs, server_duration_secs })
}

#[cfg(test)]
pub(crate) fn recognize(_audio: &[u8], _cfg: &AzureConfig, _conf: &AudioConfig, _opts: &Option<TranscribeOptions>) -> Result<RecognizeOut, SttError> {
    Ok(RecognizeOut { alternatives: vec![TranscriptAlternative { text: "hello".into(), confidence: 0.9, words: Vec::new() }], request_id: Some("test".into()), elapsed_secs: 0.01, server_duration_secs: None })
}

#[cfg(test)]
mod tests {
    use super::*;
    use golem_stt::golem::stt::types::AudioFormat;

    #[test]
    fn mime_mapping() {
        assert_eq!(map_audio_format_to_mime(AudioFormat::Wav), "audio/wav");
        assert_eq!(map_audio_format_to_mime(AudioFormat::Mp3), "audio/mpeg");
        assert_eq!(map_audio_format_to_mime(AudioFormat::Flac), "audio/flac");
        assert_eq!(map_audio_format_to_mime(AudioFormat::Ogg), "audio/ogg");
        assert_eq!(map_audio_format_to_mime(AudioFormat::Aac), "audio/aac");
    }

    #[test]
    fn retry_classification() {
        assert!(should_retry_status(429));
        assert!(should_retry_status(500));
        assert!(should_retry_status(502));
        assert!(should_retry_status(503));
        assert!(!should_retry_status(404));
    }

    #[test]
    fn test_build_azure_query_params_basic() {
        let opts = None;
        let qp = build_azure_query_params("en-US", &opts);
        assert!(qp.contains("format=detailed"));
        assert!(qp.contains("language=en-US"));
    }

    #[test]
    fn test_build_azure_query_params_with_timestamps() {
        let opts = Some(TranscribeOptions { enable_timestamps: Some(true), enable_speaker_diarization: None, language: None, model: None, profanity_filter: None, speech_context: None, enable_word_confidence: None, enable_timing_detail: None });
        let qp = build_azure_query_params("en-US", &opts);
        assert!(qp.contains("wordLevelTimestamps=true"));
    }

    #[test]
    fn test_build_azure_query_params_with_diarization() {
        let opts = Some(TranscribeOptions { enable_timestamps: None, enable_speaker_diarization: Some(true), language: None, model: None, profanity_filter: None, speech_context: None, enable_word_confidence: None, enable_timing_detail: None });
        let qp = build_azure_query_params("en-US", &opts);
        assert!(qp.contains("diarizationEnabled=true"));
    }
    
    #[test]
    fn test_build_azure_query_params_with_model_and_context_dedupe() {
        let opts = Some(TranscribeOptions { enable_timestamps: None, enable_speaker_diarization: None, language: None, model: Some("my-deploy".into()), profanity_filter: None, speech_context: Some(vec!["foo".into(), "foo".into(), "bar".into()]), enable_word_confidence: None, enable_timing_detail: None });
        let qp = build_azure_query_params("en-US", &opts);
        assert!(qp.contains("deploymentId=my-deploy"));
        assert!(qp.contains("phraseList=bar,foo") || qp.contains("phraseList=foo,bar"));
    }

    #[test]
    fn test_build_azure_query_params_with_word_confidence() {
        let opts = Some(TranscribeOptions { enable_timestamps: None, enable_speaker_diarization: None, language: None, model: None, profanity_filter: None, speech_context: None, enable_word_confidence: Some(true), enable_timing_detail: None });
        let qp = build_azure_query_params("en-US", &opts);
        assert!(qp.contains("wordLevelConfidence=true"));
    }
}
