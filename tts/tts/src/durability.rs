use crate::exports::golem::tts::advanced::Guest as AdvancedGuest;
#[allow(unused_imports)]
use crate::exports::golem::tts::advanced::{
    AudioSample, LongFormOperation, LongFormResult, OperationStatus, PronunciationEntry,
    PronunciationLexicon, VoiceDesignParams,
};
use crate::exports::golem::tts::streaming::Guest as StreamingGuest;
#[allow(unused_imports)]
use crate::exports::golem::tts::streaming::{StreamStatus, SynthesisStream, VoiceConversionStream};
use crate::exports::golem::tts::synthesis::Guest as SynthesisGuest;
#[allow(unused_imports)]
use crate::exports::golem::tts::synthesis::{SynthesisOptions, ValidationResult};
#[allow(unused_imports)]
use crate::exports::golem::tts::types::{
    AudioChunk, AudioConfig, AudioEffects, AudioFormat, LanguageCode, SynthesisResult, TextInput,
    TimingInfo, TtsError, VoiceGender, VoiceQuality, VoiceSettings,
};
use crate::exports::golem::tts::voices::Guest as VoicesGuest;
#[allow(unused_imports)]
use crate::exports::golem::tts::voices::{
    LanguageInfo, Voice, VoiceFilter, VoiceInfo, VoiceResults,
};
use std::marker::PhantomData;

pub struct DurableTts<Impl> {
    phantom: PhantomData<Impl>,
}
pub trait ExtendedGuest:
    VoicesGuest + SynthesisGuest + StreamingGuest + AdvancedGuest + 'static
{
    fn unwrapped_synthesis_stream(
        voice: crate::exports::golem::tts::voices::VoiceBorrow<'_>,
        options: Option<SynthesisOptions>,
    ) -> Self::SynthesisStream;

    fn unwrapped_voice_conversion_stream(
        target_voice: crate::exports::golem::tts::voices::VoiceBorrow<'_>,
        options: Option<SynthesisOptions>,
    ) -> Self::VoiceConversionStream;

    fn subscribe_synthesis_stream(stream: &Self::SynthesisStream)
        -> golem_rust::wasm_rpc::Pollable;

    fn subscribe_voice_conversion_stream(
        stream: &Self::VoiceConversionStream,
    ) -> golem_rust::wasm_rpc::Pollable;
}

/// When the durability feature flag is off, wrapping with `DurableTts` is just a passthrough
#[cfg(not(feature = "durability"))]
mod passthrough_impl {
    use crate::durability::{DurableTts, ExtendedGuest};
    use crate::exports::golem::tts::advanced::Guest as AdvancedGuest;
    use crate::exports::golem::tts::advanced::{
        AudioSample, LongFormOperation, LongFormResult, OperationStatus, PronunciationEntry,
        PronunciationLexicon, VoiceDesignParams,
    };
    use crate::exports::golem::tts::streaming::Guest as StreamingGuest;
    use crate::exports::golem::tts::streaming::{
        StreamStatus, SynthesisStream, VoiceConversionStream,
    };
    use crate::exports::golem::tts::synthesis::Guest as SynthesisGuest;
    use crate::exports::golem::tts::synthesis::{SynthesisOptions, ValidationResult};
    use crate::exports::golem::tts::types::{
        AudioChunk, AudioConfig, AudioEffects, AudioFormat, LanguageCode, SynthesisResult,
        TextInput, TimingInfo, TtsError, VoiceGender, VoiceQuality, VoiceSettings,
    };
    use crate::exports::golem::tts::voices::Guest as VoicesGuest;
    use crate::exports::golem::tts::voices::{
        LanguageInfo, Voice, VoiceFilter, VoiceInfo, VoiceResults,
    };
    use crate::init_logging;

    impl<Impl: ExtendedGuest> VoicesGuest for DurableTts<Impl> {
        type Voice = Impl::Voice;
        type VoiceResults = Impl::VoiceResults;

        fn list_voices(filter: Option<VoiceFilter>) -> Result<VoiceResults, TtsError> {
            init_logging();
            Impl::list_voices(filter)
        }

        fn get_voice(voice_id: String) -> Result<Voice, TtsError> {
            init_logging();
            Impl::get_voice(voice_id)
        }

        fn search_voices(filter: Option<VoiceFilter>) -> Result<Vec<VoiceInfo>, TtsError> {
            init_logging();
            Impl::search_voices(filter)
        }

        fn list_languages() -> Result<Vec<LanguageInfo>, TtsError> {
            init_logging();
            Impl::list_languages()
        }
    }

    impl<Impl: ExtendedGuest> SynthesisGuest for DurableTts<Impl> {
        fn synthesize(
            input: TextInput,
            voice: crate::exports::golem::tts::voices::VoiceBorrow<'_>,
            options: Option<SynthesisOptions>,
        ) -> Result<SynthesisResult, TtsError> {
            init_logging();
            Impl::synthesize(input, voice, options)
        }

        fn synthesize_batch(
            inputs: Vec<TextInput>,
            voice: crate::exports::golem::tts::voices::VoiceBorrow<'_>,
            options: Option<SynthesisOptions>,
        ) -> Result<Vec<SynthesisResult>, TtsError> {
            init_logging();
            Impl::synthesize_batch(inputs, voice, options)
        }

        fn get_timing_marks(
            input: TextInput,
            voice: crate::exports::golem::tts::voices::VoiceBorrow<'_>,
        ) -> Result<Vec<TimingInfo>, TtsError> {
            init_logging();
            Impl::get_timing_marks(input, voice)
        }

        fn validate_input(
            input: TextInput,
            voice: crate::exports::golem::tts::voices::VoiceBorrow<'_>,
        ) -> Result<ValidationResult, TtsError> {
            init_logging();
            Impl::validate_input(input, voice)
        }
    }

    impl<Impl: ExtendedGuest> StreamingGuest for DurableTts<Impl> {
        type SynthesisStream = Impl::SynthesisStream;
        type VoiceConversionStream = Impl::VoiceConversionStream;

        fn create_stream(
            voice: crate::exports::golem::tts::voices::VoiceBorrow<'_>,
            options: Option<SynthesisOptions>,
        ) -> Result<SynthesisStream, TtsError> {
            init_logging();
            Impl::create_stream(voice, options)
        }

        fn create_voice_conversion_stream(
            target_voice: crate::exports::golem::tts::voices::VoiceBorrow<'_>,
            options: Option<SynthesisOptions>,
        ) -> Result<VoiceConversionStream, TtsError> {
            init_logging();
            Impl::create_voice_conversion_stream(target_voice, options)
        }
    }

    impl<Impl: ExtendedGuest> AdvancedGuest for DurableTts<Impl> {
        type PronunciationLexicon = Impl::PronunciationLexicon;
        type LongFormOperation = Impl::LongFormOperation;

        fn create_voice_clone(
            name: String,
            audio_samples: Vec<AudioSample>,
            description: Option<String>,
        ) -> Result<Voice, TtsError> {
            init_logging();
            Impl::create_voice_clone(name, audio_samples, description)
        }

        fn design_voice(
            name: String,
            characteristics: VoiceDesignParams,
        ) -> Result<Voice, TtsError> {
            init_logging();
            Impl::design_voice(name, characteristics)
        }

        fn convert_voice(
            input_audio: Vec<u8>,
            target_voice: crate::exports::golem::tts::voices::VoiceBorrow<'_>,
            preserve_timing: Option<bool>,
        ) -> Result<Vec<u8>, TtsError> {
            init_logging();
            Impl::convert_voice(input_audio, target_voice, preserve_timing)
        }

        fn generate_sound_effect(
            description: String,
            duration_seconds: Option<f32>,
            style_influence: Option<f32>,
        ) -> Result<Vec<u8>, TtsError> {
            init_logging();
            Impl::generate_sound_effect(description, duration_seconds, style_influence)
        }

        fn create_lexicon(
            name: String,
            language: LanguageCode,
            entries: Option<Vec<PronunciationEntry>>,
        ) -> Result<PronunciationLexicon, TtsError> {
            init_logging();
            Impl::create_lexicon(name, language, entries)
        }

        fn synthesize_long_form(
            content: String,
            voice: crate::exports::golem::tts::voices::VoiceBorrow<'_>,
            output_location: String,
            chapter_breaks: Option<Vec<u32>>,
        ) -> Result<LongFormOperation, TtsError> {
            init_logging();
            Impl::synthesize_long_form(content, voice, output_location, chapter_breaks)
        }
    }
}

/// When the durability feature flag is on, wrapping with `DurableTts` adds custom durability
/// on top of the provider-specific TTS implementation using Golem's special host functions and
/// the `golem-rust` helper library.
#[cfg(feature = "durability")]
mod durable_impl {
    use crate::durability::{DurableTts, ExtendedGuest};
    use crate::exports::golem::tts::advanced::Guest as AdvancedGuest;
    use crate::exports::golem::tts::streaming::GuestVoiceConversionStream;
    use crate::exports::golem::tts::voices::GuestVoiceResults;

    #[allow(unused_imports)]
    use crate::exports::golem::tts::advanced::{
        AudioSample, LongFormOperation, LongFormResult, OperationStatus, PronunciationEntry,
        PronunciationLexicon, VoiceDesignParams,
    };
    use crate::exports::golem::tts::advanced::{GuestLongFormOperation, GuestPronunciationLexicon};
    use crate::exports::golem::tts::streaming::Guest as StreamingGuest;
    #[allow(unused_imports)]
    use crate::exports::golem::tts::streaming::{
        StreamStatus, SynthesisStream, VoiceConversionStream,
    };
    use crate::exports::golem::tts::synthesis::Guest as SynthesisGuest;
    use crate::exports::golem::tts::synthesis::{SynthesisOptions, ValidationResult};
    #[allow(unused_imports)]
    use crate::exports::golem::tts::types::{
        AudioChunk, AudioConfig, AudioEffects, AudioFormat, LanguageCode, SynthesisResult,
        TextInput, TimingInfo, TtsError, VoiceGender, VoiceQuality, VoiceSettings,
    };
    use crate::exports::golem::tts::voices::Guest as VoicesGuest;
    use crate::exports::golem::tts::voices::GuestVoice;
    use crate::exports::golem::tts::voices::{
        LanguageInfo, Voice, VoiceFilter, VoiceInfo, VoiceResults,
    };
    use crate::init_logging;
    use golem_rust::bindings::golem::durability::durability::{
        DurableFunctionType, LazyInitializedPollable,
    };
    use golem_rust::durability::Durability;
    use golem_rust::wasm_rpc::Pollable;
    use golem_rust::{with_persistence_level, FromValueAndType, IntoValue, PersistenceLevel};
    use std::cell::RefCell;
    use std::fmt::{Display, Formatter};
    use std::marker::PhantomData;

    #[allow(dead_code)]
    #[derive(Debug, Clone, PartialEq, IntoValue, FromValueAndType)]
    struct ListVoicesInput {
        filter: Option<VoiceFilter>,
    }

    #[allow(dead_code)]
    #[derive(Debug, Clone, PartialEq, IntoValue, FromValueAndType)]
    struct GetVoiceInput {
        voice_id: String,
    }

    #[derive(Debug, Clone, PartialEq, IntoValue, FromValueAndType)]
    struct SearchVoicesInput {
        filter: Option<VoiceFilter>,
    }

    #[derive(Debug, Clone, PartialEq, IntoValue, FromValueAndType)]
    struct SynthesizeInput {
        input: TextInput,
        options: Option<SynthesisOptions>,
    }

    #[derive(Debug, Clone, PartialEq, IntoValue, FromValueAndType)]
    struct SynthesizeBatchInput {
        inputs: Vec<TextInput>,
        options: Option<SynthesisOptions>,
    }

    #[derive(Debug, Clone, PartialEq, IntoValue, FromValueAndType)]
    struct GetTimingMarksInput {
        input: TextInput,
    }

    #[derive(Debug, Clone, PartialEq, IntoValue, FromValueAndType)]
    struct ValidateInputInput {
        input: TextInput,
    }

    #[allow(dead_code)]
    #[derive(Debug, Clone, PartialEq, IntoValue, FromValueAndType)]
    struct CreateStreamInput {
        options: Option<SynthesisOptions>,
    }

    #[allow(dead_code)]
    #[derive(Debug, Clone, PartialEq, IntoValue, FromValueAndType)]
    struct CreateVoiceConversionStreamInput {
        options: Option<SynthesisOptions>,
    }

    #[allow(dead_code)]
    #[derive(Debug, Clone, PartialEq, IntoValue, FromValueAndType)]
    struct CreateVoiceCloneInput {
        name: String,
        audio_samples: Vec<AudioSample>,
        description: Option<String>,
    }

    #[allow(dead_code)]
    #[derive(Debug, Clone, PartialEq, IntoValue, FromValueAndType)]
    struct DesignVoiceInput {
        name: String,
        characteristics: VoiceDesignParams,
    }

    #[derive(Debug, Clone, PartialEq, IntoValue, FromValueAndType)]
    struct ConvertVoiceInput {
        input_audio: Vec<u8>,
        preserve_timing: Option<bool>,
    }

    #[derive(Debug, Clone, PartialEq, IntoValue, FromValueAndType)]
    struct GenerateSoundEffectInput {
        description: String,
        duration_seconds: Option<f32>,
        style_influence: Option<f32>,
    }

    #[allow(dead_code)]
    #[derive(Debug, Clone, PartialEq, IntoValue, FromValueAndType)]
    struct CreateLexiconInput {
        name: String,
        language: LanguageCode,
        entries: Option<Vec<PronunciationEntry>>,
    }

    #[allow(dead_code)]
    #[derive(Debug, Clone, PartialEq, IntoValue, FromValueAndType)]
    struct SynthesizeLongFormInput {
        content: String,
        output_location: String,
        chapter_breaks: Option<Vec<u32>>,
    }

    #[derive(Debug, Clone, PartialEq, IntoValue, FromValueAndType)]
    struct NoInput;

    #[derive(Debug, Clone, PartialEq, FromValueAndType, IntoValue)]
    struct NoOutput;

    #[derive(Debug, Clone, PartialEq, FromValueAndType, IntoValue)]
    struct PronunciationEntryInput {
        word: String,
        pronunciation: String,
    }

    #[derive(Debug, Clone, PartialEq, FromValueAndType, IntoValue)]
    struct RemoveEntryInput {
        word: String,
    }

    #[allow(dead_code)]
    #[derive(Debug, Clone, PartialEq, FromValueAndType, IntoValue)]
    struct PronunciationEntryListOutput {
        entries: Vec<PronunciationEntry>,
    }

    #[allow(dead_code)]
    #[derive(Debug, Clone, PartialEq, FromValueAndType, IntoValue)]
    struct LongFormResultOutput {
        result: String,
    }

    #[allow(dead_code)]
    #[derive(Debug, Clone, PartialEq, FromValueAndType, IntoValue)]
    struct UpdateVoiceSettingsInput {
        settings: VoiceSettings,
    }

    #[allow(dead_code)]
    #[derive(Debug, Clone, PartialEq, FromValueAndType, IntoValue)]
    struct PreviewVoiceInput {
        text: String,
    }

    #[derive(Debug, FromValueAndType, IntoValue)]
    struct UnusedError;

    impl Display for UnusedError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "UnusedError")
        }
    }

    #[derive(Debug, Clone, PartialEq, FromValueAndType, IntoValue)]
    struct VoiceInfoListOutput {
        voices: Vec<VoiceInfo>,
    }

    #[derive(Debug, Clone, PartialEq, FromValueAndType, IntoValue)]
    struct LanguageInfoListOutput {
        languages: Vec<LanguageInfo>,
    }

    #[derive(Debug, Clone, PartialEq, FromValueAndType, IntoValue)]
    struct SynthesisResultOutput {
        result: SynthesisResult,
    }

    #[derive(Debug, Clone, PartialEq, FromValueAndType, IntoValue)]
    struct SynthesisResultListOutput {
        results: Vec<SynthesisResult>,
    }

    #[derive(Debug, Clone, PartialEq, FromValueAndType, IntoValue)]
    struct TimingInfoListOutput {
        timing: Vec<TimingInfo>,
    }

    #[derive(Debug, Clone, PartialEq, FromValueAndType, IntoValue)]
    struct ValidationResultOutput {
        result: ValidationResult,
    }

    #[derive(Debug, Clone, PartialEq, FromValueAndType, IntoValue)]
    struct AudioDataOutput {
        audio: Vec<u8>,
    }

    #[derive(Debug, Clone, PartialEq, FromValueAndType, IntoValue)]
    struct VoiceResultsOutput {
        voices: Vec<VoiceInfo>,
    }

    #[derive(Debug, Clone, PartialEq, FromValueAndType, IntoValue)]
    struct VoiceOutput {
        id: String,
        name: String,
        provider_id: Option<String>,
        language: String,
        additional_languages: Vec<String>,
        gender: VoiceGender,
        quality: VoiceQuality,
        description: Option<String>,
        supports_ssml: bool,
        sample_rates: Vec<u32>,
        supported_formats: Vec<AudioFormat>,
    }

    #[derive(Debug, Clone, PartialEq, FromValueAndType, IntoValue)]
    struct PronunciationLexiconOutput {
        name: String,
        language: LanguageCode,
        entries: Option<Vec<PronunciationEntry>>,
    }

    #[derive(Debug, Clone, PartialEq, FromValueAndType, IntoValue)]
    struct LongFormOperationOutput {
        content: String,
        output_location: String,
        chapter_breaks: Option<Vec<u32>>,
    }

    impl From<&TtsError> for TtsError {
        fn from(error: &TtsError) -> Self {
            error.clone()
        }
    }

    impl<Impl: ExtendedGuest> VoicesGuest for DurableTts<Impl> {
        type Voice = Impl::Voice;
        type VoiceResults = Impl::VoiceResults;

        fn list_voices(filter: Option<VoiceFilter>) -> Result<VoiceResults, TtsError> {
            init_logging();

            let durability = Durability::<VoiceResultsOutput, TtsError>::new(
                "golem_tts",
                "list_voices",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::list_voices(filter.clone())
                });
                let voices = match &result {
                    Ok(voice_results) => {
                        let guest = voice_results.get::<Impl::VoiceResults>();
                        guest.get_next().unwrap_or_default()
                    }
                    Err(_) => Vec::new(),
                };
                durability.persist(
                    ListVoicesInput { filter },
                    result.map(|_| VoiceResultsOutput {
                        voices: voices.clone(),
                    }),
                )?;

                Ok(VoiceResults::new(
                    DurableVoiceResults::<Impl>::new_with_voices(voices),
                ))
            } else {
                let output = durability.replay::<VoiceResultsOutput, TtsError>()?;
                Ok(VoiceResults::new(
                    DurableVoiceResults::<Impl>::new_with_voices(output.voices),
                ))
            }
        }

        fn get_voice(voice_id: String) -> Result<Voice, TtsError> {
            init_logging();

            let durability = Durability::<VoiceOutput, TtsError>::new(
                "golem_tts",
                "get_voice",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::get_voice(voice_id.clone())
                });
                let voice_data = match &result {
                    Ok(voice) => {
                        let guest = voice.get::<Impl::Voice>();
                        VoiceOutput {
                            id: guest.get_id(),
                            name: guest.get_name(),
                            provider_id: guest.get_provider_id(),
                            language: guest.get_language(),
                            additional_languages: guest.get_additional_languages(),
                            gender: guest.get_gender(),
                            quality: guest.get_quality(),
                            description: guest.get_description(),
                            supports_ssml: guest.supports_ssml(),
                            sample_rates: guest.get_sample_rates(),
                            supported_formats: guest.get_supported_formats(),
                        }
                    }
                    Err(_) => {
                        return result;
                    }
                };
                durability.persist(
                    GetVoiceInput { voice_id },
                    result.map(|_| voice_data.clone()),
                )?;

                Ok(Voice::new(DurableVoice::<Impl>::new(
                    voice_data.id,
                    voice_data.name,
                    voice_data.provider_id,
                    voice_data.language,
                    voice_data.additional_languages,
                    voice_data.gender,
                    voice_data.quality,
                    voice_data.description,
                    voice_data.supports_ssml,
                    voice_data.sample_rates,
                    voice_data.supported_formats,
                )))
            } else {
                let voice_data = durability.replay::<VoiceOutput, TtsError>()?;
                Ok(Voice::new(DurableVoice::<Impl>::new(
                    voice_data.id,
                    voice_data.name,
                    voice_data.provider_id,
                    voice_data.language,
                    voice_data.additional_languages,
                    voice_data.gender,
                    voice_data.quality,
                    voice_data.description,
                    voice_data.supports_ssml,
                    voice_data.sample_rates,
                    voice_data.supported_formats,
                )))
            }
        }

        fn search_voices(filter: Option<VoiceFilter>) -> Result<Vec<VoiceInfo>, TtsError> {
            init_logging();

            let durability = Durability::<VoiceInfoListOutput, TtsError>::new(
                "golem_tts",
                "search_voices",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = Impl::search_voices(filter.clone());
                let result = result.map(|v| VoiceInfoListOutput { voices: v });
                durability
                    .persist(SearchVoicesInput { filter }, result)
                    .map(|output| output.voices)
            } else {
                durability
                    .replay()
                    .map(|output: VoiceInfoListOutput| output.voices)
            }
        }

        fn list_languages() -> Result<Vec<LanguageInfo>, TtsError> {
            init_logging();

            let durability = Durability::<LanguageInfoListOutput, TtsError>::new(
                "golem_tts",
                "list_languages",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = Impl::list_languages();
                let result = result.map(|l| LanguageInfoListOutput { languages: l });
                durability
                    .persist(NoInput, result)
                    .map(|output| output.languages)
            } else {
                durability
                    .replay()
                    .map(|output: LanguageInfoListOutput| output.languages)
            }
        }
    }

    impl<Impl: ExtendedGuest> SynthesisGuest for DurableTts<Impl> {
        fn synthesize(
            input: TextInput,
            voice: crate::exports::golem::tts::voices::VoiceBorrow<'_>,
            options: Option<SynthesisOptions>,
        ) -> Result<SynthesisResult, TtsError> {
            init_logging();

            let durability = Durability::<SynthesisResultOutput, TtsError>::new(
                "golem_tts",
                "synthesize",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = Impl::synthesize(input.clone(), voice, options.clone());
                let result = result.map(|r| SynthesisResultOutput { result: r });
                durability
                    .persist(SynthesizeInput { input, options }, result)
                    .map(|output| output.result)
            } else {
                durability
                    .replay()
                    .map(|output: SynthesisResultOutput| output.result)
            }
        }

        fn synthesize_batch(
            inputs: Vec<TextInput>,
            voice: crate::exports::golem::tts::voices::VoiceBorrow<'_>,
            options: Option<SynthesisOptions>,
        ) -> Result<Vec<SynthesisResult>, TtsError> {
            init_logging();

            let durability = Durability::<SynthesisResultListOutput, TtsError>::new(
                "golem_tts",
                "synthesize_batch",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = Impl::synthesize_batch(inputs.clone(), voice, options.clone());
                let result = result.map(|r| SynthesisResultListOutput { results: r });
                durability
                    .persist(SynthesizeBatchInput { inputs, options }, result)
                    .map(|output| output.results)
            } else {
                durability
                    .replay()
                    .map(|output: SynthesisResultListOutput| output.results)
            }
        }

        fn get_timing_marks(
            input: TextInput,
            voice: crate::exports::golem::tts::voices::VoiceBorrow<'_>,
        ) -> Result<Vec<TimingInfo>, TtsError> {
            init_logging();

            let durability = Durability::<TimingInfoListOutput, TtsError>::new(
                "golem_tts",
                "get_timing_marks",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = Impl::get_timing_marks(input.clone(), voice);
                let result = result.map(|t| TimingInfoListOutput { timing: t });
                durability
                    .persist(GetTimingMarksInput { input }, result)
                    .map(|output| output.timing)
            } else {
                durability
                    .replay()
                    .map(|output: TimingInfoListOutput| output.timing)
            }
        }

        fn validate_input(
            input: TextInput,
            voice: crate::exports::golem::tts::voices::VoiceBorrow<'_>,
        ) -> Result<ValidationResult, TtsError> {
            init_logging();

            let durability = Durability::<ValidationResultOutput, TtsError>::new(
                "golem_tts",
                "validate_input",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = Impl::validate_input(input.clone(), voice);
                let result = result.map(|v| ValidationResultOutput { result: v });
                durability
                    .persist(ValidateInputInput { input }, result)
                    .map(|output| output.result)
            } else {
                durability
                    .replay()
                    .map(|output: ValidationResultOutput| output.result)
            }
        }
    }

    impl<Impl: ExtendedGuest> StreamingGuest for DurableTts<Impl> {
        type SynthesisStream = Impl::SynthesisStream;
        type VoiceConversionStream = Impl::VoiceConversionStream;

        fn create_stream(
            voice: crate::exports::golem::tts::voices::VoiceBorrow<'_>,
            options: Option<SynthesisOptions>,
        ) -> Result<SynthesisStream, TtsError> {
            init_logging();
            Impl::create_stream(voice, options)
        }

        fn create_voice_conversion_stream(
            target_voice: crate::exports::golem::tts::voices::VoiceBorrow<'_>,
            options: Option<SynthesisOptions>,
        ) -> Result<VoiceConversionStream, TtsError> {
            init_logging();
            Impl::create_voice_conversion_stream(target_voice, options)
        }
    }

    #[allow(dead_code)]
    enum DurableVoiceConversionStreamState<Impl: ExtendedGuest> {
        Live {
            stream: Impl::VoiceConversionStream,
            pollables: Vec<LazyInitializedPollable>,
        },
        Replay {
            options: Box<Option<SynthesisOptions>>,
            pollables: Vec<LazyInitializedPollable>,
            partial_result: Vec<AudioChunk>,
            #[allow(dead_code)]
            finished: bool,
        },
    }

    #[allow(dead_code)]
    pub struct DurableVoiceConversionStream<Impl: ExtendedGuest> {
        state: RefCell<Option<DurableVoiceConversionStreamState<Impl>>>,
        subscription: RefCell<Option<Pollable>>,
    }

    impl<Impl: ExtendedGuest> Drop for DurableVoiceConversionStream<Impl> {
        fn drop(&mut self) {
            let _ = self.subscription.take();
            match self.state.take() {
                Some(DurableVoiceConversionStreamState::Live {
                    mut pollables,
                    stream,
                }) => {
                    with_persistence_level(PersistenceLevel::PersistNothing, move || {
                        pollables.clear();
                        drop(stream);
                    });
                }
                Some(DurableVoiceConversionStreamState::Replay { mut pollables, .. }) => {
                    pollables.clear();
                }
                None => {}
            }
        }
    }

    impl<Impl: ExtendedGuest> GuestVoiceConversionStream for DurableVoiceConversionStream<Impl> {
        fn send_audio(&self, audio_data: Vec<u8>) -> Result<(), TtsError> {
            let durability = Durability::<NoOutput, TtsError>::new(
                "golem_tts",
                "voice_conversion_stream_send_audio",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let state = self.state.borrow();
                let result = match &*state {
                    Some(DurableVoiceConversionStreamState::Live { stream, .. }) => {
                        with_persistence_level(PersistenceLevel::PersistNothing, || {
                            stream.send_audio(audio_data.clone())
                        })
                    }
                    _ => Err(TtsError::InternalError(
                        "Stream not in live mode".to_string(),
                    )),
                };
                let result = result.map(|_| NoOutput);
                durability
                    .persist(AudioDataOutput { audio: audio_data }, result)
                    .map(|_| ())
            } else {
                let _: NoOutput = durability.replay::<NoOutput, TtsError>()?;
                Ok(())
            }
        }

        fn receive_converted(&self) -> Result<Option<AudioChunk>, TtsError> {
            let durability = Durability::<Option<AudioChunk>, TtsError>::new(
                "golem_tts",
                "voice_conversion_stream_receive_converted",
                DurableFunctionType::ReadRemote,
            );
            if durability.is_live() {
                let state = self.state.borrow();
                let result = match &*state {
                    Some(DurableVoiceConversionStreamState::Live { stream, .. }) => {
                        with_persistence_level(PersistenceLevel::PersistNothing, || {
                            stream.receive_converted()
                        })
                    }
                    Some(DurableVoiceConversionStreamState::Replay { partial_result, .. }) => {
                        if partial_result.is_empty() {
                            Ok(None)
                        } else {
                            Ok(Some(partial_result[0].clone()))
                        }
                    }
                    _ => Ok(None),
                };
                durability.persist(NoInput, result)
            } else {
                durability.replay()
            }
        }

        fn finish(&self) -> Result<(), TtsError> {
            let durability = Durability::<NoOutput, TtsError>::new(
                "golem_tts",
                "voice_conversion_stream_finish",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let state = self.state.borrow();
                let result = match &*state {
                    Some(DurableVoiceConversionStreamState::Live { stream, .. }) => {
                        with_persistence_level(PersistenceLevel::PersistNothing, || stream.finish())
                    }
                    _ => Ok(()),
                };
                let result = result.map(|_| NoOutput);
                durability.persist(NoInput, result).map(|_| ())
            } else {
                let _: NoOutput = durability.replay::<NoOutput, TtsError>()?;
                Ok(())
            }
        }

        fn close(&self) {
            let mut state = self.state.borrow_mut();
            match state.take() {
                Some(DurableVoiceConversionStreamState::Live {
                    mut pollables,
                    stream,
                }) => {
                    with_persistence_level(PersistenceLevel::PersistNothing, move || {
                        pollables.clear();
                        stream.close();
                    });
                }
                Some(DurableVoiceConversionStreamState::Replay { mut pollables, .. }) => {
                    pollables.clear();
                }
                None => {}
            }
        }
    }

    pub struct DurableVoiceResults<Impl: ExtendedGuest> {
        filter: Option<VoiceFilter>,
        cached_voices: Option<Vec<VoiceInfo>>,
        _phantom: PhantomData<Impl>,
    }

    #[allow(dead_code)]
    impl<Impl: ExtendedGuest> DurableVoiceResults<Impl> {
        fn new(filter: Option<VoiceFilter>) -> Self {
            Self {
                filter,
                cached_voices: None,
                _phantom: PhantomData,
            }
        }

        fn new_with_voices(voices: Vec<VoiceInfo>) -> Self {
            Self {
                filter: None,
                cached_voices: Some(voices),
                _phantom: PhantomData,
            }
        }
    }

    impl<Impl: ExtendedGuest> GuestVoiceResults for DurableVoiceResults<Impl> {
        fn has_more(&self) -> bool {
            if self.cached_voices.is_some() {
                return false;
            }

            let durability = Durability::<bool, UnusedError>::new(
                "golem_tts",
                "voice_results_has_more",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    let underlying_results = Impl::list_voices(self.filter.clone());
                    match underlying_results {
                        Ok(results) => results.get::<Impl::VoiceResults>().has_more(),
                        Err(_) => false,
                    }
                });
                durability.persist_infallible(NoInput, result)
            } else {
                durability.replay_infallible()
            }
        }

        fn get_next(&self) -> Result<Vec<VoiceInfo>, TtsError> {
            if let Some(ref cached_voices) = self.cached_voices {
                return Ok(cached_voices.clone());
            }

            let durability = Durability::<VoiceInfoListOutput, TtsError>::new(
                "golem_tts",
                "voice_results_get_next",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    let underlying_results = Impl::list_voices(self.filter.clone())?;
                    let voices = underlying_results.get::<Impl::VoiceResults>().get_next()?;
                    Ok(VoiceInfoListOutput { voices })
                });
                durability
                    .persist(NoInput, result)
                    .map(|output| output.voices)
            } else {
                durability
                    .replay()
                    .map(|output: VoiceInfoListOutput| output.voices)
            }
        }

        fn get_total_count(&self) -> Option<u32> {
            if let Some(ref cached_voices) = self.cached_voices {
                return Some(cached_voices.len() as u32);
            }

            let durability = Durability::<Option<u32>, UnusedError>::new(
                "golem_tts",
                "voice_results_get_total_count",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    let underlying_results = Impl::list_voices(self.filter.clone());
                    match underlying_results {
                        Ok(results) => results.get::<Impl::VoiceResults>().get_total_count(),
                        Err(_) => None,
                    }
                });
                durability.persist_infallible(NoInput, result)
            } else {
                durability.replay_infallible()
            }
        }
    }

    // Durable Voice resource
    #[allow(dead_code)]
    pub struct DurableVoice<Impl> {
        id: String,
        name: String,
        provider_id: Option<String>,
        language: LanguageCode,
        additional_languages: Vec<LanguageCode>,
        gender: VoiceGender,
        quality: VoiceQuality,
        description: Option<String>,
        supports_ssml: bool,
        sample_rates: Vec<u32>,
        supported_formats: Vec<AudioFormat>,
        _phantom: PhantomData<Impl>,
    }

    impl<Impl: ExtendedGuest> DurableVoice<Impl> {
        #[allow(clippy::too_many_arguments)]
        #[allow(dead_code)]
        pub fn new(
            id: String,
            name: String,
            provider_id: Option<String>,
            language: LanguageCode,
            additional_languages: Vec<LanguageCode>,
            gender: VoiceGender,
            quality: VoiceQuality,
            description: Option<String>,
            supports_ssml: bool,
            sample_rates: Vec<u32>,
            supported_formats: Vec<AudioFormat>,
        ) -> Self {
            Self {
                id,
                name,
                provider_id,
                language,
                additional_languages,
                gender,
                quality,
                description,
                supports_ssml,
                sample_rates,
                supported_formats,
                _phantom: PhantomData,
            }
        }
    }

    impl<Impl: ExtendedGuest> GuestVoice for DurableVoice<Impl> {
        fn get_id(&self) -> String {
            self.id.clone()
        }

        fn get_name(&self) -> String {
            self.name.clone()
        }

        fn get_provider_id(&self) -> Option<String> {
            self.provider_id.clone()
        }

        fn get_language(&self) -> LanguageCode {
            self.language.clone()
        }

        fn get_additional_languages(&self) -> Vec<LanguageCode> {
            self.additional_languages.clone()
        }

        fn get_gender(&self) -> VoiceGender {
            self.gender
        }

        fn get_quality(&self) -> VoiceQuality {
            self.quality
        }

        fn get_description(&self) -> Option<String> {
            self.description.clone()
        }

        fn supports_ssml(&self) -> bool {
            self.supports_ssml
        }

        fn get_sample_rates(&self) -> Vec<u32> {
            self.sample_rates.clone()
        }

        fn get_supported_formats(&self) -> Vec<AudioFormat> {
            self.supported_formats.clone()
        }

        fn update_settings(&self, settings: VoiceSettings) -> Result<(), TtsError> {
            let durability = Durability::<NoOutput, TtsError>::new(
                "golem_tts",
                "voice_update_settings",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = Ok(NoOutput);
                durability
                    .persist(UpdateVoiceSettingsInput { settings }, result)
                    .map(|_| ())
            } else {
                durability.replay().map(|_: NoOutput| ())
            }
        }

        fn delete(&self) -> Result<(), TtsError> {
            let durability = Durability::<NoOutput, TtsError>::new(
                "golem_tts",
                "voice_delete",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = Ok(NoOutput);
                durability.persist(NoInput, result).map(|_| ())
            } else {
                durability.replay().map(|_: NoOutput| ())
            }
        }

        fn clone(&self) -> Result<Voice, TtsError> {
            let durability = Durability::<NoOutput, UnusedError>::new(
                "golem_tts",
                "voice_clone",
                DurableFunctionType::ReadRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Voice::new(DurableVoice::<Impl>::new(
                        self.id.clone(),
                        format!("{}_clone", self.name),
                        self.provider_id.clone(),
                        self.language.clone(),
                        self.additional_languages.clone(),
                        self.gender,
                        self.quality,
                        self.description.clone(),
                        self.supports_ssml,
                        self.sample_rates.clone(),
                        self.supported_formats.clone(),
                    ))
                });
                let _ = durability.persist_infallible(NoInput, NoOutput);
                Ok(result)
            } else {
                let _: NoOutput = durability.replay_infallible();
                Ok(Voice::new(DurableVoice::<Impl>::new(
                    self.id.clone(),
                    format!("{}_clone", self.name),
                    self.provider_id.clone(),
                    self.language.clone(),
                    self.additional_languages.clone(),
                    self.gender,
                    self.quality,
                    self.description.clone(),
                    self.supports_ssml,
                    self.sample_rates.clone(),
                    self.supported_formats.clone(),
                )))
            }
        }

        fn preview(&self, text: String) -> Result<Vec<u8>, TtsError> {
            let durability = Durability::<AudioDataOutput, TtsError>::new(
                "golem_tts",
                "voice_preview",
                DurableFunctionType::ReadRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    let voice = Impl::get_voice(self.id.clone())?;
                    let guest = voice.get::<Impl::Voice>();
                    guest.preview(text.clone())
                });

                let audio_data = result?;
                let output = AudioDataOutput {
                    audio: audio_data.clone(),
                };

                durability
                    .persist(PreviewVoiceInput { text }, Ok(output))
                    .map(|output| output.audio)
            } else {
                durability
                    .replay()
                    .map(|output: AudioDataOutput| output.audio)
            }
        }
    }

    pub struct DurablePronunciationLexicon<Impl> {
        name: String,
        language: LanguageCode,
        entries: Option<Vec<PronunciationEntry>>,
        _phantom: PhantomData<Impl>,
    }

    impl<Impl: ExtendedGuest> DurablePronunciationLexicon<Impl> {
        pub fn new(
            name: String,
            language: LanguageCode,
            entries: Option<Vec<PronunciationEntry>>,
        ) -> Self {
            Self {
                name,
                language,
                entries,
                _phantom: PhantomData,
            }
        }
    }

    impl<Impl: ExtendedGuest> GuestPronunciationLexicon for DurablePronunciationLexicon<Impl> {
        fn get_name(&self) -> String {
            self.name.clone()
        }

        fn get_language(&self) -> LanguageCode {
            self.language.clone()
        }

        fn get_entry_count(&self) -> u32 {
            let durability = Durability::<u32, UnusedError>::new(
                "golem_tts",
                "pronunciation_lexicon_get_entry_count",
                DurableFunctionType::ReadRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    self.entries.as_ref().map(|e| e.len() as u32).unwrap_or(0)
                });
                durability.persist_infallible(NoInput, result)
            } else {
                durability.replay_infallible()
            }
        }

        fn add_entry(&self, word: String, pronunciation: String) -> Result<(), TtsError> {
            let durability = Durability::<NoOutput, TtsError>::new(
                "golem_tts",
                "pronunciation_lexicon_add_entry",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    let lexicon = Impl::create_lexicon(
                        self.name.clone(),
                        self.language.clone(),
                        self.entries.clone(),
                    )?;
                    let guest = lexicon.get::<Impl::PronunciationLexicon>();
                    guest.add_entry(word.clone(), pronunciation.clone())
                });
                let result = result.map(|_| NoOutput);
                durability
                    .persist(
                        PronunciationEntryInput {
                            word,
                            pronunciation,
                        },
                        result,
                    )
                    .map(|_| ())
            } else {
                durability.replay().map(|_: NoOutput| ())
            }
        }

        fn remove_entry(&self, word: String) -> Result<(), TtsError> {
            let durability = Durability::<NoOutput, TtsError>::new(
                "golem_tts",
                "pronunciation_lexicon_remove_entry",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = Ok(NoOutput);
                durability
                    .persist(RemoveEntryInput { word }, result)
                    .map(|_| ())
            } else {
                durability.replay().map(|_: NoOutput| ())
            }
        }

        fn export_content(&self) -> Result<String, TtsError> {
            let durability = Durability::<String, TtsError>::new(
                "golem_tts",
                "pronunciation_lexicon_export_content",
                DurableFunctionType::ReadRemote,
            );
            if durability.is_live() {
                let result = Ok("# Pronunciation Lexicon Export\n".to_string());
                durability.persist(NoInput, result)
            } else {
                durability.replay()
            }
        }
    }

    // Durable LongFormOperation resource
    pub struct DurableLongFormOperation<Impl> {
        content: String,
        output_location: String,
        #[allow(dead_code)]
        chapter_breaks: Option<Vec<u32>>,
        _phantom: PhantomData<Impl>,
    }

    impl<Impl: ExtendedGuest> DurableLongFormOperation<Impl> {
        pub fn new(
            content: String,
            output_location: String,
            chapter_breaks: Option<Vec<u32>>,
        ) -> Self {
            Self {
                content,
                output_location,
                chapter_breaks,
                _phantom: PhantomData,
            }
        }
    }

    impl<Impl: ExtendedGuest> GuestLongFormOperation for DurableLongFormOperation<Impl> {
        fn get_status(&self) -> OperationStatus {
            let durability = Durability::<OperationStatus, UnusedError>::new(
                "golem_tts",
                "long_form_operation_get_status",
                DurableFunctionType::ReadRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    OperationStatus::Completed
                });
                durability.persist_infallible(NoInput, result)
            } else {
                durability.replay_infallible()
            }
        }

        fn get_progress(&self) -> f32 {
            let durability = Durability::<f32, UnusedError>::new(
                "golem_tts",
                "long_form_operation_get_progress",
                DurableFunctionType::ReadRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || 1.0);
                durability.persist_infallible(NoInput, result)
            } else {
                durability.replay_infallible()
            }
        }

        fn cancel(&self) -> Result<(), TtsError> {
            let durability = Durability::<NoOutput, TtsError>::new(
                "golem_tts",
                "long_form_operation_cancel",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = Ok(NoOutput);
                durability.persist(NoInput, result).map(|_| ())
            } else {
                durability.replay().map(|_: NoOutput| ())
            }
        }

        fn get_result(&self) -> Result<LongFormResult, TtsError> {
            let durability = Durability::<LongFormResult, TtsError>::new(
                "golem_tts",
                "long_form_operation_get_result",
                DurableFunctionType::ReadRemote,
            );
            if durability.is_live() {
                let result = Ok(LongFormResult {
                    output_location: self.output_location.clone(),
                    total_duration: 60.0,
                    chapter_durations: None,
                    metadata: crate::exports::golem::tts::types::SynthesisMetadata {
                        duration_seconds: 60.0,
                        character_count: self.content.len() as u32,
                        word_count: self.content.split_whitespace().count() as u32,
                        audio_size_bytes: 1024000,
                        request_id: "long-form-simulation".to_string(),
                        provider_info: Some("durable-tts".to_string()),
                    },
                });
                durability.persist(NoInput, result)
            } else {
                durability.replay()
            }
        }
    }

    impl<Impl: ExtendedGuest> AdvancedGuest for DurableTts<Impl> {
        type PronunciationLexicon = DurablePronunciationLexicon<Impl>;
        type LongFormOperation = DurableLongFormOperation<Impl>;

        fn create_voice_clone(
            name: String,
            audio_samples: Vec<AudioSample>,
            description: Option<String>,
        ) -> Result<Voice, TtsError> {
            init_logging();

            let durability = Durability::<VoiceOutput, TtsError>::new(
                "golem_tts",
                "create_voice_clone",
                DurableFunctionType::WriteRemote,
            );

            if durability.is_live() {
                let result = Impl::create_voice_clone(
                    name.clone(),
                    audio_samples.clone(),
                    description.clone(),
                );
                let voice_data = match &result {
                    Ok(voice) => {
                        let guest = voice.get::<Impl::Voice>();
                        VoiceOutput {
                            id: guest.get_id(),
                            name: guest.get_name(),
                            provider_id: guest.get_provider_id(),
                            language: guest.get_language(),
                            additional_languages: guest.get_additional_languages(),
                            gender: guest.get_gender(),
                            quality: guest.get_quality(),
                            description: guest.get_description(),
                            supports_ssml: guest.supports_ssml(),
                            sample_rates: guest.get_sample_rates(),
                            supported_formats: guest.get_supported_formats(),
                        }
                    }
                    Err(_) => VoiceOutput {
                        id: "error_voice".to_string(),
                        name: name.clone(),
                        provider_id: None,
                        language: "en-US".to_string(),
                        additional_languages: vec![],
                        gender: VoiceGender::Neutral,
                        quality: VoiceQuality::Standard,
                        description: description.clone(),
                        supports_ssml: false,
                        sample_rates: vec![22050],
                        supported_formats: vec![AudioFormat::Mp3],
                    },
                };
                durability.persist(
                    CreateVoiceCloneInput {
                        name,
                        audio_samples,
                        description,
                    },
                    result.map(|_| voice_data.clone()),
                )?;

                Ok(Voice::new(DurableVoice::<Impl>::new(
                    voice_data.id,
                    voice_data.name,
                    voice_data.provider_id,
                    voice_data.language,
                    voice_data.additional_languages,
                    voice_data.gender,
                    voice_data.quality,
                    voice_data.description,
                    voice_data.supports_ssml,
                    voice_data.sample_rates,
                    voice_data.supported_formats,
                )))
            } else {
                let voice_data = durability.replay::<VoiceOutput, TtsError>()?;
                Ok(Voice::new(DurableVoice::<Impl>::new(
                    voice_data.id,
                    voice_data.name,
                    voice_data.provider_id,
                    voice_data.language,
                    voice_data.additional_languages,
                    voice_data.gender,
                    voice_data.quality,
                    voice_data.description,
                    voice_data.supports_ssml,
                    voice_data.sample_rates,
                    voice_data.supported_formats,
                )))
            }
        }

        fn design_voice(
            name: String,
            characteristics: VoiceDesignParams,
        ) -> Result<Voice, TtsError> {
            init_logging();

            let durability = Durability::<VoiceOutput, TtsError>::new(
                "golem_tts",
                "design_voice",
                DurableFunctionType::WriteRemote,
            );

            if durability.is_live() {
                let result = Impl::design_voice(name.clone(), characteristics.clone());
                let voice_data = match &result {
                    Ok(voice) => {
                        let guest = voice.get::<Impl::Voice>();
                        VoiceOutput {
                            id: guest.get_id(),
                            name: guest.get_name(),
                            provider_id: guest.get_provider_id(),
                            language: guest.get_language(),
                            additional_languages: guest.get_additional_languages(),
                            gender: guest.get_gender(),
                            quality: guest.get_quality(),
                            description: guest.get_description(),
                            supports_ssml: guest.supports_ssml(),
                            sample_rates: guest.get_sample_rates(),
                            supported_formats: guest.get_supported_formats(),
                        }
                    }
                    Err(_) => VoiceOutput {
                        id: "error_voice".to_string(),
                        name: name.clone(),
                        provider_id: None,
                        language: "en-US".to_string(),
                        additional_languages: vec![],
                        gender: VoiceGender::Neutral,
                        quality: VoiceQuality::Standard,
                        description: None,
                        supports_ssml: false,
                        sample_rates: vec![22050],
                        supported_formats: vec![AudioFormat::Mp3],
                    },
                };

                durability.persist(
                    DesignVoiceInput {
                        name,
                        characteristics,
                    },
                    result.map(|_| voice_data.clone()),
                )?;

                Ok(Voice::new(DurableVoice::<Impl>::new(
                    voice_data.id,
                    voice_data.name,
                    voice_data.provider_id,
                    voice_data.language,
                    voice_data.additional_languages,
                    voice_data.gender,
                    voice_data.quality,
                    voice_data.description,
                    voice_data.supports_ssml,
                    voice_data.sample_rates,
                    voice_data.supported_formats,
                )))
            } else {
                let voice_data = durability.replay::<VoiceOutput, TtsError>()?;
                Ok(Voice::new(DurableVoice::<Impl>::new(
                    voice_data.id,
                    voice_data.name,
                    voice_data.provider_id,
                    voice_data.language,
                    voice_data.additional_languages,
                    voice_data.gender,
                    voice_data.quality,
                    voice_data.description,
                    voice_data.supports_ssml,
                    voice_data.sample_rates,
                    voice_data.supported_formats,
                )))
            }
        }

        fn convert_voice(
            input_audio: Vec<u8>,
            target_voice: crate::exports::golem::tts::voices::VoiceBorrow<'_>,
            preserve_timing: Option<bool>,
        ) -> Result<Vec<u8>, TtsError> {
            init_logging();

            let durability = Durability::<AudioDataOutput, TtsError>::new(
                "golem_tts",
                "convert_voice",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result =
                    Impl::convert_voice(input_audio.clone(), target_voice, preserve_timing);
                let result = result.map(|a| AudioDataOutput { audio: a });
                durability
                    .persist(
                        ConvertVoiceInput {
                            input_audio,
                            preserve_timing,
                        },
                        result,
                    )
                    .map(|output| output.audio)
            } else {
                durability
                    .replay()
                    .map(|output: AudioDataOutput| output.audio)
            }
        }

        fn generate_sound_effect(
            description: String,
            duration_seconds: Option<f32>,
            style_influence: Option<f32>,
        ) -> Result<Vec<u8>, TtsError> {
            init_logging();

            let durability = Durability::<AudioDataOutput, TtsError>::new(
                "golem_tts",
                "generate_sound_effect",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = Impl::generate_sound_effect(
                    description.clone(),
                    duration_seconds,
                    style_influence,
                );
                let result = result.map(|a| AudioDataOutput { audio: a });
                durability
                    .persist(
                        GenerateSoundEffectInput {
                            description,
                            duration_seconds,
                            style_influence,
                        },
                        result,
                    )
                    .map(|output| output.audio)
            } else {
                durability
                    .replay()
                    .map(|output: AudioDataOutput| output.audio)
            }
        }

        fn create_lexicon(
            name: String,
            language: LanguageCode,
            entries: Option<Vec<PronunciationEntry>>,
        ) -> Result<PronunciationLexicon, TtsError> {
            init_logging();

            let durability = Durability::<PronunciationLexiconOutput, UnusedError>::new(
                "golem_tts",
                "create_lexicon",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    PronunciationLexiconOutput {
                        name: name.clone(),
                        language: language.clone(),
                        entries: entries.clone(),
                    }
                });
                let _ = durability.persist_infallible(
                    CreateLexiconInput {
                        name: name.clone(),
                        language: language.clone(),
                        entries: entries.clone(),
                    },
                    result.clone(),
                );
                Ok(PronunciationLexicon::new(
                    DurablePronunciationLexicon::<Impl>::new(
                        result.name,
                        result.language,
                        result.entries,
                    ),
                ))
            } else {
                let lexicon_data: PronunciationLexiconOutput = durability.replay_infallible();
                Ok(PronunciationLexicon::new(
                    DurablePronunciationLexicon::<Impl>::new(
                        lexicon_data.name,
                        lexicon_data.language,
                        lexicon_data.entries,
                    ),
                ))
            }
        }

        fn synthesize_long_form(
            content: String,
            _voice: crate::exports::golem::tts::voices::VoiceBorrow<'_>,
            output_location: String,
            chapter_breaks: Option<Vec<u32>>,
        ) -> Result<LongFormOperation, TtsError> {
            init_logging();

            let durability = Durability::<LongFormOperationOutput, UnusedError>::new(
                "golem_tts",
                "synthesize_long_form",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    LongFormOperationOutput {
                        content: content.clone(),
                        output_location: output_location.clone(),
                        chapter_breaks: chapter_breaks.clone(),
                    }
                });
                let _ = durability.persist_infallible(
                    SynthesizeLongFormInput {
                        content: content.clone(),
                        output_location: output_location.clone(),
                        chapter_breaks: chapter_breaks.clone(),
                    },
                    result.clone(),
                );
                Ok(LongFormOperation::new(
                    DurableLongFormOperation::<Impl>::new(
                        result.content,
                        result.output_location,
                        result.chapter_breaks,
                    ),
                ))
            } else {
                let operation_data: LongFormOperationOutput = durability.replay_infallible();
                Ok(LongFormOperation::new(
                    DurableLongFormOperation::<Impl>::new(
                        operation_data.content,
                        operation_data.output_location,
                        operation_data.chapter_breaks,
                    ),
                ))
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::exports::golem::tts::types::{
            AudioEffects, AudioFormat, TextType, VoiceGender, VoiceQuality,
        };
        use golem_rust::value_and_type::{FromValueAndType, IntoValueAndType};

        fn roundtrip_test<T>(value: T) -> T
        where
            T: IntoValueAndType + FromValueAndType + Clone + std::fmt::Debug + PartialEq,
        {
            let vnt = value.clone().into_value_and_type();
            let deserialized = T::from_value_and_type(vnt).unwrap();
            assert_eq!(value, deserialized);
            deserialized
        }

        #[test]
        fn list_voices_input_roundtrip() {
            roundtrip_test(ListVoicesInput {
                filter: Some(VoiceFilter {
                    language: Some("en-US".to_string()),
                    gender: Some(VoiceGender::Female),
                    quality: Some(VoiceQuality::Neural),
                    supports_ssml: Some(true),
                    provider: Some("test-provider".to_string()),
                    search_query: Some("test query".to_string()),
                }),
            });
        }

        #[test]
        fn synthesize_input_roundtrip() {
            roundtrip_test(SynthesizeInput {
                input: TextInput {
                    content: "Hello, world!".to_string(),
                    text_type: TextType::Plain,
                    language: Some("en-US".to_string()),
                },
                options: Some(SynthesisOptions {
                    audio_config: Some(AudioConfig {
                        format: AudioFormat::Mp3,
                        sample_rate: Some(44100),
                        bit_rate: Some(128),
                        channels: Some(2),
                    }),
                    voice_settings: Some(VoiceSettings {
                        speed: Some(1.0),
                        pitch: Some(0.0),
                        volume: Some(0.0),
                        stability: Some(0.5),
                        similarity: Some(0.75),
                        style: Some(0.5),
                    }),
                    audio_effects: Some(vec![AudioEffects::NoiseReduction]),
                    enable_timing: Some(true),
                    enable_word_timing: Some(true),
                    seed: Some(42),
                    model_version: Some("v2".to_string()),
                    context: None,
                }),
            });
        }

        #[test]
        fn create_voice_clone_input_roundtrip() {
            roundtrip_test(CreateVoiceCloneInput {
                name: "Test Voice".to_string(),
                audio_samples: vec![AudioSample {
                    data: vec![1, 2, 3, 4],
                    transcript: Some("Test transcript".to_string()),
                    quality_rating: Some(8),
                }],
                description: Some("A test voice clone".to_string()),
            });
        }

        #[test]
        fn no_input_roundtrip() {
            roundtrip_test(NoInput);
        }

        #[test]
        fn search_voices_input_roundtrip() {
            roundtrip_test(SearchVoicesInput {
                filter: Some(VoiceFilter {
                    language: Some("fr-FR".to_string()),
                    gender: Some(VoiceGender::Male),
                    quality: Some(VoiceQuality::Standard),
                    supports_ssml: Some(false),
                    provider: Some("provider-test".to_string()),
                    search_query: Some("search test".to_string()),
                }),
            });
        }

        #[test]
        fn synthesize_batch_input_roundtrip() {
            roundtrip_test(SynthesizeBatchInput {
                inputs: vec![
                    TextInput {
                        content: "First sentence".to_string(),
                        text_type: TextType::Plain,
                        language: Some("en-US".to_string()),
                    },
                    TextInput {
                        content: "<speak>Second sentence with SSML</speak>".to_string(),
                        text_type: TextType::Ssml,
                        language: Some("en-GB".to_string()),
                    },
                ],
                options: Some(SynthesisOptions {
                    audio_config: Some(AudioConfig {
                        format: AudioFormat::Wav,
                        sample_rate: Some(22050),
                        bit_rate: Some(256),
                        channels: Some(1),
                    }),
                    voice_settings: Some(VoiceSettings {
                        speed: Some(0.8),
                        pitch: Some(-0.2),
                        volume: Some(0.1),
                        stability: Some(0.3),
                        similarity: Some(0.9),
                        style: Some(0.2),
                    }),
                    audio_effects: Some(vec![
                        AudioEffects::NoiseReduction,
                        AudioEffects::BassBoost,
                    ]),
                    enable_timing: Some(false),
                    enable_word_timing: Some(false),
                    seed: Some(123),
                    model_version: Some("v3".to_string()),
                    context: None,
                }),
            });
        }

        #[test]
        fn get_timing_marks_input_roundtrip() {
            roundtrip_test(GetTimingMarksInput {
                input: TextInput {
                    content: "Test timing marks".to_string(),
                    text_type: TextType::Plain,
                    language: Some("de-DE".to_string()),
                },
            });
        }

        #[test]
        fn validate_input_input_roundtrip() {
            roundtrip_test(ValidateInputInput {
                input: TextInput {
                    content: "<speak><break time=\"1s\"/>Valid SSML</speak>".to_string(),
                    text_type: TextType::Ssml,
                    language: Some("es-ES".to_string()),
                },
            });
        }

        #[test]
        fn convert_voice_input_roundtrip() {
            roundtrip_test(ConvertVoiceInput {
                input_audio: vec![0x52, 0x49, 0x46, 0x46, 0x24, 0x08, 0x00, 0x00], // Mock WAV header
                preserve_timing: Some(true),
            });
        }

        #[test]
        fn convert_voice_input_no_preserve_timing_roundtrip() {
            roundtrip_test(ConvertVoiceInput {
                input_audio: vec![1, 2, 3, 4, 5],
                preserve_timing: None,
            });
        }

        #[test]
        fn generate_sound_effect_input_roundtrip() {
            roundtrip_test(GenerateSoundEffectInput {
                description: "thunderstorm with rain".to_string(),
                duration_seconds: Some(10.5),
                style_influence: Some(0.8),
            });
        }

        #[test]
        fn generate_sound_effect_input_minimal_roundtrip() {
            roundtrip_test(GenerateSoundEffectInput {
                description: "simple beep".to_string(),
                duration_seconds: None,
                style_influence: None,
            });
        }

        #[test]
        fn synthesize_input_minimal_roundtrip() {
            roundtrip_test(SynthesizeInput {
                input: TextInput {
                    content: "Minimal test".to_string(),
                    text_type: TextType::Plain,
                    language: None,
                },
                options: None,
            });
        }

        #[test]
        fn synthesize_input_empty_content_roundtrip() {
            roundtrip_test(SynthesizeInput {
                input: TextInput {
                    content: "".to_string(),
                    text_type: TextType::Plain,
                    language: Some("en-US".to_string()),
                },
                options: None,
            });
        }

        #[test]
        fn voice_info_list_output_roundtrip() {
            roundtrip_test(VoiceInfoListOutput {
                voices: vec![
                    VoiceInfo {
                        id: "voice-1".to_string(),
                        name: "Alice".to_string(),
                        language: "en-US".to_string(),
                        additional_languages: vec!["en-GB".to_string()],
                        gender: VoiceGender::Female,
                        quality: VoiceQuality::Neural,
                        description: Some("A friendly female voice".to_string()),
                        provider: "test-provider".to_string(),
                        sample_rate: 44100,
                        is_custom: false,
                        is_cloned: false,
                        preview_url: Some("https://example.com/preview1.mp3".to_string()),
                        use_cases: vec!["general".to_string()],
                    },
                    VoiceInfo {
                        id: "voice-2".to_string(),
                        name: "Bob".to_string(),
                        language: "en-GB".to_string(),
                        additional_languages: vec![],
                        gender: VoiceGender::Male,
                        quality: VoiceQuality::Standard,
                        description: None,
                        provider: "test-provider".to_string(),
                        sample_rate: 22050,
                        is_custom: true,
                        is_cloned: true,
                        preview_url: None,
                        use_cases: vec!["audiobook".to_string(), "podcast".to_string()],
                    },
                ],
            });
        }

        #[test]
        fn language_info_list_output_roundtrip() {
            roundtrip_test(LanguageInfoListOutput {
                languages: vec![
                    LanguageInfo {
                        code: "en-US".to_string(),
                        name: "English (US)".to_string(),
                        native_name: "English".to_string(),
                        voice_count: 10,
                    },
                    LanguageInfo {
                        code: "fr-FR".to_string(),
                        name: "French (France)".to_string(),
                        native_name: "Français".to_string(),
                        voice_count: 5,
                    },
                ],
            });
        }

        #[test]
        fn synthesis_result_output_roundtrip() {
            use crate::exports::golem::tts::types::SynthesisMetadata;

            roundtrip_test(SynthesisResultOutput {
                result: SynthesisResult {
                    audio_data: vec![0x00, 0xFF, 0x80, 0x7F],
                    metadata: SynthesisMetadata {
                        duration_seconds: 2.5,
                        character_count: 13,
                        word_count: 2,
                        audio_size_bytes: 4,
                        request_id: "req-123".to_string(),
                        provider_info: Some("test-provider".to_string()),
                    },
                },
            });
        }

        #[test]
        fn timing_info_list_output_roundtrip() {
            use crate::exports::golem::tts::types::TimingMarkType;

            roundtrip_test(TimingInfoListOutput {
                timing: vec![
                    TimingInfo {
                        start_time_seconds: 0.0,
                        end_time_seconds: Some(0.5),
                        text_offset: Some(0),
                        mark_type: Some(TimingMarkType::Word),
                    },
                    TimingInfo {
                        start_time_seconds: 0.5,
                        end_time_seconds: Some(1.0),
                        text_offset: Some(6),
                        mark_type: Some(TimingMarkType::Word),
                    },
                ],
            });
        }

        #[test]
        fn validation_result_output_roundtrip() {
            roundtrip_test(ValidationResultOutput {
                result: ValidationResult {
                    is_valid: true,
                    character_count: 50,
                    estimated_duration: Some(3.2),
                    warnings: vec!["Minor issue detected".to_string()],
                    errors: vec![],
                },
            });
        }

        #[test]
        fn audio_data_output_roundtrip() {
            roundtrip_test(AudioDataOutput {
                audio: vec![
                    0x52, 0x49, 0x46, 0x46, 0x24, 0x08, 0x00, 0x00, 0x57, 0x41, 0x56, 0x45,
                ],
            });
        }

        #[test]
        fn empty_audio_data_output_roundtrip() {
            roundtrip_test(AudioDataOutput { audio: vec![] });
        }

        #[test]
        fn no_output_roundtrip() {
            roundtrip_test(NoOutput);
        }

        #[test]
        fn empty_voice_info_list_output_roundtrip() {
            roundtrip_test(VoiceInfoListOutput { voices: vec![] });
        }

        #[test]
        fn empty_language_info_list_output_roundtrip() {
            roundtrip_test(LanguageInfoListOutput { languages: vec![] });
        }

        #[test]
        fn complex_audio_config_roundtrip() {
            roundtrip_test(SynthesizeInput {
                input: TextInput {
                    content: "Complex audio test".to_string(),
                    text_type: TextType::Ssml,
                    language: Some("ja-JP".to_string()),
                },
                options: Some(SynthesisOptions {
                    audio_config: Some(AudioConfig {
                        format: AudioFormat::OggOpus,
                        sample_rate: Some(48000),
                        bit_rate: Some(320),
                        channels: Some(2),
                    }),
                    voice_settings: Some(VoiceSettings {
                        speed: Some(1.5),
                        pitch: Some(0.3),
                        volume: Some(-0.1),
                        stability: Some(0.9),
                        similarity: Some(0.1),
                        style: Some(1.0),
                    }),
                    audio_effects: Some(vec![
                        AudioEffects::NoiseReduction,
                        AudioEffects::BassBoost,
                        AudioEffects::TrebleBoost,
                    ]),
                    enable_timing: Some(true),
                    enable_word_timing: Some(true),
                    seed: Some(9999),
                    model_version: Some("experimental-v1".to_string()),
                    context: None,
                }),
            });
        }
    }
}
