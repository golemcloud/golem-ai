use crate::client::AwsTranscribeClient;
use crate::conversions::{get_supported_languages, generate_job_name};
use golem_stt::durability::{DurableSTT, ExtendedTranscriptionGuest, ExtendedVocabulariesGuest, ExtendedLanguagesGuest, ExtendedGuest};
use golem_stt::golem::stt::languages::{Guest as LanguagesGuest, LanguageInfo};
use golem_stt::golem::stt::transcription::{
    Guest as TranscriptionGuest, TranscribeOptions, TranscriptionStream,
};
use golem_stt::golem::stt::types::{AudioConfig, SttError, TranscriptionResult, TranscriptAlternative, TranscriptionMetadata};
use golem_stt::golem::stt::vocabularies::{Guest as VocabulariesGuest, Vocabulary};
use log::{error, trace, warn};
use std::cell::RefCell;
use std::collections::HashMap;

mod client;
mod conversions;

struct AwsSTTComponent;

impl AwsSTTComponent {
    const ACCESS_KEY_ENV_VAR: &'static str = "AWS_ACCESS_KEY_ID";
    const SECRET_KEY_ENV_VAR: &'static str = "AWS_SECRET_ACCESS_KEY";
    const REGION_ENV_VAR: &'static str = "AWS_REGION";

    fn get_client() -> Result<AwsTranscribeClient, SttError> {
        let access_key = std::env::var(Self::ACCESS_KEY_ENV_VAR)
            .map_err(|_| SttError::Unauthorized("AWS_ACCESS_KEY_ID not set".to_string()))?;
        
        let secret_key = std::env::var(Self::SECRET_KEY_ENV_VAR)
            .map_err(|_| SttError::Unauthorized("AWS_SECRET_ACCESS_KEY not set".to_string()))?;
            
        let region = std::env::var(Self::REGION_ENV_VAR)
            .unwrap_or_else(|_| "us-east-1".to_string());
        
        Ok(AwsTranscribeClient::new(access_key, secret_key, region))
    }
}

thread_local! {
    static VOCABULARIES: RefCell<HashMap<String, Vec<String>>> = RefCell::new(HashMap::new());
}

pub struct AwsTranscriptionStream;

impl golem_stt::golem::stt::transcription::GuestTranscriptionStream for AwsTranscriptionStream {
    fn send_audio(&self, _chunk: Vec<u8>) -> Result<(), SttError> {
        Err(SttError::UnsupportedOperation("AWS Transcribe does not support real-time streaming".to_string()))
    }

    fn finish(&self) -> Result<(), SttError> {
        Err(SttError::UnsupportedOperation("AWS Transcribe does not support real-time streaming".to_string()))
    }

    fn receive_alternative(&self) -> Result<Option<TranscriptAlternative>, SttError> {
        Err(SttError::UnsupportedOperation("AWS Transcribe does not support real-time streaming".to_string()))
    }
    
    fn close(&self) {
        // No-op for now
    }
}

pub struct AwsVocabulary {
    name: String,
}

impl golem_stt::golem::stt::vocabularies::GuestVocabulary for AwsVocabulary {
    fn get_name(&self) -> String {
        self.name.clone()
    }

    fn get_phrases(&self) -> Vec<String> {
        VOCABULARIES.with(|v| {
            v.borrow()
                .get(&self.name)
                .cloned()
                .unwrap_or_default()
        })
    }

    fn delete(&self) -> Result<(), SttError> {
        VOCABULARIES.with(|v| {
            v.borrow_mut().remove(&self.name);
        });
        Ok(())
    }
}

impl TranscriptionGuest for AwsSTTComponent {
    type TranscriptionStream = AwsTranscriptionStream;

    fn transcribe(
        audio: Vec<u8>,
        _config: AudioConfig,
        options: Option<TranscribeOptions>,
    ) -> Result<TranscriptionResult, SttError> {
        golem_stt::init_logging();
        trace!("Starting AWS Transcribe transcription, audio size: {} bytes", audio.len());

        let client = Self::get_client()?;
        
        let default_language = "en-US".to_string();
        let language = options
            .as_ref()
            .and_then(|opts| opts.language.as_ref())
            .unwrap_or(&default_language);

        trace!("Using AWS Transcribe with S3 batch processing");
        
        // Use the real AWS Transcribe implementation
        let transcription_result = client.transcribe_audio_simple(&audio, language)
            .map_err(|e| {
                error!("AWS Transcribe API failed: {:?}", e);
                e
            })?;

        // Create result from AWS response
        let alternatives = vec![TranscriptAlternative {
            text: transcription_result.transcript,
            confidence: transcription_result.confidence,
            words: vec![], // Simplified implementation without word timing
        }];

        Ok(TranscriptionResult {
            alternatives,
            metadata: TranscriptionMetadata {
                duration_seconds: transcription_result.duration,
                audio_size_bytes: audio.len() as u32,
                request_id: generate_job_name(),
                model: Some("AWS Transcribe".to_string()),
                language: language.clone(),
            },
        })
    }

    fn transcribe_stream(
        _audio_config: AudioConfig,
        _options: Option<TranscribeOptions>,
    ) -> Result<TranscriptionStream, SttError> {
        // AWS Transcribe doesn't support real-time streaming like other providers
        Err(SttError::UnsupportedOperation("AWS Transcribe does not support real-time streaming".to_string()))
    }
}

impl LanguagesGuest for AwsSTTComponent {
    fn list_languages() -> Result<Vec<LanguageInfo>, SttError> {
        Ok(get_supported_languages())
    }
}

impl VocabulariesGuest for AwsSTTComponent {
    type Vocabulary = AwsVocabulary;

    fn create_vocabulary(
        name: String,
        phrases: Vec<String>,
    ) -> Result<Vocabulary, SttError> {
        VOCABULARIES.with(|v| {
            v.borrow_mut().insert(name.clone(), phrases);
        });
        
        Ok(Vocabulary::new(AwsVocabulary { name }))
    }
}

impl ExtendedTranscriptionGuest for AwsSTTComponent {}
impl ExtendedVocabulariesGuest for AwsSTTComponent {}
impl ExtendedLanguagesGuest for AwsSTTComponent {}
impl ExtendedGuest for AwsSTTComponent {}

type DurableAwsSTTComponent = DurableSTT<AwsSTTComponent>;

golem_stt::export_stt!(DurableAwsSTTComponent with_types_in golem_stt);