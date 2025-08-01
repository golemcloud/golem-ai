use crate::golem::stt::types::*;
use crate::golem::stt::transcription::TranscribeOptions;
use serde::{Deserialize, Serialize};

/// Helper functions for working with STT types

impl AudioFormat {
    pub fn to_mime_type(&self) -> &'static str {
        match self {
            AudioFormat::Wav => "audio/wav",
            AudioFormat::Mp3 => "audio/mp3",
            AudioFormat::Flac => "audio/flac",
            AudioFormat::Ogg => "audio/ogg",
            AudioFormat::Aac => "audio/aac",
            AudioFormat::Pcm => "audio/pcm",
        }
    }

    pub fn from_mime_type(mime_type: &str) -> Option<Self> {
        match mime_type.to_lowercase().as_str() {
            "audio/wav" | "audio/wave" => Some(AudioFormat::Wav),
            "audio/mp3" | "audio/mpeg" => Some(AudioFormat::Mp3),
            "audio/flac" => Some(AudioFormat::Flac),
            "audio/ogg" => Some(AudioFormat::Ogg),
            "audio/aac" => Some(AudioFormat::Aac),
            "audio/pcm" => Some(AudioFormat::Pcm),
            _ => None,
        }
    }

    pub fn file_extension(&self) -> &'static str {
        match self {
            AudioFormat::Wav => "wav",
            AudioFormat::Mp3 => "mp3",
            AudioFormat::Flac => "flac",
            AudioFormat::Ogg => "ogg",
            AudioFormat::Aac => "aac",
            AudioFormat::Pcm => "pcm",
        }
    }
}

impl AudioConfig {
    pub fn new(format: AudioFormat) -> Self {
        Self {
            format,
            sample_rate: None,
            channels: None,
        }
    }

    pub fn with_sample_rate(mut self, sample_rate: u32) -> Self {
        self.sample_rate = Some(sample_rate);
        self
    }

    pub fn with_channels(mut self, channels: u8) -> Self {
        self.channels = Some(channels);
        self
    }
}

impl TranscribeOptions {
    pub fn new() -> Self {
        Self {
            enable_timestamps: None,
            enable_speaker_diarization: None,
            language: None,
            model: None,
            profanity_filter: None,
            vocabulary_name: None,
            speech_context: None,
            enable_word_confidence: None,
            enable_timing_detail: None,
        }
    }

    pub fn with_timestamps(mut self, enabled: bool) -> Self {
        self.enable_timestamps = Some(enabled);
        self
    }

    pub fn with_speaker_diarization(mut self, enabled: bool) -> Self {
        self.enable_speaker_diarization = Some(enabled);
        self
    }

    pub fn with_language(mut self, language: String) -> Self {
        self.language = Some(language);
        self
    }

    pub fn with_model(mut self, model: String) -> Self {
        self.model = Some(model);
        self
    }

    pub fn with_word_confidence(mut self, enabled: bool) -> Self {
        self.enable_word_confidence = Some(enabled);
        self
    }

    pub fn with_timing_detail(mut self, enabled: bool) -> Self {
        self.enable_timing_detail = Some(enabled);
        self
    }
}

impl Default for TranscribeOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// Common language codes used in STT
pub mod languages {
    pub const ENGLISH_US: &str = "en-US";
    pub const ENGLISH_UK: &str = "en-GB";
    pub const SPANISH: &str = "es-ES";
    pub const FRENCH: &str = "fr-FR";
    pub const GERMAN: &str = "de-DE";
    pub const ITALIAN: &str = "it-IT";
    pub const PORTUGUESE: &str = "pt-PT";
    pub const JAPANESE: &str = "ja-JP";
    pub const KOREAN: &str = "ko-KR";
    pub const CHINESE_SIMPLIFIED: &str = "zh-CN";
    pub const CHINESE_TRADITIONAL: &str = "zh-TW";
    pub const RUSSIAN: &str = "ru-RU";
    pub const ARABIC: &str = "ar-SA";
    pub const HINDI: &str = "hi-IN";
}

/// Helper structure for request metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestMetadata {
    pub request_id: String,
    pub timestamp: u64,
    pub audio_size_bytes: u32,
    pub provider: String,
}

impl RequestMetadata {
    pub fn new(audio_size_bytes: u32, provider: &str) -> Self {
        // Generate a simple request ID based on timestamp and audio size
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let request_id = format!("{}-{}-{}", provider, timestamp, audio_size_bytes);
        
        Self {
            request_id,
            timestamp,
            audio_size_bytes,
            provider: provider.to_string(),
        }
    }
}