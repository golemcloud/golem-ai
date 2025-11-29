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
    polly::Polly,
    resources::{AwsLongFormOperation, AwsPronunciationLexicon},
};

pub mod aws_signer;
pub mod error;
pub mod polly;
pub mod resources;
pub mod types;
pub mod utils;

pub struct AwsPollyComponent;

impl VoicesGuest for AwsPollyComponent {
    #[doc = " List available voices with filtering and pagination"]
    fn list_voices(filter: Option<VoiceFilter>) -> Result<Vec<Voice>, TtsError> {
        let polly = Polly::new()?;
        polly.list_voices(filter)
    }

    #[doc = " Get specific voice by ID"]
    fn get_voice(voice_id: String) -> Result<Voice, TtsError> {
        let polly = Polly::new()?;
        let voice = polly.get_voice(voice_id)?;
        Ok(voice)
    }

    #[doc = " Get supported languages"]
    fn list_languages() -> Result<Vec<LanguageInfo>, TtsError> {
        let polly = Polly::new()?;
        polly.list_languages()
    }
}

impl SynthesisGuest for AwsPollyComponent {
    #[doc = " Convert text to speech (removed async)"]
    fn synthesize(
        input: TextInput,
        voice: Voice,
        options: Option<SynthesisOptions>,
    ) -> Result<SynthesisResult, TtsError> {
        let polly = Polly::new()?;
        let voice_name = voice.id.clone();
        polly.synthesize(input, voice_name, options)
    }

    #[doc = " Batch synthesis for multiple inputs (removed async)"]
    fn synthesize_batch(
        inputs: Vec<TextInput>,
        voice: Voice,
        options: Option<SynthesisOptions>,
    ) -> Result<Vec<SynthesisResult>, TtsError> {
        let polly = Polly::new()?;
        let voice_name = voice.id.clone();
        polly.synthesize_batch(inputs, voice_name, options)
    }

    #[doc = " Get timing information without audio synthesis"]
    fn get_timing_marks(input: TextInput, voice: Voice) -> Result<Vec<TimingInfo>, TtsError> {
        let polly = Polly::new()?;
        let voice_name = voice.id.clone();
        polly.get_timing_marks(input, voice_name)
    }

    #[doc = " Validate text before synthesis"]
    fn validate_input(input: TextInput, voice: Voice) -> Result<ValidationResult, TtsError> {
        let polly = Polly::new()?;
        let voice_name = voice.id.clone();
        polly.validate_input(input, voice_name)
    }
}

impl AdvancedGuest for AwsPollyComponent {
    type PronunciationLexicon = AwsPronunciationLexicon;

    type LongFormOperation = AwsLongFormOperation;

    #[doc = " Voice cloning and creation (removed async)"]
    fn create_voice_clone(
        name: String,
        audio_samples: Vec<AudioSample>,
        description: Option<String>,
    ) -> Result<Voice, TtsError> {
        let polly = Polly::new()?;
        let voice = polly.create_voice_clone(name, audio_samples, description)?;
        Ok(voice)
    }

    #[doc = " Design synthetic voice (removed async)"]
    fn design_voice(name: String, characteristics: VoiceDesignParams) -> Result<Voice, TtsError> {
        let polly = Polly::new()?;
        let voice = polly.design_voice(name, characteristics)?;
        Ok(voice)
    }

    #[doc = " Voice-to-voice conversion (removed async)"]
    fn convert_voice(
        input_audio: Vec<u8>,
        target_voice: Voice,
        preserve_timing: Option<bool>,
    ) -> Result<Vec<u8>, TtsError> {
        let polly = Polly::new()?;
        let target_voice_name = target_voice.id.clone();
        polly.convert_voice(input_audio, target_voice_name, preserve_timing)
    }

    #[doc = " Generate sound effects from text description (removed async)"]
    fn generate_sound_effect(
        description: String,
        duration_seconds: Option<f32>,
        style_influence: Option<f32>,
    ) -> Result<Vec<u8>, TtsError> {
        let polly = Polly::new()?;
        polly.generate_sound_effect(description, duration_seconds, style_influence)
    }

    #[doc = " Create custom pronunciation lexicon"]
    fn create_lexicon(
        name: String,
        language: LanguageCode,
        entries: Option<Vec<PronunciationEntry>>,
    ) -> Result<PronunciationLexicon, TtsError> {
        let polly = Polly::new()?;
        let lexicon = polly.create_lexicon(name, language, entries)?;
        Ok(PronunciationLexicon::new(lexicon))
    }

    #[doc = " Long-form content synthesis with optimization (removed async)"]
    fn synthesize_long_form(
        content: String,
        voice: Voice,
        chapter_breaks: Option<Vec<u32>>,
    ) -> Result<LongFormOperation, TtsError> {
        let polly = Polly::new()?;
        let voice_name = voice.id.clone();
        let operation = polly.synthesize_long_form(content, voice_name, chapter_breaks)?;
        Ok(LongFormOperation::new(operation))
    }
}

impl ExtendedAdvancedTrait for AwsPollyComponent {
    fn unwrappered_created_lexicon(
        name: String,
        language: LanguageCode,
        entries: Option<Vec<PronunciationEntry>>,
    ) -> Result<Self::PronunciationLexicon, TtsError> {
        let client = Polly::new()?;
        client.create_lexicon(name, language, entries)
    }

    fn unwrappered_synthesize_long_form(
        content: String,
        voice: Voice,
        chapter_breaks: Option<Vec<u32>>,
        task_id: Option<String>,
    ) -> Result<Self::LongFormOperation, TtsError> {
        if let Some(task_id) = task_id {
            Ok(AwsLongFormOperation::from(task_id))
        } else {
            let client = Polly::new()?;
            let voice_id = voice.id.clone();
            client.synthesize_long_form(content, voice_id, chapter_breaks)
        }
    }
}

type DurableAwsPollyComponent = DurableTTS<AwsPollyComponent>;

golem_tts::export_tts!(DurableAwsPollyComponent with_types_in golem_tts);
