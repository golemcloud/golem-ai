use crate::client::WhisperClient;
use crate::conversions::{to_wit_error, to_wit_error_from_whisper, to_wit_result};
use golem_stt::config::WhisperConfig;
use golem_stt::durability::make_request_key;
use golem_stt::exports::golem::stt::languages::{Guest as LanguagesGuest, LanguageInfo};
use golem_stt::exports::golem::stt::transcription::{
    self, Guest as TranscriptionGuest, TranscribeOptions,
};
use golem_stt::exports::golem::stt::{types as wit_types, vocabularies};
use golem_stt::init_logging_from_env;
use log::info;

pub struct Component;

// Durability handled by DurableStt wrapper

fn build_client() -> Result<WhisperClient, wit_types::SttError> {
    let cfg = WhisperConfig::from_env();
    init_logging_from_env(cfg.common.log_level.clone());
    WhisperClient::new(cfg).map_err(|e| wit_types::SttError::InternalError(format!("{e:?}")))
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
        let whisper_config = crate::config::WhisperConfig::from_env();
        whisper_config.common.validate_audio_size(&audio)?;

        // Direct API call - durability handled by DurableStt wrapper
        info!("Processing request {request_key}");

        // Handle large files properly - don't silently truncate
        let audio_size = u32::try_from(audio.len()).map_err(|_| {
            wit_types::SttError::InvalidAudio(format!(
                "Audio file too large: {} bytes exceeds maximum supported size",
                audio.len()
            ))
        })?;

        let (status, body) =
            match wstd::runtime::block_on(client.transcribe(audio, &config, &options)) {
                Ok(b) => b,
                Err(e) => return Err(to_wit_error(e)),
            };

        if !(200..300).contains(&status) {
            return Err(to_wit_error_from_whisper(status, &body));
        }

        let language = options
            .as_ref()
            .and_then(|o| o.language.clone())
            .unwrap_or_else(|| "en-US".into());
        let model = options.as_ref().and_then(|o| o.model.clone());

        let result = to_wit_result(&body, &language, model.as_deref())?;

        // No caching needed - durability handled by DurableStt wrapper
        info!("Processed request {request_key}");

        Ok(result)
    }

    fn transcribe_stream(
        _config: wit_types::AudioConfig,
        _options: Option<TranscribeOptions>,
    ) -> Result<transcription::TranscriptionStream, wit_types::SttError> {
        // Whisper doesn't support streaming, return error immediately
        Err(wit_types::SttError::UnsupportedOperation(
            "Streaming transcription is not supported by Whisper. Use batch transcription instead."
                .to_string(),
        ))
    }
}
