use golem_stt::golem::stt::transcription::{Guest as TranscriptionGuest, AudioConfig, TranscribeOptions, TranscriptionResult, TranscribeRequest};
use golem_stt::golem::stt::types::SttError;

pub mod config;
pub mod error;
mod recognize;
mod batch;

pub struct AzureTranscriptionComponent;

impl TranscriptionGuest for AzureTranscriptionComponent {

    fn transcribe(
        audio: Vec<u8>,
        config: AudioConfig,
        options: Option<TranscribeOptions>,
    ) -> Result<TranscriptionResult, SttError> {
        let cfg = crate::config::AzureConfig::load()?;
        crate::batch::transcribe_impl(audio, &cfg, options, config)
    }

    fn multi_transcribe(requests: Vec<TranscribeRequest>) -> Result<Vec<TranscriptionResult>, SttError> {
        let mut results = Vec::with_capacity(requests.len());
        for req in requests {
            results.push(Self::transcribe(req.audio, req.config, req.options)?);
        }
        Ok(results)
    }
}



