use golem_rust::{FromValueAndType, IntoValue};
use std::cell::RefCell;

use golem_tts::{
    client::TtsClient,
    config::get_env,
    golem::tts::{
        advanced::{GuestLongFormOperation, GuestPronunciationLexicon, LongFormResult, OperationStatus},
        types::{LanguageCode, TtsError},
    },
};
use log::trace;
use reqwest::{Client, Method};
use serde::{Deserialize, Serialize};

use crate::{
    elevenlabs::Elevenlabs,
    error::{from_http_error, unsupported},
    types::{AddRulesRequest, ElVoiceSettings, PronunciationRule, RemoveRulesRequest, UpdateLexiconRuleResponse, VerifiedLanguage},
};

#[derive(Serialize, Deserialize, Clone, Debug, IntoValue, FromValueAndType)]
pub struct VoiceResponse {
    pub voice_id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<VoiceLabels>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<ElVoiceSettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified_languages: Option<Vec<VerifiedLanguage>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_owner: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_legacy: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_mixed: Option<bool>,
}
#[derive(Serialize, Deserialize, Clone, Debug, IntoValue, FromValueAndType)]
pub struct VoiceLabels {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub age: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gender: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_case: Option<String>,
}


pub struct ElPronunciationLexicon {
    pub id: String,
    pub name: String,
    pub language: LanguageCode,
    pub version_id: RefCell<String>,
    pub rules_count: RefCell<u32>,
}

impl GuestPronunciationLexicon for ElPronunciationLexicon {
    fn get_name(&self) -> String {
        self.name.clone()
    }

    fn get_language(&self) -> LanguageCode {
        self.language.clone()
    }

    fn get_entry_count(&self) -> u32 {
        *self.rules_count.borrow()
    }

    #[doc = " Add pronunciation rule"]
    fn add_entry(&self, word: String, pronunciation: String) -> Result<(), TtsError> {
        let rule = if pronunciation
            .chars()
            .any(|c| "əɪɛɔʊʌɑɒæɜɪʏøœɯɤɐɞɘɵɨɵʉɪʊ".contains(c))
        {
            PronunciationRule {
                string_to_replace: word,
                rule_type: "phoneme".to_string(),
                alias: None,
                phoneme: Some(pronunciation),
                alphabet: Some("ipa".to_string()),
            }
        } else {
            PronunciationRule {
                string_to_replace: word,
                rule_type: "alias".to_string(),
                alias: Some(pronunciation),
                phoneme: None,
                alphabet: None,
            }
        };

        let request = AddRulesRequest { rules: vec![rule] };
        let path = format!("/v1/pronunciation-dictionaries/{}/add-rules", self.id);
        let elevenlabs = Elevenlabs::new()?;

        let response = elevenlabs
            .client
            .make_request::<UpdateLexiconRuleResponse, AddRulesRequest, (), _>(
                Method::POST,
                &path,
                request,
                None,
                None,
                from_http_error,
            )?;
        *self.rules_count.borrow_mut() += 1;
        trace!("Add Entry response : {response:?}");
        Ok(())
    }

    #[doc = " Remove pronunciation rule"]
    fn remove_entry(&self, word: String) -> Result<(), TtsError> {
        let request = RemoveRulesRequest {
            rule_strings: vec![word],
        };
        let path = format!("/v1/pronunciation-dictionaries/{}/remove-rules", self.id);
        let elevenlabs = Elevenlabs::new()?;

        let reuslt = elevenlabs
            .client
            .make_request::<UpdateLexiconRuleResponse, RemoveRulesRequest, (), _>(
                Method::POST,
                &path,
                request,
                None,
                None,
                from_http_error,
            )?;
        *self.rules_count.borrow_mut() -= 1;
        trace!("Remove Entry response : {reuslt:?}");
        Ok(())
    }

    #[doc = " Export lexicon content"]
    fn export_content(&self) -> Result<String, TtsError> {
        let base_url = get_env("TTS_PROVIDER_ENDPOINT")
            .ok()
            .unwrap_or("https://api.elevenlabs.io".to_string());
        let api_key = get_env("ELEVENLABS_API_KEY")?;
        let path = format!(
            "/v1/pronunciation-dictionaries/{}/{}/download",
            self.id,
            self.version_id.borrow()
        );
        let url = format!("{}{}", base_url, path);
        let request = Client::new()
            .request(Method::GET, &url)
            .header("xi-api-key", api_key);

        match request.send() {
            Ok(response) => {
                if response.status().is_success() {
                    response.text().map_err(|e| {
                        TtsError::InternalError(format!("Failed to read PLS content: {}", e))
                    })
                } else {
                    Err(from_http_error(response))
                }
            }
            Err(err) => Err(TtsError::NetworkError(format!("Request failed: {}", err))),
        }
    }
}

pub struct ElLongFormSynthesis;

impl GuestLongFormOperation for ElLongFormSynthesis {
    fn get_status(&self) -> OperationStatus {
        OperationStatus::Cancelled
    }

    fn get_progress(&self) -> f32 {
        100.0
    }

    fn cancel(&self) -> Result<(), TtsError> {
        unsupported("Long-form synthesis not yet implemented for ElevenLabs TTS")
    }

    fn get_result(&self) -> Result<LongFormResult, TtsError> {
        unsupported("Long-form synthesis not yet implemented for ElevenLabs TTS")
    }
}
