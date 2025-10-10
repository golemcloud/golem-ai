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
    elevenlabs::Elevenlabs,
    error::unsupported,
    resources::{ElLongFormSynthesis, ElPronunciationLexicon},
};

pub mod elevenlabs;
pub mod error;
pub mod resources;
pub mod types;
pub mod utils;

pub struct ElevenLabsTtsComponent;

impl SynthesisGuest for ElevenLabsTtsComponent {
    #[doc = " Convert text to speech (removed async)"]
    fn synthesize(
        input: TextInput,
        voice: Voice,
        options: Option<SynthesisOptions>,
    ) -> Result<SynthesisResult, TtsError> {
        let client = Elevenlabs::new()?;
        let voice_name = voice.id.clone();
        client.synthesize(input, voice_name, options)
    }

    #[doc = " Batch synthesis for multiple inputs (removed async)"]
    fn synthesize_batch(
        inputs: Vec<TextInput>,
        voice: Voice,
        options: Option<SynthesisOptions>,
    ) -> Result<Vec<SynthesisResult>, TtsError> {
        let client = Elevenlabs::new()?;
        let voice_name = voice.id.clone();
        client.synthesize_batch(inputs, voice_name, options)
    }

    #[doc = " Get timing information without audio synthesis"]
    fn get_timing_marks(input: TextInput, voice: Voice) -> Result<Vec<TimingInfo>, TtsError> {
        let client = Elevenlabs::new()?;
        let voice_name = voice.id.clone();
        client.get_timing_marks(input, voice_name)
    }

    #[doc = " Validate text before synthesis"]
    fn validate_input(input: TextInput, voice: Voice) -> Result<ValidationResult, TtsError> {
        let client = Elevenlabs::new()?;
        let voice_name = voice.id.clone();
        client.validate_input(input, voice_name)
    }
}

impl AdvancedGuest for ElevenLabsTtsComponent {
    type PronunciationLexicon = ElPronunciationLexicon;

    type LongFormOperation = ElLongFormSynthesis;

    #[doc = " Voice cloning and creation (removed async)"]
    fn create_voice_clone(
        name: String,
        audio_samples: Vec<AudioSample>,
        description: Option<String>,
    ) -> Result<Voice, TtsError> {
        let client = Elevenlabs::new()?;
        client.create_voice_clone(name, audio_samples, description)
    }

    #[doc = " Design synthetic voice (removed async)"]
    fn design_voice(name: String, characteristics: VoiceDesignParams) -> Result<Voice, TtsError> {
        let client = Elevenlabs::new()?;
        client.design_voice(name, characteristics)
    }

    #[doc = " Voice-to-voice conversion (removed async)"]
    fn convert_voice(
        input_audio: Vec<u8>,
        target_voice: Voice,
        preserve_timing: Option<bool>,
    ) -> Result<Vec<u8>, TtsError> {
        let client = Elevenlabs::new()?;
        let target_voice_name = target_voice.id.clone();
        client.convert_voice(input_audio, target_voice_name, preserve_timing)
    }

    #[doc = " Generate sound effects from text description (removed async)"]
    fn generate_sound_effect(
        description: String,
        duration_seconds: Option<f32>,
        style_influence: Option<f32>,
    ) -> Result<Vec<u8>, TtsError> {
        let client = Elevenlabs::new()?;
        client.generate_sound_effect(description, duration_seconds, style_influence)
    }

    #[doc = " Create custom pronunciation lexicon"]
    fn create_lexicon(
        name: String,
        language: LanguageCode,
        entries: Option<Vec<PronunciationEntry>>,
    ) -> Result<PronunciationLexicon, TtsError> {
        let client = Elevenlabs::new()?;
      let  lexicon =  client.create_lexicon(name, language, entries)?;
        Ok(PronunciationLexicon::new(lexicon))
    }

    #[doc = " Long-form content synthesis with optimization (removed async)"]
    fn synthesize_long_form(
        _content: String,
        _voice: Voice,
        _output_location: String,
        _chapter_breaks: Option<Vec<u32>>,
    ) -> Result<LongFormOperation, TtsError> {
        unsupported("Long-form content synthesis is not supported by Elvenlabs")
    }
}


impl VoicesGuest for ElevenLabsTtsComponent {
    #[doc = " List available voices with filtering and pagination"]
    fn list_voices(filter: Option<VoiceFilter>) -> Result<Vec<Voice>, TtsError> {
        let client = Elevenlabs::new()?;
        client.list_voices(filter)
    }

    #[doc = " Get specific voice by ID"]
    fn get_voice(voice_id: String) -> Result<Voice, TtsError> {
        let client = Elevenlabs::new()?;
        client.get_voice(voice_id)
    }

    #[doc = " Get supported languages"]
    fn list_languages() -> Result<Vec<LanguageInfo>, TtsError> {
        let client = Elevenlabs::new()?;
        client.list_languages()
    }
}
impl ExtendedAdvancedTrait for ElevenLabsTtsComponent {
    fn unwrappered_created_lexicon(
        name: String,
        language: LanguageCode,
        entries: Option<Vec<PronunciationEntry>>,
    ) -> Result<Self::PronunciationLexicon,TtsError> {
        let client  = Elevenlabs::new()?;
        client.create_lexicon(name, language, entries)
    }

    fn unwrappered_synthesize_long_form(
        content: String,
        voice: Voice,
        output_location: String,
        chapter_breaks: Option<Vec<u32>>,
    ) -> Result<Self::LongFormOperation, TtsError> {
        let client = Elevenlabs::new()?;
        let voice_id = voice.id.clone();
        client.synthesize_long_form(content, voice_id, output_location, chapter_breaks)
    }
}

type DurableElevenLabsTtsComponent = DurableTTS<ElevenLabsTtsComponent>;

golem_tts::export_tts!(DurableElevenLabsTtsComponent with_types_in golem_tts);
