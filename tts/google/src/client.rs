use base64::{engine::general_purpose, Engine as _};
use golem_tts::config::{get_endpoint_config, get_max_retries_config, get_timeout_config};
use golem_tts::error::{from_reqwest_error, internal_error, tts_error_from_status};
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
            initial_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct AuthToken {
    pub access_token: String,
    pub token_type: String,
    pub expires_at: std::time::SystemTime,
}

/// The Google Cloud TTS API client
/// Based on https://cloud.google.com/text-to-speech/docs/reference/rest
#[derive(Clone)]
pub struct GoogleTtsApi {
    client: Client,
    base_url: String,
    rate_limit_config: RateLimitConfig,
    #[allow(dead_code)]
    project_id: Option<String>,
    #[allow(dead_code)]
    credentials_path: Option<String>,
}

impl GoogleTtsApi {
    pub fn new(
        credentials_path: Option<String>,
        project_id: Option<String>,
    ) -> Result<Self, TtsError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(get_timeout_config()))
            .build()
            .map_err(|err| from_reqwest_error("Failed to create HTTP client", err))?;

        let base_url = get_endpoint_config("https://texttospeech.googleapis.com/v1");

        Ok(Self {
            client,
            base_url,
            rate_limit_config: RateLimitConfig::default(),
            project_id,
            credentials_path,
        })
    }

    fn get_access_token(&self) -> Result<String, TtsError> {
        if let Some(ref creds_path) = self.credentials_path {
            return self.get_token_from_service_account(creds_path);
        }

        self.get_token_from_metadata_service()
    }

    fn get_token_from_service_account(&self, creds_path: &str) -> Result<String, TtsError> {
        let creds_content = std::fs::read_to_string(creds_path)
            .map_err(|e| internal_error(format!("Failed to read service account file: {}", e)))?;

        let creds: ServiceAccountCredentials = serde_json::from_str(&creds_content)
            .map_err(|e| internal_error(format!("Failed to parse service account file: {}", e)))?;

        let jwt = self.create_jwt_assertion(&creds)?;

        let response = self
            .client
            .post("https://oauth2.googleapis.com/token")
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
                ("assertion", &jwt),
            ])
            .send()
            .map_err(|err| from_reqwest_error("Failed to send token request", err))?;

        if !response.status().is_success() {
            let status = response.status();
            let _body = response.text().unwrap_or_default();
            return Err(tts_error_from_status(status));
        }

        let token_response: TokenResponse = response
            .json()
            .map_err(|err| from_reqwest_error("Failed to parse JWT token response", err))?;
        Ok(token_response.access_token)
    }

    fn get_token_from_metadata_service(&self) -> Result<String, TtsError> {
        let response = self.client
            .get("http://metadata.google.internal/computeMetadata/v1/instance/service-accounts/default/token")
            .header("Metadata-Flavor", "Google")
            .send()
            .map_err(|err| from_reqwest_error("Failed to get metadata token", err))?;

        if !response.status().is_success() {
            return Err(internal_error("Failed to get token from metadata service"));
        }

        let token_response: TokenResponse = response
            .json()
            .map_err(|err| from_reqwest_error("Failed to parse metadata token response", err))?;
        Ok(token_response.access_token)
    }

    fn create_jwt_assertion(&self, creds: &ServiceAccountCredentials) -> Result<String, TtsError> {
        use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
        use serde_json::json;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let claims = json!({
            "iss": creds.client_email,
            "scope": "https://www.googleapis.com/auth/cloud-platform",
            "aud": "https://oauth2.googleapis.com/token",
            "exp": now + 3600,
            "iat": now
        });

        let key = EncodingKey::from_rsa_pem(creds.private_key.as_bytes())
            .map_err(|e| internal_error(format!("Failed to parse private key: {}", e)))?;

        let header = Header::new(Algorithm::RS256);

        encode(&header, &claims, &key)
            .map_err(|e| internal_error(format!("Failed to create JWT: {}", e)))
    }

    fn create_request(&self, method: Method, url: &str) -> Result<RequestBuilder, TtsError> {
        let access_token = self.get_access_token()?;

        Ok(self
            .client
            .request(method, url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json"))
    }

    fn execute_with_retry<F>(&self, operation: F) -> Result<Response, TtsError>
    where
        F: Fn() -> Result<Response, TtsError>,
    {
        let mut delay = self.rate_limit_config.initial_delay;

        for attempt in 0..=self.rate_limit_config.max_retries {
            match operation() {
                Ok(response) => {
                    if response.status().is_success() {
                        return Ok(response);
                    } else if (response.status().as_u16() == 429
                        || response.status().as_u16() >= 500)
                        && attempt < self.rate_limit_config.max_retries
                    {
                        trace!(
                            "Request failed with status {}, retrying in {:?}",
                            response.status(),
                            delay
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
                    }

                    let status = response.status();
                    return Err(tts_error_from_status(status));
                }
                Err(e) => {
                    if attempt < self.rate_limit_config.max_retries {
                        trace!(
                            "Request failed with error: {:?}, retrying in {:?}",
                            e,
                            delay
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
                    }
                    return Err(e);
                }
            }
        }

        Err(internal_error("Max retries exceeded"))
    }

    pub fn list_voices(&self, language_code: Option<&str>) -> Result<ListVoicesResponse, TtsError> {
        let mut url = format!("{}/voices", self.base_url);

        if let Some(lang) = language_code {
            url.push_str(&format!("?languageCode={}", urlencoding::encode(lang)));
        }

        self.execute_with_retry(|| {
            let request = self.create_request(Method::GET, &url)?;
            request
                .send()
                .map_err(|err| from_reqwest_error("Failed to send request", err))
        })
        .and_then(parse_response)
    }

    pub fn text_to_speech(&self, request: &SynthesizeSpeechRequest) -> Result<Vec<u8>, TtsError> {
        let url = format!("{}/text:synthesize", self.base_url);

        let response = self.execute_with_retry(|| {
            let req = self.create_request(Method::POST, &url)?;
            req.json(request)
                .send()
                .map_err(|err| from_reqwest_error("Failed to send synthesis request", err))
        })?;

        let synthesis_response: SynthesizeSpeechResponse = parse_response(response)?;
        general_purpose::STANDARD
            .decode(&synthesis_response.audio_content)
            .map_err(|e| internal_error(format!("Failed to decode audio content: {}", e)))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceAccountCredentials {
    #[serde(rename = "type")]
    pub type_: String,
    pub project_id: String,
    pub private_key_id: String,
    pub private_key: String,
    pub client_email: String,
    pub client_id: String,
    pub auth_uri: String,
    pub token_uri: String,
    pub auth_provider_x509_cert_url: String,
    pub client_x509_cert_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: Option<String>,
    pub expires_in: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListVoicesResponse {
    pub voices: Vec<Voice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Voice {
    #[serde(rename = "languageCodes")]
    pub language_codes: Vec<String>,
    pub name: String,
    #[serde(rename = "ssmlGender")]
    pub ssml_gender: SsmlVoiceGender,
    #[serde(rename = "naturalSampleRateHertz")]
    pub natural_sample_rate_hertz: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[allow(clippy::enum_variant_names)]
pub enum SsmlVoiceGender {
    SsmlVoiceGenderUnspecified,
    Male,
    Female,
    Neutral,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesizeSpeechRequest {
    pub input: SynthesisInput,
    pub voice: VoiceSelectionParams,
    #[serde(rename = "audioConfig")]
    pub audio_config: AudioConfig,
    #[serde(
        rename = "advancedVoiceOptions",
        skip_serializing_if = "Option::is_none"
    )]
    pub advanced_voice_options: Option<AdvancedVoiceOptions>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesisInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssml: Option<String>,
    #[serde(rename = "multiSpeakerMarkup", skip_serializing_if = "Option::is_none")]
    pub multi_speaker_markup: Option<MultiSpeakerMarkup>,
    #[serde(
        rename = "customPronunciations",
        skip_serializing_if = "Option::is_none"
    )]
    pub custom_pronunciations: Option<CustomPronunciations>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceSelectionParams {
    #[serde(rename = "languageCode")]
    pub language_code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "ssmlGender", skip_serializing_if = "Option::is_none")]
    pub ssml_gender: Option<SsmlVoiceGender>,
    #[serde(rename = "customVoice", skip_serializing_if = "Option::is_none")]
    pub custom_voice: Option<CustomVoiceParams>,
    #[serde(rename = "voiceClone", skip_serializing_if = "Option::is_none")]
    pub voice_clone: Option<VoiceCloneParams>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    #[serde(rename = "audioEncoding")]
    pub audio_encoding: AudioEncoding,
    #[serde(rename = "speakingRate", skip_serializing_if = "Option::is_none")]
    pub speaking_rate: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pitch: Option<f64>,
    #[serde(rename = "volumeGainDb", skip_serializing_if = "Option::is_none")]
    pub volume_gain_db: Option<f64>,
    #[serde(rename = "sampleRateHertz", skip_serializing_if = "Option::is_none")]
    pub sample_rate_hertz: Option<i32>,
    #[serde(rename = "effectsProfileId", skip_serializing_if = "Option::is_none")]
    pub effects_profile_id: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[allow(clippy::enum_variant_names)]
pub enum AudioEncoding {
    AudioEncodingUnspecified,
    Linear16,
    Mp3,
    OggOpus,
    Mulaw,
    Alaw,
    Pcm,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedVoiceOptions {
    #[serde(
        rename = "lowLatencyJourneySynthesis",
        skip_serializing_if = "Option::is_none"
    )]
    pub low_latency_journey_synthesis: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiSpeakerMarkup {
    pub turns: Vec<Turn>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Turn {
    pub speaker: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomPronunciations {
    pub pronunciations: Vec<CustomPronunciationParams>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomPronunciationParams {
    pub phrase: String,
    #[serde(rename = "phoneticEncoding")]
    pub phonetic_encoding: PhoneticEncoding,
    pub pronunciation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[allow(clippy::enum_variant_names)]
pub enum PhoneticEncoding {
    PhoneticEncodingUnspecified,
    PhoneticEncodingIpa,
    PhoneticEncodingXSampa,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomVoiceParams {
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceCloneParams {
    #[serde(rename = "voiceCloningKey")]
    pub voice_cloning_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesizeSpeechResponse {
    #[serde(rename = "audioContent")]
    pub audio_content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingSynthesizeRequest {
    #[serde(rename = "streamingConfig", skip_serializing_if = "Option::is_none")]
    pub streaming_config: Option<StreamingSynthesizeConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<StreamingSynthesisInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingSynthesizeConfig {
    pub voice: VoiceSelectionParams,
    #[serde(
        rename = "streamingAudioConfig",
        skip_serializing_if = "Option::is_none"
    )]
    pub streaming_audio_config: Option<StreamingAudioConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingAudioConfig {
    #[serde(rename = "audioEncoding")]
    pub audio_encoding: AudioEncoding,
    #[serde(rename = "sampleRateHertz", skip_serializing_if = "Option::is_none")]
    pub sample_rate_hertz: Option<i32>,
    #[serde(rename = "speakingRate", skip_serializing_if = "Option::is_none")]
    pub speaking_rate: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingSynthesisInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingSynthesizeResponse {
    #[serde(rename = "audioContent")]
    pub audio_content: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesizeLongAudioRequest {
    pub parent: String,
    pub input: SynthesisInput,
    #[serde(rename = "audioConfig")]
    pub audio_config: AudioConfig,
    #[serde(rename = "outputGcsUri")]
    pub output_gcs_uri: String,
    pub voice: VoiceSelectionParams,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchSynthesisParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_chunk_size: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_overlap: Option<usize>,
}

fn parse_response<T: DeserializeOwned + Debug>(response: Response) -> Result<T, TtsError> {
    if !response.status().is_success() {
        let status = response.status();
        return Err(tts_error_from_status(status));
    }

    response
        .json::<T>()
        .map_err(|err| from_reqwest_error("Failed to parse response JSON", err))
}
