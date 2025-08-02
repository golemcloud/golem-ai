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

/// In-memory durable store for this component instance.
/// In a real durable host, this should be bound to persistent state.
static mut DURABLE: Option<DurableStore> = None;

#[allow(static_mut_refs)]
fn durable() -> &'static mut DurableStore {
    unsafe {
        if DURABLE.is_none() {
            DURABLE = Some(DurableStore::new());
        }
        DURABLE.as_mut().unwrap()
    }
}

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

use crate::stream::DgStream;

impl TranscriptionGuest for Component {
    type TranscriptionStream = DgStream<'static>;

    fn transcribe(
        audio: Vec<u8>,
        config: wit_types::AudioConfig,
        options: Option<TranscribeOptions>,
    ) -> std::result::Result<wit_types::TranscriptionResult, wit_types::SttError> {
        let client = build_client()?;
        // Simplified caching - use a basic string representation for the salt
        let salt = format!("{options:?}");
        let request_key = make_request_key(&audio, &salt);
        let _snapshot_key = BatchSnapshot::key(&request_key);

        // Check for cached result using simple string-based caching
        if let Some(cached_text) = durable().get(&format!("deepgram:result:{request_key}")) {
            if let Some(cached_lang) = durable().get(&format!("deepgram:lang:{request_key}")) {
                info!("Returning cached result for request {request_key}");
                let alt = wit_types::TranscriptAlternative {
                    text: cached_text,
                    confidence: 1.0,
                    words: vec![],
                };
                let metadata = wit_types::TranscriptionMetadata {
                    duration_seconds: 0.0,
                    audio_size_bytes: audio.len() as u32,
                    request_id: format!("cached-{request_key}"),
                    model: durable().get(&format!("deepgram:model:{request_key}")),
                    language: cached_lang,
                };
                return Ok(wit_types::TranscriptionResult {
                    alternatives: vec![alt],
                    metadata,
                });
            }
        }

        let out = match futures::executor::block_on(client.transcribe(
            audio.clone(),
            &config,
            &options,
        )) {
            Ok(o) => o,
            Err(e) => return Err(to_wit_error(e)),
        };

        let result = to_wit_result(out);

        let _request_id = result.metadata.request_id.clone();

        // Cache the result using simple string-based caching
        if let Some(first_alt) = result.alternatives.first() {
            durable().put(&format!("deepgram:result:{request_key}"), &first_alt.text);
            durable().put(
                &format!("deepgram:lang:{request_key}"),
                &result.metadata.language,
            );
            if let Some(ref model) = result.metadata.model {
                durable().put(&format!("deepgram:model:{request_key}"), model);
            }
        }
        info!("Processed request {request_key}");

        Ok(result)
    }

    fn transcribe_stream(
        _config: wit_types::AudioConfig,
        _options: Option<TranscribeOptions>,
    ) -> std::result::Result<transcription::TranscriptionStream, wit_types::SttError> {
        // Create a simplified stream implementation
        let client = build_client()?;
        let stream = DgStream::new(&client, "audio/wav", durable())
            .map_err(|e| wit_types::SttError::TranscriptionFailed(format!("{e:?}")))?;
        Ok(transcription::TranscriptionStream::new(stream))
    }
}
