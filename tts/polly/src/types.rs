use serde::{Deserialize, Serialize};

use crate::resources::{ VoiceResponse};

#[derive(Serialize, Clone)]
pub struct SynthesizeSpeechParams {
    #[serde(rename = "Engine", skip_serializing_if = "Option::is_none")]
    pub engine: Option<String>,
    #[serde(rename = "LanguageCode", skip_serializing_if = "Option::is_none")]
    pub language_code: Option<String>,
    #[serde(rename = "LexiconNames", skip_serializing_if = "Option::is_none")]
    pub lexicon_names: Option<Vec<String>>,
    #[serde(rename = "OutputFormat", skip_serializing_if = "Option::is_none")]
    pub output_format: Option<String>,
    #[serde(rename = "SampleRate", skip_serializing_if = "Option::is_none")]
    pub sample_rate: Option<String>,
    #[serde(rename = "SpeechMarkTypes", skip_serializing_if = "Option::is_none")]
    pub speech_mark_types: Option<Vec<String>>,
    #[serde(rename = "Text")]
    pub text: String,
    #[serde(rename = "TextType", skip_serializing_if = "Option::is_none")]
    pub text_type: Option<String>,
    #[serde(rename = "VoiceId")]
    pub voice_id: String,
}

#[derive(Deserialize, Debug)]
pub struct SynthesizeSpeechResponse {
    pub audio_stream: Vec<u8>,
    #[serde(default)]
    pub content_type: String,
    #[serde(default)]
    pub request_characters: u32,
}

#[derive(Serialize, Clone)]
pub struct PutLexiconRequest {
    #[serde(rename = "Content")]
    pub content: String,
    #[serde(rename = "Name")]
    pub name: String,
}

#[derive(Serialize, Clone)]
pub struct StartSpeechSynthesisTaskRequest {
    #[serde(rename = "Engine", skip_serializing_if = "Option::is_none")]
    pub engine: Option<String>,
    #[serde(rename = "LanguageCode", skip_serializing_if = "Option::is_none")]
    pub language_code: Option<String>,
    #[serde(rename = "LexiconNames", skip_serializing_if = "Option::is_none")]
    pub lexicon_names: Option<Vec<String>>,
    #[serde(rename = "OutputFormat")]
    pub output_format: String,
    #[serde(rename = "OutputS3BucketName")]
    pub output_s3_bucket_name: String,
    #[serde(rename = "OutputS3KeyPrefix", skip_serializing_if = "Option::is_none")]
    pub output_s3_key_prefix: Option<String>,
    #[serde(rename = "SampleRate", skip_serializing_if = "Option::is_none")]
    pub sample_rate: Option<String>,
    #[serde(rename = "SnsTopicArn", skip_serializing_if = "Option::is_none")]
    pub sns_topic_arn: Option<String>,
    #[serde(rename = "SpeechMarkTypes", skip_serializing_if = "Option::is_none")]
    pub speech_mark_types: Option<Vec<String>>,
    #[serde(rename = "Text")]
    pub text: String,
    #[serde(rename = "TextType", skip_serializing_if = "Option::is_none")]
    pub text_type: Option<String>,
    #[serde(rename = "VoiceId")]
    pub voice_id: String,
}

#[derive(Deserialize, Debug)]
pub struct SpeechMark {
    pub time: u32,
    pub r#type: String,
    pub start: Option<u32>,
    pub end: Option<u32>,
    pub value: String,
}

#[derive(Deserialize, Debug)]
pub struct GetLexiconResponse {
    #[serde(rename = "Lexicon")]
    pub lexicon: AwsLexicon,
    #[serde(rename = "LexiconAttributes")]
    pub lexicon_attributes: LexiconAttributes,
}

#[derive(Deserialize, Debug, Clone)]
pub struct AwsLexicon {
    #[serde(rename = "Content")]
    pub content: String,
    #[serde(rename = "Name")]
    pub name: String,
}

#[derive(Deserialize, Debug)]
pub struct LexiconAttributes {
    #[serde(rename = "Alphabet")]
    pub alphabet: String,
    #[serde(rename = "LanguageCode")]
    pub language_code: String,
    #[serde(rename = "LastModified")]
    pub last_modified: f64,
    #[serde(rename = "LexiconArn")]
    pub lexicon_arn: String,
    #[serde(rename = "LexemesCount")]
    pub lexemes_count: u32,
    #[serde(rename = "Size")]
    pub size: u32,
}

#[derive(Deserialize, Debug)]
pub struct StartSpeechSynthesisTaskResponse {
    #[serde(rename = "SynthesisTask")]
    pub synthesis_task: SynthesisTask,
}

#[derive(Deserialize, Debug)]
pub struct GetSpeechSynthesisTaskResponse {
    #[serde(rename = "SynthesisTask")]
    pub synthesis_task: SynthesisTask,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SynthesisTask {
    #[serde(rename = "TaskId")]
    pub task_id: String,
    #[serde(rename = "TaskStatus")]
    pub task_status: String,
    #[serde(rename = "TaskStatusReason", skip_serializing_if = "Option::is_none")]
    pub task_status_reason: Option<String>,
    #[serde(rename = "OutputUri", skip_serializing_if = "Option::is_none")]
    pub output_uri: Option<String>,
    #[serde(rename = "CreationTime")]
    pub creation_time: f64,  // Unix timestamp as floating-point number
    #[serde(rename = "RequestCharacters")]
    pub request_characters: u32,
    #[serde(rename = "VoiceId")]
    pub voice_id: String,
    #[serde(rename = "Engine")]
    pub engine: String,
    #[serde(rename = "OutputFormat")]
    pub output_format: String,
}

#[derive(Serialize)]
pub struct ListVoiceParam {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engine: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_additional_language_codes: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub language_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_token: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct ListVoiceResponse {
    #[serde(rename = "NextToken")]
    pub next_token: Option<String>,
    #[serde(rename = "Voices")]
    pub voices: Option<Vec<VoiceResponse>>,
}
