use golem_stt::golem::stt::vocabularies::{Guest as VocabulariesGuest, Vocabulary};
use golem_stt::golem::stt::types::SttError;
use std::collections::HashMap;
use std::sync::Mutex;
use once_cell::sync::Lazy;
use crate::config::AmazonConfig;
use crate::signer::{sigv4_headers, SigV4Params};

#[derive(Clone)]
struct StoredVocabulary { phrases: Vec<String> }

static VOCABULARIES: Lazy<Mutex<HashMap<String, StoredVocabulary>>> = Lazy::new(|| Mutex::new(HashMap::new()));

pub struct AmazonVocabulariesComponent;

impl VocabulariesGuest for AmazonVocabulariesComponent {
    type Vocabulary = AmazonVocabulary;

    fn create_vocabulary(name: String, phrases: Vec<String>) -> Result<Vocabulary, SttError> {
        let cfg = AmazonConfig::load()?;
        let host = cfg.endpoint.clone().unwrap_or_else(|| format!("https://transcribe.{}.amazonaws.com", cfg.region));
        let url = format!("{}/", host.trim_end_matches('/'));
        let client = reqwest::Client::builder().timeout(std::time::Duration::from_secs(cfg.timeout_secs)).build().map_err(|e| SttError::InternalError(format!("client {e}")))?;
        let language = std::env::var("AWS_VOCAB_LANGUAGE").unwrap_or_else(|_| "en-US".into());
        let body = serde_json::json!({
            "VocabularyName": name,
            "LanguageCode": language,
            "Phrases": phrases,
        });
        let parsed = reqwest::Url::parse(&url).map_err(|e| SttError::InternalError(format!("url {e}")))?;
        let host_hdr = parsed.host_str().ok_or_else(|| SttError::InternalError("no host".into()))?;
        let params = SigV4Params { access_key: cfg.access_key.clone(), secret_key: cfg.secret_key.clone(), session_token: cfg.session_token.clone(), region: cfg.region.clone(), service: "transcribe".into() };
        let amz_target = "Transcribe.CreateVocabulary".to_string();
        let payload = serde_json::to_vec(&body).map_err(|e| SttError::InternalError(format!("json {e}")))?;
        let (amz_date, authorization, security_header, content_type) = sigv4_headers(&params, "POST", host_hdr, "/", &amz_target, &payload);
        let mut req = client.post(url).header("x-amz-date", amz_date).header("x-amz-target", amz_target).header("authorization", authorization).header("content-type", content_type);
        if let Some((k, v)) = security_header { req = req.header(k, v); }
        let resp = req.body(payload).send().map_err(|e| SttError::NetworkError(format!("{e}")))?;
        let status = resp.status().as_u16();
        if !(200..300).contains(&status) { return Err(crate::error::map_http_status(status)); }
        let mut map = VOCABULARIES.lock().map_err(|_| SttError::InternalError("lock".into()))?;
        map.insert(name.clone(), StoredVocabulary { phrases });
        Ok(Vocabulary::new(AmazonVocabulary { name }))
    }
}

pub struct AmazonVocabulary { name: String }

impl golem_stt::golem::stt::vocabularies::GuestVocabulary for AmazonVocabulary {
    fn get_name(&self) -> String { self.name.clone() }
    fn get_phrases(&self) -> Vec<String> {
        if let Ok(cfg) = AmazonConfig::load() {
            let host = cfg.endpoint.clone().unwrap_or_else(|| format!("https://transcribe.{}.amazonaws.com", cfg.region));
            let url = format!("{}/", host.trim_end_matches('/'));
            if let Ok(client) = reqwest::Client::builder().timeout(std::time::Duration::from_secs(cfg.timeout_secs)).build() {
                let body = serde_json::json!({ "VocabularyName": self.name });
                if let Ok(parsed) = reqwest::Url::parse(&url) {
                    if let Some(host_hdr) = parsed.host_str() {
                        let params = SigV4Params { access_key: cfg.access_key.clone(), secret_key: cfg.secret_key.clone(), session_token: cfg.session_token.clone(), region: cfg.region.clone(), service: "transcribe".into() };
                        let amz_target = "Transcribe.GetVocabulary".to_string();
                        if let Ok(payload) = serde_json::to_vec(&body) {
                            let (amz_date, authorization, security_header, content_type) = sigv4_headers(&params, "POST", host_hdr, "/", &amz_target, &payload);
                            let mut req = client.post(url).header("x-amz-date", amz_date).header("x-amz-target", amz_target).header("authorization", authorization).header("content-type", content_type);
                            if let Some((k, v)) = security_header { req = req.header(k, v); }
                            if let Ok(resp) = req.body(payload).send() {
                                if resp.status().is_success() {
                                    if let Ok(v) = resp.json::<serde_json::Value>() {
                                        if let Some(arr) = v.get("Phrases").and_then(|p| p.as_array()) {
                                            return arr.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect();
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        if let Ok(map) = VOCABULARIES.lock() {
            if let Some(stored) = map.get(&self.name) { return stored.phrases.clone(); }
        }
        vec![]
    }
    fn delete(&self) -> Result<(), SttError> {
        let cfg = AmazonConfig::load()?;
        let host = cfg.endpoint.clone().unwrap_or_else(|| format!("https://transcribe.{}.amazonaws.com", cfg.region));
        let url = format!("{}/", host.trim_end_matches('/'));
        let client = reqwest::Client::builder().timeout(std::time::Duration::from_secs(cfg.timeout_secs)).build().map_err(|e| SttError::InternalError(format!("client {e}")))?;
        let body = serde_json::json!({ "VocabularyName": self.name });
        let parsed = reqwest::Url::parse(&url).map_err(|e| SttError::InternalError(format!("url {e}")))?;
        let host_hdr = parsed.host_str().ok_or_else(|| SttError::InternalError("no host".into()))?;
        let params = SigV4Params { access_key: cfg.access_key.clone(), secret_key: cfg.secret_key.clone(), session_token: cfg.session_token.clone(), region: cfg.region.clone(), service: "transcribe".into() };
        let amz_target = "Transcribe.DeleteVocabulary".to_string();
        let payload = serde_json::to_vec(&body).map_err(|e| SttError::InternalError(format!("json {e}")))?;
        let (amz_date, authorization, security_header, content_type) = sigv4_headers(&params, "POST", host_hdr, "/", &amz_target, &payload);
        let mut req = client.post(url).header("x-amz-date", amz_date).header("x-amz-target", amz_target).header("authorization", authorization).header("content-type", content_type);
        if let Some((k, v)) = security_header { req = req.header(k, v); }
        let resp = req.body(payload).send().map_err(|e| SttError::NetworkError(format!("{e}")))?;
        let status = resp.status().as_u16();
        if !(200..300).contains(&status) { return Err(crate::error::map_http_status(status)); }
        if let Ok(mut map) = VOCABULARIES.lock() { map.remove(&self.name); }
        Ok(())
    }
}

