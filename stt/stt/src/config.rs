use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct CommonConfig {
    pub endpoint: Option<String>,
    pub timeout_secs: u64,
    pub max_retries: u32,
    pub log_level: Option<String>,
}

impl Default for CommonConfig {
    fn default() -> Self {
        Self {
            endpoint: None,
            timeout_secs: 30,
            max_retries: 3,
            log_level: None,
        }
    }
}

impl CommonConfig {
    pub fn from_env() -> Self {
        Self {
            endpoint: std::env::var("STT_PROVIDER_ENDPOINT").ok(),
            timeout_secs: std::env::var("STT_PROVIDER_TIMEOUT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(30),
            max_retries: std::env::var("STT_PROVIDER_MAX_RETRIES")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3),
            log_level: std::env::var("STT_PROVIDER_LOG_LEVEL").ok(),
        }
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
        self.common
            .endpoint
            .clone()
            .unwrap_or_else(|| "https://api.deepgram.com".to_string())
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
        if let Some(e) = &self.common.endpoint {
            return Ok(e.clone());
        }
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
        }
    }

    pub fn effective_endpoint(&self) -> Result<String, crate::errors::InternalSttError> {
        if let Some(e) = &self.common.endpoint {
            return Ok(e.clone());
        }
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
        self.common.endpoint.clone().unwrap_or_else(|| {
            "https://speech.googleapis.com/v1p1beta1/speech:recognize".to_string()
        })
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
        self.common
            .endpoint
            .clone()
            .unwrap_or_else(|| "https://api.openai.com/v1/audio/transcriptions".to_string())
    }
}
