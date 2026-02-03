use bytes::Bytes;
use golem_tts::durability::{DurableTts, ExtendedGuest};
use golem_tts::golem::tts::advanced::{AudioSample, LongFormResult, VoiceDesignParams};
// use golem_tts::golem::tts::streaming::SynthesisOptions;
use golem_tts::golem::tts::synthesis::{SynthesisOptions as WitSynthesisOptions, ValidationResult};
use golem_tts::golem::tts::types::{
    AudioChunk, AudioFormat, SynthesisMetadata, SynthesisResult, TextInput, TimingInfo, TtsError,
};
use golem_tts::golem::tts::voices::{VoiceFilter, VoiceGender, VoiceInfo, VoiceQuality};
use golem_tts::guest::{StreamRequest, SynthesisRequest, TtsGuest};
use golem_tts::http::{HttpClient, WstdHttpClient};
use golem_rust::Uuid;
use base64::Engine;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use wstd::runtime::block_on;

mod gcp_auth;

static API_CLIENT: OnceCell<GoogleTtsApi<WstdHttpClient>> = OnceCell::new();

struct GoogleTtsComponent;

impl GoogleTtsComponent {
    fn create_or_get_client(
    ) -> Result<&'static GoogleTtsApi<WstdHttpClient>, golem_tts::error::Error> {
        API_CLIENT.get_or_try_init(|| {
            let service_acc_key = if let Ok(creds_json_file) =
                std::env::var("GOOGLE_APPLICATION_CREDENTIALS")
            {
                let bytes = std::fs::read(&creds_json_file).map_err(|err| {
                    golem_tts::error::Error::AuthError(format!(
                        "Failed to read Google credentials file: {err}"
                    ))
                })?;
                let service_acc_key: gcp_auth::ServiceAccountKey =
                    serde_json::from_slice(&bytes).map_err(|err| {
                        golem_tts::error::Error::AuthError(format!(
                            "Failed to parse Google credentials: {err}"
                        ))
                    })?;
                service_acc_key
            } else {
                let project_id = std::env::var("GOOGLE_CLOUD_PROJECT").map_err(|err| {
                    golem_tts::error::Error::EnvVariablesNotSet(format!(
                        "Failed to load GOOGLE_CLOUD_PROJECT: {err}"
                    ))
                })?;

                let client_email = std::env::var("GOOGLE_CLIENT_EMAIL").map_err(|err| {
                    golem_tts::error::Error::EnvVariablesNotSet(format!(
                        "Failed to load GOOGLE_CLIENT_EMAIL: {err}"
                    ))
                })?;

                let private_key = std::env::var("GOOGLE_PRIVATE_KEY").map_err(|err| {
                    golem_tts::error::Error::EnvVariablesNotSet(format!(
                        "Failed to load GOOGLE_PRIVATE_KEY: {err}"
                    ))
                })?;

                gcp_auth::ServiceAccountKey::new(project_id, client_email, private_key)
            };

            let api_client = GoogleTtsApi::new(service_acc_key, WstdHttpClient::new())?;
            Ok(api_client)
        })
    }
}

impl TtsGuest for GoogleTtsComponent {
    type SynthesisStream = GoogleTtsStream;
    type VoiceConversionStream = GoogleTtsStream;

    fn list_voices(filter: Option<VoiceFilter>) -> Result<Vec<VoiceInfo>, TtsError> {
        let voices = block_on(async {
            let client = Self::create_or_get_client()?;
            client.list_voices().await
        })?;
        Ok(apply_voice_filter(voices, filter))
    }

    fn get_voice(voice_id: String) -> Result<VoiceInfo, TtsError> {
        let voices = Self::list_voices(None)?;
        voices
            .into_iter()
            .find(|voice| voice.id == voice_id)
            .ok_or_else(|| TtsError::VoiceNotFound(voice_id))
    }

    fn search_voices(
        query: String,
        filter: Option<VoiceFilter>,
    ) -> Result<Vec<VoiceInfo>, TtsError> {
        let voices = Self::list_voices(filter)?;
        Ok(voices
            .into_iter()
            .filter(|voice| voice.name.to_lowercase().contains(&query.to_lowercase()))
            .collect())
    }

    fn list_languages() -> Result<Vec<String>, TtsError> {
        Ok(vec![
            "en-US".to_string(),
            "en-GB".to_string(),
            "es-ES".to_string(),
            "fr-FR".to_string(),
            "de-DE".to_string(),
        ])
    }

    fn synthesize(request: SynthesisRequest) -> Result<SynthesisResult, TtsError> {
        block_on(async {
            let client = Self::create_or_get_client()?;
            client
                .synthesize(request.input, request.voice_id, request.options)
                .await
                .map_err(Into::into)
        })
    }

    fn synthesize_batch(
        requests: Vec<SynthesisRequest>,
    ) -> Result<Vec<SynthesisResult>, TtsError> {
        requests
            .into_iter()
            .map(Self::synthesize)
            .collect::<Result<Vec<_>, _>>()
    }

    fn get_timing_marks(
        _input: TextInput,
        _voice_id: String,
    ) -> Result<Vec<TimingInfo>, TtsError> {
        Ok(Vec::new())
    }

    fn validate_input(
        input: TextInput,
        _voice_id: String,
    ) -> Result<ValidationResult, TtsError> {
        Ok(ValidationResult {
            is_valid: !input.content.trim().is_empty(),
            character_count: input.content.chars().count() as u32,
            estimated_duration: None,
            warnings: Vec::new(),
            errors: Vec::new(),
        })
    }

    fn create_stream(request: StreamRequest) -> Result<Self::SynthesisStream, TtsError> {
        Ok(GoogleTtsStream::new(request))
    }

    fn create_voice_conversion_stream(
        request: StreamRequest,
    ) -> Result<Self::VoiceConversionStream, TtsError> {
        Ok(GoogleTtsStream::new(request))
    }

    fn create_voice_clone(
        _name: String,
        _audio_samples: Vec<AudioSample>,
        _description: Option<String>,
    ) -> Result<String, TtsError> {
        Err(TtsError::UnsupportedOperation(
            "Google TTS voice cloning unsupported".to_string(),
        ))
    }

    fn design_voice(
        _name: String,
        _characteristics: VoiceDesignParams,
    ) -> Result<String, TtsError> {
        Err(TtsError::UnsupportedOperation(
            "Google TTS voice design unsupported".to_string(),
        ))
    }

    fn convert_voice(
        _input_audio: Vec<u8>,
        _target_voice: String,
        _preserve_timing: Option<bool>,
    ) -> Result<SynthesisResult, TtsError> {
        Err(TtsError::UnsupportedOperation(
            "Voice conversion unsupported".to_string(),
        ))
    }

    fn generate_sound_effect(
        _description: String,
        _duration_seconds: Option<f32>,
        _style_influence: Option<f32>,
    ) -> Result<SynthesisResult, TtsError> {
        Err(TtsError::UnsupportedOperation(
            "Sound effects unsupported".to_string(),
        ))
    }

    fn synthesize_long_form(
        content: String,
        voice_id: String,
        _output_location: String,
        _chapter_breaks: Option<Vec<u32>>,
    ) -> Result<LongFormResult, TtsError> {
        let result = Self::synthesize(SynthesisRequest {
            input: TextInput {
                content,
                text_type: golem_tts::golem::tts::types::TextType::Plain,
                language: None,
            },
            voice_id,
            options: None,
        })?;
        Ok(LongFormResult {
            output_location: "inline".to_string(),
            total_duration: result.metadata.duration_seconds,
            chapter_durations: None,
            metadata: result.metadata,
        })
    }
}

impl ExtendedGuest for GoogleTtsComponent {}

type DurableGoogleComponent = DurableTts<GoogleTtsComponent>;

golem_tts::export_tts!(DurableGoogleComponent with_types_in golem_tts);

struct GoogleTtsStream {
    request: StreamRequest,
    buffer: RefCell<Vec<AudioChunk>>,
    finished: RefCell<bool>,
}

impl GoogleTtsStream {
    fn new(request: StreamRequest) -> Self {
        Self {
            request,
            buffer: RefCell::new(Vec::new()),
            finished: RefCell::new(false),
        }
    }
}

impl golem_tts::guest::TtsStreamGuest for GoogleTtsStream {
    fn send_text(&self, input: TextInput) -> Result<(), TtsError> {
        let result = GoogleTtsComponent::synthesize(SynthesisRequest {
            input,
            voice_id: self.request.voice_id.clone(),
            options: self.request.options.clone(),
        })?;
        let chunk = AudioChunk {
            data: result.audio_data,
            sequence_number: 0,
            is_final: true,
            timing_info: None,
        };
        self.buffer.borrow_mut().push(chunk);
        Ok(())
    }

    fn finish(&self) -> Result<(), TtsError> {
        *self.finished.borrow_mut() = true;
        Ok(())
    }

    fn receive_chunk(&self) -> Result<Option<AudioChunk>, TtsError> {
        Ok(self.buffer.borrow_mut().pop())
    }

    fn has_pending_audio(&self) -> bool {
        !self.buffer.borrow().is_empty()
    }

    fn close(&self) {}
}

impl golem_tts::guest::VoiceConversionStreamGuest for GoogleTtsStream {
    fn send_audio(&self, _audio_data: Vec<u8>) -> Result<(), TtsError> {
        Err(TtsError::UnsupportedOperation(
            "Voice conversion unsupported".to_string(),
        ))
    }

    fn receive_converted(&self) -> Result<Option<AudioChunk>, TtsError> {
        Ok(None)
    }

    fn finish(&self) -> Result<(), TtsError> {
        Ok(())
    }

    fn close(&self) {}
}

#[derive(Clone)]
struct GoogleTtsApi<HC: HttpClient + Clone> {
    auth: gcp_auth::GcpAuth<HC>,
    http_client: HC,
}

impl<HC: HttpClient + Clone> GoogleTtsApi<HC> {
    fn new(
        service_account_key: gcp_auth::ServiceAccountKey,
        http_client: HC,
    ) -> Result<Self, golem_tts::error::Error> {
        let auth = gcp_auth::GcpAuth::new(service_account_key, http_client.clone())
            .map_err(|err| golem_tts::error::Error::AuthError(err.to_string()))?;
        Ok(Self { auth, http_client })
    }

    async fn list_voices(&self) -> Result<Vec<VoiceInfo>, golem_tts::error::Error> {
        let token = self
            .auth
            .get_access_token()
            .await
            .map_err(|err| golem_tts::error::Error::AuthError(err.to_string()))?;
        let uri = format!(
            "https://texttospeech.googleapis.com/v1/voices?languageCode=en-US"
        );
        let request = http::Request::builder()
            .method(http::Method::GET)
            .uri(&uri)
            .header(http::header::AUTHORIZATION, format!("Bearer {token}"))
            .body(Bytes::new())
            .map_err(|err| golem_tts::error::Error::Http("voices".to_string(), err.into()))?;
        let response = self
            .http_client
            .execute(request)
            .await
            .map_err(|err| golem_tts::error::Error::Http("voices".to_string(), err))?;
        if !response.status().is_success() {
            return Err(golem_tts::error::Error::ServiceUnavailable(
                String::from_utf8_lossy(response.body()).to_string(),
            ));
        }
        let body: GoogleVoicesResponse = serde_json::from_slice(response.body())
            .map_err(|err| golem_tts::error::Error::Internal(err.to_string()))?;
        Ok(body
            .voices
            .into_iter()
            .map(|voice| VoiceInfo {
                id: voice.name.clone(),
                name: voice.name.clone(),
                language: voice.language_codes.first().cloned().unwrap_or_default(),
                additional_languages: voice.language_codes.clone(),
                gender: match voice.ssml_gender.as_str() {
                    "MALE" => VoiceGender::Male,
                    "FEMALE" => VoiceGender::Female,
                    _ => VoiceGender::Neutral,
                },
                quality: VoiceQuality::Neural,
                description: None,
                provider: "google".to_string(),
                sample_rate: voice.natural_sample_rate_hertz,
                is_custom: false,
                is_cloned: false,
                preview_url: None,
                use_cases: Vec::new(),
            })
            .collect())
    }

    async fn synthesize(
        &self,
        input: TextInput,
        voice_id: String,
        options: Option<WitSynthesisOptions>,
    ) -> Result<SynthesisResult, golem_tts::error::Error> {
        let token = self
            .auth
            .get_access_token()
            .await
            .map_err(|err| golem_tts::error::Error::AuthError(err.to_string()))?;
        let request_body = GoogleSynthesisRequest::new(input, voice_id, options)?;
        let request_json = serde_json::to_vec(&request_body)
            .map_err(|err| golem_tts::error::Error::Internal(err.to_string()))?;
        let request = http::Request::builder()
            .method(http::Method::POST)
            .uri("https://texttospeech.googleapis.com/v1/text:synthesize")
            .header(http::header::AUTHORIZATION, format!("Bearer {token}"))
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(Bytes::from(request_json))
            .map_err(|err| golem_tts::error::Error::Http("synthesize".to_string(), err.into()))?;
        let response = self
            .http_client
            .execute(request)
            .await
            .map_err(|err| golem_tts::error::Error::Http("synthesize".to_string(), err))?;
        if !response.status().is_success() {
            return Err(golem_tts::error::Error::SynthesisFailed(
                String::from_utf8_lossy(response.body()).to_string(),
            ));
        }
        let body: GoogleSynthesisResponse = serde_json::from_slice(response.body())
            .map_err(|err| golem_tts::error::Error::Internal(err.to_string()))?;
        let audio = base64::engine::general_purpose::STANDARD
            .decode(body.audio_content.as_bytes())
            .map_err(|err| golem_tts::error::Error::Internal(err.to_string()))?;
        Ok(SynthesisResult {
            audio_data: audio.clone(),
            metadata: SynthesisMetadata {
                duration_seconds: 0.0,
                character_count: request_body.input.text.chars().count() as u32,
                word_count: request_body.input.text.split_whitespace().count() as u32,
                audio_size_bytes: audio.len() as u32,
                request_id: Uuid::new_v4().to_string(),
                provider_info: Some("google".to_string()),
            },
        })
    }
}

fn apply_voice_filter(mut voices: Vec<VoiceInfo>, filter: Option<VoiceFilter>) -> Vec<VoiceInfo> {
    if let Some(filter) = filter {
        if let Some(language) = filter.language {
            voices.retain(|voice| voice.language == language);
        }
        if let Some(query) = filter.search_query {
            voices.retain(|voice| voice.name.to_lowercase().contains(&query.to_lowercase()));
        }
    }
    voices
}

#[derive(Debug, Deserialize)]
struct GoogleVoicesResponse {
    voices: Vec<GoogleVoice>,
}

#[derive(Debug, Deserialize)]
struct GoogleVoice {
    name: String,
    #[serde(rename = "languageCodes")]
    language_codes: Vec<String>,
    #[serde(rename = "ssmlGender")]
    ssml_gender: String,
    #[serde(rename = "naturalSampleRateHertz")]
    natural_sample_rate_hertz: u32,
}

#[derive(Debug, Serialize)]
struct GoogleSynthesisRequest {
    input: GoogleInput,
    voice: GoogleVoiceConfig,
    audio_config: GoogleAudioConfig,
}

impl GoogleSynthesisRequest {
    fn new(
        input: TextInput,
        voice_id: String,
        options: Option<WitSynthesisOptions>,
    ) -> Result<Self, golem_tts::error::Error> {
        let audio_format = options
            .as_ref()
            .and_then(|opts| opts.audio_config.as_ref())
            .map(|cfg| cfg.format)
            .unwrap_or(AudioFormat::Mp3);
        Ok(Self {
            input: GoogleInput { text: input.content },
            voice: GoogleVoiceConfig {
                name: voice_id,
                language_code: "en-US".to_string(),
            },
            audio_config: GoogleAudioConfig {
                audio_encoding: match audio_format {
                    AudioFormat::Mp3 => "MP3",
                    AudioFormat::Wav => "LINEAR16",
                    AudioFormat::Pcm => "LINEAR16",
                    AudioFormat::OggOpus => "OGG_OPUS",
                    AudioFormat::Aac => "MP3",
                    AudioFormat::Flac => "FLAC",
                    AudioFormat::Mulaw => "MULAW",
                    AudioFormat::Alaw => "ALAW",
                }
                .to_string(),
                speaking_rate: options
                    .as_ref()
                    .and_then(|opts| opts.voice_settings.as_ref())
                    .and_then(|settings| settings.speed),
                pitch: options
                    .as_ref()
                    .and_then(|opts| opts.voice_settings.as_ref())
                    .and_then(|settings| settings.pitch),
            },
        })
    }
}

#[derive(Debug, Serialize)]
struct GoogleInput {
    text: String,
}

#[derive(Debug, Serialize)]
struct GoogleVoiceConfig {
    name: String,
    #[serde(rename = "languageCode")]
    language_code: String,
}

#[derive(Debug, Serialize)]
struct GoogleAudioConfig {
    #[serde(rename = "audioEncoding")]
    audio_encoding: String,
    #[serde(skip_serializing_if = "Option::is_none", rename = "speakingRate")]
    speaking_rate: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pitch: Option<f32>,
}

#[derive(Debug, Deserialize)]
struct GoogleSynthesisResponse {
    #[serde(rename = "audioContent")]
    audio_content: String,
}
