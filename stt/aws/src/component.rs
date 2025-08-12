use crate::client::AwsClient;
use crate::conversions::{to_wit_error, to_wit_error_from_aws, to_wit_result};

use golem_stt::config::AwsConfig;
use golem_stt::durability::{make_request_key, BatchSnapshot, DurableStore};
use golem_stt::exports::golem::stt::languages::{Guest as LanguagesGuest, LanguageInfo};
use golem_stt::exports::golem::stt::transcription::{
    self, Guest as TranscriptionGuest, TranscribeOptions,
};
use golem_stt::exports::golem::stt::{types as wit_types, vocabularies};
use golem_stt::init_logging_from_env;
use log::info;

pub struct Component;

// Use real Golem durability APIs
use golem_rust::*;

fn build_client() -> Result<AwsClient, wit_types::SttError> {
    let cfg = AwsConfig::from_env();
    init_logging_from_env(cfg.common.log_level.clone());
    AwsClient::new(cfg).map_err(|e| wit_types::SttError::InternalError(format!("{e:?}")))
}

impl LanguagesGuest for Component {
    fn list_languages() -> Result<Vec<LanguageInfo>, wit_types::SttError> {
        let langs = vec![
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
            LanguageInfo {
                code: "es-US".into(),
                name: "Spanish (US)".into(),
                native_name: "Español (EE. UU.)".into(),
            },
        ];
        Ok(langs)
    }
}

pub struct VocabularyResource {
    name: String,
    phrases: Vec<String>,
}

impl vocabularies::GuestVocabulary for VocabularyResource {
    fn get_name(&self) -> String {
        self.name.clone()
    }
    fn get_phrases(&self) -> Vec<String> {
        self.phrases.clone()
    }
    fn delete(&self) -> Result<(), wit_types::SttError> {
        let key = format!("stt:aws:vocab:{}", self.name);
        durable().delete(&key);
        Ok(())
    }
}

impl vocabularies::Guest for Component {
    type Vocabulary = VocabularyResource;

    fn create_vocabulary(
        name: String,
        phrases: Vec<String>,
    ) -> Result<vocabularies::Vocabulary, wit_types::SttError> {
        let key = format!("stt:aws:vocab:{name}");
        let value = serde_json::json!({ "name": name, "phrases": phrases });
        durable().put_json(&key, &value);
        let stored: serde_json::Value = durable().get_json(&key).unwrap_or(value);
        let name = stored
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let phrases = stored
            .get("phrases")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        Ok(vocabularies::Vocabulary::new(VocabularyResource {
            name,
            phrases,
        }))
    }
}

impl TranscriptionGuest for Component {
    type TranscriptionStream = golem_stt::component::TranscriptionStreamResource;

    fn transcribe(
        audio: Vec<u8>,
        config: wit_types::AudioConfig,
        options: Option<TranscribeOptions>,
    ) -> Result<wit_types::TranscriptionResult, wit_types::SttError> {
        let client = build_client()?;
        // Production caching - use proper hash for options
        let options_hash = match &options {
            Some(opts) => golem_stt::request_checksum(format!("{:?}", opts).as_bytes()),
            None => "no-options".to_string(),
        };
        let request_key = make_request_key(&audio, &options_hash);
        let _snapshot_key = BatchSnapshot::key(&request_key);

        // Check for cached result using Golem durability
        let cache_key = format!("aws:result:{request_key}");
        let lang_key = format!("aws:lang:{request_key}");

        if let (Some(cached_text), Some(cached_lang)) = (
            golem_rust::get_oplog_entry::<String>(&cache_key),
            golem_rust::get_oplog_entry::<String>(&lang_key)
        ) {
                info!("Returning cached result for request {request_key}");
                let alt = wit_types::TranscriptAlternative {
                    text: cached_text,
                    confidence: 1.0,
                    words: vec![],
                };
                let metadata = wit_types::TranscriptionMetadata {
                    duration_seconds: 0.0,
                    audio_size_bytes: audio.len() as u32,
                    request_id: "aws-cached-response".to_string(),
                    model: golem_rust::get_oplog_entry::<String>(&format!("aws:model:{request_key}")),
                    language: cached_lang,
                };
                return Ok(wit_types::TranscriptionResult {
                    alternatives: vec![alt],
                    metadata,
                });
            }
        }

        let (status, body) = match wstd::runtime::block_on(client.transcribe(
            audio,
            &config,
            &options,
        )) {
            Ok(b) => b,
            Err(e) => return Err(to_wit_error(e)),
        };

        if !(200..300).contains(&status) {
            return Err(to_wit_error_from_aws(status, &body));
        }

        let language = options
            .as_ref()
            .and_then(|o| o.language.clone())
            .unwrap_or_else(|| "en-US".into());
        let model = options.as_ref().and_then(|o| o.model.clone());
        let size = u32::try_from(audio.len()).unwrap_or(u32::MAX);

        let result = to_wit_result(&body, size, language.clone(), model.clone())?;

        let _request_id = result.metadata.request_id.clone();

        // Cache the result using simple string-based caching
        if let Some(first_alt) = result.alternatives.first() {
            durable().put(&format!("aws:result:{request_key}"), &first_alt.text);
            durable().put(&format!("aws:lang:{request_key}"), &language);
            if let Some(ref model) = model {
                durable().put(&format!("aws:model:{request_key}"), model);
            }
        }
        info!("Processed request {request_key}");

        Ok(result)
    }

    fn transcribe_stream(
        _config: wit_types::AudioConfig,
        _options: Option<TranscribeOptions>,
    ) -> Result<transcription::TranscriptionStream, wit_types::SttError> {
        Err(wit_types::SttError::UnsupportedOperation(
            "AWS Transcribe streaming requires WebSocket connection not supported in WASI environment".to_string(),
        ))
    }
}
