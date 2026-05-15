use crate::{AdvancedVideoGenerationProvider, LipSyncProvider, VideoGenerationProvider};
use std::marker::PhantomData;

/// Wraps a Video implementation with custom durability
pub struct DurableVideo<Impl> {
    phantom: PhantomData<Impl>,
}

/// Trait implemented by provider crates in addition to the three native Video provider traits
/// so `DurableVideo` can be parameterised by a single type that supplies all of them.
///
/// All three sub-traits (`VideoGenerationProvider`, `LipSyncProvider`,
/// `AdvancedVideoGenerationProvider`) must agree on the same `ProviderConfig`
/// type so that the durable wrapper can thread a single `provider_config`
/// value through every method.
pub trait ExtendedVideoGenerationProvider:
    VideoGenerationProvider
    + LipSyncProvider<ProviderConfig = <Self as VideoGenerationProvider>::ProviderConfig>
    + AdvancedVideoGenerationProvider<
        ProviderConfig = <Self as VideoGenerationProvider>::ProviderConfig,
    > + 'static
{
}

/// When the durability feature flag is off, `DurableVideo<Impl>` is a transparent wrapper that
/// forwards every call to the inner provider without any oplog persistence.
#[cfg(not(feature = "golem"))]
mod passthrough_impl {
    use crate::durability::{DurableVideo, ExtendedVideoGenerationProvider};
    use crate::model::advanced::{
        ExtendVideoOptions, GenerateVideoEffectsOptions, MultImageGenerationOptions,
    };
    use crate::model::types::{
        AudioSource, BaseVideo, GenerationConfig, LipSyncVideo, MediaInput, VideoError,
        VideoResult, VoiceInfo,
    };
    use crate::{AdvancedVideoGenerationProvider, LipSyncProvider, VideoGenerationProvider};

    impl<Impl: ExtendedVideoGenerationProvider> VideoGenerationProvider for DurableVideo<Impl> {
        type ProviderConfig = <Impl as VideoGenerationProvider>::ProviderConfig;

        fn generate(
            provider_config: Self::ProviderConfig,
            input: MediaInput,
            config: GenerationConfig,
        ) -> Result<String, VideoError> {
            Impl::generate(provider_config, input, config)
        }

        fn poll(
            provider_config: Self::ProviderConfig,
            job_id: String,
        ) -> Result<VideoResult, VideoError> {
            Impl::poll(provider_config, job_id)
        }

        fn cancel(
            provider_config: Self::ProviderConfig,
            job_id: String,
        ) -> Result<String, VideoError> {
            Impl::cancel(provider_config, job_id)
        }
    }

    impl<Impl: ExtendedVideoGenerationProvider> LipSyncProvider for DurableVideo<Impl> {
        type ProviderConfig = <Impl as VideoGenerationProvider>::ProviderConfig;

        fn generate_lip_sync(
            provider_config: Self::ProviderConfig,
            video: LipSyncVideo,
            audio: AudioSource,
        ) -> Result<String, VideoError> {
            Impl::generate_lip_sync(provider_config, video, audio)
        }

        fn list_voices(
            provider_config: Self::ProviderConfig,
            language: Option<String>,
        ) -> Result<Vec<VoiceInfo>, VideoError> {
            Impl::list_voices(provider_config, language)
        }
    }

    impl<Impl: ExtendedVideoGenerationProvider> AdvancedVideoGenerationProvider for DurableVideo<Impl> {
        type ProviderConfig = <Impl as VideoGenerationProvider>::ProviderConfig;

        fn extend_video(
            provider_config: Self::ProviderConfig,
            options: ExtendVideoOptions,
        ) -> Result<String, VideoError> {
            Impl::extend_video(provider_config, options)
        }

        fn upscale_video(
            provider_config: Self::ProviderConfig,
            input: BaseVideo,
        ) -> Result<String, VideoError> {
            Impl::upscale_video(provider_config, input)
        }

        fn generate_video_effects(
            provider_config: Self::ProviderConfig,
            options: GenerateVideoEffectsOptions,
        ) -> Result<String, VideoError> {
            Impl::generate_video_effects(provider_config, options)
        }

        fn multi_image_generation(
            provider_config: Self::ProviderConfig,
            options: MultImageGenerationOptions,
        ) -> Result<String, VideoError> {
            Impl::multi_image_generation(provider_config, options)
        }
    }
}

/// When the `golem` feature flag is on, wrapping with `DurableVideo` adds custom durability
/// on top of the provider-specific Video implementation using Golem's special host functions and
/// the `golem-rust` helper library.
///
/// There will be custom durability entries saved in the oplog, with the full Video request and configuration
/// stored as input, and the full response stored as output. To serialize these in a way it is
/// observable by oplog consumers, each relevant data type has to be converted to/from `ValueAndType`
/// which is implemented using the type classes and builder in the `golem-rust` library.
///
/// NOTE: `provider_config` is intentionally **not** persisted in the oplog input
/// payloads because it can carry secrets (API keys etc.).
#[cfg(feature = "golem")]
mod durable_impl {
    use crate::durability::{DurableVideo, ExtendedVideoGenerationProvider};
    use crate::model::advanced::{
        ExtendVideoOptions, GenerateVideoEffectsOptions, MultImageGenerationOptions,
    };
    use crate::model::types::{
        AudioSource, BaseVideo, GenerationConfig, LipSyncVideo, MediaInput, VideoError,
        VideoResult, VoiceInfo,
    };
    use crate::{
        init_logging, AdvancedVideoGenerationProvider, LipSyncProvider, VideoGenerationProvider,
    };
    use golem_rust::bindings::golem::durability::durability::DurableFunctionType;
    use golem_rust::durability::Durability;
    use golem_rust::{with_persistence_level, FromValueAndType, IntoValue, PersistenceLevel};
    use std::fmt::{Display, Formatter};

    impl<Impl: ExtendedVideoGenerationProvider> VideoGenerationProvider for DurableVideo<Impl> {
        type ProviderConfig = <Impl as VideoGenerationProvider>::ProviderConfig;

        fn generate(
            provider_config: Self::ProviderConfig,
            input: MediaInput,
            config: GenerationConfig,
        ) -> Result<String, VideoError> {
            init_logging();
            let durability = Durability::<String, VideoError>::new(
                "golem_ai_video",
                "generate",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let input_clone = input.clone();
                let config_clone = config.clone();
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::generate(provider_config, input_clone, config_clone)
                });
                // NOTE: `provider_config` deliberately not included in the persisted input,
                // because it can carry secrets (API keys etc.).
                durability.persist(GenerateInput { input, config }, result)
            } else {
                durability.replay()
            }
        }

        fn poll(
            provider_config: Self::ProviderConfig,
            job_id: String,
        ) -> Result<VideoResult, VideoError> {
            init_logging();
            let durability = Durability::<VideoResult, VideoError>::new(
                "golem_ai_video",
                "poll",
                DurableFunctionType::ReadRemote,
            );
            if durability.is_live() {
                let job_id_clone = job_id.clone();
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::poll(provider_config, job_id_clone)
                });
                // NOTE: `provider_config` deliberately not included in the persisted input,
                // because it can carry secrets (API keys etc.).
                durability.persist(PollInput { job_id }, result)
            } else {
                durability.replay()
            }
        }

        fn cancel(
            provider_config: Self::ProviderConfig,
            job_id: String,
        ) -> Result<String, VideoError> {
            init_logging();
            let durability = Durability::<String, VideoError>::new(
                "golem_ai_video",
                "cancel",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let job_id_clone = job_id.clone();
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::cancel(provider_config, job_id_clone)
                });
                // NOTE: `provider_config` deliberately not included in the persisted input,
                // because it can carry secrets (API keys etc.).
                durability.persist(CancelInput { job_id }, result)
            } else {
                durability.replay()
            }
        }
    }

    impl<Impl: ExtendedVideoGenerationProvider> LipSyncProvider for DurableVideo<Impl> {
        type ProviderConfig = <Impl as VideoGenerationProvider>::ProviderConfig;

        fn generate_lip_sync(
            provider_config: Self::ProviderConfig,
            video: LipSyncVideo,
            audio: AudioSource,
        ) -> Result<String, VideoError> {
            init_logging();
            let durability = Durability::<String, VideoError>::new(
                "golem_ai_video",
                "generate_lip_sync",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let video_clone = video.clone();
                let audio_clone = audio.clone();
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::generate_lip_sync(provider_config, video_clone, audio_clone)
                });
                // NOTE: `provider_config` deliberately not included in the persisted input,
                // because it can carry secrets (API keys etc.).
                durability.persist(GenerateLipSyncInput { video, audio }, result)
            } else {
                durability.replay()
            }
        }

        fn list_voices(
            provider_config: Self::ProviderConfig,
            language: Option<String>,
        ) -> Result<Vec<VoiceInfo>, VideoError> {
            init_logging();
            let durability = Durability::<Vec<VoiceInfo>, VideoError>::new(
                "golem_ai_video",
                "list_voices",
                DurableFunctionType::ReadRemote,
            );
            if durability.is_live() {
                let language_clone = language.clone();
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::list_voices(provider_config, language_clone)
                });
                // NOTE: `provider_config` deliberately not included in the persisted input,
                // because it can carry secrets (API keys etc.).
                durability.persist(ListVoicesInput { language }, result)
            } else {
                durability.replay()
            }
        }
    }

    impl<Impl: ExtendedVideoGenerationProvider> AdvancedVideoGenerationProvider for DurableVideo<Impl> {
        type ProviderConfig = <Impl as VideoGenerationProvider>::ProviderConfig;

        fn extend_video(
            provider_config: Self::ProviderConfig,
            options: ExtendVideoOptions,
        ) -> Result<String, VideoError> {
            init_logging();
            let durability = Durability::<String, VideoError>::new(
                "golem_ai_video",
                "extend_video",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let options_clone = options.clone();
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::extend_video(provider_config, options_clone)
                });
                // NOTE: `provider_config` deliberately not included in the persisted input,
                // because it can carry secrets (API keys etc.).
                durability.persist(options, result)
            } else {
                durability.replay()
            }
        }

        fn upscale_video(
            provider_config: Self::ProviderConfig,
            input: BaseVideo,
        ) -> Result<String, VideoError> {
            init_logging();
            let durability = Durability::<String, VideoError>::new(
                "golem_ai_video",
                "upscale_video",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let input_clone = input.clone();
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::upscale_video(provider_config, input_clone)
                });
                // NOTE: `provider_config` deliberately not included in the persisted input,
                // because it can carry secrets (API keys etc.).
                durability.persist(UpscaleVideoInput { input }, result)
            } else {
                durability.replay()
            }
        }

        fn generate_video_effects(
            provider_config: Self::ProviderConfig,
            options: GenerateVideoEffectsOptions,
        ) -> Result<String, VideoError> {
            init_logging();
            let durability = Durability::<String, VideoError>::new(
                "golem_ai_video",
                "generate_video_effects",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let options_clone = options.clone();
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::generate_video_effects(provider_config, options_clone)
                });
                // NOTE: `provider_config` deliberately not included in the persisted input,
                // because it can carry secrets (API keys etc.).
                durability.persist(options, result)
            } else {
                durability.replay()
            }
        }

        fn multi_image_generation(
            provider_config: Self::ProviderConfig,
            options: MultImageGenerationOptions,
        ) -> Result<String, VideoError> {
            init_logging();
            let durability = Durability::<String, VideoError>::new(
                "golem_ai_video",
                "multi_image_generation",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let options_clone = options.clone();
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::multi_image_generation(provider_config, options_clone)
                });
                // NOTE: `provider_config` deliberately not included in the persisted input,
                // because it can carry secrets (API keys etc.).
                durability.persist(options, result)
            } else {
                durability.replay()
            }
        }
    }

    #[derive(Debug, Clone, PartialEq, IntoValue, FromValueAndType)]
    struct GenerateInput {
        input: MediaInput,
        config: GenerationConfig,
    }

    #[derive(Debug, Clone, PartialEq, IntoValue, FromValueAndType)]
    struct PollInput {
        job_id: String,
    }

    #[derive(Debug, Clone, PartialEq, IntoValue, FromValueAndType)]
    struct CancelInput {
        job_id: String,
    }

    #[derive(Debug, Clone, PartialEq, IntoValue, FromValueAndType)]
    struct GenerateLipSyncInput {
        video: LipSyncVideo,
        audio: AudioSource,
    }

    #[derive(Debug, Clone, PartialEq, IntoValue, FromValueAndType)]
    struct ListVoicesInput {
        language: Option<String>,
    }

    #[derive(Debug, Clone, PartialEq, IntoValue, FromValueAndType)]
    struct UpscaleVideoInput {
        input: BaseVideo,
    }

    #[allow(dead_code)]
    #[derive(Debug, FromValueAndType, IntoValue)]
    struct UnusedError;

    impl Display for UnusedError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "UnusedError")
        }
    }

    impl From<&VideoError> for VideoError {
        fn from(error: &VideoError) -> Self {
            error.clone()
        }
    }
}
