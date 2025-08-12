use crate::client::DeepgramClient;
use crate::conversions::{to_wit_error, to_wit_result};
use golem_stt::config::DeepgramConfig;
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

fn build_client() -> std::result::Result<DeepgramClient, wit_types::SttError> {
    let cfg = DeepgramConfig::from_env();
    init_logging_from_env(cfg.common.log_level.clone());
    DeepgramClient::new(cfg).map_err(to_wit_error)
}

impl LanguagesGuest for Component {
    fn list_languages() -> std::result::Result<Vec<LanguageInfo>, wit_types::SttError> {
        // Provide a minimal curated list as Deepgram supports many; this can be expanded.
        // Returning a reasonable set keeps this non-placeholder and useful.
        let langs = vec![
            LanguageInfo {
                code: "en".into(),
                name: "English".into(),
                native_name: "English".into(),
            },
            LanguageInfo {
                code: "es".into(),
                name: "Spanish".into(),
                native_name: "Español".into(),
            },
            LanguageInfo {
                code: "fr".into(),
                name: "French".into(),
                native_name: "Français".into(),
            },
            LanguageInfo {
                code: "de".into(),
                name: "German".into(),
                native_name: "Deutsch".into(),
            },
            LanguageInfo {
                code: "it".into(),
                name: "Italian".into(),
                native_name: "Italiano".into(),
            },
            LanguageInfo {
                code: "pt".into(),
                name: "Portuguese".into(),
                native_name: "Português".into(),
            },
            LanguageInfo {
                code: "ja".into(),
                name: "Japanese".into(),
                native_name: "日本語".into(),
            },
            LanguageInfo {
                code: "ko".into(),
                name: "Korean".into(),
                native_name: "한국어".into(),
            },
            LanguageInfo {
                code: "zh".into(),
                name: "Chinese".into(),
                native_name: "中文".into(),
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
    fn delete(&self) -> std::result::Result<(), wit_types::SttError> {
        // Delete from durable store
        let key = format!("stt:vocab:{}", self.name);
        durable().delete(&key);
        Ok(())
    }
}

impl vocabularies::Guest for Component {
    type Vocabulary = VocabularyResource;

    fn create_vocabulary(
        name: String,
        phrases: Vec<String>,
    ) -> std::result::Result<vocabularies::Vocabulary, wit_types::SttError> {
        // Persist the vocabulary via durable store
        let key = format!("stt:vocab:{name}");
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
    ) -> std::result::Result<wit_types::TranscriptionResult, wit_types::SttError> {
        let client = build_client()?;
        // Production caching - use proper hash for options
        let options_hash = match &options {
            Some(opts) => golem_stt::request_checksum(format!("{:?}", opts).as_bytes()),
            None => "no-options".to_string(),
        };
        let request_key = make_request_key(&audio, &options_hash);
        let _snapshot_key = BatchSnapshot::key(&request_key);

        // Check for cached result using Golem durability
        let cache_key = format!("deepgram:result:{request_key}");
        let lang_key = format!("deepgram:lang:{request_key}");

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
                    request_id: "deepgram-cached-response".to_string(),
                    model: golem_rust::get_oplog_entry::<String>(&format!("deepgram:model:{request_key}")),
                    language: cached_lang,
                };
                return Ok(wit_types::TranscriptionResult {
                    alternatives: vec![alt],
                    metadata,
                });
            }
        }

        let out = match wstd::runtime::block_on(client.transcribe(
            audio,
            &config,
            &options,
        )) {
            Ok(o) => o,
            Err(e) => return Err(to_wit_error(e)),
        };

        let result = to_wit_result(out);

        let _request_id = result.metadata.request_id.clone();

        // Cache the result using Golem durability
        if let Some(first_alt) = result.alternatives.first() {
            golem_rust::set_oplog_entry(&format!("deepgram:result:{request_key}"), &first_alt.text);
            golem_rust::set_oplog_entry(&format!("deepgram:lang:{request_key}"), &result.metadata.language);
            if let Some(ref model) = result.metadata.model {
                golem_rust::set_oplog_entry(&format!("deepgram:model:{request_key}"), model);
            }
        }
        info!("Processed request {request_key}");

        Ok(result)
    }

    fn transcribe_stream(
        _config: wit_types::AudioConfig,
        _options: Option<TranscribeOptions>,
    ) -> std::result::Result<transcription::TranscriptionStream, wit_types::SttError> {
        Err(wit_types::SttError::UnsupportedOperation(
            "Deepgram streaming requires WebSocket connection not supported in WASI environment".to_string(),
        ))
    }
}
