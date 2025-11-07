use crate::aws_signer::PollySigner;
use bytes::Bytes;
use golem_tts::{
    client::{ApiClient, TtsClient},
    config::get_env,
    golem::tts::{
        advanced::{AudioSample, LanguageCode, PronunciationEntry, Voice, VoiceDesignParams},
        synthesis::{SynthesisOptions, TextInput, TimingInfo, ValidationResult},
        types::{SynthesisMetadata, SynthesisResult, TextType, TtsError},
        voices::{LanguageInfo, VoiceFilter},
    },
};
use http::Request;
use log::trace;
use reqwest::header::{HeaderName, HeaderValue};
use reqwest::{header::HeaderMap, Method};
use serde_json;

use crate::{
    error::{from_http_error, unsupported},
    resources::{AwsLongFormOperation, AwsPronunciationLexicon},
    types::{
        GetLexiconResponse, ListVoiceParam, ListVoiceResponse, PutLexiconRequest,
        StartSpeechSynthesisTaskRequest, StartSpeechSynthesisTaskResponse, SynthesizeSpeechParams,
    },
    utils::{create_pls_content, estimate_text_duration},
};

#[derive(Clone)]
pub struct Polly {
    pub client: ApiClient,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub region: String,
    pub base_url: String,
    pub signer: PollySigner,
    pub bucket: String,
}

impl TtsClient for Polly {
    type ClientLongFormOperation = AwsLongFormOperation;

    type ClientPronunciationLexicon = AwsPronunciationLexicon;

    fn new() -> Result<Self, TtsError> {
        let access_key_id = get_env("AWS_ACCESS_KEY_ID")?;
        let bucket = get_env("AWS_S3_BUCKET").map_err(|_| {
            TtsError::InvalidConfiguration(
                "AWS_S3_BUCKET environment variable is required for long-form synthesis"
                    .to_string(),
            )
        })?;
        let secret_access_key = get_env("AWS_SECRET_ACCESS_KEY")?;
        let region = get_env("AWS_REGION")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "us-east-1".to_string());
        let base_url = get_env("TTS_PROVIDER_ENDPOINT")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| format!("https://polly.{}.amazonaws.com", region));

        let client = ApiClient::new(base_url.clone(), HeaderMap::new())?;

        let signer = PollySigner::new(
            access_key_id.clone(),
            secret_access_key.clone(),
            region.clone(),
        );

        Ok(Self {
            client,
            access_key_id,
            secret_access_key,
            region,
            base_url,
            signer,
            bucket: bucket,
        })
    }

    fn synthesize(
        &self,
        input: TextInput,
        voice: String,
        options: Option<SynthesisOptions>,
    ) -> Result<SynthesisResult, TtsError> {
        let mut engine = options
            .as_ref()
            .and_then(|opts| opts.model_id.clone())
            .map(|value| value.trim().to_lowercase())
            .filter(|value| !value.is_empty());

        if engine.is_none() {
            if let Ok(voice_details) = self.get_voice(voice.clone()) {
                let supported: Vec<String> = voice_details
                    .quality
                    .split(',')
                    .map(|value| value.trim().to_lowercase())
                    .filter(|value| !value.is_empty())
                    .collect();

                if supported.iter().any(|value| value == "neural") {
                    engine = Some("neural".to_string());
                } else if let Some(first) = supported.first() {
                    engine = Some(first.clone());
                }
            }
        }

        let output_format = options
            .as_ref()
            .and_then(|opts| {
                opts.audio_config
                    .as_ref()
                    .map(|config| config.format.clone())
            })
            .unwrap_or_else(|| "mp3".to_string());

        let sample_rate = options
            .as_ref()
            .and_then(|opts| opts.audio_config.as_ref())
            .and_then(|config| config.sample_rate)
            .map(|value| value.to_string());

        let text_type = if input.text_type == TextType::Ssml {
            Some("ssml".to_string())
        } else {
            Some("text".to_string())
        };

        let body = SynthesizeSpeechParams {
            engine,
            language_code: input.language.clone(),
            lexicon_names: None,
            output_format: Some(output_format),
            sample_rate,
            speech_mark_types: None,
            text: input.content.clone(),
            text_type,
            voice_id: voice.clone(),
        };

        let body_json =
            serde_json::to_string(&body).map_err(|e| TtsError::InternalError(e.to_string()))?;

        let full_uri = format!("{}/v1/speech", self.base_url);
        let request = Request::builder()
            .method("POST")
            .uri(full_uri)
            .header("content-type", "application/x-amz-json-1.0")
            .header("x-amz-target", "Polly_2016-06-10.SynthesizeSpeech")
            .body(body_json.as_bytes().to_vec().into())
            .map_err(|e| TtsError::InternalError(e.to_string()))?;
        let signed_request = self
            .signer
            .sign_request(request)
            .map_err(|e| TtsError::InternalError(e.to_string()))?;
        let mut headers = HeaderMap::new();
        for (key, value) in signed_request.headers().iter() {
            let key = HeaderName::from_bytes(key.as_str().as_bytes())
                .map_err(|_| TtsError::InternalError("Invalid header name".to_string()))?;
            let value = HeaderValue::from_bytes(value.as_bytes())
                .map_err(|_| TtsError::InternalError("Invalid header value".to_string()))?;
            headers.insert(key, value);
        }
        let response = self
            .client
            .make_audio_request::<SynthesizeSpeechParams, (), _>(
                Method::POST,
                "/v1/speech",
                body,
                None,
                Some(&headers),
                from_http_error,
            )?;

        let audio_size_bytes = response.len() as u32;
        Ok(SynthesisResult {
            audio_data: response,
            metadata: SynthesisMetadata {
                duration_seconds: 0.0,
                character_count: input.content.len() as u32,
                word_count: 0,
                audio_size_bytes,
                request_id: "".to_string(),
                provider_info: Some("AWS Polly".to_string()),
            },
        })
    }

    fn synthesize_batch(
        &self,
        inputs: Vec<TextInput>,
        voice: String,
        options: Option<SynthesisOptions>,
    ) -> Result<Vec<SynthesisResult>, TtsError> {
        let mut results = Vec::with_capacity(inputs.len());
        for input in inputs {
            let result = self.synthesize(input, voice.clone(), options.clone())?;
            results.push(result);
        }

        Ok(results)
    }

    fn get_timing_marks(
        &self,
        _input: TextInput,
        _voice: String,
    ) -> Result<Vec<TimingInfo>, TtsError> {
        unsupported("Timing marks without audio synthesise is not supported by AWS Polly")
    }

    fn validate_input(
        &self,
        input: TextInput,
        voice: String,
    ) -> Result<ValidationResult, TtsError> {
        let text = input.content;
        let mut warnings = Vec::new();
        let mut errors = Vec::new();

        if text.len() > 3000 {
            errors.push("Text exceeds maximum length of 3000 characters for AWS Polly".to_string());
        }

        if text.trim_start().starts_with('<')
            && (!text.contains("</speak>") || !text.contains("<speak"))
        {
            errors.push("Invalid SSML format - missing speak tags".to_string());
        }

        if text.trim().is_empty() {
            errors.push("Text cannot be empty".to_string());
        }

        if voice.is_empty() {
            errors.push("Voice ID cannot be empty".to_string());
        }

        if text.chars().any(|c| c as u32 > 127) && text.trim_start().starts_with('<') {
            warnings.push("Non-ASCII characters in SSML may cause issues".to_string());
        }

        Ok(ValidationResult {
            is_valid: errors.is_empty(),
            errors,
            warnings,
            character_count: text.len() as u32,
            estimated_duration: Some(estimate_text_duration(&text)),
        })
    }

    fn list_voices(&self, filter: Option<VoiceFilter>) -> Result<Vec<Voice>, TtsError> {
        trace!("Listing available voices.");
        let params = ListVoiceParam {
            engine: filter.as_ref().and_then(|f| f.quality.clone()),
            include_additional_language_codes: Some(true),
            language_code: filter.as_ref().map(|f| f.language.clone().unwrap()),
            next_token: None,
        };

        let query = serde_urlencoded::to_string(&params)
            .map_err(|e| TtsError::InternalError(e.to_string()))?;
        let full_uri = format!("{}/v1/voices?{}", self.base_url, query);

        let request = Request::builder()
            .method("GET")
            .uri(full_uri)
            .body(Bytes::new())
            .map_err(|e| TtsError::InternalError(e.to_string()))?;
        let signed_request = self
            .signer
            .sign_request(request)
            .map_err(|e| TtsError::InternalError(e.to_string()))?;
        let mut headers = HeaderMap::new();
        for (key, value) in signed_request.headers().iter() {
            let key = HeaderName::from_bytes(key.as_str().as_bytes())
                .map_err(|_| TtsError::InternalError("Invalid header name".to_string()))?;
            let value = HeaderValue::from_bytes(value.as_bytes())
                .map_err(|_| TtsError::InternalError("Invalid header value".to_string()))?;
            headers.insert(key, value);
        }
        let response = self
            .client
            .make_request::<ListVoiceResponse, (), ListVoiceParam, _>(
                Method::GET,
                "/v1/voices",
                (),
                Some(&params),
                Some(&headers),
                from_http_error,
            )?;

        let voices = response.voices.unwrap_or_default();
        Ok(voices.iter().map(|v| Voice::from(v.clone())).collect())
    }

    fn get_voice(&self, voice_id: String) -> Result<Voice, TtsError> {
        let full_uri = format!("{}/v1/voices", self.base_url);
        let request = Request::builder()
            .method("GET")
            .uri(full_uri)
            .body(Bytes::new())
            .map_err(|e| TtsError::InternalError(e.to_string()))?;
        let signed_request = self
            .signer
            .sign_request(request)
            .map_err(|e| TtsError::InternalError(e.to_string()))?;
        let mut headers = HeaderMap::new();
        for (key, value) in signed_request.headers().iter() {
            let key = HeaderName::from_bytes(key.as_str().as_bytes())
                .map_err(|_| TtsError::InternalError("Invalid header name".to_string()))?;
            let value = HeaderValue::from_bytes(value.as_bytes())
                .map_err(|_| TtsError::InternalError("Invalid header value".to_string()))?;
            headers.insert(key, value);
        }
        let result = self.client.make_request::<ListVoiceResponse, (), (), _>(
            Method::GET,
            "/v1/voices",
            (),
            None,
            Some(&headers),
            from_http_error,
        )?;
        if let Some(voices) = result.voices {
            for voice in voices {
                if voice.name == voice_id {
                    return Ok(Voice::from(voice));
                }
            }
        }
        Err(TtsError::VoiceNotFound(voice_id.to_string()))
    }

    fn list_languages(&self) -> Result<Vec<LanguageInfo>, TtsError> {
        Ok(vec![
            LanguageInfo {
                code: "en-US".to_string(),
                name: "English (US)".to_string(),
                native_name: "English (United States)".to_string(),
                voice_count: 16,
            },
            LanguageInfo {
                code: "en-GB".to_string(),
                name: "English (UK)".to_string(),
                native_name: "English (United Kingdom)".to_string(),
                voice_count: 5,
            },
            LanguageInfo {
                code: "en-AU".to_string(),
                name: "English (Australia)".to_string(),
                native_name: "English (Australia)".to_string(),
                voice_count: 2,
            },
            LanguageInfo {
                code: "en-IN".to_string(),
                name: "English (India)".to_string(),
                native_name: "English (India)".to_string(),
                voice_count: 3,
            },
            LanguageInfo {
                code: "es-ES".to_string(),
                name: "Spanish (Spain)".to_string(),
                native_name: "Español (España)".to_string(),
                voice_count: 4,
            },
            LanguageInfo {
                code: "es-MX".to_string(),
                name: "Spanish (Mexico)".to_string(),
                native_name: "Español (México)".to_string(),
                voice_count: 2,
            },
            LanguageInfo {
                code: "es-US".to_string(),
                name: "Spanish (US)".to_string(),
                native_name: "Español (Estados Unidos)".to_string(),
                voice_count: 3,
            },
            LanguageInfo {
                code: "fr-FR".to_string(),
                name: "French (France)".to_string(),
                native_name: "Français (France)".to_string(),
                voice_count: 4,
            },
            LanguageInfo {
                code: "fr-CA".to_string(),
                name: "French (Canada)".to_string(),
                native_name: "Français (Canada)".to_string(),
                voice_count: 1,
            },
            LanguageInfo {
                code: "de-DE".to_string(),
                name: "German".to_string(),
                native_name: "Deutsch".to_string(),
                voice_count: 3,
            },
            LanguageInfo {
                code: "it-IT".to_string(),
                name: "Italian".to_string(),
                native_name: "Italiano".to_string(),
                voice_count: 2,
            },
            LanguageInfo {
                code: "pt-PT".to_string(),
                name: "Portuguese (Portugal)".to_string(),
                native_name: "Português (Portugal)".to_string(),
                voice_count: 2,
            },
            LanguageInfo {
                code: "pt-BR".to_string(),
                name: "Portuguese (Brazil)".to_string(),
                native_name: "Português (Brasil)".to_string(),
                voice_count: 3,
            },
            LanguageInfo {
                code: "ja-JP".to_string(),
                name: "Japanese".to_string(),
                native_name: "日本語".to_string(),
                voice_count: 3,
            },
            LanguageInfo {
                code: "ko-KR".to_string(),
                name: "Korean".to_string(),
                native_name: "한국어".to_string(),
                voice_count: 1,
            },
            LanguageInfo {
                code: "zh-CN".to_string(),
                name: "Chinese (Simplified)".to_string(),
                native_name: "中文（简体）".to_string(),
                voice_count: 1,
            },
            LanguageInfo {
                code: "cmn-CN".to_string(),
                name: "Chinese Mandarin".to_string(),
                native_name: "普通话".to_string(),
                voice_count: 1,
            },
            LanguageInfo {
                code: "ar".to_string(),
                name: "Arabic".to_string(),
                native_name: "العربية".to_string(),
                voice_count: 1,
            },
            LanguageInfo {
                code: "hi-IN".to_string(),
                name: "Hindi".to_string(),
                native_name: "हिन्दी".to_string(),
                voice_count: 2,
            },
            LanguageInfo {
                code: "ru-RU".to_string(),
                name: "Russian".to_string(),
                native_name: "Русский".to_string(),
                voice_count: 2,
            },
            LanguageInfo {
                code: "nl-NL".to_string(),
                name: "Dutch".to_string(),
                native_name: "Nederlands".to_string(),
                voice_count: 2,
            },
            LanguageInfo {
                code: "pl-PL".to_string(),
                name: "Polish".to_string(),
                native_name: "Polski".to_string(),
                voice_count: 2,
            },
            LanguageInfo {
                code: "sv-SE".to_string(),
                name: "Swedish".to_string(),
                native_name: "Svenska".to_string(),
                voice_count: 1,
            },
            LanguageInfo {
                code: "nb-NO".to_string(),
                name: "Norwegian".to_string(),
                native_name: "Norsk".to_string(),
                voice_count: 1,
            },
            LanguageInfo {
                code: "da-DK".to_string(),
                name: "Danish".to_string(),
                native_name: "Dansk".to_string(),
                voice_count: 2,
            },
            LanguageInfo {
                code: "tr-TR".to_string(),
                name: "Turkish".to_string(),
                native_name: "Türkçe".to_string(),
                voice_count: 1,
            },
            LanguageInfo {
                code: "ro-RO".to_string(),
                name: "Romanian".to_string(),
                native_name: "Română".to_string(),
                voice_count: 1,
            },
            LanguageInfo {
                code: "cy-GB".to_string(),
                name: "Welsh".to_string(),
                native_name: "Cymraeg".to_string(),
                voice_count: 1,
            },
            LanguageInfo {
                code: "is-IS".to_string(),
                name: "Icelandic".to_string(),
                native_name: "Íslenska".to_string(),
                voice_count: 2,
            },
        ])
    }

    fn create_voice_clone(
        &self,
        _name: String,
        _audio_samples: Vec<AudioSample>,
        _description: Option<String>,
    ) -> Result<Voice, TtsError> {
        unsupported("Voice cloning is not supported by AWS Polly")
    }

    fn design_voice(
        &self,
        _name: String,
        _characteristics: VoiceDesignParams,
    ) -> Result<Voice, TtsError> {
        unsupported("Voice design is not supported by AWS Polly")
    }

    fn convert_voice(
        &self,
        _input_audio: Vec<u8>,
        _target_voice: String,
        _preserve_timing: Option<bool>,
    ) -> Result<Vec<u8>, TtsError> {
        unsupported("Voice-to-voice conversion is not supported by AWS Polly")
    }

    fn generate_sound_effect(
        &self,
        _description: String,
        _duration_seconds: Option<f32>,
        _style_influence: Option<f32>,
    ) -> Result<Vec<u8>, TtsError> {
        unsupported("Sound effect generation is not supported by AWS Polly")
    }

    fn create_lexicon(
        &self,
        name: String,
        language: LanguageCode,
        entries: Option<Vec<PronunciationEntry>>,
    ) -> Result<Self::ClientPronunciationLexicon, TtsError> {
        let Some(entries) = entries else {
            return Err(TtsError::RequestError(
                "PronunciationEntry is empty.".to_string(),
            ));
        };

        let pls_content = create_pls_content(language.as_str(), &entries);

        let body = PutLexiconRequest {
            content: pls_content,
            name: name.to_string(),
        };

        let put_path = format!("/v1/lexicons/{}", name);

        let body_json =
            serde_json::to_string(&body).map_err(|e| TtsError::InternalError(e.to_string()))?;

        let full_uri = format!("{}{}", self.base_url, put_path);
        let request = Request::builder()
            .method("PUT")
            .uri(full_uri)
            .header("content-type", "application/json")
            .body(body_json.as_bytes().to_vec().into())
            .map_err(|e| TtsError::InternalError(e.to_string()))?;
        let signed_request = self
            .signer
            .sign_request(request)
            .map_err(|e| TtsError::InternalError(e.to_string()))?;
        let mut headers = HeaderMap::new();
        for (key, value) in signed_request.headers().iter() {
            let key = HeaderName::from_bytes(key.as_str().as_bytes())
                .map_err(|_| TtsError::InternalError("Invalid header name".to_string()))?;
            let value = HeaderValue::from_bytes(value.as_bytes())
                .map_err(|_| TtsError::InternalError("Invalid header value".to_string()))?;
            headers.insert(key, value);
        }
        let _response = self
            .client
            .make_request::<serde_json::Value, PutLexiconRequest, (), _>(
                Method::PUT,
                &put_path,
                body,
                None,
                Some(&headers),
                from_http_error,
            )?;

        let get_path = format!("/v1/lexicons/{}", name);

        let full_uri = format!("{}{}", self.base_url, get_path);
        let request = Request::builder()
            .method("GET")
            .uri(full_uri)
            .body(Bytes::new())
            .map_err(|e| TtsError::InternalError(e.to_string()))?;
        let signed_request = self
            .signer
            .sign_request(request)
            .map_err(|e| TtsError::InternalError(e.to_string()))?;
        let mut headers = HeaderMap::new();
        for (key, value) in signed_request.headers().iter() {
            let key = HeaderName::from_bytes(key.as_str().as_bytes())
                .map_err(|_| TtsError::InternalError("Invalid header name".to_string()))?;
            let value = HeaderValue::from_bytes(value.as_bytes())
                .map_err(|_| TtsError::InternalError("Invalid header value".to_string()))?;
            headers.insert(key, value);
        }
        let response = self.client.make_request::<GetLexiconResponse, (), (), _>(
            Method::GET,
            &get_path,
            (),
            None,
            Some(&headers),
            from_http_error,
        )?;

        Ok(AwsPronunciationLexicon::new(
            response.lexicon,
            language,
            response.lexicon_attributes,
        ))
    }

    fn synthesize_long_form(
        &self,
        content: String,
        voice: String,
        _chapter_breaks: Option<Vec<u32>>,
    ) -> Result<Self::ClientLongFormOperation, TtsError> {
        let key_prefix = "test-audio-files/test8-long-form".to_string();
        let output_location = format!("s3://{}/{}", self.bucket.clone(), key_prefix);

        let body = StartSpeechSynthesisTaskRequest {
            text: content,
            engine: Some("long-form".to_string()),
            language_code: None,
            lexicon_names: None,
            output_format: "mp3".to_string(),
            output_s3_bucket_name: self.bucket.clone(),
            output_s3_key_prefix: Some(key_prefix),
            sample_rate: None,
            sns_topic_arn: None,
            speech_mark_types: None,
            text_type: None,
            voice_id: voice,
        };
        let path = "/v1/synthesisTasks".to_string();
        let body_json =
            serde_json::to_string(&body).map_err(|e| TtsError::InternalError(e.to_string()))?;

        let full_uri = format!("{}{}", self.base_url, path);
        let request = Request::builder()
            .method("POST")
            .uri(full_uri)
            .header("content-type", "application/x-amz-json-1.0")
            .body(body_json.as_bytes().to_vec().into())
            .map_err(|e| TtsError::InternalError(e.to_string()))?;
        let signed_request = self
            .signer
            .sign_request(request)
            .map_err(|e| TtsError::InternalError(e.to_string()))?;
        let mut headers = HeaderMap::new();
        for (key, value) in signed_request.headers().iter() {
            let key = HeaderName::from_bytes(key.as_str().as_bytes())
                .map_err(|_| TtsError::InternalError("Invalid header name".to_string()))?;
            let value = HeaderValue::from_bytes(value.as_bytes())
                .map_err(|_| TtsError::InternalError("Invalid header value".to_string()))?;
            headers.insert(key, value);
        }
        let response = self.client.make_request::<StartSpeechSynthesisTaskResponse, StartSpeechSynthesisTaskRequest, (), _>(Method::POST, &path, body, None, Some(&headers), from_http_error)?;

        Ok(AwsLongFormOperation::new(
            response.synthesis_task,
            output_location,
        ))
    }
}
