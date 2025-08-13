use crate::config::AmazonConfig;
use crate::transcribe::{transcribe_once};
use golem_stt::golem::stt::transcription::{AudioConfig, TranscribeOptions, TranscriptionResult};
use golem_stt::golem::stt::types::SttError;

pub fn transcribe_impl(audio: Vec<u8>, cfg: &AmazonConfig, options: Option<TranscribeOptions>, config: AudioConfig) -> Result<TranscriptionResult, SttError> {
    transcribe_once(audio, cfg, options, config)
}

