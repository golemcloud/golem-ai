#[allow(unused_imports)]
use crate::model::advanced::{
    AudioSample, LongFormOperation, LongFormResult, OperationStatus, PronunciationEntry,
    PronunciationLexicon, VoiceDesignParams,
};
#[allow(unused_imports)]
use crate::model::synthesis::{SynthesisOptions, ValidationResult};
#[allow(unused_imports)]
use crate::model::types::{
    AudioChunk, AudioConfig, AudioEffects, AudioFormat, LanguageCode, SynthesisResult, TextInput,
    TimingInfo, TtsError, VoiceGender, VoiceQuality, VoiceSettings,
};
#[allow(unused_imports)]
use crate::model::voices::{LanguageInfo, Voice, VoiceFilter, VoiceInfo, VoiceResults};
use crate::{AdvancedTtsProvider, StreamingVoiceProvider, SynthesizeProvider, VoiceProvider};
use std::marker::PhantomData;

pub struct DurableTts<Impl> {
    phantom: PhantomData<Impl>,
}
/// Provider trait used by `DurableTts<Impl>` to implement durability.
///
/// All four sub-traits (`VoiceProvider`, `StreamingVoiceProvider`,
/// `SynthesizeProvider`, `AdvancedTtsProvider`) must agree on the same
/// `ProviderConfig` type so that the durable wrapper can thread a single
/// `provider_config` value through every method on every trait.
pub trait ExtendedTtsProvider:
    VoiceProvider
    + StreamingVoiceProvider<ProviderConfig = <Self as VoiceProvider>::ProviderConfig>
    + SynthesizeProvider<ProviderConfig = <Self as VoiceProvider>::ProviderConfig>
    + AdvancedTtsProvider<ProviderConfig = <Self as VoiceProvider>::ProviderConfig>
    + 'static
{
}

/// When the durability feature flag is off, `DurableTts<Impl>` is a transparent wrapper that
/// forwards every call to the inner provider without any oplog persistence.
#[cfg(not(feature = "golem"))]
mod passthrough_impl {
    use crate::durability::{DurableTts, ExtendedTtsProvider};
    use crate::init_logging;
    use crate::model::advanced::{
        AudioSample, LongFormOperation, PronunciationEntry, PronunciationLexicon, VoiceDesignParams,
    };
    use crate::model::streaming::{SynthesisStream, VoiceConversionStream};
    use crate::model::synthesis::{SynthesisOptions, ValidationResult};
    use crate::model::types::{LanguageCode, SynthesisResult, TextInput, TimingInfo, TtsError};
    use crate::model::voices::{LanguageInfo, Voice, VoiceFilter, VoiceInfo, VoiceResults};
    use crate::{AdvancedTtsProvider, StreamingVoiceProvider, SynthesizeProvider, VoiceProvider};

    impl<Impl: ExtendedTtsProvider> VoiceProvider for DurableTts<Impl> {
        type Voice = Impl::Voice;
        type VoiceResults = Impl::VoiceResults;
        type ProviderConfig = <Impl as VoiceProvider>::ProviderConfig;

        async fn list_voices(
            provider_config: Self::ProviderConfig,
            filter: Option<VoiceFilter>,
        ) -> Result<VoiceResults, TtsError> {
            init_logging();
            Impl::list_voices(provider_config, filter).await
        }

        async fn get_voice(
            provider_config: Self::ProviderConfig,
            voice_id: String,
        ) -> Result<Voice, TtsError> {
            init_logging();
            Impl::get_voice(provider_config, voice_id).await
        }

        async fn search_voices(
            provider_config: Self::ProviderConfig,
            filter: Option<VoiceFilter>,
        ) -> Result<Vec<VoiceInfo>, TtsError> {
            init_logging();
            Impl::search_voices(provider_config, filter).await
        }

        async fn list_languages(
            provider_config: Self::ProviderConfig,
        ) -> Result<Vec<LanguageInfo>, TtsError> {
            init_logging();
            Impl::list_languages(provider_config).await
        }
    }

    impl<Impl: ExtendedTtsProvider> SynthesizeProvider for DurableTts<Impl> {
        type ProviderConfig = <Impl as VoiceProvider>::ProviderConfig;

        async fn synthesize(
            provider_config: Self::ProviderConfig,
            input: TextInput,
            voice: crate::model::voices::VoiceBorrow<'_>,
            options: Option<SynthesisOptions>,
        ) -> Result<SynthesisResult, TtsError> {
            init_logging();
            Impl::synthesize(provider_config, input, voice, options).await
        }

        async fn synthesize_batch(
            provider_config: Self::ProviderConfig,
            inputs: Vec<TextInput>,
            voice: crate::model::voices::VoiceBorrow<'_>,
            options: Option<SynthesisOptions>,
        ) -> Result<Vec<SynthesisResult>, TtsError> {
            init_logging();
            Impl::synthesize_batch(provider_config, inputs, voice, options).await
        }

        async fn get_timing_marks(
            provider_config: Self::ProviderConfig,
            input: TextInput,
            voice: crate::model::voices::VoiceBorrow<'_>,
        ) -> Result<Vec<TimingInfo>, TtsError> {
            init_logging();
            Impl::get_timing_marks(provider_config, input, voice).await
        }

        async fn validate_input(
            provider_config: Self::ProviderConfig,
            input: TextInput,
            voice: crate::model::voices::VoiceBorrow<'_>,
        ) -> Result<ValidationResult, TtsError> {
            init_logging();
            Impl::validate_input(provider_config, input, voice).await
        }
    }

    impl<Impl: ExtendedTtsProvider> StreamingVoiceProvider for DurableTts<Impl> {
        type SynthesisStream = Impl::SynthesisStream;
        type VoiceConversionStream = Impl::VoiceConversionStream;
        type ProviderConfig = <Impl as VoiceProvider>::ProviderConfig;

        async fn create_stream(
            provider_config: Self::ProviderConfig,
            voice: crate::model::voices::VoiceBorrow<'_>,
            options: Option<SynthesisOptions>,
        ) -> Result<SynthesisStream, TtsError> {
            init_logging();
            Impl::create_stream(provider_config, voice, options).await
        }

        async fn create_voice_conversion_stream(
            provider_config: Self::ProviderConfig,
            target_voice: crate::model::voices::VoiceBorrow<'_>,
            options: Option<SynthesisOptions>,
        ) -> Result<VoiceConversionStream, TtsError> {
            init_logging();
            Impl::create_voice_conversion_stream(provider_config, target_voice, options).await
        }
    }

    impl<Impl: ExtendedTtsProvider> AdvancedTtsProvider for DurableTts<Impl> {
        type PronunciationLexicon = Impl::PronunciationLexicon;
        type LongFormOperation = Impl::LongFormOperation;
        type ProviderConfig = <Impl as VoiceProvider>::ProviderConfig;

        async fn create_voice_clone(
            provider_config: Self::ProviderConfig,
            name: String,
            audio_samples: Vec<AudioSample>,
            description: Option<String>,
        ) -> Result<Voice, TtsError> {
            init_logging();
            Impl::create_voice_clone(provider_config, name, audio_samples, description).await
        }

        async fn design_voice(
            provider_config: Self::ProviderConfig,
            name: String,
            characteristics: VoiceDesignParams,
        ) -> Result<Voice, TtsError> {
            init_logging();
            Impl::design_voice(provider_config, name, characteristics).await
        }

        async fn convert_voice(
            provider_config: Self::ProviderConfig,
            input_audio: Vec<u8>,
            target_voice: crate::model::voices::VoiceBorrow<'_>,
            preserve_timing: Option<bool>,
        ) -> Result<Vec<u8>, TtsError> {
            init_logging();
            Impl::convert_voice(provider_config, input_audio, target_voice, preserve_timing).await
        }

        async fn generate_sound_effect(
            provider_config: Self::ProviderConfig,
            description: String,
            duration_seconds: Option<f32>,
            style_influence: Option<f32>,
        ) -> Result<Vec<u8>, TtsError> {
            init_logging();
            Impl::generate_sound_effect(
                provider_config,
                description,
                duration_seconds,
                style_influence,
            )
            .await
        }

        async fn create_lexicon(
            provider_config: Self::ProviderConfig,
            name: String,
            language: LanguageCode,
            entries: Option<Vec<PronunciationEntry>>,
        ) -> Result<PronunciationLexicon, TtsError> {
            init_logging();
            Impl::create_lexicon(provider_config, name, language, entries).await
        }

        async fn synthesize_long_form(
            provider_config: Self::ProviderConfig,
            content: String,
            voice: crate::model::voices::VoiceBorrow<'_>,
            output_location: String,
            chapter_breaks: Option<Vec<u32>>,
        ) -> Result<LongFormOperation, TtsError> {
            init_logging();
            Impl::synthesize_long_form(
                provider_config,
                content,
                voice,
                output_location,
                chapter_breaks,
            )
            .await
        }
    }
}

/// When the durability feature flag is on, wrapping with `DurableTts` adds custom durability
/// on top of the provider-specific TTS implementation using Golem's special host functions and
/// the `golem-rust` helper library.
#[cfg(feature = "golem")]
mod durable_impl {
    use crate::durability::{DurableTts, ExtendedTtsProvider};

    #[allow(unused_imports)]
    use crate::model::advanced::{
        AudioSample, LongFormOperation, LongFormResult, OperationStatus, PronunciationEntry,
        PronunciationLexicon, VoiceDesignParams,
    };
    #[allow(unused_imports)]
    use crate::model::streaming::{StreamStatus, SynthesisStream, VoiceConversionStream};

    use crate::model::synthesis::{SynthesisOptions, ValidationResult};
    #[allow(unused_imports)]
    use crate::model::types::{
        AudioChunk, AudioConfig, AudioEffects, AudioFormat, LanguageCode, SynthesisResult,
        TextInput, TimingInfo, TtsError, VoiceGender, VoiceQuality, VoiceSettings,
    };

    use crate::model::voices::{LanguageInfo, Voice, VoiceFilter, VoiceInfo, VoiceResults};
    use crate::{
        init_logging, AdvancedTtsProvider, LongFormOperationInterface,
        PronunciationLexiconInterface, StreamingVoiceProvider, SynthesizeProvider, TtsFuture,
        VoiceInterface, VoiceProvider, VoiceResultsInterface,
    };
    use golem_rust::durability::{Durability, DurableFunctionType};
    use golem_rust::{FromSchema, IntoSchema};
    use std::fmt::{Display, Formatter};
    use std::marker::PhantomData;

    #[allow(dead_code)]
    #[derive(Debug, Clone, PartialEq, IntoSchema, FromSchema)]
    struct ListVoicesInput {
        filter: Option<VoiceFilter>,
    }

    #[allow(dead_code)]
    #[derive(Debug, Clone, PartialEq, IntoSchema, FromSchema)]
    struct GetVoiceInput {
        voice_id: String,
    }

    #[derive(Debug, Clone, PartialEq, IntoSchema, FromSchema)]
    struct SearchVoicesInput {
        filter: Option<VoiceFilter>,
    }

    #[derive(Debug, Clone, PartialEq, IntoSchema, FromSchema)]
    struct SynthesizeInput {
        input: TextInput,
        options: Option<SynthesisOptions>,
    }

    #[derive(Debug, Clone, PartialEq, IntoSchema, FromSchema)]
    struct SynthesizeBatchInput {
        inputs: Vec<TextInput>,
        options: Option<SynthesisOptions>,
    }

    #[derive(Debug, Clone, PartialEq, IntoSchema, FromSchema)]
    struct GetTimingMarksInput {
        input: TextInput,
    }

    #[derive(Debug, Clone, PartialEq, IntoSchema, FromSchema)]
    struct ValidateInputInput {
        input: TextInput,
    }

    #[allow(dead_code)]
    #[derive(Debug, Clone, PartialEq, IntoSchema, FromSchema)]
    struct CreateStreamInput {
        options: Option<SynthesisOptions>,
    }

    #[allow(dead_code)]
    #[derive(Debug, Clone, PartialEq, IntoSchema, FromSchema)]
    struct CreateVoiceConversionStreamInput {
        options: Option<SynthesisOptions>,
    }

    #[allow(dead_code)]
    #[derive(Debug, Clone, PartialEq, IntoSchema, FromSchema)]
    struct CreateVoiceCloneInput {
        name: String,
        audio_samples: Vec<AudioSample>,
        description: Option<String>,
    }

    #[allow(dead_code)]
    #[derive(Debug, Clone, PartialEq, IntoSchema, FromSchema)]
    struct DesignVoiceInput {
        name: String,
        characteristics: VoiceDesignParams,
    }

    #[derive(Debug, Clone, PartialEq, IntoSchema, FromSchema)]
    struct ConvertVoiceInput {
        input_audio: Vec<u8>,
        preserve_timing: Option<bool>,
    }

    #[derive(Debug, Clone, PartialEq, IntoSchema, FromSchema)]
    struct GenerateSoundEffectInput {
        description: String,
        duration_seconds: Option<f32>,
        style_influence: Option<f32>,
    }

    #[allow(dead_code)]
    #[derive(Debug, Clone, PartialEq, IntoSchema, FromSchema)]
    struct CreateLexiconInput {
        name: String,
        language: LanguageCode,
        entries: Option<Vec<PronunciationEntry>>,
    }

    #[allow(dead_code)]
    #[derive(Debug, Clone, PartialEq, IntoSchema, FromSchema)]
    struct SynthesizeLongFormInput {
        content: String,
        output_location: String,
        chapter_breaks: Option<Vec<u32>>,
    }

    #[derive(Debug, Clone, PartialEq, IntoSchema, FromSchema)]
    struct NoInput;

    #[derive(Debug, Clone, PartialEq, FromSchema, IntoSchema)]
    struct NoOutput;

    #[derive(Debug, Clone, PartialEq, FromSchema, IntoSchema)]
    struct PronunciationEntryInput {
        word: String,
        pronunciation: String,
    }

    #[derive(Debug, Clone, PartialEq, FromSchema, IntoSchema)]
    struct RemoveEntryInput {
        word: String,
    }

    #[allow(dead_code)]
    #[derive(Debug, Clone, PartialEq, FromSchema, IntoSchema)]
    struct PronunciationEntryListOutput {
        entries: Vec<PronunciationEntry>,
    }

    #[allow(dead_code)]
    #[derive(Debug, Clone, PartialEq, FromSchema, IntoSchema)]
    struct LongFormResultOutput {
        result: String,
    }

    #[allow(dead_code)]
    #[derive(Debug, Clone, PartialEq, FromSchema, IntoSchema)]
    struct UpdateVoiceSettingsInput {
        settings: VoiceSettings,
    }

    #[allow(dead_code)]
    #[derive(Debug, Clone, PartialEq, FromSchema, IntoSchema)]
    struct PreviewVoiceInput {
        text: String,
    }

    #[derive(Debug, FromSchema, IntoSchema)]
    struct UnusedError;

    impl Display for UnusedError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "UnusedError")
        }
    }

    #[derive(Debug, Clone, PartialEq, FromSchema, IntoSchema)]
    struct VoiceInfoListOutput {
        voices: Vec<VoiceInfo>,
    }

    #[derive(Debug, Clone, PartialEq, FromSchema, IntoSchema)]
    struct LanguageInfoListOutput {
        languages: Vec<LanguageInfo>,
    }

    #[derive(Debug, Clone, PartialEq, FromSchema, IntoSchema)]
    struct SynthesisResultOutput {
        result: SynthesisResult,
    }

    #[derive(Debug, Clone, PartialEq, FromSchema, IntoSchema)]
    struct SynthesisResultListOutput {
        results: Vec<SynthesisResult>,
    }

    #[derive(Debug, Clone, PartialEq, FromSchema, IntoSchema)]
    struct TimingInfoListOutput {
        timing: Vec<TimingInfo>,
    }

    #[derive(Debug, Clone, PartialEq, FromSchema, IntoSchema)]
    struct ValidationResultOutput {
        result: ValidationResult,
    }

    #[derive(Debug, Clone, PartialEq, FromSchema, IntoSchema)]
    struct AudioDataOutput {
        audio: Vec<u8>,
    }

    #[derive(Debug, Clone, PartialEq, FromSchema, IntoSchema)]
    struct VoiceResultsOutput {
        voices: Vec<VoiceInfo>,
    }

    #[derive(Debug, Clone, PartialEq, FromSchema, IntoSchema)]
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

    #[derive(Debug, Clone, PartialEq, FromSchema, IntoSchema)]
    struct PronunciationLexiconOutput {
        name: String,
        language: LanguageCode,
        entries: Option<Vec<PronunciationEntry>>,
    }

    #[derive(Debug, Clone, PartialEq, FromSchema, IntoSchema)]
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

    impl<Impl: ExtendedTtsProvider> VoiceProvider for DurableTts<Impl> {
        type Voice = Impl::Voice;
        type VoiceResults = Impl::VoiceResults;
        type ProviderConfig = <Impl as VoiceProvider>::ProviderConfig;

        async fn list_voices(
            provider_config: Self::ProviderConfig,
            filter: Option<VoiceFilter>,
        ) -> Result<VoiceResults, TtsError> {
            init_logging();

            let durability = Durability::<VoiceResultsOutput, TtsError>::new(
                "golem_ai_tts",
                "list_voices",
                DurableFunctionType::WriteRemote,
                &ListVoicesInput {
                    filter: filter.clone(),
                },
            );
            // NOTE: `provider_config` deliberately not included in the persisted input,
            // because it can carry secrets (API keys etc.).
            let output = durability
                .run_async(|| async {
                    let voice_results =
                        Impl::list_voices(provider_config.clone(), filter.clone()).await?;
                    let voices = voice_results
                        .get::<Impl::VoiceResults>()
                        .get_next()
                        .await
                        .unwrap_or_default();
                    Ok(VoiceResultsOutput { voices })
                })
                .await?;
            Ok(VoiceResults::new(DurableVoiceResults::<Impl>::new_live(
                provider_config,
                filter,
                output.voices,
            )))
        }

        async fn get_voice(
            provider_config: Self::ProviderConfig,
            voice_id: String,
        ) -> Result<Voice, TtsError> {
            init_logging();

            let durability = Durability::<VoiceOutput, TtsError>::new(
                "golem_ai_tts",
                "get_voice",
                DurableFunctionType::WriteRemote,
                &GetVoiceInput {
                    voice_id: voice_id.clone(),
                },
            );
            // NOTE: `provider_config` deliberately not included in the persisted input.
            let voice_data = durability
                .run_async(|| async {
                    Impl::get_voice(provider_config.clone(), voice_id.clone())
                        .await
                        .map(|voice| {
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
                        })
                })
                .await?;
            Ok(Voice::new(DurableVoice::<Impl>::new(
                provider_config,
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

        async fn search_voices(
            provider_config: Self::ProviderConfig,
            filter: Option<VoiceFilter>,
        ) -> Result<Vec<VoiceInfo>, TtsError> {
            init_logging();

            let durability = Durability::<VoiceInfoListOutput, TtsError>::new(
                "golem_ai_tts",
                "search_voices",
                DurableFunctionType::WriteRemote,
                &SearchVoicesInput {
                    filter: filter.clone(),
                },
            );
            // NOTE: `provider_config` deliberately not included in the persisted input.
            durability
                .run_async(|| async {
                    Impl::search_voices(provider_config, filter)
                        .await
                        .map(|voices| VoiceInfoListOutput { voices })
                })
                .await
                .map(|output| output.voices)
        }

        async fn list_languages(
            provider_config: Self::ProviderConfig,
        ) -> Result<Vec<LanguageInfo>, TtsError> {
            init_logging();

            let durability = Durability::<LanguageInfoListOutput, TtsError>::new(
                "golem_ai_tts",
                "list_languages",
                DurableFunctionType::WriteRemote,
                &NoInput,
            );
            // NOTE: `provider_config` deliberately not included in the persisted input.
            durability
                .run_async(|| async {
                    Impl::list_languages(provider_config)
                        .await
                        .map(|languages| LanguageInfoListOutput { languages })
                })
                .await
                .map(|output| output.languages)
        }
    }

    impl<Impl: ExtendedTtsProvider> SynthesizeProvider for DurableTts<Impl> {
        type ProviderConfig = <Impl as VoiceProvider>::ProviderConfig;

        async fn synthesize(
            provider_config: Self::ProviderConfig,
            input: TextInput,
            voice: crate::model::voices::VoiceBorrow<'_>,
            options: Option<SynthesisOptions>,
        ) -> Result<SynthesisResult, TtsError> {
            init_logging();

            let durability = Durability::<SynthesisResultOutput, TtsError>::new(
                "golem_ai_tts",
                "synthesize",
                DurableFunctionType::WriteRemote,
                &SynthesizeInput {
                    input: input.clone(),
                    options: options.clone(),
                },
            );
            // NOTE: `provider_config` deliberately not included in the persisted input.
            durability
                .run_async(|| async {
                    Impl::synthesize(provider_config, input, voice, options)
                        .await
                        .map(|result| SynthesisResultOutput { result })
                })
                .await
                .map(|output| output.result)
        }

        async fn synthesize_batch(
            provider_config: Self::ProviderConfig,
            inputs: Vec<TextInput>,
            voice: crate::model::voices::VoiceBorrow<'_>,
            options: Option<SynthesisOptions>,
        ) -> Result<Vec<SynthesisResult>, TtsError> {
            init_logging();

            let durability = Durability::<SynthesisResultListOutput, TtsError>::new(
                "golem_ai_tts",
                "synthesize_batch",
                DurableFunctionType::WriteRemote,
                &SynthesizeBatchInput {
                    inputs: inputs.clone(),
                    options: options.clone(),
                },
            );
            // NOTE: `provider_config` deliberately not included in the persisted input.
            durability
                .run_async(|| async {
                    Impl::synthesize_batch(provider_config, inputs, voice, options)
                        .await
                        .map(|results| SynthesisResultListOutput { results })
                })
                .await
                .map(|output| output.results)
        }

        async fn get_timing_marks(
            provider_config: Self::ProviderConfig,
            input: TextInput,
            voice: crate::model::voices::VoiceBorrow<'_>,
        ) -> Result<Vec<TimingInfo>, TtsError> {
            init_logging();

            let durability = Durability::<TimingInfoListOutput, TtsError>::new(
                "golem_ai_tts",
                "get_timing_marks",
                DurableFunctionType::WriteRemote,
                &GetTimingMarksInput {
                    input: input.clone(),
                },
            );
            // NOTE: `provider_config` deliberately not included in the persisted input.
            durability
                .run_async(|| async {
                    Impl::get_timing_marks(provider_config, input, voice)
                        .await
                        .map(|timing| TimingInfoListOutput { timing })
                })
                .await
                .map(|output| output.timing)
        }

        async fn validate_input(
            provider_config: Self::ProviderConfig,
            input: TextInput,
            voice: crate::model::voices::VoiceBorrow<'_>,
        ) -> Result<ValidationResult, TtsError> {
            init_logging();

            let durability = Durability::<ValidationResultOutput, TtsError>::new(
                "golem_ai_tts",
                "validate_input",
                DurableFunctionType::WriteRemote,
                &ValidateInputInput {
                    input: input.clone(),
                },
            );
            // NOTE: `provider_config` deliberately not included in the persisted input.
            durability
                .run_async(|| async {
                    Impl::validate_input(provider_config, input, voice)
                        .await
                        .map(|result| ValidationResultOutput { result })
                })
                .await
                .map(|output| output.result)
        }
    }

    impl<Impl: ExtendedTtsProvider> StreamingVoiceProvider for DurableTts<Impl> {
        type SynthesisStream = Impl::SynthesisStream;
        type VoiceConversionStream = Impl::VoiceConversionStream;
        type ProviderConfig = <Impl as VoiceProvider>::ProviderConfig;

        async fn create_stream(
            provider_config: Self::ProviderConfig,
            voice: crate::model::voices::VoiceBorrow<'_>,
            options: Option<SynthesisOptions>,
        ) -> Result<SynthesisStream, TtsError> {
            init_logging();
            Impl::create_stream(provider_config, voice, options).await
        }

        async fn create_voice_conversion_stream(
            provider_config: Self::ProviderConfig,
            target_voice: crate::model::voices::VoiceBorrow<'_>,
            options: Option<SynthesisOptions>,
        ) -> Result<VoiceConversionStream, TtsError> {
            init_logging();
            Impl::create_voice_conversion_stream(provider_config, target_voice, options).await
        }
    }

    pub struct DurableVoiceResults<Impl: ExtendedTtsProvider> {
        provider_config: <Impl as VoiceProvider>::ProviderConfig,
        filter: Option<VoiceFilter>,
        cached_voices: Option<Vec<VoiceInfo>>,
        _phantom: PhantomData<Impl>,
    }

    #[allow(dead_code)]
    impl<Impl: ExtendedTtsProvider> DurableVoiceResults<Impl> {
        fn new_live(
            provider_config: <Impl as VoiceProvider>::ProviderConfig,
            filter: Option<VoiceFilter>,
            voices: Vec<VoiceInfo>,
        ) -> Self {
            Self {
                provider_config,
                filter,
                cached_voices: Some(voices),
                _phantom: PhantomData,
            }
        }
    }

    impl<Impl: ExtendedTtsProvider> VoiceResultsInterface for DurableVoiceResults<Impl> {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }

        fn has_more(&self) -> bool {
            false
        }

        fn get_next(&self) -> TtsFuture<'_, Vec<VoiceInfo>> {
            Box::pin(async move {
                if let Some(ref cached_voices) = self.cached_voices {
                    return Ok(cached_voices.clone());
                }

                let durability = Durability::<VoiceInfoListOutput, TtsError>::new(
                    "golem_ai_tts",
                    "voice_results_get_next",
                    DurableFunctionType::WriteRemote,
                    &NoInput,
                );
                durability
                    .run_async(|| async {
                        let provider_config = self.provider_config.clone();
                        let filter = self.filter.clone();
                        let underlying_results = Impl::list_voices(provider_config, filter).await?;
                        let voices = underlying_results
                            .get::<Impl::VoiceResults>()
                            .get_next()
                            .await?;
                        Ok(VoiceInfoListOutput { voices })
                    })
                    .await
                    .map(|output| output.voices)
            })
        }

        fn get_total_count(&self) -> Option<u32> {
            self.cached_voices
                .as_ref()
                .map(|voices| voices.len() as u32)
        }
    }

    // Durable Voice resource
    #[allow(dead_code)]
    pub struct DurableVoice<Impl: ExtendedTtsProvider> {
        provider_config: <Impl as VoiceProvider>::ProviderConfig,
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

    impl<Impl: ExtendedTtsProvider> DurableVoice<Impl> {
        #[allow(clippy::too_many_arguments)]
        #[allow(dead_code)]
        pub fn new(
            provider_config: <Impl as VoiceProvider>::ProviderConfig,
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
                provider_config,
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

    impl<Impl: ExtendedTtsProvider> VoiceInterface for DurableVoice<Impl> {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }

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

        fn update_settings(&self, settings: VoiceSettings) -> TtsFuture<'_, ()> {
            Box::pin(async move {
                let durability = Durability::<NoOutput, TtsError>::new(
                    "golem_ai_tts",
                    "voice_update_settings",
                    DurableFunctionType::WriteRemote,
                    &UpdateVoiceSettingsInput {
                        settings: settings.clone(),
                    },
                );
                durability.run(|| Ok(NoOutput)).map(|_| ())
            })
        }

        fn delete(&self) -> TtsFuture<'_, ()> {
            Box::pin(async move {
                let durability = Durability::<NoOutput, TtsError>::new(
                    "golem_ai_tts",
                    "voice_delete",
                    DurableFunctionType::WriteRemote,
                    &NoInput,
                );
                durability.run(|| Ok(NoOutput)).map(|_| ())
            })
        }

        fn clone(&self) -> Result<Voice, TtsError> {
            let durability = Durability::<NoOutput, UnusedError>::new(
                "golem_ai_tts",
                "voice_clone",
                DurableFunctionType::ReadRemote,
                &NoInput,
            );
            durability.run_infallible(|| NoOutput);
            Ok(Voice::new(DurableVoice::<Impl>::new(
                self.provider_config.clone(),
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

        fn preview(&self, text: String) -> TtsFuture<'_, Vec<u8>> {
            Box::pin(async move {
                let durability = Durability::<AudioDataOutput, TtsError>::new(
                    "golem_ai_tts",
                    "voice_preview",
                    DurableFunctionType::ReadRemote,
                    &PreviewVoiceInput { text: text.clone() },
                );
                durability
                    .run_async(|| async {
                        let voice =
                            Impl::get_voice(self.provider_config.clone(), self.id.clone()).await?;
                        let guest = voice.get::<Impl::Voice>();
                        guest
                            .preview(text)
                            .await
                            .map(|audio| AudioDataOutput { audio })
                    })
                    .await
                    .map(|output| output.audio)
            })
        }
    }

    pub struct DurablePronunciationLexicon<Impl: ExtendedTtsProvider> {
        provider_config: <Impl as VoiceProvider>::ProviderConfig,
        name: String,
        language: LanguageCode,
        entries: Option<Vec<PronunciationEntry>>,
        _phantom: PhantomData<Impl>,
    }

    impl<Impl: ExtendedTtsProvider> DurablePronunciationLexicon<Impl> {
        pub fn new(
            provider_config: <Impl as VoiceProvider>::ProviderConfig,
            name: String,
            language: LanguageCode,
            entries: Option<Vec<PronunciationEntry>>,
        ) -> Self {
            Self {
                provider_config,
                name,
                language,
                entries,
                _phantom: PhantomData,
            }
        }
    }

    impl<Impl: ExtendedTtsProvider> PronunciationLexiconInterface
        for DurablePronunciationLexicon<Impl>
    {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }

        fn get_name(&self) -> String {
            self.name.clone()
        }

        fn get_language(&self) -> LanguageCode {
            self.language.clone()
        }

        fn get_entry_count(&self) -> u32 {
            let durability = Durability::<u32, UnusedError>::new(
                "golem_ai_tts",
                "pronunciation_lexicon_get_entry_count",
                DurableFunctionType::ReadRemote,
                &NoInput,
            );
            durability.run_infallible(|| self.entries.as_ref().map(|e| e.len() as u32).unwrap_or(0))
        }

        fn add_entry(&self, word: String, pronunciation: String) -> TtsFuture<'_, ()> {
            Box::pin(async move {
                let durability = Durability::<NoOutput, TtsError>::new(
                    "golem_ai_tts",
                    "pronunciation_lexicon_add_entry",
                    DurableFunctionType::WriteRemote,
                    &PronunciationEntryInput {
                        word: word.clone(),
                        pronunciation: pronunciation.clone(),
                    },
                );
                durability
                    .run_async(|| async {
                        let lexicon = Impl::create_lexicon(
                            self.provider_config.clone(),
                            self.name.clone(),
                            self.language.clone(),
                            self.entries.clone(),
                        )
                        .await?;
                        let guest = lexicon.get::<Impl::PronunciationLexicon>();
                        guest.add_entry(word, pronunciation).await.map(|_| NoOutput)
                    })
                    .await
                    .map(|_| ())
            })
        }

        fn remove_entry(&self, word: String) -> TtsFuture<'_, ()> {
            Box::pin(async move {
                let durability = Durability::<NoOutput, TtsError>::new(
                    "golem_ai_tts",
                    "pronunciation_lexicon_remove_entry",
                    DurableFunctionType::WriteRemote,
                    &RemoveEntryInput { word: word.clone() },
                );
                durability.run(|| Ok(NoOutput)).map(|_| ())
            })
        }

        fn export_content(&self) -> TtsFuture<'_, String> {
            Box::pin(async move {
                let durability = Durability::<String, TtsError>::new(
                    "golem_ai_tts",
                    "pronunciation_lexicon_export_content",
                    DurableFunctionType::ReadRemote,
                    &NoInput,
                );
                durability.run(|| Ok("# Pronunciation Lexicon Export\n".to_string()))
            })
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

    impl<Impl: ExtendedTtsProvider> DurableLongFormOperation<Impl> {
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

    impl<Impl: ExtendedTtsProvider> LongFormOperationInterface for DurableLongFormOperation<Impl> {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }

        fn get_status(&self) -> TtsFuture<'_, OperationStatus> {
            Box::pin(async move {
                Ok({
                    let durability = Durability::<OperationStatus, UnusedError>::new(
                        "golem_ai_tts",
                        "long_form_operation_get_status",
                        DurableFunctionType::ReadRemote,
                        &NoInput,
                    );
                    durability.run_infallible(|| OperationStatus::Completed)
                })
            })
        }

        fn get_progress(&self) -> TtsFuture<'_, f32> {
            Box::pin(async move {
                Ok({
                    let durability = Durability::<f32, UnusedError>::new(
                        "golem_ai_tts",
                        "long_form_operation_get_progress",
                        DurableFunctionType::ReadRemote,
                        &NoInput,
                    );
                    durability.run_infallible(|| 1.0)
                })
            })
        }

        fn cancel(&self) -> TtsFuture<'_, ()> {
            Box::pin(async move {
                let durability = Durability::<NoOutput, TtsError>::new(
                    "golem_ai_tts",
                    "long_form_operation_cancel",
                    DurableFunctionType::WriteRemote,
                    &NoInput,
                );
                durability.run(|| Ok(NoOutput)).map(|_| ())
            })
        }

        fn get_result(&self) -> TtsFuture<'_, LongFormResult> {
            Box::pin(async move {
                let durability = Durability::<LongFormResult, TtsError>::new(
                    "golem_ai_tts",
                    "long_form_operation_get_result",
                    DurableFunctionType::ReadRemote,
                    &NoInput,
                );
                durability.run(|| {
                    Ok(LongFormResult {
                        output_location: self.output_location.clone(),
                        total_duration: 60.0,
                        chapter_durations: None,
                        metadata: crate::model::types::SynthesisMetadata {
                            duration_seconds: 60.0,
                            character_count: self.content.len() as u32,
                            word_count: self.content.split_whitespace().count() as u32,
                            audio_size_bytes: 1024000,
                            request_id: "long-form-simulation".to_string(),
                            provider_info: Some("durable-tts".to_string()),
                        },
                    })
                })
            })
        }
    }

    impl<Impl: ExtendedTtsProvider> AdvancedTtsProvider for DurableTts<Impl> {
        type PronunciationLexicon = DurablePronunciationLexicon<Impl>;
        type LongFormOperation = DurableLongFormOperation<Impl>;
        type ProviderConfig = <Impl as VoiceProvider>::ProviderConfig;

        async fn create_voice_clone(
            provider_config: Self::ProviderConfig,
            name: String,
            audio_samples: Vec<AudioSample>,
            description: Option<String>,
        ) -> Result<Voice, TtsError> {
            init_logging();

            let durability = Durability::<VoiceOutput, TtsError>::new(
                "golem_ai_tts",
                "create_voice_clone",
                DurableFunctionType::WriteRemote,
                &CreateVoiceCloneInput {
                    name: name.clone(),
                    audio_samples: audio_samples.clone(),
                    description: description.clone(),
                },
            );

            // NOTE: `provider_config` deliberately not included in the persisted input.
            let voice_data = durability
                .run_async(|| async {
                    Impl::create_voice_clone(
                        provider_config.clone(),
                        name.clone(),
                        audio_samples.clone(),
                        description.clone(),
                    )
                    .await
                    .map(|voice| {
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
                    })
                })
                .await?;
            Ok(Voice::new(DurableVoice::<Impl>::new(
                provider_config,
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

        async fn design_voice(
            provider_config: Self::ProviderConfig,
            name: String,
            characteristics: VoiceDesignParams,
        ) -> Result<Voice, TtsError> {
            init_logging();

            let durability = Durability::<VoiceOutput, TtsError>::new(
                "golem_ai_tts",
                "design_voice",
                DurableFunctionType::WriteRemote,
                &DesignVoiceInput {
                    name: name.clone(),
                    characteristics: characteristics.clone(),
                },
            );

            // NOTE: `provider_config` deliberately not included in the persisted input.
            let voice_data = durability
                .run_async(|| async {
                    Impl::design_voice(
                        provider_config.clone(),
                        name.clone(),
                        characteristics.clone(),
                    )
                    .await
                    .map(|voice| {
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
                    })
                })
                .await?;
            Ok(Voice::new(DurableVoice::<Impl>::new(
                provider_config,
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

        async fn convert_voice(
            provider_config: Self::ProviderConfig,
            input_audio: Vec<u8>,
            target_voice: crate::model::voices::VoiceBorrow<'_>,
            preserve_timing: Option<bool>,
        ) -> Result<Vec<u8>, TtsError> {
            init_logging();

            let durability = Durability::<AudioDataOutput, TtsError>::new(
                "golem_ai_tts",
                "convert_voice",
                DurableFunctionType::WriteRemote,
                &ConvertVoiceInput {
                    input_audio: input_audio.clone(),
                    preserve_timing,
                },
            );
            // NOTE: `provider_config` deliberately not included in the persisted input.
            durability
                .run_async(|| async {
                    Impl::convert_voice(provider_config, input_audio, target_voice, preserve_timing)
                        .await
                        .map(|audio| AudioDataOutput { audio })
                })
                .await
                .map(|output| output.audio)
        }

        async fn generate_sound_effect(
            provider_config: Self::ProviderConfig,
            description: String,
            duration_seconds: Option<f32>,
            style_influence: Option<f32>,
        ) -> Result<Vec<u8>, TtsError> {
            init_logging();

            let durability = Durability::<AudioDataOutput, TtsError>::new(
                "golem_ai_tts",
                "generate_sound_effect",
                DurableFunctionType::WriteRemote,
                &GenerateSoundEffectInput {
                    description: description.clone(),
                    duration_seconds,
                    style_influence,
                },
            );
            // NOTE: `provider_config` deliberately not included in the persisted input.
            durability
                .run_async(|| async {
                    Impl::generate_sound_effect(
                        provider_config,
                        description,
                        duration_seconds,
                        style_influence,
                    )
                    .await
                    .map(|audio| AudioDataOutput { audio })
                })
                .await
                .map(|output| output.audio)
        }

        async fn create_lexicon(
            provider_config: Self::ProviderConfig,
            name: String,
            language: LanguageCode,
            entries: Option<Vec<PronunciationEntry>>,
        ) -> Result<PronunciationLexicon, TtsError> {
            init_logging();

            let durability = Durability::<PronunciationLexiconOutput, UnusedError>::new(
                "golem_ai_tts",
                "create_lexicon",
                DurableFunctionType::WriteRemote,
                &CreateLexiconInput {
                    name: name.clone(),
                    language: language.clone(),
                    entries: entries.clone(),
                },
            );
            // NOTE: `provider_config` deliberately not included in the persisted input.
            let lexicon_data = durability.run_infallible(|| PronunciationLexiconOutput {
                name: name.clone(),
                language: language.clone(),
                entries: entries.clone(),
            });
            Ok(PronunciationLexicon::new(
                DurablePronunciationLexicon::<Impl>::new(
                    provider_config,
                    lexicon_data.name,
                    lexicon_data.language,
                    lexicon_data.entries,
                ),
            ))
        }

        async fn synthesize_long_form(
            _provider_config: Self::ProviderConfig,
            content: String,
            _voice: crate::model::voices::VoiceBorrow<'_>,
            output_location: String,
            chapter_breaks: Option<Vec<u32>>,
        ) -> Result<LongFormOperation, TtsError> {
            init_logging();

            let durability = Durability::<LongFormOperationOutput, UnusedError>::new(
                "golem_ai_tts",
                "synthesize_long_form",
                DurableFunctionType::WriteRemote,
                &SynthesizeLongFormInput {
                    content: content.clone(),
                    output_location: output_location.clone(),
                    chapter_breaks: chapter_breaks.clone(),
                },
            );
            // NOTE: `provider_config` deliberately not included in the persisted input.
            let operation_data = durability.run_infallible(|| LongFormOperationOutput {
                content: content.clone(),
                output_location: output_location.clone(),
                chapter_breaks: chapter_breaks.clone(),
            });
            Ok(LongFormOperation::new(
                DurableLongFormOperation::<Impl>::new(
                    operation_data.content,
                    operation_data.output_location,
                    operation_data.chapter_breaks,
                ),
            ))
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::model::types::{AudioEffects, AudioFormat, TextType, VoiceGender, VoiceQuality};
        use golem_rust::{FromSchema, IntoSchema};

        fn roundtrip_test<T: IntoSchema + FromSchema + Clone + std::fmt::Debug + PartialEq>(
            value: T,
        ) -> T {
            golem_rust::schema::try_into_schema_graph::<T>().unwrap();
            let schema_value = value.to_value();
            let deserialized = T::from_value(&schema_value).unwrap();
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
            use crate::model::types::SynthesisMetadata;

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
            use crate::model::types::TimingMarkType;

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
