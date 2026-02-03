use bytes::Bytes;
use golem_tts::durability::{DurableTts, ExtendedGuest};
use golem_tts::golem::tts::advanced::{AudioSample, LongFormResult, VoiceDesignParams};
// use golem_tts::golem::tts::streaming::SynthesisOptions;
use golem_tts::golem::tts::synthesis::{SynthesisOptions as WitSynthesisOptions, ValidationResult};
use golem_tts::golem::tts::types::{
    AudioChunk, SynthesisMetadata, SynthesisResult, TextInput, TimingInfo, TtsError, VoiceSettings,
};
use golem_tts::golem::tts::voices::{VoiceFilter, VoiceGender, VoiceInfo, VoiceQuality};
use golem_tts::guest::{StreamRequest, SynthesisRequest, TtsGuest};
use golem_tts::http::{HttpClient, WstdHttpClient};
use golem_rust::Uuid;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use wstd::runtime::block_on;

static API_CLIENT: OnceCell<ElevenLabsApi<WstdHttpClient>> = OnceCell::new();

struct ElevenLabsComponent;

impl ElevenLabsComponent {
    fn create_or_get_client() -> Result<&'static ElevenLabsApi<WstdHttpClient>, golem_tts::error::Error> {
        API_CLIENT.get_or_try_init(|| {
            let api_key = std::env::var("ELEVENLABS_API_KEY").map_err(|err| {
                golem_tts::error::Error::EnvVariablesNotSet(format!(
                    "Failed to load ELEVENLABS_API_KEY: {err}"
                ))
            })?;
            let model_version = std::env::var("ELEVENLABS_MODEL_VERSION").ok();
            Ok(ElevenLabsApi::new(api_key, model_version, WstdHttpClient::new()))
        })
    }
}

impl TtsGuest for ElevenLabsComponent {
    type SynthesisStream = ElevenLabsStream;
    type VoiceConversionStream = ElevenLabsStream;

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
        Ok(ElevenLabsStream::new(request))
    }

    fn create_voice_conversion_stream(
        request: StreamRequest,
    ) -> Result<Self::VoiceConversionStream, TtsError> {
        Ok(ElevenLabsStream::new(request))
    }

    fn create_voice_clone(
        _name: String,
        _audio_samples: Vec<AudioSample>,
        _description: Option<String>,
    ) -> Result<String, TtsError> {
        Err(TtsError::UnsupportedOperation(
            "ElevenLabs cloning requires UI workflow".to_string(),
        ))
    }

    fn design_voice(
        _name: String,
        _characteristics: VoiceDesignParams,
    ) -> Result<String, TtsError> {
        Err(TtsError::UnsupportedOperation(
            "Voice design unsupported".to_string(),
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

impl ExtendedGuest for ElevenLabsComponent {}

type DurableElevenLabsComponent = DurableTts<ElevenLabsComponent>;

golem_tts::export_tts!(DurableElevenLabsComponent with_types_in golem_tts);

struct ElevenLabsStream {
    request: StreamRequest,
    buffer: RefCell<Vec<AudioChunk>>,
    finished: RefCell<bool>,
}

impl ElevenLabsStream {
    fn new(request: StreamRequest) -> Self {
        Self {
            request,
            buffer: RefCell::new(Vec::new()),
            finished: RefCell::new(false),
        }
    }
}

impl golem_tts::guest::TtsStreamGuest for ElevenLabsStream {
    fn send_text(&self, input: TextInput) -> Result<(), TtsError> {
        let result = ElevenLabsComponent::synthesize(SynthesisRequest {
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

impl golem_tts::guest::VoiceConversionStreamGuest for ElevenLabsStream {
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
struct ElevenLabsApi<HC: HttpClient> {
    api_key: String,
    model_version: Option<String>,
    http_client: HC,
}

impl<HC: HttpClient> ElevenLabsApi<HC> {
    fn new(api_key: String, model_version: Option<String>, http_client: HC) -> Self {
        Self {
            api_key,
            model_version,
            http_client,
        }
    }

    async fn list_voices(&self) -> Result<Vec<VoiceInfo>, golem_tts::error::Error> {
        let request = http::Request::builder()
            .method(http::Method::GET)
            .uri("https://api.elevenlabs.io/v1/voices")
            .header("xi-api-key", &self.api_key)
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
        let body: ElevenLabsVoicesResponse = serde_json::from_slice(response.body())
            .map_err(|err| golem_tts::error::Error::Internal(err.to_string()))?;
        Ok(body
            .voices
            .into_iter()
            .map(|voice| VoiceInfo {
                id: voice.voice_id.clone(),
                name: voice.name.clone(),
                language: voice.labels.language.unwrap_or_else(|| "en-US".to_string()),
                additional_languages: Vec::new(),
                gender: VoiceGender::Neutral,
                quality: VoiceQuality::Neural,
                description: Some(voice.description.clone()),
                provider: "elevenlabs".to_string(),
                sample_rate: 24000,
                is_custom: voice.labels.voice_type.as_deref() == Some("custom"),
                is_cloned: voice.labels.voice_type.as_deref() == Some("cloned"),
                preview_url: voice.preview_url,
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
        let request_body = ElevenLabsSynthesisRequest {
            text: input.content,
            model_id: self.model_version.clone(),
            voice_settings: options.and_then(|opts| opts.voice_settings).map(|settings| {
                ElevenLabsVoiceSettings::from(settings)
            }),
        };
        let request_json = serde_json::to_vec(&request_body)
            .map_err(|err| golem_tts::error::Error::Internal(err.to_string()))?;
        let uri = format!("https://api.elevenlabs.io/v1/text-to-speech/{voice_id}");
        let request = http::Request::builder()
            .method(http::Method::POST)
            .uri(&uri)
            .header("xi-api-key", &self.api_key)
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(Bytes::from(request_json))
            .map_err(|err| golem_tts::error::Error::Http(voice_id.clone(), err.into()))?;
        let response = self
            .http_client
            .execute(request)
            .await
            .map_err(|err| golem_tts::error::Error::Http(voice_id.clone(), err))?;
        if !response.status().is_success() {
            return Err(golem_tts::error::Error::SynthesisFailed(
                String::from_utf8_lossy(response.body()).to_string(),
            ));
        }
        let audio = response.body().to_vec();
        Ok(SynthesisResult {
            audio_data: audio.clone(),
            metadata: SynthesisMetadata {
                duration_seconds: 0.0,
                character_count: request_body.text.chars().count() as u32,
                word_count: request_body.text.split_whitespace().count() as u32,
                audio_size_bytes: audio.len() as u32,
                request_id: Uuid::new_v4().to_string(),
                provider_info: Some("elevenlabs".to_string()),
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
struct ElevenLabsVoicesResponse {
    voices: Vec<ElevenLabsVoice>,
}

#[derive(Debug, Deserialize)]
struct ElevenLabsVoice {
    voice_id: String,
    name: String,
    description: String,
    preview_url: Option<String>,
    labels: ElevenLabsVoiceLabels,
}

#[derive(Debug, Deserialize)]
struct ElevenLabsVoiceLabels {
    language: Option<String>,
    voice_type: Option<String>,
}

#[derive(Debug, Serialize)]
struct ElevenLabsSynthesisRequest {
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    model_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    voice_settings: Option<ElevenLabsVoiceSettings>,
}

#[derive(Debug, Serialize)]
struct ElevenLabsVoiceSettings {
    stability: Option<f32>,
    similarity_boost: Option<f32>,
    style: Option<f32>,
    use_speaker_boost: Option<bool>,
}

impl From<VoiceSettings> for ElevenLabsVoiceSettings {
    fn from(settings: VoiceSettings) -> Self {
        Self {
            stability: settings.stability,
            similarity_boost: settings.similarity,
            style: settings.style,
            use_speaker_boost: None,
        }
    }
}
