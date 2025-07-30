use golem_stt::golem::stt::transcription::{AudioConfig, TranscribeOptions, TranscriptAlternative};
use golem_stt::golem::stt::types::SttError;
use crate::config::GoogleConfig;

#[cfg(not(test))]
pub(crate) fn recognize(
    audio: &[u8],
    cfg: &GoogleConfig,
    _conf: &AudioConfig,
    opts: &Option<TranscribeOptions>,
) -> Result<Vec<TranscriptAlternative>, SttError> {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    use reqwest::Client;
    use serde::Deserialize;
    use serde_json::json;

    let token = crate::auth::fetch_token(cfg)?.access_token;

    let language = opts
        .as_ref()
        .and_then(|o| o.language.clone())
        .unwrap_or_else(|| "en-US".to_string());

    let payload = json!({
        "config": {
            "languageCode": language,
        },
        "audio": {
            "content": STANDARD.encode(audio),
        }
    });

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(cfg.timeout_secs))
        .build()
        .map_err(|e| SttError::InternalError(format!("client build {e}")))?;

    let endpoint = cfg
        .endpoint
        .clone()
        .unwrap_or_else(|| crate::constants::GOOGLE_SPEECH_ENDPOINT.to_string());

    let resp = client
        .post(&endpoint)
        .bearer_auth(token)
        .json(&payload)
        .send()
        .map_err(|e| SttError::NetworkError(format!("{e}")))?;

    let status = resp.status().as_u16();
    if !(200..300).contains(&status) {
        return Err(crate::error::map_http_status(status));
    }

    #[derive(Deserialize)]
    struct AltObj {
        transcript: String,
        confidence: Option<f32>,
    }
    #[derive(Deserialize)]
    struct ResultObj {
        alternatives: Vec<AltObj>,
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
        for alt in res.alternatives {
            collected.push(TranscriptAlternative {
                text: alt.transcript,
                confidence: alt.confidence.unwrap_or(0.0),
                words: Vec::new(),
            });
        }
    }

    Ok(collected)
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