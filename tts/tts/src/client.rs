use reqwest::{Client, Method, Response};
use std::time::Duration;

use reqwest::header::HeaderMap;
use serde::{de::DeserializeOwned, Serialize};

use crate::config::get_parsed_env;
use crate::golem::tts::advanced::{
    AudioSample, LanguageCode, PronunciationEntry, VoiceDesignParams,
};
use crate::golem::tts::synthesis::{
    SynthesisOptions, SynthesisResult, TextInput, TimingInfo, ValidationResult,
};
use crate::golem::tts::types::TtsError;
use crate::golem::tts::voices::{LanguageInfo, Voice, VoiceFilter};

#[derive(Clone)]
pub struct ApiClient {
    pub client: Client,
    pub base_url: String,
    pub rate_limit_config: RateLimitConfig,
    pub auth_headers: HeaderMap,
}

#[derive(Clone)]
pub struct RateLimitConfig {
    pub max_retries: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub backoff_multiplier: f64,
}

impl ApiClient {
    pub fn new(base_url: String, auth_headers: HeaderMap) -> Result<Self, TtsError> {
        let timeout = get_parsed_env("TTS_PROVIDER_TIMEOUT", 30_u64);
        let max_retries = get_parsed_env("TTS_PROVIDER_MAX_RETRIES", 3_u32);

        let rate_limit_config = RateLimitConfig {
            max_retries,
            initial_delay: Duration::from_millis(1000),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
        };

        let client = Client::builder()
            .timeout(Duration::from_secs(timeout))
            .connect_timeout(Duration::from_secs(timeout))
            .default_headers(auth_headers.clone())
            .build()
            .map_err(|err| {
                TtsError::InternalError(format!("Failed to create HTTP client: {err}"))
            })?;

        Ok(Self {
            client,
            base_url,
            rate_limit_config,
            auth_headers,
        })
    }

    pub fn make_request<R: DeserializeOwned, B: Serialize + Clone, Q: Serialize, F>(
        &self,
        method: Method,
        path: &str,
        body: B,
        query_params: Option<&Q>,
        headers: Option<&HeaderMap>,
        handle_error: F,
    ) -> Result<R, TtsError>
    where
        F: Fn(Response) -> TtsError,
    {
        let url = format!("{}{}", self.base_url, path);
        let mut request = self.client.request(method.clone(), &url);

        if let Some(headers) = headers {
            for (key, value) in headers {
                request = request.header(key, value);
            }
        }

        if let Some(query) = query_params {
            request = request.query(query);
        }

        // Only set body for non-GET requests
        if method != Method::GET {
            let string_body = serde_json::to_string(&body).map_err(|e| {
                TtsError::RequestError(format!("Failed to serialize request: {}", e))
            })?;

            request = request.body(string_body);
        }

        match request.send() {
            Ok(response) => {
                if response.status().is_success() {
                    let text = response.text().map_err(|e| {
                        TtsError::InternalError(format!("Failed to parse response to text: {}", e))
                    })?;
                    serde_json::from_str::<R>(&text).map_err(|e| {
                        TtsError::InternalError(format!("Failed to parse response to json: {}", e))
                    })
                } else {
                    Err(handle_error(response))
                }
            }
            Err(err) => Err(TtsError::NetworkError(format!("Request failed: {}", err))),
        }
    }

    pub fn make_audio_request<B: Serialize + Clone, Q: Serialize, F>(
        &self,
        method: Method,
        path: &str,
        body: B,
        query_params: Option<&Q>,
        headers: Option<&HeaderMap>,
        handle_error: F,
    ) -> Result<Vec<u8>, TtsError>
    where
        F: Fn(Response) -> TtsError,
    {
        let string_body = serde_json::to_string(&body)
            .map_err(|e| TtsError::RequestError(format!("Failed to serialize request: {}", e)))?;

        let url = format!("{}{}", self.base_url, path);
        let mut request = self.client.request(method, &url);

        if let Some(headers) = headers {
            for (key, value) in headers {
                request = request.header(key, value);
            }
        }

        if let Some(params) = query_params {
            request = request.query(params);
        }

        match request.body(string_body).send() {
            Ok(response) => {
                if response.status().is_success() {
                    match response.bytes() {
                        Ok(bytes) => Ok(bytes.to_vec()),
                        Err(err) => Err(TtsError::InternalError(format!(
                            "Failed to read audio data: {}",
                            err
                        ))),
                    }
                } else {
                    Err(handle_error(response))
                }
            }
            Err(err) => Err(TtsError::NetworkError(format!("Request failed: {}", err))),
        }
    }

    pub fn retry_request<R: DeserializeOwned, B: Serialize + Clone, Q: Serialize, F>(
        &self,
        method: Method,
        path: &str,
        body: B,
        query_params: Option<&Q>,
        handle_error: F,
    ) -> Result<R, TtsError>
    where
        F: Fn(Response) -> TtsError,
    {
        let mut delay = self.rate_limit_config.initial_delay;

        for attempt in 0..=self.rate_limit_config.max_retries {
            match self.make_request::<R, B, Q, &F>(
                method.clone(),
                path,
                body.clone(),
                query_params,
                None,
                &handle_error,
            ) {
                Ok(result) => return Ok(result),
                Err(TtsError::RateLimited(_)) if attempt < self.rate_limit_config.max_retries => {
                    std::thread::sleep(delay);
                    delay = std::cmp::min(
                        Duration::from_millis(
                            (delay.as_millis() as f64 * self.rate_limit_config.backoff_multiplier)
                                as u64,
                        ),
                        self.rate_limit_config.max_delay,
                    );
                }
                Err(TtsError::ServiceUnavailable(_))
                    if attempt < self.rate_limit_config.max_retries =>
                {
                    std::thread::sleep(delay);
                    delay = std::cmp::min(
                        Duration::from_millis(
                            (delay.as_millis() as f64 * self.rate_limit_config.backoff_multiplier)
                                as u64,
                        ),
                        self.rate_limit_config.max_delay,
                    );
                }
                Err(e) => return Err(e),
            }
        }

        Err(TtsError::RateLimited(429))
    }

    pub fn retry_audio_request<B: Serialize + Clone, Q: Serialize, F>(
        &self,
        method: Method,
        path: &str,
        body: B,
        query_params: Option<&Q>,
        handle_error: F,
    ) -> Result<Vec<u8>, TtsError>
    where
        F: Fn(Response) -> TtsError,
    {
        let mut delay = self.rate_limit_config.initial_delay;

        for attempt in 0..=self.rate_limit_config.max_retries {
            match self.make_audio_request::<B, Q, &F>(
                method.clone(),
                path,
                body.clone(),
                query_params,
                None,
                &handle_error,
            ) {
                Ok(result) => return Ok(result),
                Err(TtsError::RateLimited(_)) if attempt < self.rate_limit_config.max_retries => {
                    std::thread::sleep(delay);
                    delay = std::cmp::min(
                        Duration::from_millis(
                            (delay.as_millis() as f64 * self.rate_limit_config.backoff_multiplier)
                                as u64,
                        ),
                        self.rate_limit_config.max_delay,
                    );
                }
                Err(TtsError::ServiceUnavailable(_))
                    if attempt < self.rate_limit_config.max_retries =>
                {
                    std::thread::sleep(delay);
                    delay = std::cmp::min(
                        Duration::from_millis(
                            (delay.as_millis() as f64 * self.rate_limit_config.backoff_multiplier)
                                as u64,
                        ),
                        self.rate_limit_config.max_delay,
                    );
                }
                Err(e) => return Err(e),
            }
        }

        Err(TtsError::RateLimited(429))
    }
}

pub trait TtsClient: 'static {
    type ClientLongFormOperation;
    type ClientPronunciationLexicon;

    fn new() -> Result<Self, TtsError>
    where
        Self: Sized;

    fn synthesize(
        &self,
        input: TextInput,
        voice: String,
        options: Option<SynthesisOptions>,
    ) -> Result<SynthesisResult, TtsError>;

    fn synthesize_batch(
        &self,
        inputs: Vec<TextInput>,
        voice: String,
        options: Option<SynthesisOptions>,
    ) -> Result<Vec<SynthesisResult>, TtsError>;

    fn get_timing_marks(
        &self,
        input: TextInput,
        voice: String,
    ) -> Result<Vec<TimingInfo>, TtsError>;

    fn validate_input(&self, input: TextInput, voice: String)
        -> Result<ValidationResult, TtsError>;

    fn list_voices(&self, filter: Option<VoiceFilter>) -> Result<Vec<Voice>, TtsError>;

    fn get_voice(&self, voice_id: String) -> Result<Voice, TtsError>;

    fn list_languages(&self) -> Result<Vec<LanguageInfo>, TtsError>;

    fn create_voice_clone(
        &self,
        name: String,
        audio_samples: Vec<AudioSample>,
        description: Option<String>,
    ) -> Result<Voice, TtsError>;

    fn design_voice(
        &self,
        name: String,
        characteristics: VoiceDesignParams,
    ) -> Result<Voice, TtsError>;

    fn convert_voice(
        &self,
        input_audio: Vec<u8>,
        target_voice: String,
        preserve_timing: Option<bool>,
    ) -> Result<Vec<u8>, TtsError>;

    fn generate_sound_effect(
        &self,
        description: String,
        duration_seconds: Option<f32>,
        style_influence: Option<f32>,
    ) -> Result<Vec<u8>, TtsError>;

    fn create_lexicon(
        &self,
        name: String,
        language: LanguageCode,
        entries: Option<Vec<PronunciationEntry>>,
    ) -> Result<Self::ClientPronunciationLexicon, TtsError>;

    fn synthesize_long_form(
        &self,
        content: String,
        voice: String,
        chapter_breaks: Option<Vec<u32>>,
    ) -> Result<Self::ClientLongFormOperation, TtsError>;
}
