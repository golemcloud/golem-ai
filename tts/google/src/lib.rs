use golem_tts::{
    client::TtsClient,
    durability::{DurableTTS, ExtendedAdvancedTrait},
    golem::tts::{
        advanced::{
            AudioSample, Guest as AdvancedGuest, LanguageCode, LongFormOperation,
            PronunciationEntry, PronunciationLexicon, TtsError, Voice, VoiceDesignParams,
        },
        synthesis::{
            Guest as SynthesisGuest, SynthesisOptions, SynthesisResult, TextInput, TimingInfo,
            ValidationResult,
        },
        voices::{Guest as VoicesGuest, LanguageInfo, VoiceFilter},
    },
};

use crate::{
    error::unsupported,
    google::Google,
    resources::{GoogleLongFormOperation, GooglePronunciationLexicon},
};

pub mod auth;
pub mod error;
pub mod google;
pub mod resources;
pub mod types;
pub mod utils;

pub struct GoogleTtsComponent;

impl SynthesisGuest for GoogleTtsComponent {
    #[doc = " Convert text to speech (removed async)"]
    fn synthesize(
        input: TextInput,
        voice: Voice,
        options: Option<SynthesisOptions>,
    ) -> Result<SynthesisResult, TtsError> {
        let google = Google::new()?;
        let voice_name = voice.id.clone();
        google.synthesize(input, voice_name, options)
    }

    #[doc = " Batch synthesis for multiple inputs (removed async)"]
    fn synthesize_batch(
        inputs: Vec<TextInput>,
        voice: Voice,
        options: Option<SynthesisOptions>,
    ) -> Result<Vec<SynthesisResult>, TtsError> {
        let google = Google::new()?;
        let voice_name = voice.id.clone();
        google.synthesize_batch(inputs, voice_name, options)
    }

    #[doc = " Get timing information without audio synthesis"]
    fn get_timing_marks(input: TextInput, voice: Voice) -> Result<Vec<TimingInfo>, TtsError> {
        let google = Google::new()?;
        let voice_name = voice.id.clone();
        google.get_timing_marks(input, voice_name)
    }

    #[doc = " Validate text before synthesis"]
    fn validate_input(input: TextInput, voice: Voice) -> Result<ValidationResult, TtsError> {
        let google = Google::new()?;
        let voice_name = voice.id.clone();
        google.validate_input(input, voice_name)
    }
}

impl VoicesGuest for GoogleTtsComponent {
    #[doc = " List available voices with filtering and pagination"]
    fn list_voices(filter: Option<VoiceFilter>) -> Result<Vec<Voice>, TtsError> {
        let google = Google::new()?;
        google.list_voices(filter)
    }

    #[doc = " Get specific voice by ID"]
    fn get_voice(voice_id: String) -> Result<Voice, TtsError> {
        let google = Google::new()?;
        google.get_voice(voice_id)
    }

    #[doc = " Get supported languages"]
    fn list_languages() -> Result<Vec<LanguageInfo>, TtsError> {
        let google = Google::new()?;
        google.list_languages()
    }
}

impl AdvancedGuest for GoogleTtsComponent {
    type PronunciationLexicon = GooglePronunciationLexicon;

    type LongFormOperation = GoogleLongFormOperation;

    #[doc = " Voice cloning and creation (removed async)"]
    fn create_voice_clone(
        _name: String,
        _audio_samples: Vec<AudioSample>,
        _description: Option<String>,
    ) -> Result<Voice, TtsError> {
        unsupported("Google TTS does not support voice cloning")
    }

    #[doc = " Design synthetic voice (removed async)"]
    fn design_voice(name: String, characteristics: VoiceDesignParams) -> Result<Voice, TtsError> {
        let google = Google::new()?;
        let voice = google.design_voice(name, characteristics)?;
        Ok(voice)
    }

    #[doc = " Voice-to-voice conversion (removed async)"]
    fn convert_voice(
        _input_audio: Vec<u8>,
        _target_voice: Voice,
        _preserve_timing: Option<bool>,
    ) -> Result<Vec<u8>, TtsError> {
        unsupported("Google TTS does not support voice conversion")
    }

    #[doc = " Generate sound effects from text description (removed async)"]
    fn generate_sound_effect(
        _description: String,
        _duration_seconds: Option<f32>,
        _style_influence: Option<f32>,
    ) -> Result<Vec<u8>, TtsError> {
        unsupported("Google TTS does not support sound effect generation")
    }

    #[doc = " Create custom pronunciation lexicon"]
    fn create_lexicon(
        _name: String,
        _language: LanguageCode,
        _entries: Option<Vec<PronunciationEntry>>,
    ) -> Result<PronunciationLexicon, TtsError> {
        unsupported("Google TTS does not support custom pronunciation lexicons")
    }

    #[doc = " Long-form content synthesis with optimization (removed async)"]
    fn synthesize_long_form(
        content: String,
        voice: Voice,
        chapter_breaks: Option<Vec<u32>>,
    ) -> Result<LongFormOperation, TtsError> {
        let google = Google::new()?;
        let voice_name = voice.id.clone();
        let operation =
            google.synthesize_long_form(content, voice_name, chapter_breaks)?;
        Ok(LongFormOperation::new(operation))
    }
}

impl ExtendedAdvancedTrait for GoogleTtsComponent {
    fn unwrappered_created_lexicon(
        name: String,
        language: LanguageCode,
        entries: Option<Vec<PronunciationEntry>>,
    ) -> Result<Self::PronunciationLexicon, TtsError> {
        let client = Google::new()?;
        client.create_lexicon(name, language, entries)
    }

    fn unwrappered_synthesize_long_form(
        content: String,
        voice: Voice,
        chapter_breaks: Option<Vec<u32>>,
        _task_id: Option<String>,
    ) -> Result<Self::LongFormOperation, TtsError> {
        let client = Google::new()?;
        let voice_id = voice.id.clone();
        client.synthesize_long_form(content, voice_id, chapter_breaks)
    }
}

type DurableGoogleTtsComponent = DurableTTS<GoogleTtsComponent>;

golem_tts::export_tts!(DurableGoogleTtsComponent with_types_in golem_tts);
