//! Advanced interface implementation for AWS Polly

use crate::types;
use crate::voices::VoiceImpl;
use crate::wit_advanced;
use crate::wit_types;
use crate::wit_voices;

pub struct PronunciationLexiconImpl;
impl wit_advanced::GuestPronunciationLexicon for PronunciationLexiconImpl {
    fn get_name(&self) -> String {
        String::new()
    }
    fn get_language(&self) -> String {
        String::new()
    }
    fn get_entry_count(&self) -> u32 {
        0
    }
    fn add_entry(&self, _w: String, _p: String) -> Result<(), wit_types::TtsError> {
        Ok(())
    }
    fn remove_entry(&self, _w: String) -> Result<(), wit_types::TtsError> {
        Ok(())
    }
    fn export_content(&self) -> Result<String, wit_types::TtsError> {
        Ok(String::new())
    }
}

pub struct LongFormOperationImpl;
impl wit_advanced::GuestLongFormOperation for LongFormOperationImpl {
    fn get_status(&self) -> wit_advanced::OperationStatus {
        wit_advanced::OperationStatus::Pending
    }
    fn get_progress(&self) -> f32 {
        0.0
    }
    fn cancel(&self) -> Result<(), wit_types::TtsError> {
        Ok(())
    }
    fn get_result(&self) -> Result<wit_advanced::LongFormResult, wit_types::TtsError> {
        Err(types::internal_error("Not implemented"))
    }
}

pub fn create_voice_clone(
    _name: String,
    _samples: Vec<wit_advanced::AudioSample>,
    _desc: Option<String>,
) -> Result<wit_voices::Voice, wit_types::TtsError> {
    return Err(types::internal_error(
        "Polly does not support voice cloning",
    ));
}

pub fn design_voice(
    _name: String,
    _params: wit_advanced::VoiceDesignParams,
) -> Result<wit_voices::Voice, wit_types::TtsError> {
    return Err(types::internal_error("Polly does not support voice design"));
}

pub fn convert_voice(
    _audio: Vec<u8>,
    _voice: &VoiceImpl,
    _preserve: Option<bool>,
) -> Result<Vec<u8>, wit_types::TtsError> {
    return Err(types::internal_error(
        "Polly does not support voice conversion",
    ));
}

pub fn generate_sound_effect(
    _desc: String,
    _dur: Option<f32>,
    _infl: Option<f32>,
) -> Result<Vec<u8>, wit_types::TtsError> {
    return Err(types::internal_error(
        "Polly does not support sound effects",
    ));
}

pub fn create_lexicon(
    _name: String,
    _lang: String,
    _entries: Option<Vec<wit_advanced::PronunciationEntry>>,
) -> Result<wit_advanced::PronunciationLexicon, wit_types::TtsError> {
    return Err(types::internal_error(
        "Polly lexicon management via WIT not implemented",
    ));
}

pub fn synthesize_long_form(
    _content: String,
    _voice: &VoiceImpl,
    _loc: String,
    _breaks: Option<Vec<u32>>,
) -> Result<wit_advanced::LongFormOperation, wit_types::TtsError> {
    return Err(types::internal_error(
        "Polly long-form synthesis not implemented",
    ));
}
