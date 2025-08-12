use golem_stt::golem::stt::vocabularies::{Guest as VocabulariesGuest, Vocabulary};
use golem_stt::golem::stt::types::SttError;
use std::collections::HashMap;
use std::sync::Mutex;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

#[derive(Clone)]
struct StoredVocabulary {
    id: Option<String>,
    phrases: Vec<String>,
}

static VOCABULARIES: Lazy<Mutex<HashMap<String, StoredVocabulary>>> = Lazy::new(|| Mutex::new(HashMap::new()));

#[derive(Serialize)]
struct CreatePhraseListRequest {
    display_name: String,
    phrases: Vec<String>,
}

#[derive(Deserialize)]
struct CreatePhraseListResponse {
    #[serde(default)]
    id: Option<String>,
}

pub struct AzureVocabulariesComponent;

impl VocabulariesGuest for AzureVocabulariesComponent {
    type Vocabulary = AzureVocabulary;

    fn create_vocabulary(name: String, phrases: Vec<String>) -> Result<Vocabulary, SttError> {
        let cfg = crate::config::AzureConfig::load()?;
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(cfg.timeout_secs))
            .build()
            .map_err(|e| SttError::InternalError(format!("client build {e}")))?;

        let endpoint = cfg
            .endpoint
            .clone()
            .unwrap_or_else(|| format!("https://{}.api.cognitive.microsoft.com/speechtotext/v3.1/phraselists", cfg.region));

        let body = CreatePhraseListRequest {
            display_name: name.clone(),
            phrases: phrases.clone(),
        };

        let resp = client
            .post(&endpoint)
            .header("Ocp-Apim-Subscription-Key", &cfg.subscription_key)
            .json(&body)
            .send()
            .map_err(|e| SttError::NetworkError(format!("API request failed: {e}")))?;

        let status = resp.status().as_u16();
        if status != 201 && status != 200 {
            return Err(crate::error::map_http_status(status));
        }

        let id = if let Ok(parsed) = resp.json::<CreatePhraseListResponse>() {
            parsed.id
        } else {
            None
        };

        let mut map = VOCABULARIES
            .lock()
            .map_err(|_| SttError::InternalError("failed to lock vocabularies".into()))?;
        map.insert(
            name.clone(),
            StoredVocabulary {
                id,
                phrases,
            },
        );

        Ok(Vocabulary::new(AzureVocabulary { name }))
    }
}

pub struct AzureVocabulary {
    name: String,
}

impl golem_stt::golem::stt::vocabularies::GuestVocabulary for AzureVocabulary {
    fn get_name(&self) -> String {
        self.name.clone()
    }

    fn get_phrases(&self) -> Vec<String> {
        if let Ok(map) = VOCABULARIES.lock() {
            if let Some(stored) = map.get(&self.name) {
                if let Some(id) = &stored.id {
                    if let Ok(cfg) = crate::config::AzureConfig::load() {
                        if let Ok(client) = reqwest::Client::builder().timeout(std::time::Duration::from_secs(cfg.timeout_secs)).build() {
                            let base = cfg
                                .endpoint
                                .clone()
                                .unwrap_or_else(|| format!("https://{}.api.cognitive.microsoft.com/speechtotext/v3.1/phraselists", cfg.region));
                            let url = format!("{}/{}", base.trim_end_matches('/'), id);
                            if let Ok(resp) = client.get(&url).header("Ocp-Apim-Subscription-Key", &cfg.subscription_key).send() {
                                if resp.status().is_success() {
                                    #[derive(serde::Deserialize)]
                                    struct GetResp { phrases: Option<Vec<String>> }
                                    if let Ok(gr) = resp.json::<GetResp>() { return gr.phrases.unwrap_or_default(); }
                                }
                            }
                        }
                    }
                }
                return stored.phrases.clone();
            }
        }
        Vec::new()
    }

    fn delete(&self) -> Result<(), SttError> {
        let cfg = crate::config::AzureConfig::load()?;
        let mut map = VOCABULARIES
            .lock()
            .map_err(|_| SttError::InternalError("failed to lock vocabularies".into()))?;
        if let Some(stored) = map.get(&self.name).cloned() {
            if let Some(id) = stored.id {
                let client = reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(cfg.timeout_secs))
                    .build()
                    .map_err(|e| SttError::InternalError(format!("client build {e}")))?;
                let base = cfg
                    .endpoint
                    .clone()
                    .unwrap_or_else(|| format!("https://{}.api.cognitive.microsoft.com/speechtotext/v3.1/phraselists", cfg.region));
                let url = format!("{}/{}", base.trim_end_matches('/'), id);
                let resp = client
                    .delete(&url)
                    .header("Ocp-Apim-Subscription-Key", &cfg.subscription_key)
                    .send()
                    .map_err(|e| SttError::NetworkError(format!("API request failed: {e}")))?;
                let status = resp.status().as_u16();
                if status != 204 && status != 200 {
                    return Err(crate::error::map_http_status(status));
                }
            }
            map.remove(&self.name);
        }
        Ok(())
    }
}
