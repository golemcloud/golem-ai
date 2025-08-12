use golem_stt::golem::stt::vocabularies::{Guest as VocabulariesGuest, Vocabulary};
use golem_stt::golem::stt::types::SttError;
use std::collections::HashMap;
use std::sync::Mutex;
use once_cell::sync::Lazy;
use serde::{Serialize};

static VOCABULARIES: Lazy<Mutex<HashMap<String, Vec<String>>>> = Lazy::new(|| Mutex::new(HashMap::new()));

#[derive(Serialize)]
struct CreatePhraseListRequest {
    display_name: String,
    phrases: Vec<String>,
}

pub struct AzureVocabulariesComponent;

impl VocabulariesGuest for AzureVocabulariesComponent {
    type Vocabulary = AzureVocabulary;

    fn create_vocabulary(name: String, phrases: Vec<String>) -> Result<Vocabulary, SttError> {
        let mut vocabularies = VOCABULARIES.lock().map_err(|_| SttError::InternalError("failed to lock vocabularies".into()))?;

        if vocabularies.contains_key(&name) {
            return Err(SttError::UnsupportedOperation(format!("vocabulary '{}' already exists", name)));
        }

        let cfg = crate::config::AzureConfig::load()?;
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(cfg.timeout_secs))
            .build()
            .map_err(|e| SttError::InternalError(format!("client build {e}")))?;

        let endpoint = cfg.endpoint.clone().unwrap_or_else(|| {
            format!("https://{}.api.cognitive.microsoft.com/speechtotext/v3.1/phraselists", cfg.region)
        });

        let request_body = CreatePhraseListRequest {
            display_name: name.clone(),
            phrases,
        };

        let response = client.post(&endpoint)
            .header("Ocp-Apim-Subscription-Key", &cfg.subscription_key)
            .json(&request_body)
            .send()
            .map_err(|e| SttError::NetworkError(format!("API request failed: {e}")))?;

        let status = response.status().as_u16();
        if status != 201 { // 201 Created is expected for successful creation
            return Err(crate::error::map_http_status(status));
        }

        vocabularies.insert(name.clone(), request_body.phrases);

        let vocabulary = AzureVocabulary { name };
        Ok(Vocabulary::new(vocabulary))
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
        VOCABULARIES.lock()
            .map(|v| v.get(&self.name).cloned().unwrap_or_default())
            .unwrap_or_default()
    }

    fn delete(&self) -> Result<(), SttError> {
        let mut vocabularies = VOCABULARIES.lock().map_err(|_| SttError::InternalError("failed to lock vocabularies".into()))?;
        vocabularies.remove(&self.name);
        Ok(())
    }
}
