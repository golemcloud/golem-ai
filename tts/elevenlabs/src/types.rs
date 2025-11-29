use serde::{Deserialize, Serialize};

use golem_rust::{FromValueAndType, IntoValue};

use crate::resources::VoiceResponse;

// Voice cloning request/response structures
#[derive(Serialize, Debug, Clone)]
pub struct AddVoiceRequest {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remove_background_noise: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct AddVoiceResponse {
    pub voice_id: String,
    pub requires_verification: bool,
}

// Sound effects request/response structures
#[derive(Serialize, Debug, Clone)]
pub struct SoundEffectsRequest {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_influence: Option<f32>,
}

// PVC create request/response structures
#[derive(Serialize, Debug, Clone)]
pub struct PvcCreateRequest {
    pub name: String,
    pub language: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<std::collections::HashMap<String, String>>,
}

#[derive(Deserialize, Debug)]
pub struct PvcCreateResponse {
    pub voice_id: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct PronunciationRule {
    pub string_to_replace: String,
    #[serde(rename = "type")]
    pub rule_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phoneme: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alphabet: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct CreateLexiconFromRulesRequest {
    pub rules: Vec<PronunciationRule>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_access: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct AddRulesRequest {
    pub rules: Vec<PronunciationRule>,
}

#[derive(Serialize, Debug, Clone)]
pub struct RemoveRulesRequest {
    pub rule_strings: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct CreateLexiconResponse {
    pub id: String,
    pub name: String,
    pub created_by: String,
    pub creation_time_unix: i64,
    pub version_id: String,
    pub version_rules_num: u32,
    pub permission_on_resource: String,
    pub description: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct UpdateLexiconRuleResponse {
    pub id: String,
    pub version_id: String,
    pub version_rules_num: u32,
}

#[derive(Serialize, Clone, Debug)]
pub struct SynthesisRequest {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_settings: Option<SynthesisVoiceSettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pronunciation_dictionary_locators: Option<Vec<PronunciationDictionaryLocator>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apply_text_normalization: Option<String>, // "auto", "on", "off"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_request_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_request_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apply_language_text_normalization: Option<bool>,
}

#[derive(Serialize, Clone, Debug)]
pub struct SynthesisVoiceSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stability: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub similarity_boost: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_speaker_boost: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed: Option<f32>,
}

#[derive(Serialize, Clone, Debug)]
pub struct PronunciationDictionaryLocator {
    pub pronunciation_dictionary_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_id: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
pub struct ListVoicesQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_page_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_size: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_direction: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fine_tuning_state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collection_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_total_count: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_ids: Option<Vec<String>>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct ListVoicesResponse {
    pub voices: Vec<VoiceResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_more: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_page_token: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, IntoValue, FromValueAndType)]
pub struct ElVoiceSettings {
    pub stability: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_speaker_boost: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub similarity_boost: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed: Option<f64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, IntoValue, FromValueAndType)]
pub struct VerifiedLanguage {
    pub language: String,
    pub model_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview_url: Option<String>,
}
