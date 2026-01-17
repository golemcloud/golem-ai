//! Advanced TTS features for ElevenLabs
//!
//! Voice cloning, sound effects, and long-form content synthesis.

use crate::client;
use crate::types;
use crate::voices::VoiceImpl;
use crate::wit_advanced;
use crate::wit_types;
use crate::wit_voices;
use std::cell::RefCell;

// ============================================================
// PRONUNCIATION LEXICON IMPLEMENTATION
// ============================================================

struct PronunciationLexiconState {
    name: String,
    language: String,
    entries: Vec<(String, String)>,
}

/// Pronunciation lexicon resource with interior mutability
pub struct PronunciationLexiconImpl {
    state: RefCell<PronunciationLexiconState>,
}

impl PronunciationLexiconImpl {
    pub fn new(name: String, language: String) -> Self {
        return PronunciationLexiconImpl {
            state: RefCell::new(PronunciationLexiconState {
                name,
                language,
                entries: Vec::new(),
            }),
        };
    }
}

impl wit_advanced::GuestPronunciationLexicon for PronunciationLexiconImpl {
    fn get_name(&self) -> String {
        let state = self.state.borrow();
        return state.name.clone();
    }

    fn get_language(&self) -> String {
        let state = self.state.borrow();
        return state.language.clone();
    }

    fn get_entry_count(&self) -> u32 {
        let state = self.state.borrow();
        return state.entries.len() as u32;
    }

    fn add_entry(&self, word: String, pronunciation: String) -> Result<(), wit_types::TtsError> {
        let mut state = self.state.borrow_mut();
        state.entries.push((word, pronunciation));
        return Ok(());
    }

    fn remove_entry(&self, word: String) -> Result<(), wit_types::TtsError> {
        let mut state = self.state.borrow_mut();
        let initial_len = state.entries.len();
        state.entries.retain(|(w, _)| w != &word);

        if state.entries.len() == initial_len {
            return Err(types::internal_error("Entry not found"));
        }

        return Ok(());
    }

    fn export_content(&self) -> Result<String, wit_types::TtsError> {
        let state = self.state.borrow();
        let mut content = String::new();
        content.push_str(&format!("# Lexicon: {}\n", state.name));
        content.push_str(&format!("# Language: {}\n", state.language));
        content.push_str("# Format: word -> pronunciation\n\n");

        for (word, pronunciation) in state.entries.iter() {
            content.push_str(&format!("{} -> {}\n", word, pronunciation));
        }

        return Ok(content);
    }
}

// ============================================================
// LONG-FORM OPERATION IMPLEMENTATION
// ============================================================

struct LongFormOperationState {
    content: String,
    voice_id: String,
    output_location: String,
    chapter_breaks: Option<Vec<u32>>,
    status: wit_advanced::OperationStatus,
    progress: f32,
    result: Option<wit_advanced::LongFormResult>,
}

/// Long-form synthesis operation with interior mutability
pub struct LongFormOperationImpl {
    state: RefCell<LongFormOperationState>,
}

impl LongFormOperationImpl {
    pub fn new(
        content: String,
        voice_id: String,
        output_location: String,
        chapter_breaks: Option<Vec<u32>>,
    ) -> Self {
        return LongFormOperationImpl {
            state: RefCell::new(LongFormOperationState {
                content,
                voice_id,
                output_location,
                chapter_breaks,
                status: wit_advanced::OperationStatus::Pending,
                progress: 0.0,
                result: None,
            }),
        };
    }

    /// Execute the long-form synthesis
    pub fn execute(&self) -> Result<(), wit_types::TtsError> {
        {
            let mut state = self.state.borrow_mut();
            state.status = wit_advanced::OperationStatus::Processing;
        }

        let (content, voice_id, output_location) = {
            let state = self.state.borrow();
            (
                state.content.clone(),
                state.voice_id.clone(),
                state.output_location.clone(),
            )
        };

        let chunk_size: usize = 4000;
        let total_len = content.len();
        let total_chunks = (total_len + chunk_size - 1) / chunk_size;

        let mut all_audio: Vec<u8> = Vec::new();
        let mut chunk_index: usize = 0;
        let mut offset: usize = 0;

        while offset < total_len {
            let end = core::cmp::min(offset + chunk_size, total_len);

            let mut break_point = end;
            if end < total_len {
                let search_start = if end > 100 { end - 100 } else { offset };
                let search_text = &content[search_start..end];

                for (i, c) in search_text.chars().rev().enumerate() {
                    if c == '.' || c == '!' || c == '?' {
                        break_point = end - i;
                        break;
                    }
                }
            }

            let chunk_text = &content[offset..break_point];

            let audio_chunk = client::synthesize_api(
                &voice_id,
                chunk_text,
                &client::get_model_version(),
                "mp3_44100_128",
            )?;

            all_audio.extend(audio_chunk);

            chunk_index = chunk_index + 1;
            {
                let mut state = self.state.borrow_mut();
                state.progress = (chunk_index as f32) / (total_chunks as f32);
            }
            offset = break_point;
        }

        let char_count = content.len() as u32;
        let word_count = types::count_words(&content);
        let duration = types::estimate_duration(char_count);

        let mut state = self.state.borrow_mut();
        state.result = Some(wit_advanced::LongFormResult {
            output_location: output_location.clone(),
            total_duration: duration,
            chapter_durations: None,
            metadata: wit_types::SynthesisMetadata {
                duration_seconds: duration,
                character_count: char_count,
                word_count,
                audio_size_bytes: all_audio.len() as u32,
                request_id: format!("longform-{}", char_count),
                provider_info: Some("elevenlabs".to_string()),
            },
        });

        state.status = wit_advanced::OperationStatus::Completed;
        state.progress = 1.0;

        return Ok(());
    }
}

impl wit_advanced::GuestLongFormOperation for LongFormOperationImpl {
    fn get_status(&self) -> wit_advanced::OperationStatus {
        let state = self.state.borrow();
        return state.status.clone();
    }

    fn get_progress(&self) -> f32 {
        let state = self.state.borrow();
        return state.progress;
    }

    fn cancel(&self) -> Result<(), wit_types::TtsError> {
        let mut state = self.state.borrow_mut();
        if state.status == wit_advanced::OperationStatus::Processing {
            state.status = wit_advanced::OperationStatus::Cancelled;
            return Ok(());
        }

        return Err(types::internal_error(
            "Operation cannot be cancelled in current state",
        ));
    }

    fn get_result(&self) -> Result<wit_advanced::LongFormResult, wit_types::TtsError> {
        let state = self.state.borrow();
        match &state.result {
            Some(r) => {
                return Ok(r.clone());
            }
            None => {
                return Err(types::internal_error("Result not available yet"));
            }
        }
    }
}

// ============================================================
// ADVANCED INTERFACE FUNCTIONS
// ============================================================

/// Clone a voice from audio samples
pub fn create_voice_clone(
    name: String,
    audio_samples: Vec<wit_advanced::AudioSample>,
    description: Option<String>,
) -> Result<wit_voices::Voice, wit_types::TtsError> {
    if audio_samples.is_empty() {
        return Err(types::invalid_text_error(
            "At least one audio sample is required",
        ));
    }

    return Err(types::unsupported_operation_error(
        "Voice cloning requires multipart upload - implementation pending",
    ));
}

/// Design a synthetic voice
pub fn design_voice(
    _name: String,
    _characteristics: wit_advanced::VoiceDesignParams,
) -> Result<wit_voices::Voice, wit_types::TtsError> {
    return Err(types::unsupported_operation_error(
        "Voice design API is in beta and not yet implemented",
    ));
}

/// Convert audio to target voice
pub fn convert_voice(
    _input_audio: Vec<u8>,
    _target_voice: &VoiceImpl,
    _preserve_timing: Option<bool>,
) -> Result<Vec<u8>, wit_types::TtsError> {
    return Err(types::unsupported_operation_error(
        "Voice conversion requires real-time streaming - not implemented",
    ));
}

/// Generate sound effect from text description
pub fn generate_sound_effect(
    description: String,
    duration_seconds: Option<f32>,
    style_influence: Option<f32>,
) -> Result<Vec<u8>, wit_types::TtsError> {
    let audio_data =
        client::generate_sound_effect_api(&description, duration_seconds, style_influence)?;

    return Ok(audio_data);
}

/// Create a pronunciation lexicon
pub fn create_lexicon(
    name: String,
    language: String,
    entries: Option<Vec<wit_advanced::PronunciationEntry>>,
) -> Result<wit_advanced::PronunciationLexicon, wit_types::TtsError> {
    let lexicon = PronunciationLexiconImpl::new(name, language);

    if let Some(entry_list) = entries {
        let mut state = lexicon.state.borrow_mut();
        for entry in entry_list.iter() {
            state
                .entries
                .push((entry.word.clone(), entry.pronunciation.clone()));
        }
    }

    return Ok(wit_advanced::PronunciationLexicon::new(lexicon));
}

/// Synthesize long-form content
pub fn synthesize_long_form(
    content: String,
    voice: &VoiceImpl,
    output_location: String,
    chapter_breaks: Option<Vec<u32>>,
) -> Result<wit_advanced::LongFormOperation, wit_types::TtsError> {
    if content.is_empty() {
        return Err(types::invalid_text_error("Content is empty"));
    }

    let operation = LongFormOperationImpl::new(
        content,
        voice.voice_id().to_string(),
        output_location,
        chapter_breaks,
    );

    operation.execute()?;

    return Ok(wit_advanced::LongFormOperation::new(operation));
}
