use golem_tts::config::{get_endpoint_config, get_max_retries_config, get_timeout_config};
use golem_tts::error::{from_reqwest_error, internal_error, tts_error_from_status};
use golem_tts::golem::tts::types::TtsError;
use log::trace;
use reqwest::{Client, Method, RequestBuilder, Response};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::time::Duration;

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

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct QuotaInfo {
    pub character_count: u32,
    pub character_limit: u32,
    pub can_extend_character_limit: bool,
    pub allowed_to_extend_character_limit: bool,
    pub next_character_count_reset_unix: i64,
    pub voice_limit: u32,
    pub max_voice_add_edits: u32,
    pub voice_add_edit_counter: u32,
    pub professional_voice_limit: u32,
    pub can_extend_voice_limit: bool,
    pub can_use_instant_voice_cloning: bool,
    pub can_use_professional_voice_cloning: bool,
    pub currency: Option<String>,
    pub status: String,
}

/// The ElevenLabs TTS API client
/// Based on https://elevenlabs.io/docs/api-reference/
#[derive(Clone)]
pub struct ElevenLabsTtsApi {
    client: Client,
    api_key: String,
    base_url: String,
    rate_limit_config: RateLimitConfig,
    model_version: String,
}

impl ElevenLabsTtsApi {
    pub fn new(api_key: String, model_version: String) -> Self {
        let timeout_secs = get_timeout_config();
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .expect("Failed to initialize HTTP client");

        let base_url = get_endpoint_config("https://api.elevenlabs.io");

        Self {
            api_key,
            client,
            base_url,
            rate_limit_config: RateLimitConfig::default(),
            model_version,
        }
    }

    fn create_request(&self, method: Method, url: &str) -> RequestBuilder {
        self.client
            .request(method, url)
            .header("xi-api-key", &self.api_key)
    }

    pub fn get_model_version(&self) -> &str {
        &self.model_version
    }

    fn execute_with_retry<F>(&self, operation: F) -> Result<Response, TtsError>
    where
        F: Fn() -> Result<Response, TtsError>,
    {
        let mut attempt = 0;
        let mut delay = self.rate_limit_config.initial_delay;

        loop {
            match operation() {
                Ok(response) => {
                    let status = response.status();

                    if status.is_success() {
                        return Ok(response);
                    } else if status.as_u16() == 429 || status.as_u16() == 503 {
                        if attempt >= self.rate_limit_config.max_retries {
                            trace!(
                                "Max retries ({}) exceeded for rate limiting",
                                self.rate_limit_config.max_retries
                            );
                            return Err(tts_error_from_status(status));
                        }

                        trace!(
                            "Rate limited ({}), retrying in {:?} (attempt {}/{})",
                            status,
                            delay,
                            attempt + 1,
                            self.rate_limit_config.max_retries
                        );

                        let retry_delay = if let Some(retry_after) =
                            response.headers().get("retry-after")
                        {
                            if let Ok(seconds) = retry_after.to_str().unwrap_or("").parse::<u64>() {
                                Duration::from_secs(seconds)
                            } else {
                                delay
                            }
                        } else {
                            delay
                        };

                        std::thread::sleep(retry_delay);

                        attempt += 1;
                        delay = std::cmp::min(
                            Duration::from_millis(
                                (delay.as_millis() as f64
                                    * self.rate_limit_config.backoff_multiplier)
                                    as u64,
                            ),
                            self.rate_limit_config.max_delay,
                        );
                    } else {
                        return Err(tts_error_from_status(status));
                    }
                }
                Err(e) => return Err(e),
            }
        }
    }

    fn execute_stream_with_retry<F>(&self, operation: F) -> Result<Response, TtsError>
    where
        F: Fn() -> Result<Response, TtsError>,
    {
        let mut attempt = 0;
        let mut delay = self.rate_limit_config.initial_delay;

        loop {
            match operation() {
                Ok(response) => {
                    let status = response.status();

                    if status.is_success() {
                        return Ok(response);
                    } else if status.as_u16() == 429 || status.as_u16() == 503 {
                        if attempt >= self.rate_limit_config.max_retries {
                            trace!(
                                "Max retries ({}) exceeded for rate limiting",
                                self.rate_limit_config.max_retries
                            );
                            return Err(tts_error_from_status(status));
                        }

                        trace!(
                            "Rate limited ({}), retrying in {:?} (attempt {}/{})",
                            status,
                            delay,
                            attempt + 1,
                            self.rate_limit_config.max_retries
                        );

                        std::thread::sleep(delay);

                        attempt += 1;
                        delay = std::cmp::min(
                            Duration::from_millis(
                                (delay.as_millis() as f64
                                    * self.rate_limit_config.backoff_multiplier)
                                    as u64,
                            ),
                            self.rate_limit_config.max_delay,
                        );
                    } else {
                        return Err(tts_error_from_status(status));
                    }
                }
                Err(e) => return Err(e),
            }
        }
    }

    pub fn list_voices(
        &self,
        params: Option<ListVoicesParams>,
    ) -> Result<ListVoicesResponse, TtsError> {
        trace!("Listing voices");

        let mut url = format!("{}/v2/voices", self.base_url);

        if let Some(params) = params {
            let mut query_params = Vec::new();

            if let Some(page_size) = params.page_size {
                query_params.push(format!("page_size={}", page_size));
            }
            if let Some(search) = params.search {
                query_params.push(format!("search={}", urlencoding::encode(&search)));
            }
            if let Some(sort) = params.sort {
                query_params.push(format!("sort={}", sort));
            }
            if let Some(sort_direction) = params.sort_direction {
                query_params.push(format!("sort_direction={}", sort_direction));
            }
            if let Some(voice_type) = params.voice_type {
                query_params.push(format!("voice_type={}", voice_type));
            }
            if let Some(category) = params.category {
                query_params.push(format!("category={}", category));
            }
            if let Some(next_page_token) = params.next_page_token {
                query_params.push(format!(
                    "next_page_token={}",
                    urlencoding::encode(&next_page_token)
                ));
            }
            if let Some(include_total_count) = params.include_total_count {
                query_params.push(format!("include_total_count={}", include_total_count));
            }

            if !query_params.is_empty() {
                url.push('?');
                url.push_str(&query_params.join("&"));
            }
        }

        let response = self.execute_with_retry(|| {
            self.create_request(Method::GET, &url)
                .send()
                .map_err(|e| internal_error(format!("Failed to list voices: {e}")))
        })?;

        parse_response(response)
    }

    pub fn get_voice(&self, voice_id: &str) -> Result<Voice, TtsError> {
        trace!("Getting voice: {voice_id}");

        let url = format!("{}/v1/voices/{}", self.base_url, voice_id);

        let response = self.execute_with_retry(|| {
            self.create_request(Method::GET, &url)
                .send()
                .map_err(|e| internal_error(format!("Failed to get voice: {e}")))
        })?;

        parse_response(response)
    }

    pub fn text_to_speech(
        &self,
        voice_id: &str,
        request: &TextToSpeechRequest,
        params: Option<TextToSpeechParams>,
    ) -> Result<Vec<u8>, TtsError> {
        trace!("Converting text to speech with voice: {voice_id}");

        let mut url = format!("{}/v1/text-to-speech/{}", self.base_url, voice_id);

        if let Some(params) = params {
            let mut query_params = Vec::new();

            if let Some(enable_logging) = params.enable_logging {
                query_params.push(format!("enable_logging={}", enable_logging));
            }
            if let Some(optimize_streaming_latency) = params.optimize_streaming_latency {
                query_params.push(format!(
                    "optimize_streaming_latency={}",
                    optimize_streaming_latency
                ));
            }
            if let Some(output_format) = params.output_format {
                query_params.push(format!("output_format={}", output_format));
            }

            if !query_params.is_empty() {
                url.push('?');
                url.push_str(&query_params.join("&"));
            }
        }

        let response = self.execute_with_retry(|| {
            self.create_request(Method::POST, &url)
                .json(request)
                .send()
                .map_err(|e| internal_error(format!("Failed to synthesize speech: {e}")))
        })?;

        if response.status().is_success() {
            let audio_data = response
                .bytes()
                .map_err(|err| from_reqwest_error("Failed to read audio response", err))?;
            Ok(audio_data.to_vec())
        } else {
            let status = response.status();
            let error_body = response
                .text()
                .map_err(|err| from_reqwest_error("Failed to receive error response body", err))?;

            trace!("Received {status} response from ElevenLabs API: {error_body:?}");
            Err(tts_error_from_status(status))
        }
    }

    pub fn synthesize_long_form_batch(
        &self,
        voice_id: &str,
        content: &str,
        options: Option<&TextToSpeechParams>,
        max_chunk_size: usize,
    ) -> Result<Vec<Vec<u8>>, TtsError> {
        trace!("Synthesizing long-form content with batch processing");

        let chunks = self.split_content_intelligently(content, max_chunk_size);
        let mut audio_chunks = Vec::new();

        for (i, chunk) in chunks.iter().enumerate() {
            trace!(
                "Processing chunk {}/{}: {} characters",
                i + 1,
                chunks.len(),
                chunk.len()
            );

            let supports_language_code = !self.model_version.contains("multilingual");

            let request = TextToSpeechRequest {
                text: chunk.clone(),
                model_id: Some(self.model_version.clone()),
                language_code: if supports_language_code {
                    Some("en".to_string())
                } else {
                    None
                },
                voice_settings: None,
                pronunciation_dictionary_locators: None,
                seed: None,
                previous_text: if i > 0 {
                    Some(chunks[i - 1].clone())
                } else {
                    None
                },
                next_text: if i < chunks.len() - 1 {
                    Some(chunks[i + 1].clone())
                } else {
                    None
                },
                previous_request_ids: None,
                next_request_ids: None,
                apply_text_normalization: Some("auto".to_string()),
                apply_language_text_normalization: Some(false),
                use_pvc_as_ivc: Some(false),
            };

            let audio_data = self.text_to_speech(voice_id, &request, options.cloned())?;
            audio_chunks.push(audio_data);

            if i < chunks.len() - 1 {
                std::thread::sleep(Duration::from_millis(100));
            }
        }

        Ok(audio_chunks)
    }

    fn split_content_intelligently(&self, content: &str, max_chunk_size: usize) -> Vec<String> {
        if content.len() <= max_chunk_size {
            return vec![content.to_string()];
        }

        let mut chunks = Vec::new();
        let mut current_chunk = String::new();

        let sentences: Vec<&str> = content
            .split(['.', '!', '?'])
            .filter(|s| !s.trim().is_empty())
            .collect();

        for sentence in sentences {
            let sentence = sentence.trim();
            if sentence.is_empty() {
                continue;
            }

            let sentence_with_punct = format!("{}.", sentence);

            if !current_chunk.is_empty()
                && (current_chunk.len() + sentence_with_punct.len() + 1) > max_chunk_size
            {
                chunks.push(current_chunk.trim().to_string());
                current_chunk = String::new();
            }

            if sentence_with_punct.len() > max_chunk_size {
                let word_chunks =
                    self.split_at_word_boundaries(&sentence_with_punct, max_chunk_size);
                for word_chunk in word_chunks {
                    if !current_chunk.is_empty() {
                        chunks.push(current_chunk.trim().to_string());
                        current_chunk = String::new();
                    }
                    chunks.push(word_chunk);
                }
            } else {
                if !current_chunk.is_empty() {
                    current_chunk.push(' ');
                }
                current_chunk.push_str(&sentence_with_punct);
            }
        }

        if !current_chunk.trim().is_empty() {
            chunks.push(current_chunk.trim().to_string());
        }

        chunks
    }

    fn split_at_word_boundaries(&self, text: &str, max_size: usize) -> Vec<String> {
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut chunks = Vec::new();
        let mut current_chunk = String::new();

        for word in words {
            if !current_chunk.is_empty() && (current_chunk.len() + word.len() + 1) > max_size {
                chunks.push(current_chunk.trim().to_string());
                current_chunk = String::new();
            }

            if !current_chunk.is_empty() {
                current_chunk.push(' ');
            }
            current_chunk.push_str(word);
        }

        if !current_chunk.trim().is_empty() {
            chunks.push(current_chunk.trim().to_string());
        }

        chunks
    }

    #[allow(dead_code)]
    pub fn get_quota_info(&self) -> Result<QuotaInfo, TtsError> {
        trace!("Getting user quota information");

        let url = format!("{}/v1/user", self.base_url);

        let response = self.execute_with_retry(|| {
            self.create_request(Method::GET, &url)
                .send()
                .map_err(|e| internal_error(format!("Failed to get quota info: {e}")))
        })?;

        parse_response(response)
    }

    pub fn text_to_speech_stream(
        &self,
        voice_id: &str,
        request: &TextToSpeechRequest,
        params: Option<TextToSpeechParams>,
    ) -> Result<reqwest::Response, TtsError> {
        trace!("Streaming text to speech with voice: {voice_id}");

        let mut url = format!("{}/v1/text-to-speech/{}/stream", self.base_url, voice_id);

        if let Some(params) = params {
            let mut query_params = Vec::new();

            if let Some(enable_logging) = params.enable_logging {
                query_params.push(format!("enable_logging={}", enable_logging));
            }
            if let Some(optimize_streaming_latency) = params.optimize_streaming_latency {
                query_params.push(format!(
                    "optimize_streaming_latency={}",
                    optimize_streaming_latency
                ));
            }
            if let Some(output_format) = params.output_format {
                query_params.push(format!("output_format={}", output_format));
            }

            if !query_params.is_empty() {
                url.push('?');
                url.push_str(&query_params.join("&"));
            }
        }

        let response = self.execute_stream_with_retry(|| {
            self.create_request(Method::POST, &url)
                .json(request)
                .send()
                .map_err(|e| internal_error(format!("Failed to start streaming synthesis: {e}")))
        })?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(tts_error_from_status(status));
        }

        Ok(response)
    }

    pub fn get_models(&self) -> Result<Vec<Model>, TtsError> {
        trace!("Getting available models");

        let url = format!("{}/v1/models", self.base_url);

        let response = self.execute_with_retry(|| {
            self.create_request(Method::GET, &url)
                .send()
                .map_err(|e| internal_error(format!("Failed to get models: {e}")))
        })?;

        parse_response(response)
    }

    #[allow(dead_code)]
    pub fn get_user_subscription(&self) -> Result<UserSubscription, TtsError> {
        trace!("Getting user subscription info");

        let url = format!("{}/v1/user/subscription", self.base_url);

        let response = self.execute_with_retry(|| {
            self.create_request(Method::GET, &url)
                .send()
                .map_err(|e| internal_error(format!("Failed to get user subscription: {e}")))
        })?;

        parse_response(response)
    }

    pub fn create_voice(&self, request: &CreateVoiceRequest) -> Result<Voice, TtsError> {
        trace!("Creating voice clone: {}", request.name);

        let url = format!("{}/v1/voices/add", self.base_url);

        let boundary = format!("----elevenlabs-{}", uuid::Uuid::new_v4());
        let mut form_data = Vec::new();

        form_data.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
        form_data.extend_from_slice(b"Content-Disposition: form-data; name=\"name\"\r\n\r\n");
        form_data.extend_from_slice(request.name.as_bytes());
        form_data.extend_from_slice(b"\r\n");

        if let Some(ref description) = request.description {
            form_data.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
            form_data
                .extend_from_slice(b"Content-Disposition: form-data; name=\"description\"\r\n\r\n");
            form_data.extend_from_slice(description.as_bytes());
            form_data.extend_from_slice(b"\r\n");
        }

        if let Some(ref labels) = request.labels {
            form_data.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
            form_data.extend_from_slice(b"Content-Disposition: form-data; name=\"labels\"\r\n\r\n");
            form_data.extend_from_slice(labels.as_bytes());
            form_data.extend_from_slice(b"\r\n");
        }

        for (i, file) in request.files.iter().enumerate() {
            form_data.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
            form_data.extend_from_slice(
                format!(
                    "Content-Disposition: form-data; name=\"files\"; filename=\"audio_{}.wav\"\r\n",
                    i
                )
                .as_bytes(),
            );
            form_data.extend_from_slice(b"Content-Type: audio/wav\r\n\r\n");
            form_data.extend(file.data.iter());
            form_data.extend_from_slice(b"\r\n");
        }

        form_data.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());

        let response = self.execute_with_retry(|| {
            let request = self
                .client
                .post(&url)
                .header("xi-api-key", &self.api_key)
                .header(
                    "Content-Type",
                    format!("multipart/form-data; boundary={}", boundary),
                )
                .body(form_data.clone());

            match request.send() {
                Ok(response) => {
                    if response.status().is_success() {
                        Ok(response)
                    } else {
                        let status = response.status();
                        match response.text() {
                            Ok(error_body) => Err(internal_error(format!(
                                "Failed to create voice: status code {} - {}",
                                status, error_body
                            ))),
                            Err(_) => Err(internal_error(format!(
                                "Failed to create voice: status code {}",
                                status
                            ))),
                        }
                    }
                }
                Err(e) => Err(internal_error(format!("Failed to create voice: {e}"))),
            }
        })?;

        parse_response(response)
    }

    pub fn delete_voice(&self, voice_id: &str) -> Result<(), TtsError> {
        trace!("Deleting voice: {voice_id}");

        let url = format!("{}/v1/voices/{}", self.base_url, voice_id);

        let response = self.execute_with_retry(|| {
            self.create_request(Method::DELETE, &url)
                .send()
                .map_err(|e| internal_error(format!("Failed to delete voice: {e}")))
        })?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(tts_error_from_status(response.status()))
        }
    }

    pub fn speech_to_speech(
        &self,
        voice_id: &str,
        request: &SpeechToSpeechRequest,
        params: Option<SpeechToSpeechParams>,
    ) -> Result<Vec<u8>, TtsError> {
        trace!("Converting speech to speech with voice: {voice_id}");

        let mut url = format!("{}/v1/speech-to-speech/{}", self.base_url, voice_id);

        if let Some(params) = params {
            let mut query_params = Vec::new();

            if let Some(enable_logging) = params.enable_logging {
                query_params.push(format!("enable_logging={}", enable_logging));
            }
            if let Some(optimize_streaming_latency) = params.optimize_streaming_latency {
                query_params.push(format!(
                    "optimize_streaming_latency={}",
                    optimize_streaming_latency
                ));
            }
            if let Some(output_format) = params.output_format {
                query_params.push(format!("output_format={}", output_format));
            }
            if let Some(remove_background_noise) = params.remove_background_noise {
                query_params.push(format!(
                    "remove_background_noise={}",
                    remove_background_noise
                ));
            }

            if !query_params.is_empty() {
                url.push('?');
                url.push_str(&query_params.join("&"));
            }
        }

        use base64::Engine;
        let audio_base64 = base64::engine::general_purpose::STANDARD.encode(&request.audio_data);

        let json_request = SpeechToSpeechJsonRequest {
            audio: audio_base64,
            model_id: request
                .model_id
                .clone()
                .unwrap_or("eleven_english_sts_v2".to_string()),
            voice_settings: request.voice_settings.clone(),
            seed: request.seed,
        };

        let response = self.execute_with_retry(|| {
            self.create_request(Method::POST, &url)
                .json(&json_request)
                .send()
                .map_err(|e| internal_error(format!("Failed to convert speech: {e}")))
        })?;

        if response.status().is_success() {
            let audio_data = response
                .bytes()
                .map_err(|err| from_reqwest_error("Failed to read audio response", err))?;
            Ok(audio_data.to_vec())
        } else {
            let status = response.status();
            let error_body = response
                .text()
                .map_err(|err| from_reqwest_error("Failed to receive error response body", err))?;

            trace!("Received {status} response from ElevenLabs API: {error_body:?}");
            Err(tts_error_from_status(status))
        }
    }

    pub fn create_sound_effect(
        &self,
        request: &SoundEffectRequest,
        params: Option<SoundEffectParams>,
    ) -> Result<Vec<u8>, TtsError> {
        trace!("Creating sound effect: {}", request.text);

        let mut url = format!("{}/v1/sound-generation", self.base_url);

        if let Some(params) = params {
            let mut query_params = Vec::new();

            if let Some(output_format) = params.output_format {
                query_params.push(format!("output_format={}", output_format));
            }

            if !query_params.is_empty() {
                url.push('?');
                url.push_str(&query_params.join("&"));
            }
        }

        let response = self.execute_with_retry(|| {
            self.create_request(Method::POST, &url)
                .json(request)
                .send()
                .map_err(|e| internal_error(format!("Failed to create sound effect: {e}")))
        })?;

        if response.status().is_success() {
            let audio_data = response
                .bytes()
                .map_err(|err| from_reqwest_error("Failed to read audio response", err))?;
            Ok(audio_data.to_vec())
        } else {
            let status = response.status();
            let error_body = response
                .text()
                .map_err(|err| from_reqwest_error("Failed to receive error response body", err))?;

            trace!("Received {status} response from ElevenLabs API: {error_body:?}");
            Err(tts_error_from_status(status))
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListVoicesParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_page_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_size: Option<u32>,
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
    pub include_total_count: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListVoicesResponse {
    pub voices: Vec<Voice>,
    pub has_more: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_page_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Voice {
    pub voice_id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub category: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<VoiceSettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub samples: Option<Vec<VoiceSample>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub high_quality_base_model_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub available_for_tiers: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceSettings {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceSample {
    pub sample_id: String,
    pub file_name: String,
    pub mime_type: String,
    pub size_bytes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_secs: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextToSpeechRequest {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_settings: Option<VoiceSettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pronunciation_dictionary_locators: Option<Vec<PronunciationDictionaryLocator>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_request_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_request_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apply_text_normalization: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apply_language_text_normalization: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_pvc_as_ivc: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PronunciationDictionaryLocator {
    pub pronunciation_dictionary_id: String,
    pub version_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextToSpeechParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_logging: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optimize_streaming_latency: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_format: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    pub model_id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub can_be_finetuned: bool,
    pub can_do_text_to_speech: bool,
    pub can_do_voice_conversion: bool,
    pub can_use_style: bool,
    pub can_use_speaker_boost: bool,
    pub serves_pro_voices: bool,
    pub token_cost_factor: f32,
    pub languages: Vec<LanguageInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_characters_request_free_tier: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_characters_request_subscribed_tier: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageInfo {
    pub language_id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSubscription {
    pub tier: String,
    pub character_count: u32,
    pub character_limit: u32,
    pub can_extend_character_limit: bool,
    pub allowed_to_extend_character_limit: bool,
    pub next_character_count_reset_unix: u64,
    pub voice_limit: u32,
    pub max_voice_add_edits: u32,
    pub voice_add_edit_counter: u32,
    pub professional_voice_limit: u32,
    pub can_extend_voice_limit: bool,
    pub can_use_instant_voice_cloning: bool,
    pub can_use_professional_voice_cloning: bool,
    pub currency: String,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct CreateVoiceRequest {
    pub name: String,
    pub description: Option<String>,
    pub files: Vec<AudioFile>,
    pub labels: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AudioFile {
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct SpeechToSpeechRequest {
    pub audio_data: Vec<u8>,
    pub model_id: Option<String>,
    pub voice_settings: Option<VoiceSettings>,
    pub seed: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeechToSpeechJsonRequest {
    pub audio: String,
    pub model_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_settings: Option<VoiceSettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeechToSpeechParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_logging: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optimize_streaming_latency: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remove_background_noise: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoundEffectRequest {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_influence: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoundEffectParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_format: Option<String>,
}

fn parse_response<T: DeserializeOwned + Debug>(response: Response) -> Result<T, TtsError> {
    let status = response.status();

    trace!("Received response from ElevenLabs API: {response:?}");

    if status.is_success() {
        let body = response
            .json::<T>()
            .map_err(|err| from_reqwest_error("Failed to decode response body", err))?;

        trace!("Received response from ElevenLabs API: {body:?}");

        Ok(body)
    } else {
        let error_body = response
            .text()
            .map_err(|err| from_reqwest_error("Failed to receive error response body", err))?;

        trace!("Received {status} response from ElevenLabs API: {error_body:?}");

        Err(tts_error_from_status(status))
    }
}
