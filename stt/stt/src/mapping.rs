use serde::{Deserialize, Serialize};

/// Output structs mirror golem:stt types but local, so providers can convert
/// without importing generated bindings in the shared crate.
#[derive(Debug, Serialize, Deserialize)]
pub struct WordSegmentOut {
    pub text: String,
    pub start_time: f32,
    pub end_time: f32,
    pub confidence: Option<f32>,
    pub speaker_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TranscriptAlternativeOut {
    pub text: String,
    pub confidence: f32,
    pub words: Vec<WordSegmentOut>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TranscriptionMetadataOut {
    pub duration_seconds: f32,
    pub audio_size_bytes: u32,
    pub request_id: String,
    pub model: Option<String>,
    pub language: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TranscriptionResultOut {
    pub alternatives: Vec<TranscriptAlternativeOut>,
    pub metadata: TranscriptionMetadataOut,
}
