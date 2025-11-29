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
    deepgram::Deepgram,
    resources::{DeepgramLongFormOperation, DeepgramPronunciationLexicon},
};

pub mod deepgram;
pub mod error;
pub mod resources;
pub mod utils;

pub struct DeepgramComponent;

impl SynthesisGuest for DeepgramComponent {
    #[doc = " Convert text to speech (removed async)"]
    fn synthesize(
        input: TextInput,
        voice: Voice,
        options: Option<SynthesisOptions>,
    ) -> Result<SynthesisResult, TtsError> {
        let deepgram = Deepgram::new()?;
        let voice_canonical_name = voice.name.clone(); // Use canonical name for synthesis
        deepgram.synthesize(input, voice_canonical_name, options)
    }

    #[doc = " Batch synthesis for multiple inputs (removed async)"]
    fn synthesize_batch(
        inputs: Vec<TextInput>,
        voice: Voice,
        options: Option<SynthesisOptions>,
    ) -> Result<Vec<SynthesisResult>, TtsError> {
        let deepgram = Deepgram::new()?;
        let voice_canonical_name = voice.name.clone(); // Use canonical name for synthesis
        deepgram.synthesize_batch(inputs, voice_canonical_name, options)
    }

    #[doc = " Get timing information without audio synthesis"]
    fn get_timing_marks(input: TextInput, voice: Voice) -> Result<Vec<TimingInfo>, TtsError> {
        let deepgram = Deepgram::new()?;
        let voice_canonical_name = voice.name.clone(); // Use canonical name
        deepgram.get_timing_marks(input, voice_canonical_name)
    }

    #[doc = " Validate text before synthesis"]
    fn validate_input(input: TextInput, voice: Voice) -> Result<ValidationResult, TtsError> {
        let deepgram = Deepgram::new()?;
        let voice_canonical_name = voice.name.clone(); // Use canonical name
        deepgram.validate_input(input, voice_canonical_name)
    }
}

impl AdvancedGuest for DeepgramComponent {
    type PronunciationLexicon = DeepgramPronunciationLexicon;

    type LongFormOperation = DeepgramLongFormOperation;

    #[doc = " Voice cloning and creation (removed async)"]
    fn create_voice_clone(
        name: String,
        audio_samples: Vec<AudioSample>,
        description: Option<String>,
    ) -> Result<Voice, TtsError> {
        let deepgram = Deepgram::new()?;
        let voice = deepgram.create_voice_clone(name, audio_samples, description)?;
        Ok(voice)
    }

    #[doc = " Design synthetic voice (removed async)"]
    fn design_voice(name: String, characteristics: VoiceDesignParams) -> Result<Voice, TtsError> {
        let deepgram = Deepgram::new()?;
        let voice = deepgram.design_voice(name, characteristics)?;
        Ok(voice)
    }

    #[doc = " Voice-to-voice conversion (removed async)"]
    fn convert_voice(
        input_audio: Vec<u8>,
        target_voice: Voice,
        preserve_timing: Option<bool>,
    ) -> Result<Vec<u8>, TtsError> {
        let deepgram = Deepgram::new()?;
        let target_voice_canonical_name = target_voice.name.clone(); // Use canonical name
        deepgram.convert_voice(input_audio, target_voice_canonical_name, preserve_timing)
    }

    #[doc = " Generate sound effects from text description (removed async)"]
    fn generate_sound_effect(
        description: String,
        duration_seconds: Option<f32>,
        style_influence: Option<f32>,
    ) -> Result<Vec<u8>, TtsError> {
        let deepgram = Deepgram::new()?;
        deepgram.generate_sound_effect(description, duration_seconds, style_influence)
    }

    #[doc = " Create custom pronunciation lexicon"]
    fn create_lexicon(
        name: String,
        language: LanguageCode,
        entries: Option<Vec<PronunciationEntry>>,
    ) -> Result<PronunciationLexicon, TtsError> {
        let deepgram = Deepgram::new()?;
        let lexicon = deepgram.create_lexicon(name, language, entries)?;
        Ok(PronunciationLexicon::new(lexicon))
    }

    #[doc = " Long-form content synthesis with optimization (removed async)"]
    fn synthesize_long_form(
        content: String,
        voice: Voice,
        chapter_breaks: Option<Vec<u32>>,
    ) -> Result<LongFormOperation, TtsError> {
        let deepgram = Deepgram::new()?;
        let voice_canonical_name = voice.name.clone(); // Use canonical name
        let operation =
            deepgram.synthesize_long_form(content, voice_canonical_name, chapter_breaks)?;
        Ok(LongFormOperation::new(operation))
    }
}

impl VoicesGuest for DeepgramComponent {
    #[doc = " List available voices with filtering and pagination"]
    fn list_voices(filter: Option<VoiceFilter>) -> Result<Vec<Voice>, TtsError> {
        let deepgram = Deepgram::new()?;
        deepgram.list_voices(filter)
    }

    #[doc = " Get specific voice by ID"]
    fn get_voice(voice_id: String) -> Result<Voice, TtsError> {
        let deepgram = Deepgram::new()?;
        deepgram.get_voice(voice_id)
    }

    #[doc = " Get supported languages"]
    fn list_languages() -> Result<Vec<LanguageInfo>, TtsError> {
        let deepgram = Deepgram::new()?;
        deepgram.list_languages()
    }
}

impl ExtendedAdvancedTrait for DeepgramComponent {
    fn unwrappered_created_lexicon(
        name: String,
        language: LanguageCode,
        entries: Option<Vec<PronunciationEntry>>,
    ) -> Result<Self::PronunciationLexicon, golem_tts::golem::tts::types::TtsError> {
        let client = Deepgram::new()?;
        client.create_lexicon(name, language, entries)
    }

    fn unwrappered_synthesize_long_form(
        content: String,
        voice: Voice,
        chapter_breaks: Option<Vec<u32>>,
        _task_id: Option<String>,
    ) -> Result<Self::LongFormOperation, golem_tts::golem::tts::types::TtsError> {
        let client = Deepgram::new()?;
        let voice_canonical_name = voice.name.clone(); // Use canonical name
        client.synthesize_long_form(content, voice_canonical_name, chapter_breaks)
    }
}

type DurableDeepgramComponent = DurableTTS<DeepgramComponent>;

golem_tts::export_tts!(DurableDeepgramComponent with_types_in golem_tts);
