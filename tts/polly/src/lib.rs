use bytes::Bytes;
use chrono::Utc;
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
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use wstd::runtime::block_on;

mod aws_signer;

static API_CLIENT: OnceCell<PollyApi<WstdHttpClient>> = OnceCell::new();

struct PollyComponent;

impl PollyComponent {
    fn create_or_get_client() -> Result<&'static PollyApi<WstdHttpClient>, golem_tts::error::Error> {
        API_CLIENT.get_or_try_init(|| {
            let region = std::env::var("AWS_REGION")
                .map_err(|err| golem_tts::error::Error::EnvVariablesNotSet(format!("Failed to load AWS_REGION: {err}")))?;
            let access_key = std::env::var("AWS_ACCESS_KEY_ID")
                .map_err(|err| golem_tts::error::Error::EnvVariablesNotSet(format!("Failed to load AWS_ACCESS_KEY_ID: {err}")))?;
            let secret_key = std::env::var("AWS_SECRET_ACCESS_KEY")
                .map_err(|err| golem_tts::error::Error::EnvVariablesNotSet(format!("Failed to load AWS_SECRET_ACCESS_KEY: {err}")))?;
            let session_token = std::env::var("AWS_SESSION_TOKEN").ok();
            Ok(PollyApi::new(
                access_key,
                secret_key,
                session_token,
                region,
                WstdHttpClient::new(),
            ))
        })
    }
}

impl TtsGuest for PollyComponent {
    type SynthesisStream = PollyStream;
    type VoiceConversionStream = PollyStream;

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
        Ok(PollyStream::new(request))
    }

    fn create_voice_conversion_stream(
        request: StreamRequest,
    ) -> Result<Self::VoiceConversionStream, TtsError> {
        Ok(PollyStream::new(request))
    }

    fn create_voice_clone(
        _name: String,
        _audio_samples: Vec<AudioSample>,
        _description: Option<String>,
    ) -> Result<String, TtsError> {
        Err(TtsError::UnsupportedOperation(
            "Polly voice cloning unsupported".to_string(),
        ))
    }

    fn design_voice(
        _name: String,
        _characteristics: VoiceDesignParams,
    ) -> Result<String, TtsError> {
        Err(TtsError::UnsupportedOperation(
            "Polly voice design unsupported".to_string(),
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

impl ExtendedGuest for PollyComponent {}

type DurablePollyComponent = DurableTts<PollyComponent>;

golem_tts::export_tts!(DurablePollyComponent with_types_in golem_tts);

struct PollyStream {
    request: StreamRequest,
    buffer: RefCell<Vec<AudioChunk>>,
    finished: RefCell<bool>,
}

impl PollyStream {
    fn new(request: StreamRequest) -> Self {
        Self {
            request,
            buffer: RefCell::new(Vec::new()),
            finished: RefCell::new(false),
        }
    }
}

impl golem_tts::guest::TtsStreamGuest for PollyStream {
    fn send_text(&self, input: TextInput) -> Result<(), TtsError> {
        let result = PollyComponent::synthesize(SynthesisRequest {
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

impl golem_tts::guest::VoiceConversionStreamGuest for PollyStream {
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
struct PollyApi<HC: HttpClient> {
    signer: aws_signer::AwsSignatureV4,
    session_token: Option<String>,
    http_client: HC,
}

impl<HC: HttpClient> PollyApi<HC> {
    fn new(
        access_key: String,
        secret_key: String,
        session_token: Option<String>,
        region: String,
        http_client: HC,
    ) -> Self {
        Self {
            signer: aws_signer::AwsSignatureV4::for_polly(access_key, secret_key, region),
            session_token,
            http_client,
        }
    }

    async fn list_voices(&self) -> Result<Vec<VoiceInfo>, golem_tts::error::Error> {
        let region = self.signer.get_region();
        let uri = format!("https://polly.{region}.amazonaws.com/v1/voices");
        let request = http::Request::builder()
            .method(http::Method::GET)
            .uri(&uri)
            .body(Bytes::new())
            .map_err(|err| golem_tts::error::Error::Http("voices".to_string(), err.into()))?;
        let mut request = self
            .signer
            .sign_request(request, Utc::now())
            .map_err(|err| golem_tts::error::Error::Internal(err.to_string()))?;
        if let Some(token) = &self.session_token {
            request
                .headers_mut()
                .insert("x-amz-security-token", token.parse().unwrap());
        }
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
        let body: PollyVoicesResponse = serde_json::from_slice(response.body())
            .map_err(|err| golem_tts::error::Error::Internal(err.to_string()))?;
        Ok(body
            .voices
            .into_iter()
            .map(|voice| VoiceInfo {
                id: voice.id.clone(),
                name: voice.name.clone(),
                language: voice.language_code.clone(),
                additional_languages: Vec::new(),
                gender: if voice.gender == "Female" {
                    VoiceGender::Female
                } else {
                    VoiceGender::Male
                },
                quality: VoiceQuality::Neural,
                description: None,
                provider: "aws-polly".to_string(),
                sample_rate: 22050,
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
        let region = self.signer.get_region();
        let uri = format!("https://polly.{region}.amazonaws.com/v1/speech");
        let request_body = PollySynthesisRequest::new(input, voice_id, options)?;
        let request_json = serde_json::to_vec(&request_body)
            .map_err(|err| golem_tts::error::Error::Internal(err.to_string()))?;
        let request = http::Request::builder()
            .method(http::Method::POST)
            .uri(&uri)
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(Bytes::from(request_json))
            .map_err(|err| golem_tts::error::Error::Http("synthesize".to_string(), err.into()))?;
        let mut request = self
            .signer
            .sign_request(request, Utc::now())
            .map_err(|err| golem_tts::error::Error::Internal(err.to_string()))?;
        if let Some(token) = &self.session_token {
            request
                .headers_mut()
                .insert("x-amz-security-token", token.parse().unwrap());
        }
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
        let audio = response.body().to_vec();
        Ok(SynthesisResult {
            audio_data: audio.clone(),
            metadata: SynthesisMetadata {
                duration_seconds: 0.0,
                character_count: request_body.text.chars().count() as u32,
                word_count: request_body.text.split_whitespace().count() as u32,
                audio_size_bytes: audio.len() as u32,
                request_id: Uuid::new_v4().to_string(),
                provider_info: Some("aws-polly".to_string()),
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
struct PollyVoicesResponse {
    #[serde(rename = "Voices")]
    voices: Vec<PollyVoice>,
}

#[derive(Debug, Deserialize)]
struct PollyVoice {
    #[serde(rename = "Id")]
    id: String,
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "LanguageCode")]
    language_code: String,
    #[serde(rename = "Gender")]
    gender: String,
}

#[derive(Debug, Serialize)]
struct PollySynthesisRequest {
    #[serde(rename = "Text")]
    text: String,
    #[serde(rename = "VoiceId")]
    voice_id: String,
    #[serde(rename = "OutputFormat")]
    output_format: String,
    #[serde(rename = "TextType")]
    text_type: String,
}

impl PollySynthesisRequest {
    fn new(
        input: TextInput,
        voice_id: String,
        options: Option<WitSynthesisOptions>,
    ) -> Result<Self, golem_tts::error::Error> {
        let format = options
            .as_ref()
            .and_then(|opts| opts.audio_config.as_ref())
            .map(|cfg| cfg.format)
            .unwrap_or(AudioFormat::Mp3);
        Ok(Self {
            text: input.content,
            voice_id,
            output_format: match format {
                AudioFormat::Mp3 => "mp3",
                AudioFormat::Wav => "pcm",
                AudioFormat::Pcm => "pcm",
                AudioFormat::OggOpus => "ogg_vorbis",
                AudioFormat::Aac => "aac",
                AudioFormat::Flac => "pcm",
                AudioFormat::Mulaw => "pcm",
                AudioFormat::Alaw => "pcm",
            }
            .to_string(),
            text_type: match input.text_type {
                golem_tts::golem::tts::types::TextType::Plain => "text",
                golem_tts::golem::tts::types::TextType::Ssml => "ssml",
            }
            .to_string(),
        })
    }
}
