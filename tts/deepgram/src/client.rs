use golem_tts::config::{get_endpoint_config, get_max_retries_config, get_timeout_config};
use golem_tts::error::{from_reqwest_error, tts_error_from_status};
use golem_tts::golem::tts::types::TtsError;
use log::trace;
use reqwest::{Client, Method, RequestBuilder, Response};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::time::Duration;

/// Rate limiting configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    pub max_retries: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub backoff_multiplier: f64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_retries: get_max_retries_config(),
            initial_delay: Duration::from_millis(1000),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
        }
    }
}

/// Deepgram TTS API client
/// Based on https://developers.deepgram.com/docs/text-to-speech
#[derive(Clone)]
pub struct DeepgramTtsApi {
    client: Client,
    api_key: String,
    base_url: String,
    api_version: String,
    rate_limit_config: RateLimitConfig,
}

impl DeepgramTtsApi {
    pub fn new(api_key: String, api_version: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(get_timeout_config()))
            .build()
            .unwrap();

        let base_url = get_endpoint_config("https://api.deepgram.com");

        Self {
            client,
            api_key,
            base_url,
            api_version,
            rate_limit_config: RateLimitConfig::default(),
        }
    }

    pub fn with_rate_limit_config(mut self, config: RateLimitConfig) -> Self {
        self.rate_limit_config = config;
        self
    }

    fn create_request(&self, method: Method, url: &str) -> RequestBuilder {
        self.client
            .request(method, url)
            .header("Authorization", format!("Token {}", self.api_key))
            .header("Content-Type", "application/json")
    }

    fn execute_with_retry<F>(&self, operation: F) -> Result<Response, TtsError>
    where
        F: Fn() -> Result<Response, TtsError>,
    {
        let mut delay = self.rate_limit_config.initial_delay;
        let max_retries = self.rate_limit_config.max_retries;

        for attempt in 0..=max_retries {
            match operation() {
                Ok(response) => {
                    if response.status().is_success() {
                        if attempt > 0 {
                            trace!("Deepgram TTS request succeeded after {} retries", attempt);
                        }
                        return Ok(response);
                    } else if response.status().as_u16() == 429 && attempt < max_retries {
                        let wait_time = response
                            .headers()
                            .get("retry-after")
                            .and_then(|h| h.to_str().ok())
                            .and_then(|s| s.parse::<u64>().ok())
                            .map(Duration::from_secs)
                            .unwrap_or(delay);

                        trace!(
                            "Deepgram API rate limited (429), waiting {}ms before retry {} of {}",
                            wait_time.as_millis(),
                            attempt + 1,
                            max_retries
                        );

                        std::thread::sleep(wait_time);
                        delay = std::cmp::min(
                            Duration::from_millis(
                                (delay.as_millis() as f64
                                    * self.rate_limit_config.backoff_multiplier)
                                    as u64,
                            ),
                            self.rate_limit_config.max_delay,
                        );
                        continue;
                    } else if response.status().as_u16() >= 500 && attempt < max_retries {
                        trace!(
                            "Deepgram API server error ({}), waiting {}ms before retry {} of {}",
                            response.status().as_u16(),
                            delay.as_millis(),
                            attempt + 1,
                            max_retries
                        );

                        std::thread::sleep(delay);
                        delay = std::cmp::min(
                            Duration::from_millis(
                                (delay.as_millis() as f64
                                    * self.rate_limit_config.backoff_multiplier)
                                    as u64,
                            ),
                            self.rate_limit_config.max_delay,
                        );
                        continue;
                    } else {
                        if attempt > 0 && response.status().as_u16() == 429 {
                            return Err(TtsError::RateLimited(delay.as_secs() as u32));
                        }
                        return Err(tts_error_from_status(response.status()));
                    }
                }
                Err(e) => {
                    if attempt < max_retries {
                        trace!(
                            "Deepgram API network error, waiting {}ms before retry {} of {}: {}",
                            delay.as_millis(),
                            attempt + 1,
                            max_retries,
                            e
                        );

                        std::thread::sleep(delay);
                        delay = std::cmp::min(
                            Duration::from_millis(
                                (delay.as_millis() as f64
                                    * self.rate_limit_config.backoff_multiplier)
                                    as u64,
                            ),
                            self.rate_limit_config.max_delay,
                        );
                        continue;
                    } else {
                        return Err(TtsError::NetworkError(format!(
                            "Deepgram API network error after {} retries: {}",
                            max_retries, e
                        )));
                    }
                }
            }
        }

        Err(TtsError::InternalError("Max retries exceeded".to_string()))
    }

    pub fn text_to_speech(
        &self,
        request: &TextToSpeechRequest,
        params: Option<&TextToSpeechParams>,
    ) -> Result<Vec<u8>, TtsError> {
        let response = self.text_to_speech_with_metadata(request, params)?;
        Ok(response.audio_data)
    }

    pub fn text_to_speech_with_metadata(
        &self,
        request: &TextToSpeechRequest,
        params: Option<&TextToSpeechParams>,
    ) -> Result<TtsResponse, TtsError> {
        let url = if let Some(p) = params {
            format!(
                "{}/{}/speak?{}",
                self.base_url,
                self.api_version,
                p.to_query_string()
            )
        } else {
            format!("{}/{}/speak", self.base_url, self.api_version)
        };

        trace!("Making TTS request to: {}", url);

        let request_clone = request.clone();

        let operation = || {
            let req = self.create_request(Method::POST, &url).json(&request_clone);

            match req.send() {
                Ok(response) => Ok(response),
                Err(e) => Err(from_reqwest_error("TTS request failed", e)),
            }
        };

        let response = self.execute_with_retry(operation)?;

        if !response.status().is_success() {
            return Err(tts_error_from_status(response.status()));
        }

        let metadata =
            TtsResponseMetadata::from_response_headers(response.headers()).ok_or_else(|| {
                TtsError::InternalError("Missing required response headers".to_string())
            })?;

        match response.bytes() {
            Ok(bytes) => Ok(TtsResponse {
                audio_data: bytes.to_vec(),
                metadata,
            }),
            Err(e) => Err(from_reqwest_error("Failed to read response bytes", e)),
        }
    }

    pub fn text_to_speech_stream(
        &self,
        request: &TextToSpeechRequest,
        params: Option<&TextToSpeechParams>,
    ) -> Result<reqwest::Response, TtsError> {
        let url = if let Some(p) = params {
            format!(
                "{}/{}/speak?{}",
                self.base_url,
                self.api_version,
                p.to_query_string()
            )
        } else {
            format!("{}/{}/speak", self.base_url, self.api_version)
        };

        trace!("Making streaming TTS request to: {}", url);

        let request_clone = request.clone();

        let operation = || {
            let req = self.create_request(Method::POST, &url).json(&request_clone);

            match req.send() {
                Ok(response) => Ok(response),
                Err(e) => Err(from_reqwest_error("Streaming TTS request failed", e)),
            }
        };

        let response = self.execute_with_retry(operation)?;

        if !response.status().is_success() {
            return Err(tts_error_from_status(response.status()));
        }

        Ok(response)
    }

    pub fn get_models_filtered(
        &self,
        filters: &VoiceFilters,
    ) -> Result<ModelListResponse, TtsError> {
        let models = get_available_models();
        Ok(ModelListResponse::filter_by(models, filters))
    }

    pub fn search_models(&self, query: &str) -> Result<Vec<Model>, TtsError> {
        let filters = VoiceFilters::new().with_search(query.to_string());
        let response = self.get_models_filtered(&filters)?;
        Ok(response.models)
    }
}

#[derive(Debug, Clone)]
pub struct TtsResponseMetadata {
    #[allow(dead_code)]
    pub content_type: String,
    #[allow(dead_code)]
    pub dg_request_id: String,
    pub dg_model_name: String,
    #[allow(dead_code)]
    pub dg_model_uuid: String,
    pub dg_char_count: u32,
    #[allow(dead_code)]
    pub content_length: Option<u64>,
    #[allow(dead_code)]
    pub date: String,
}

impl TtsResponseMetadata {
    pub fn from_response_headers(headers: &reqwest::header::HeaderMap) -> Option<Self> {
        Some(Self {
            content_type: headers.get("content-type")?.to_str().ok()?.to_string(),
            dg_request_id: headers.get("dg-request-id")?.to_str().ok()?.to_string(),
            dg_model_name: headers.get("dg-model-name")?.to_str().ok()?.to_string(),
            dg_model_uuid: headers.get("dg-model-uuid")?.to_str().ok()?.to_string(),
            dg_char_count: headers.get("dg-char-count")?.to_str().ok()?.parse().ok()?,
            content_length: headers
                .get("content-length")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse().ok()),
            date: headers
                .get("date")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("Unknown")
                .to_string(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct TtsResponse {
    pub audio_data: Vec<u8>,
    pub metadata: TtsResponseMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextToSpeechRequest {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextToSpeechParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample_rate: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bit_rate: Option<u32>,
}

impl Default for TextToSpeechParams {
    fn default() -> Self {
        Self {
            model: None,
            encoding: Some("linear16".to_string()),
            container: None,
            sample_rate: Some(24000),
            bit_rate: None,
        }
    }
}

impl TextToSpeechParams {
    pub fn to_query_string(&self) -> String {
        let mut params = Vec::new();

        if let Some(model) = &self.model {
            params.push(format!("model={}", model));
        }
        if let Some(encoding) = &self.encoding {
            params.push(format!("encoding={}", encoding));
        }
        if let Some(container) = &self.container {
            params.push(format!("container={}", container));
        }
        if let Some(sample_rate) = self.sample_rate {
            params.push(format!("sample_rate={}", sample_rate));
        }
        if let Some(bit_rate) = self.bit_rate {
            params.push(format!("bit_rate={}", bit_rate));
        }

        params.join("&")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    pub name: String,
    pub voice_id: String,
    pub language: String,
    pub accent: String,
    pub gender: String,
    pub age: String,
    pub characteristics: Vec<String>,
    pub use_cases: Vec<String>,
    pub version: ModelVersion,
    pub quality: VoiceQuality,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ModelVersion {
    Aura1,
    Aura2,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VoiceQuality {
    Standard,
    Premium,
    Professional,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Voice {
    pub voice_id: String,
    pub name: String,
    pub language: String,
    pub accent: String,
    pub gender: String,
    pub age: String,
    pub characteristics: Vec<String>,
    pub use_cases: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct VoiceFilters {
    pub language: Option<String>,
    pub gender: Option<String>,
    pub accent: Option<String>,
    pub version: Option<ModelVersion>,
    pub quality: Option<VoiceQuality>,
    pub search_query: Option<String>,
}

impl VoiceFilters {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn with_language(mut self, language: String) -> Self {
        self.language = Some(language);
        self
    }

    pub fn with_gender(mut self, gender: String) -> Self {
        self.gender = Some(gender);
        self
    }

    pub fn with_version(mut self, version: ModelVersion) -> Self {
        self.version = Some(version);
        self
    }

    pub fn with_search(mut self, query: String) -> Self {
        self.search_query = Some(query);
        self
    }
}

#[derive(Debug, Clone)]
pub struct ModelListResponse {
    pub models: Vec<Model>,
    #[allow(dead_code)]
    pub total_count: usize,
    #[allow(dead_code)]
    pub filtered_count: usize,
}

impl ModelListResponse {
    pub fn filter_by(models: Vec<Model>, filters: &VoiceFilters) -> Self {
        let total_count = models.len();

        let filtered_models: Vec<Model> = models
            .into_iter()
            .filter(|model| {
                if let Some(ref lang) = filters.language {
                    if !model.language.to_lowercase().contains(&lang.to_lowercase()) {
                        return false;
                    }
                }

                if let Some(ref gender) = filters.gender {
                    if !model.gender.to_lowercase().contains(&gender.to_lowercase()) {
                        return false;
                    }
                }

                if let Some(ref accent) = filters.accent {
                    if !model.accent.to_lowercase().contains(&accent.to_lowercase()) {
                        return false;
                    }
                }

                if let Some(ref version) = filters.version {
                    if model.version != *version {
                        return false;
                    }
                }

                if let Some(ref quality) = filters.quality {
                    if model.quality != *quality {
                        return false;
                    }
                }

                if let Some(ref query) = filters.search_query {
                    let query_lower = query.to_lowercase();
                    let matches_name = model.name.to_lowercase().contains(&query_lower);
                    let matches_characteristics = model
                        .characteristics
                        .iter()
                        .any(|c| c.to_lowercase().contains(&query_lower));
                    let matches_use_cases = model
                        .use_cases
                        .iter()
                        .any(|u| u.to_lowercase().contains(&query_lower));

                    if !matches_name && !matches_characteristics && !matches_use_cases {
                        return false;
                    }
                }

                true
            })
            .collect();

        let filtered_count = filtered_models.len();

        Self {
            models: filtered_models,
            total_count,
            filtered_count,
        }
    }
}

/// Based on the documentation at https://developers.deepgram.com/docs/tts-models
pub fn get_available_models() -> Vec<Model> {
    vec![
        // Aura-2 Featured English Voices
        Model {
            name: "thalia".to_string(),
            voice_id: "aura-2-thalia-en".to_string(),
            language: "en-us".to_string(),
            accent: "American".to_string(),
            gender: "feminine".to_string(),
            age: "Adult".to_string(),
            characteristics: vec![
                "Clear".to_string(),
                "Confident".to_string(),
                "Energetic".to_string(),
                "Enthusiastic".to_string(),
            ],
            use_cases: vec![
                "Casual chat".to_string(),
                "Customer service".to_string(),
                "IVR".to_string(),
            ],
            version: ModelVersion::Aura2,
            quality: VoiceQuality::Premium,
        },
        Model {
            name: "andromeda".to_string(),
            voice_id: "aura-2-andromeda-en".to_string(),
            language: "en-us".to_string(),
            accent: "American".to_string(),
            gender: "feminine".to_string(),
            age: "Adult".to_string(),
            characteristics: vec![
                "Casual".to_string(),
                "Expressive".to_string(),
                "Comfortable".to_string(),
            ],
            use_cases: vec!["Customer service".to_string(), "IVR".to_string()],
            version: ModelVersion::Aura2,
            quality: VoiceQuality::Premium,
        },
        Model {
            name: "helena".to_string(),
            voice_id: "aura-2-helena-en".to_string(),
            language: "en-us".to_string(),
            accent: "American".to_string(),
            gender: "feminine".to_string(),
            age: "Adult".to_string(),
            characteristics: vec![
                "Caring".to_string(),
                "Natural".to_string(),
                "Positive".to_string(),
                "Friendly".to_string(),
                "Raspy".to_string(),
            ],
            use_cases: vec!["IVR".to_string(), "Casual chat".to_string()],
            version: ModelVersion::Aura2,
            quality: VoiceQuality::Premium,
        },
        Model {
            name: "apollo".to_string(),
            voice_id: "aura-2-apollo-en".to_string(),
            language: "en-us".to_string(),
            accent: "American".to_string(),
            gender: "masculine".to_string(),
            age: "Adult".to_string(),
            characteristics: vec![
                "Confident".to_string(),
                "Comfortable".to_string(),
                "Casual".to_string(),
            ],
            use_cases: vec!["Casual chat".to_string()],
            version: ModelVersion::Aura2,
            quality: VoiceQuality::Premium,
        },
        Model {
            name: "arcas".to_string(),
            voice_id: "aura-2-arcas-en".to_string(),
            language: "en-us".to_string(),
            accent: "American".to_string(),
            gender: "masculine".to_string(),
            age: "Adult".to_string(),
            characteristics: vec![
                "Natural".to_string(),
                "Smooth".to_string(),
                "Clear".to_string(),
                "Comfortable".to_string(),
            ],
            use_cases: vec!["Customer service".to_string(), "Casual chat".to_string()],
            version: ModelVersion::Aura2,
            quality: VoiceQuality::Premium,
        },
        Model {
            name: "aries".to_string(),
            voice_id: "aura-2-aries-en".to_string(),
            language: "en-us".to_string(),
            accent: "American".to_string(),
            gender: "masculine".to_string(),
            age: "Adult".to_string(),
            characteristics: vec![
                "Warm".to_string(),
                "Energetic".to_string(),
                "Caring".to_string(),
            ],
            use_cases: vec!["Casual chat".to_string()],
            version: ModelVersion::Aura2,
            quality: VoiceQuality::Premium,
        },
        // Additional Aura-2 English voices
        Model {
            name: "asteria".to_string(),
            voice_id: "aura-2-asteria-en".to_string(),
            language: "en-us".to_string(),
            accent: "American".to_string(),
            gender: "feminine".to_string(),
            age: "Adult".to_string(),
            characteristics: vec![
                "Clear".to_string(),
                "Confident".to_string(),
                "Knowledgeable".to_string(),
                "Energetic".to_string(),
            ],
            use_cases: vec!["Advertising".to_string()],
            version: ModelVersion::Aura2,
            quality: VoiceQuality::Professional,
        },
        Model {
            name: "athena".to_string(),
            voice_id: "aura-2-athena-en".to_string(),
            language: "en-us".to_string(),
            accent: "American".to_string(),
            gender: "feminine".to_string(),
            age: "Mature".to_string(),
            characteristics: vec![
                "Calm".to_string(),
                "Smooth".to_string(),
                "Professional".to_string(),
            ],
            use_cases: vec!["Storytelling".to_string()],
            version: ModelVersion::Aura2,
            quality: VoiceQuality::Professional,
        },
        Model {
            name: "atlas".to_string(),
            voice_id: "aura-2-atlas-en".to_string(),
            language: "en-us".to_string(),
            accent: "American".to_string(),
            gender: "masculine".to_string(),
            age: "Mature".to_string(),
            characteristics: vec![
                "Enthusiastic".to_string(),
                "Confident".to_string(),
                "Approachable".to_string(),
                "Friendly".to_string(),
            ],
            use_cases: vec!["Advertising".to_string()],
            version: ModelVersion::Aura2,
            quality: VoiceQuality::Professional,
        },
        // British voice
        Model {
            name: "draco".to_string(),
            voice_id: "aura-2-draco-en".to_string(),
            language: "en-gb".to_string(),
            accent: "British".to_string(),
            gender: "masculine".to_string(),
            age: "Adult".to_string(),
            characteristics: vec![
                "Warm".to_string(),
                "Approachable".to_string(),
                "Trustworthy".to_string(),
                "Baritone".to_string(),
            ],
            use_cases: vec!["Storytelling".to_string()],
            version: ModelVersion::Aura2,
            quality: VoiceQuality::Premium,
        },
        // Australian voice
        Model {
            name: "hyperion".to_string(),
            voice_id: "aura-2-hyperion-en".to_string(),
            language: "en-au".to_string(),
            accent: "Australian".to_string(),
            gender: "masculine".to_string(),
            age: "Adult".to_string(),
            characteristics: vec![
                "Caring".to_string(),
                "Warm".to_string(),
                "Empathetic".to_string(),
            ],
            use_cases: vec!["Interview".to_string()],
            version: ModelVersion::Aura2,
            quality: VoiceQuality::Premium,
        },
        // Filipino voice
        Model {
            name: "amalthea".to_string(),
            voice_id: "aura-2-amalthea-en".to_string(),
            language: "en-ph".to_string(),
            accent: "Filipino".to_string(),
            gender: "feminine".to_string(),
            age: "Young Adult".to_string(),
            characteristics: vec![
                "Engaging".to_string(),
                "Natural".to_string(),
                "Cheerful".to_string(),
            ],
            use_cases: vec!["Casual chat".to_string()],
            version: ModelVersion::Aura2,
            quality: VoiceQuality::Premium,
        },
        // Spanish voices
        Model {
            name: "celeste".to_string(),
            voice_id: "aura-2-celeste-es".to_string(),
            language: "es-co".to_string(),
            accent: "Colombian".to_string(),
            gender: "feminine".to_string(),
            age: "Young Adult".to_string(),
            characteristics: vec![
                "Clear".to_string(),
                "Energetic".to_string(),
                "Positive".to_string(),
                "Friendly".to_string(),
                "Enthusiastic".to_string(),
            ],
            use_cases: vec![
                "Casual Chat".to_string(),
                "Advertising".to_string(),
                "IVR".to_string(),
            ],
            version: ModelVersion::Aura2,
            quality: VoiceQuality::Premium,
        },
        Model {
            name: "estrella".to_string(),
            voice_id: "aura-2-estrella-es".to_string(),
            language: "es-mx".to_string(),
            accent: "Mexican".to_string(),
            gender: "feminine".to_string(),
            age: "Mature".to_string(),
            characteristics: vec![
                "Approachable".to_string(),
                "Natural".to_string(),
                "Calm".to_string(),
                "Comfortable".to_string(),
                "Expressive".to_string(),
            ],
            use_cases: vec!["Casual Chat".to_string(), "Interview".to_string()],
            version: ModelVersion::Aura2,
            quality: VoiceQuality::Premium,
        },
        Model {
            name: "nestor".to_string(),
            voice_id: "aura-2-nestor-es".to_string(),
            language: "es-es".to_string(),
            accent: "Peninsular".to_string(),
            gender: "masculine".to_string(),
            age: "Adult".to_string(),
            characteristics: vec![
                "Calm".to_string(),
                "Professional".to_string(),
                "Approachable".to_string(),
                "Clear".to_string(),
                "Confident".to_string(),
            ],
            use_cases: vec!["Casual Chat".to_string(), "Customer Service".to_string()],
            version: ModelVersion::Aura2,
            quality: VoiceQuality::Premium,
        },
        // Aura-1 legacy voices
        Model {
            name: "asteria-v1".to_string(),
            voice_id: "aura-asteria-en".to_string(),
            language: "en-us".to_string(),
            accent: "American".to_string(),
            gender: "feminine".to_string(),
            age: "Adult".to_string(),
            characteristics: vec![
                "Clear".to_string(),
                "Confident".to_string(),
                "Knowledgeable".to_string(),
                "Energetic".to_string(),
            ],
            use_cases: vec!["Advertising".to_string()],
            version: ModelVersion::Aura1,
            quality: VoiceQuality::Standard,
        },
        Model {
            name: "luna-v1".to_string(),
            voice_id: "aura-luna-en".to_string(),
            language: "en-us".to_string(),
            accent: "American".to_string(),
            gender: "feminine".to_string(),
            age: "Young Adult".to_string(),
            characteristics: vec![
                "Friendly".to_string(),
                "Natural".to_string(),
                "Engaging".to_string(),
            ],
            use_cases: vec!["IVR".to_string()],
            version: ModelVersion::Aura1,
            quality: VoiceQuality::Standard,
        },
        Model {
            name: "stella".to_string(),
            voice_id: "aura-stella-en".to_string(),
            language: "en-us".to_string(),
            accent: "American".to_string(),
            gender: "feminine".to_string(),
            age: "Adult".to_string(),
            characteristics: vec![
                "Clear".to_string(),
                "Professional".to_string(),
                "Engaging".to_string(),
            ],
            use_cases: vec!["Customer service".to_string()],
            version: ModelVersion::Aura1,
            quality: VoiceQuality::Standard,
        },
        Model {
            name: "athena-v1".to_string(),
            voice_id: "aura-athena-en".to_string(),
            language: "en-gb".to_string(),
            accent: "British".to_string(),
            gender: "feminine".to_string(),
            age: "Mature".to_string(),
            characteristics: vec![
                "Calm".to_string(),
                "Smooth".to_string(),
                "Professional".to_string(),
            ],
            use_cases: vec!["Storytelling".to_string()],
            version: ModelVersion::Aura1,
            quality: VoiceQuality::Standard,
        },
        Model {
            name: "orion-v1".to_string(),
            voice_id: "aura-orion-en".to_string(),
            language: "en-us".to_string(),
            accent: "American".to_string(),
            gender: "masculine".to_string(),
            age: "Adult".to_string(),
            characteristics: vec![
                "Approachable".to_string(),
                "Comfortable".to_string(),
                "Calm".to_string(),
                "Polite".to_string(),
            ],
            use_cases: vec!["Informative".to_string()],
            version: ModelVersion::Aura1,
            quality: VoiceQuality::Standard,
        },
        Model {
            name: "zeus-v1".to_string(),
            voice_id: "aura-zeus-en".to_string(),
            language: "en-us".to_string(),
            accent: "American".to_string(),
            gender: "masculine".to_string(),
            age: "Adult".to_string(),
            characteristics: vec![
                "Deep".to_string(),
                "Trustworthy".to_string(),
                "Smooth".to_string(),
            ],
            use_cases: vec!["IVR".to_string()],
            version: ModelVersion::Aura1,
            quality: VoiceQuality::Standard,
        },
    ]
}

fn _parse_response<T: DeserializeOwned + Debug>(response: Response) -> Result<T, TtsError> {
    let status = response.status();
    if !status.is_success() {
        return Err(tts_error_from_status(status));
    }

    match response.json::<T>() {
        Ok(parsed) => {
            trace!("Parsed response: {:?}", parsed);
            Ok(parsed)
        }
        Err(e) => {
            trace!("Failed to parse response: {:?}", e);
            Err(from_reqwest_error("Failed to parse response", e))
        }
    }
}
