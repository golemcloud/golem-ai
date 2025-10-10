use serde::{Deserialize, Serialize};

use crate::resources::VoiceResponse;

#[derive(Serialize, Clone, Debug)]
pub struct SynthesisRequest {
    pub input: SynthesisInput,
    pub voice: VoiceSelectionParams,
    #[serde(rename = "audioConfig")]
    pub audio_config: AudioConfigData,
}

#[derive(Serialize, Clone, Debug)]
#[serde(untagged)]
pub enum SynthesisInput {
    Text { text: String },
    Ssml { ssml: String },
}

#[derive(Serialize, Clone, Debug)]
pub struct VoiceSelectionParams {
    #[serde(rename = "languageCode")]
    pub language_code: String,
    pub name: String,
    #[serde(rename = "ssmlGender")]
    pub ssml_gender: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AudioConfigData {
    #[serde(rename = "audioEncoding")]
    pub audio_encoding: String,
    #[serde(rename = "sampleRateHertz", skip_serializing_if = "Option::is_none")]
    pub sample_rate_hertz: Option<u32>,
    #[serde(rename = "speakingRate", skip_serializing_if = "Option::is_none")]
    pub speaking_rate: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pitch: Option<f32>,
    #[serde(rename = "volumeGainDb", skip_serializing_if = "Option::is_none")]
    pub volume_gain_db: Option<f32>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct SynthesisResponse {
    #[serde(rename = "audioContent")]
    pub audio_content: String,
    #[serde(rename = "timepoints", skip_serializing_if = "Option::is_none")]
    pub timepoints: Option<Vec<Timepoint>>,
    #[serde(rename = "audioConfig", skip_serializing_if = "Option::is_none")]
    pub audio_config: Option<AudioConfigData>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Timepoint {
    #[serde(rename = "markName", skip_serializing_if = "Option::is_none")]
    pub mark_name: Option<String>,
    #[serde(rename = "timeSeconds")]
    pub time_seconds: f32,
}

#[derive(Deserialize)]
pub struct ListVoicesResponse {
    pub voices: Vec<VoiceResponse>,
}
