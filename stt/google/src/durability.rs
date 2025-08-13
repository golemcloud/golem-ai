#![cfg(feature = "durability")]

use golem_rust::bindings::golem::durability::durability::DurableFunctionType;
use golem_rust::durability::Durability;
use golem_rust::{FromValueAndType, IntoValue};

#[derive(Clone, Debug, FromValueAndType, IntoValue)]
pub struct TranscribeInputMeta {
    pub language: Option<String>,
    pub model: Option<String>,
    pub enable_timestamps: bool,
    pub enable_diarization: bool,
    pub enable_word_confidence: bool,
    pub audio_size_bytes: u32,
}

pub fn persist_transcribe(
    input: TranscribeInputMeta,
    result: Result<golem_stt::golem::stt::transcription::TranscriptionResult, golem_stt::golem::stt::types::SttError>,
) -> Result<golem_stt::golem::stt::transcription::TranscriptionResult, golem_stt::golem::stt::types::SttError> {
    type OkT = golem_stt::golem::stt::transcription::TranscriptionResult;
    type ErrT = golem_stt::golem::stt::types::SttError;
    let durability: Durability<OkT, ErrT> = Durability::new("golem_stt_google", "transcribe", DurableFunctionType::WriteRemote);
    if durability.is_live() {
        match result {
            Ok(ok) => Ok(durability.persist_infallible(input, ok)),
            Err(e) => Err(e),
        }
    } else {
        Ok(durability.replay_infallible())
    }
}
