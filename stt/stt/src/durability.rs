use crate::exports::golem::stt::languages::Guest as LanguagesGuest;
use crate::exports::golem::stt::transcription::Guest as TranscriptionGuest;
use crate::exports::golem::stt::vocabularies::Guest as VocabulariesGuest;
use crate::request_checksum;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

/// Minimal durability facade to mirror existing Golem patterns without importing exec crate internals.
/// In real deployment, integrate with golem-rust durable state APIs if available in the host.
/// Here we provide an in-memory fallback to allow CI to run without a host.
/// The provider components should treat this as a best-effort cache aiding idempotency/resume.
#[derive(Default)]
pub struct DurableStore {
    // Simple in-memory map keyed by operation id.
    // In production the host will persist this; within unit tests this is fine.
    inner: std::collections::HashMap<String, Vec<u8>>,
}

impl DurableStore {
    pub fn new() -> Self {
        Self {
            inner: std::collections::HashMap::new(),
        }
    }

    pub fn put_json<T: Serialize>(&mut self, key: &str, value: &T) {
        if let Ok(bytes) = serde_json::to_vec(value) {
            self.inner.insert(key.to_string(), bytes);
        }
    }

    pub fn get_json<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Option<T> {
        self.inner
            .get(key)
            .and_then(|v| serde_json::from_slice(v).ok())
    }

    pub fn put(&mut self, key: &str, value: &str) {
        self.inner
            .insert(key.to_string(), value.as_bytes().to_vec());
    }

    pub fn get(&self, key: &str) -> Option<String> {
        self.inner
            .get(key)
            .and_then(|v| String::from_utf8(v.clone()).ok())
    }

    pub fn delete(&mut self, key: &str) {
        self.inner.remove(key);
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.inner.contains_key(key)
    }
}

/// Batch operation durable snapshot allowing resume/idempotency.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchSnapshot {
    pub request_id: String,          // Provider request id or synthetic id
    pub input_checksum: String,      // Checksum of (audio bytes + options)
    pub last_result: Option<String>, // Serialized transcription-result JSON
}

impl BatchSnapshot {
    pub fn key(request_key: &str) -> String {
        format!("stt:batch:{request_key}")
    }
}

/// Streaming operation durable snapshot. Track send/receive state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamSnapshot {
    pub stream_id: String,         // Provider stream id or synthetic id
    pub sent_bytes: u64,           // Total bytes sent
    pub finished: bool,            // Input finished flag
    pub last_alt_index: u64,       // Last delivered alternative index
    pub pending_alts: Vec<String>, // Serialized transcript-alternative JSONs
}

impl StreamSnapshot {
    pub fn key(stream_key: &str) -> String {
        format!("stt:stream:{stream_key}")
    }
}

/// Build a stable request key from audio and a salt (e.g., options hash).
pub fn make_request_key(audio: &[u8], salt: &str) -> String {
    let mut data = Vec::with_capacity(audio.len() + salt.len());
    data.extend_from_slice(audio);
    data.extend_from_slice(salt.as_bytes());
    request_checksum(&data)
}

/// Wraps an STT implementation with custom durability
pub struct DurableStt<Impl> {
    phantom: PhantomData<Impl>,
}

/// Trait to be implemented in addition to the STT `Guest` traits when wrapping it with `DurableStt`.
pub trait ExtendedGuest: LanguagesGuest + TranscriptionGuest + VocabulariesGuest + 'static {}

/// When the durability feature flag is off, wrapping with `DurableStt` is just a passthrough
#[cfg(not(feature = "durability"))]
mod passthrough_impl {
    use crate::durability::{DurableStt, ExtendedGuest};
    use crate::exports::golem::stt::languages::{Guest as LanguagesGuest, LanguageInfo};
    use crate::exports::golem::stt::transcription::{
        self, Guest as TranscriptionGuest, TranscribeOptions,
    };
    use crate::exports::golem::stt::types::{SttError, TranscriptionResult};
    use crate::exports::golem::stt::vocabularies::Guest as VocabulariesGuest;

    impl<Impl: ExtendedGuest> LanguagesGuest for DurableStt<Impl> {
        fn list_languages() -> Result<Vec<LanguageInfo>, SttError> {
            Impl::list_languages()
        }
    }

    impl<Impl: ExtendedGuest> TranscriptionGuest for DurableStt<Impl> {
        type TranscriptionStream = Impl::TranscriptionStream;

        fn transcribe(
            audio: Vec<u8>,
            config: crate::exports::golem::stt::types::AudioConfig,
            options: Option<TranscribeOptions>,
        ) -> Result<TranscriptionResult, SttError> {
            Impl::transcribe(audio, config, options)
        }

        fn transcribe_stream(
            config: crate::exports::golem::stt::types::AudioConfig,
            options: Option<TranscribeOptions>,
        ) -> Result<transcription::TranscriptionStream, SttError> {
            Impl::transcribe_stream(config, options)
        }
    }

    impl<Impl: ExtendedGuest> VocabulariesGuest for DurableStt<Impl> {
        type Vocabulary = Impl::Vocabulary;

        fn create_vocabulary(
            name: String,
            phrases: Vec<String>,
        ) -> Result<crate::exports::golem::stt::vocabularies::Vocabulary, SttError> {
            Impl::create_vocabulary(name, phrases)
        }
    }
}

/// When the durability feature flag is on, wrap provider impls with golem-rust Durability
#[cfg(feature = "durability")]
mod durable_impl {
    use crate::durability::{DurableStt, ExtendedGuest};
    use crate::exports::golem::stt::languages::{Guest as LanguagesGuest, LanguageInfo};
    use crate::exports::golem::stt::transcription::{
        self, Guest as TranscriptionGuest, TranscribeOptions,
    };
    use crate::exports::golem::stt::types::{AudioConfig, SttError, TranscriptionResult};
    use crate::exports::golem::stt::vocabularies::{self, Guest as VocabulariesGuest};
    use crate::init_logging_from_env;
    use golem_rust::bindings::golem::durability::durability::DurableFunctionType;
    use golem_rust::durability::Durability;
    use golem_rust::{with_persistence_level, PersistenceLevel};

    #[derive(Debug, Clone)]
    struct ListLanguagesInput;

    #[derive(Debug, Clone)]
    struct TranscribeInput {
        audio: Vec<u8>,
        config: AudioConfig,
        options: Option<TranscribeOptions>,
    }

    #[derive(Debug, Clone)]
    struct CreateVocabularyInput {
        name: String,
        phrases: Vec<String>,
    }

    impl<Impl: ExtendedGuest> LanguagesGuest for DurableStt<Impl> {
        fn list_languages() -> Result<Vec<LanguageInfo>, SttError> {
            init_logging_from_env(None);
            let durability = Durability::<Vec<LanguageInfo>, SttError>::new(
                "golem_stt",
                "list_languages",
                DurableFunctionType::ReadRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::list_languages()
                });
                durability.persist(ListLanguagesInput, result)
            } else {
                durability.replay()
            }
        }
    }

    impl<Impl: ExtendedGuest> TranscriptionGuest for DurableStt<Impl> {
        type TranscriptionStream = Impl::TranscriptionStream;

        fn transcribe(
            audio: Vec<u8>,
            config: AudioConfig,
            options: Option<TranscribeOptions>,
        ) -> Result<TranscriptionResult, SttError> {
            init_logging_from_env(None);
            let durability = Durability::<TranscriptionResult, SttError>::new(
                "golem_stt",
                "transcribe",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let input = TranscribeInput {
                    audio: audio.clone(),
                    config: config.clone(),
                    options: options.clone(),
                };
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::transcribe(audio, config, options)
                });
                durability.persist(input, result)
            } else {
                durability.replay()
            }
        }

        fn transcribe_stream(
            config: AudioConfig,
            options: Option<TranscribeOptions>,
        ) -> Result<transcription::TranscriptionStream, SttError> {
            // Streaming is best-effort pass-through.
            Impl::transcribe_stream(config, options)
        }
    }

    impl<Impl: ExtendedGuest> VocabulariesGuest for DurableStt<Impl> {
        type Vocabulary = Impl::Vocabulary;

        fn create_vocabulary(
            name: String,
            phrases: Vec<String>,
        ) -> Result<vocabularies::Vocabulary, SttError> {
            init_logging_from_env(None);
            let durability = Durability::<vocabularies::Vocabulary, SttError>::new(
                "golem_stt",
                "create_vocabulary",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let input = CreateVocabularyInput {
                    name: name.clone(),
                    phrases: phrases.clone(),
                };
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::create_vocabulary(name, phrases)
                });
                durability.persist(input, result)
            } else {
                durability.replay()
            }
        }
    }
}
