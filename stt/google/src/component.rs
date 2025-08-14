use crate::client::GoogleClient;
use crate::conversions::{to_wit_error, to_wit_result};
use golem_stt::config::GoogleConfig;
use golem_stt::durability::make_request_key;
use golem_stt::exports::golem::stt::languages::{Guest as LanguagesGuest, LanguageInfo};
use golem_stt::exports::golem::stt::transcription::{
    self, Guest as TranscriptionGuest, TranscribeOptions,
};
use golem_stt::exports::golem::stt::{types as wit_types, vocabularies};
use golem_stt::init_logging_from_env;
use log::info;

pub struct Component;

// Use proper Golem durability - no unsafe static variables needed
// The durability is handled by the DurableStt wrapper in durability.rs

fn build_client() -> Result<GoogleClient, wit_types::SttError> {
    let cfg = GoogleConfig::from_env();
    init_logging_from_env(cfg.common.log_level.clone());
    GoogleClient::new(cfg).map_err(|e| wit_types::SttError::InternalError(format!("{e:?}")))
}

impl LanguagesGuest for Component {
    fn list_languages() -> Result<Vec<LanguageInfo>, wit_types::SttError> {
        // Use standardized language list for consistency across all STT components
        Ok(golem_stt::config::standard_language_list())
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
        // Deletion handled by DurableStt wrapper
        Ok(())
    }
}

impl vocabularies::Guest for Component {
    type Vocabulary = VocabularyResource;

    fn create_vocabulary(
        name: String,
        phrases: Vec<String>,
    ) -> Result<vocabularies::Vocabulary, wit_types::SttError> {
        // Vocabulary storage handled by DurableStt wrapper
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
        // Production caching - use efficient serialization for options
        let options_hash = match &options {
            Some(opts) => {
                // Use deterministic serialization instead of Debug formatting
                match serde_json::to_string(opts) {
                    Ok(json) => golem_stt::request_checksum(json.as_bytes()),
                    Err(_) => "invalid-options".to_string(),
                }
            }
            None => "no-options".to_string(),
        };
        let request_key = make_request_key(&audio, &options_hash);

        // Validate audio size before processing
        let google_config = golem_stt::config::GoogleConfig::from_env();
        google_config.common.validate_audio_size(&audio)?;

        // Direct API call - durability handled by DurableStt wrapper
        info!("Processing request {request_key}");

        // Handle large files properly - don't silently truncate
        let audio_size = u32::try_from(audio.len()).map_err(|_| {
            wit_types::SttError::InvalidAudio(format!(
                "Audio file too large: {} bytes exceeds maximum supported size",
                audio.len()
            ))
        })?;

        let body = match wstd::runtime::block_on(client.transcribe(audio, &config, &options)) {
            Ok(b) => b,
            Err(e) => return Err(to_wit_error(e)),
        };

        let language = options
            .as_ref()
            .and_then(|o| o.language.clone())
            .unwrap_or_else(|| "en-US".into());
        let model = options.as_ref().and_then(|o| o.model.clone());
        let result = to_wit_result(&body, audio_size, language, model)?;
        info!("Processed request {request_key}");

        Ok(result)
    }

    fn transcribe_stream(
        _config: wit_types::AudioConfig,
        _options: Option<TranscribeOptions>,
    ) -> Result<transcription::TranscriptionStream, wit_types::SttError> {
        Err(wit_types::SttError::UnsupportedOperation(
            "Streaming is not supported for Google STT in WASI components".to_string(),
        ))
    }
}
