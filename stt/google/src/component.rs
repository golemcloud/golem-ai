use crate::client::GoogleClient;
use crate::conversions::{to_wit_error, to_wit_result};
use golem_stt::config::GoogleConfig;
use golem_stt::durability::{make_request_key, BatchSnapshot, DurableStore};
use golem_stt::exports::golem::stt::languages::{Guest as LanguagesGuest, LanguageInfo};
use golem_stt::exports::golem::stt::transcription::{
    self, Guest as TranscriptionGuest, TranscribeOptions,
};
use golem_stt::exports::golem::stt::{types as wit_types, vocabularies};
use golem_stt::init_logging_from_env;
use log::info;

pub struct Component;

#[allow(static_mut_refs)]
static mut DURABLE: Option<DurableStore> = None;

#[allow(static_mut_refs)]
fn durable() -> &'static mut DurableStore {
    unsafe {
        #[allow(static_mut_refs)]
        if DURABLE.is_none() {
            DURABLE = Some(DurableStore::new());
        }
        DURABLE.as_mut().unwrap()
    }
}

fn build_client() -> Result<GoogleClient, wit_types::SttError> {
    let cfg = GoogleConfig::from_env();
    init_logging_from_env(cfg.common.log_level.clone());
    GoogleClient::new(cfg).map_err(|e| wit_types::SttError::InternalError(format!("{e:?}")))
}

impl LanguagesGuest for Component {
    fn list_languages() -> Result<Vec<LanguageInfo>, wit_types::SttError> {
        // Provide a standard set; Google supports many.
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
                code: "es-ES".into(),
                name: "Spanish (Spain)".into(),
                native_name: "Español (España)".into(),
            },
            LanguageInfo {
                code: "fr-FR".into(),
                name: "French".into(),
                native_name: "Français".into(),
            },
            LanguageInfo {
                code: "de-DE".into(),
                name: "German".into(),
                native_name: "Deutsch".into(),
            },
            LanguageInfo {
                code: "it-IT".into(),
                name: "Italian".into(),
                native_name: "Italiano".into(),
            },
            LanguageInfo {
                code: "pt-BR".into(),
                name: "Portuguese (Brazil)".into(),
                native_name: "Português (Brasil)".into(),
            },
            LanguageInfo {
                code: "ja-JP".into(),
                name: "Japanese".into(),
                native_name: "日本語".into(),
            },
            LanguageInfo {
                code: "ko-KR".into(),
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
    fn delete(&self) -> Result<(), wit_types::SttError> {
        let key = format!("stt:google:vocab:{}", self.name);
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
        let key = format!("stt:google:vocab:{name}");
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

        // Check for cached result using simple string-based caching
        if let Some(cached_text) = durable().get(&format!("google:result:{request_key}")) {
            if let Some(cached_lang) = durable().get(&format!("google:lang:{request_key}")) {
                info!("Returning cached result for request {request_key}");
                let alt = wit_types::TranscriptAlternative {
                    text: cached_text,
                    confidence: 1.0,
                    words: vec![],
                };
                let metadata = wit_types::TranscriptionMetadata {
                    duration_seconds: 0.0,
                    audio_size_bytes: audio.len() as u32,
                    request_id: "google-cached-response".to_string(),
                    model: durable().get(&format!("google:model:{request_key}")),
                    language: cached_lang,
                };
                return Ok(wit_types::TranscriptionResult {
                    alternatives: vec![alt],
                    metadata,
                });
            }
        }

        let audio_size = audio.len() as u32;

        let body = match wstd::runtime::block_on(client.transcribe(
            audio,
            &config,
            &options,
        )) {
            Ok(b) => b,
            Err(e) => return Err(to_wit_error(e)),
        };

        let language = options
            .as_ref()
            .and_then(|o| o.language.clone())
            .unwrap_or_else(|| "en-US".into());
        let model = options.as_ref().and_then(|o| o.model.clone());
        let _size = u32::try_from(audio_size as usize).unwrap_or(u32::MAX);

        let result = to_wit_result(&body, audio_size, language, model)?;

        let _request_id = result.metadata.request_id.clone();

        // Cache the result using simple string-based caching
        if let Some(first_alt) = result.alternatives.first() {
            durable().put(&format!("google:result:{request_key}"), &first_alt.text);
            durable().put(
                &format!("google:lang:{request_key}"),
                &result.metadata.language,
            );
            if let Some(ref model) = result.metadata.model {
                durable().put(&format!("google:model:{request_key}"), model);
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
            "Google Cloud Speech streaming requires gRPC connection not supported in WASI environment".to_string(),
        ))
    }
}
