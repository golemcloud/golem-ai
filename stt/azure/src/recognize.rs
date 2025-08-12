use golem_stt::golem::stt::transcription::{AudioConfig, TranscribeOptions, TranscriptAlternative};
use golem_stt::golem::stt::types::SttError;
use crate::config::AzureConfig;

fn map_audio_format_to_mime(format: golem_stt::golem::stt::types::AudioFormat) -> &'static str {
    use golem_stt::golem::stt::types::AudioFormat as F;
    match format {
        F::Wav | F::Pcm => "audio/wav",
        F::Mp3 => "audio/mpeg",
        F::Flac => "audio/flac",
        F::Ogg => "audio/ogg",
        F::Aac => "audio/aac",
    }
}

fn should_retry_status(status: u16) -> bool {
    status == 429 || status == 500 || status == 502 || status == 503
}

#[cfg(not(test))]
pub(crate) fn recognize(
    audio: &[u8],
    cfg: &AzureConfig,
    conf: &AudioConfig,
    opts: &Option<TranscribeOptions>,
) -> Result<Vec<TranscriptAlternative>, SttError> {
    use reqwest::Client;
    use serde::Deserialize;

    let endpoint = cfg.endpoint.clone().unwrap_or_else(|| {
        format!("https://{}.stt.speech.microsoft.com/speech/recognition/conversation/cognitiveservices/v1", cfg.region)
    });

    let language = opts
        .as_ref()
        .and_then(|o| o.language.clone())
        .unwrap_or_else(|| "en-US".to_string());

    let mut url = endpoint;
    url.push_str(&format!("?format=detailed&language={}", language));

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(cfg.timeout_secs))
        .build()
        .map_err(|e| SttError::InternalError(format!("client build {e}")))?;

    let mut attempt: u32 = 0;
    let max_attempts = cfg.max_retries.max(1);
    let resp = loop {
        match client
            .post(&url)
            .header("Ocp-Apim-Subscription-Key", &cfg.subscription_key)
            .header("Accept", "application/json")
            .header("Content-Type", map_audio_format_to_mime(conf.format))
            .body(audio.to_vec())
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
    struct NBest {
        #[serde(rename = "Display")]
        display: Option<String>,
        #[serde(rename = "Lexical")]
        lexical: Option<String>,
        #[serde(rename = "Confidence")]
        confidence: Option<f32>,
    }

    #[derive(Deserialize)]
    struct ResultObj {
        #[serde(rename = "DisplayText")]
        display_text: Option<String>,
        #[serde(rename = "NBest")]
        nbest: Option<Vec<NBest>>,
    }

    #[derive(Deserialize)]
    struct ApiResp {
        results: Vec<ResultObj>,
    }

    let api_resp: ApiResp = resp
        .json()
        .map_err(|e| SttError::InternalError(format!("json parse {e}")))?;

    let mut collected = Vec::new();
    for res in api_resp.results {
        if let Some(nbest) = res.nbest {
            for alt in nbest {
                let text = alt.display.or(alt.lexical).unwrap_or_default();
                let confidence = alt.confidence.unwrap_or(0.0);
                collected.push(TranscriptAlternative {
                    text,
                    confidence,
                    words: Vec::new(),
                });
            }
        } else if let Some(display_text) = res.display_text {
            collected.push(TranscriptAlternative {
                text: display_text,
                confidence: 0.0,
                words: Vec::new(),
            });
        }
    }

    Ok(collected)
}

#[cfg(test)]
pub(crate) fn recognize(
    _audio: &[u8],
    _cfg: &AzureConfig,
    _conf: &AudioConfig,
    _opts: &Option<TranscribeOptions>,
) -> Result<Vec<TranscriptAlternative>, SttError> {
    Ok(vec![TranscriptAlternative { text: "hello".into(), confidence: 0.9, words: Vec::new() }])
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
}
