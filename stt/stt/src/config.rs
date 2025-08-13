use crate::exports::golem::stt::languages::LanguageInfo;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct CommonConfig {
    pub timeout_secs: u64,
    pub max_retries: u32,
    pub log_level: Option<String>,
    pub max_audio_size_mb: u32,
}

impl Default for CommonConfig {
    fn default() -> Self {
        Self {
            timeout_secs: 30,
            max_retries: 3,
            log_level: None,
            max_audio_size_mb: 100, // 100MB default limit
        }
    }
}

impl CommonConfig {
    pub fn from_env() -> Self {
        Self {
            timeout_secs: std::env::var("STT_PROVIDER_TIMEOUT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(30),
            max_retries: std::env::var("STT_PROVIDER_MAX_RETRIES")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3),
            log_level: std::env::var("STT_PROVIDER_LOG_LEVEL").ok(),
            max_audio_size_mb: std::env::var("STT_MAX_AUDIO_SIZE_MB")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(100),
        }
    }

    pub fn validate_audio_size(&self, audio: &[u8]) -> Result<(), crate::errors::InternalSttError> {
        let size_mb = audio.len() as f64 / (1024.0 * 1024.0);
        if size_mb > self.max_audio_size_mb as f64 {
            return Err(crate::errors::InternalSttError::InvalidAudio(format!(
                "Audio file too large: {:.1}MB exceeds limit of {}MB",
                size_mb, self.max_audio_size_mb
            )));
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct DeepgramConfig {
    pub common: CommonConfig,
    pub api_key: Option<String>,
}

impl DeepgramConfig {
    pub fn from_env() -> Self {
        let common = CommonConfig::from_env();
        Self {
            common,
            api_key: std::env::var("DEEPGRAM_API_KEY").ok(),
        }
    }

    pub fn effective_endpoint(&self) -> String {
        // Always use the real Deepgram API
        "https://api.deepgram.com".to_string()
    }
}

#[derive(Debug, Clone)]
pub struct AzureConfig {
    pub common: CommonConfig,
    pub speech_key: Option<String>,
    pub speech_region: Option<String>,
}

impl AzureConfig {
    pub fn from_env() -> Self {
        let common = CommonConfig::from_env();
        Self {
            common,
            speech_key: std::env::var("AZURE_SPEECH_KEY").ok(),
            speech_region: std::env::var("AZURE_SPEECH_REGION").ok(),
        }
    }

    pub fn effective_endpoint(&self) -> Result<String, crate::errors::InternalSttError> {
        // Always use the real Azure Speech Services API
        let region = self.speech_region.as_ref().ok_or_else(|| {
            crate::errors::InternalSttError::unauthorized("AZURE_SPEECH_REGION not set")
        })?;
        Ok(format!(
            "https://{region}.stt.speech.microsoft.com/speech/recognition/conversation/cognitiveservices/v1"
        ))
    }
}

#[derive(Debug, Clone)]
pub struct AwsConfig {
    pub common: CommonConfig,
    pub access_key_id: Option<String>,
    pub secret_access_key: Option<String>,
    pub session_token: Option<String>,
    pub region: Option<String>,
    pub s3_bucket: Option<String>,
}

impl AwsConfig {
    pub fn from_env() -> Self {
        let common = CommonConfig::from_env();
        Self {
            common,
            access_key_id: std::env::var("AWS_ACCESS_KEY_ID").ok(),
            secret_access_key: std::env::var("AWS_SECRET_ACCESS_KEY").ok(),
            session_token: std::env::var("AWS_SESSION_TOKEN").ok(),
            region: std::env::var("AWS_REGION").ok(),
            s3_bucket: std::env::var("AWS_S3_BUCKET").ok(),
        }
    }

    pub fn effective_endpoint(&self) -> Result<String, crate::errors::InternalSttError> {
        // Always use the real AWS Transcribe API
        let region = self
            .region
            .as_ref()
            .ok_or_else(|| crate::errors::InternalSttError::unauthorized("AWS_REGION not set"))?;
        Ok(format!("https://transcribe.{region}.amazonaws.com"))
    }
}

#[derive(Debug, Clone)]
pub struct GoogleConfig {
    pub common: CommonConfig,
    pub application_credentials: Option<String>,
    pub cloud_project: Option<String>,
    pub access_token: Option<String>,
}

impl GoogleConfig {
    pub fn from_env() -> Self {
        let common = CommonConfig::from_env();
        Self {
            common,
            application_credentials: std::env::var("GOOGLE_APPLICATION_CREDENTIALS").ok(),
            cloud_project: std::env::var("GOOGLE_CLOUD_PROJECT").ok(),
            access_token: std::env::var("GOOGLE_ACCESS_TOKEN").ok(),
        }
    }

    pub fn effective_endpoint(&self) -> String {
        // Always use the real Google Cloud Speech-to-Text API
        "https://speech.googleapis.com/v1p1beta1/speech:recognize".to_string()
    }
}

#[derive(Debug, Clone)]
pub struct WhisperConfig {
    pub common: CommonConfig,
    pub api_key: Option<String>,
}

impl WhisperConfig {
    pub fn from_env() -> Self {
        let common = CommonConfig::from_env();
        Self {
            common,
            api_key: std::env::var("OPENAI_API_KEY").ok(),
        }
    }

    pub fn effective_endpoint(&self) -> String {
        // Always use the real OpenAI Whisper API
        "https://api.openai.com/v1/audio/transcriptions".to_string()
    }
}

/// Standard language list that all STT components should support
/// This ensures consistency across providers while covering the most common languages
pub fn standard_language_list() -> Vec<LanguageInfo> {
    vec![
        // English variants
        LanguageInfo {
            code: "en-US".into(),
            name: "English (US)".into(),
            native_name: "English (US)".into(),
        },
        LanguageInfo {
            code: "en-GB".into(),
            name: "English (UK)".into(),
            native_name: "English (UK)".into(),
        },
        // Major European languages
        LanguageInfo {
            code: "es-ES".into(),
            name: "Spanish".into(),
            native_name: "Español".into(),
        },
        LanguageInfo {
            code: "fr-FR".into(),
            name: "French".into(),
            native_name: "Français".into(),
        },
        LanguageInfo {
            code: "de-DE".into(),
            name: "German".into(),
            native_name: "Deutsch".into(),
        },
        LanguageInfo {
            code: "it-IT".into(),
            name: "Italian".into(),
            native_name: "Italiano".into(),
        },
        LanguageInfo {
            code: "pt-BR".into(),
            name: "Portuguese (Brazil)".into(),
            native_name: "Português (Brasil)".into(),
        },
        // Major Asian languages
        LanguageInfo {
            code: "ja-JP".into(),
            name: "Japanese".into(),
            native_name: "日本語".into(),
        },
        LanguageInfo {
            code: "ko-KR".into(),
            name: "Korean".into(),
            native_name: "한국어".into(),
        },
        LanguageInfo {
            code: "zh-CN".into(),
            name: "Chinese (Simplified)".into(),
            native_name: "中文 (简体)".into(),
        },
    ]
}
