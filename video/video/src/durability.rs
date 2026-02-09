#[allow(unused_imports)]
use crate::model::types::{
    AudioSource, BaseVideo, EffectType, GenerationConfig, InputImage, Kv, LipSyncVideo, MediaInput,
    VideoError, VideoResult, VoiceInfo,
};
use crate::{AdvancedVideoGenerationProvider, LipSyncProvider, VideoGenerationProvider};
use std::marker::PhantomData;

/// Wraps a Video implementation with custom durability
pub struct DurableVideo<Impl> {
    phantom: PhantomData<Impl>,
}

/// Trait to be implemented in addition to the Video `Guest` traits when wrapping it with `DurableVideo`.
pub trait ExtendedVideoGenerationProvider:
    VideoGenerationProvider + LipSyncProvider + AdvancedVideoGenerationProvider + 'static
{
}

/// When the durability feature flag is off, wrapping with `DurableVideo` is just a passthrough
#[cfg(not(feature = "durability"))]
mod passthrough_impl {
    use crate::durability::{DurableVideo, ExtendedVideoGenerationProvider};
    use crate::model::advanced::{
        ExtendVideoOptions, GenerateVideoEffectsOptions, Guest as AdvancedGuest,
        MultImageGenerationOptions,
    };
    use crate::model::lip_sync::Guest as LipSyncGuest;
    use crate::model::types::{
        AudioSource, BaseVideo, EffectType, GenerationConfig, InputImage, Kv, LipSyncVideo,
        MediaInput, VideoError, VideoResult, VoiceInfo,
    };
    use crate::model::video_generation::Guest as VideoGenerationGuest;

    impl<Impl: ExtendedVideoGenerationProvider> VideoGenerationGuest for DurableVideo<Impl> {
        fn generate(input: MediaInput, config: GenerationConfig) -> Result<String, VideoError> {
            Impl::generate(input, config)
        }

        fn poll(job_id: String) -> Result<VideoResult, VideoError> {
            Impl::poll(job_id)
        }

        fn cancel(job_id: String) -> Result<String, VideoError> {
            Impl::cancel(job_id)
        }
    }

    impl<Impl: ExtendedVideoGenerationProvider> LipSyncGuest for DurableVideo<Impl> {
        fn generate_lip_sync(
            video: LipSyncVideo,
            audio: AudioSource,
        ) -> Result<String, VideoError> {
            Impl::generate_lip_sync(video, audio)
        }

        fn list_voices(language: Option<String>) -> Result<Vec<VoiceInfo>, VideoError> {
            Impl::list_voices(language)
        }
    }

    impl<Impl: ExtendedVideoGenerationProvider> AdvancedGuest for DurableVideo<Impl> {
        fn extend_video(options: ExtendVideoOptions) -> Result<String, VideoError> {
            Impl::extend_video(options)
        }

        fn upscale_video(input: BaseVideo) -> Result<String, VideoError> {
            Impl::upscale_video(input)
        }

        fn generate_video_effects(
            options: GenerateVideoEffectsOptions,
        ) -> Result<String, VideoError> {
            Impl::generate_video_effects(options)
        }

        fn multi_image_generation(
            options: MultImageGenerationOptions,
        ) -> Result<String, VideoError> {
            Impl::multi_image_generation(options)
        }
    }
}

/// When the durability feature flag is on, wrapping with `DurableVideo` adds custom durability
/// on top of the provider-specific Video implementation using Golem's special host functions and
/// the `golem-rust` helper library.
///
/// There will be custom durability entries saved in the oplog, with the full Video request and configuration
/// stored as input, and the full response stored as output. To serialize these in a way it is
/// observable by oplog consumers, each relevant data type has to be converted to/from `ValueAndType`
/// which is implemented using the type classes and builder in the `golem-rust` library.
#[cfg(feature = "durability")]
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
        fn generate(input: MediaInput, config: GenerationConfig) -> Result<String, VideoError> {
            init_logging();
            let durability = Durability::<String, VideoError>::new(
                "golem_video",
                "generate",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::generate(input.clone(), config.clone())
                });
                durability.persist(GenerateInput { input, config }, result)
            } else {
                durability.replay()
            }
        }

        fn poll(job_id: String) -> Result<VideoResult, VideoError> {
            init_logging();
            let durability = Durability::<VideoResult, VideoError>::new(
                "golem_video",
                "poll",
                DurableFunctionType::ReadRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::poll(job_id.clone())
                });
                durability.persist(PollInput { job_id }, result)
            } else {
                durability.replay()
            }
        }

        fn cancel(job_id: String) -> Result<String, VideoError> {
            init_logging();
            let durability = Durability::<String, VideoError>::new(
                "golem_video",
                "cancel",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::cancel(job_id.clone())
                });
                durability.persist(CancelInput { job_id }, result)
            } else {
                durability.replay()
            }
        }
    }

    impl<Impl: ExtendedVideoGenerationProvider> LipSyncProvider for DurableVideo<Impl> {
        fn generate_lip_sync(
            video: LipSyncVideo,
            audio: AudioSource,
        ) -> Result<String, VideoError> {
            init_logging();
            let durability = Durability::<String, VideoError>::new(
                "golem_video",
                "generate_lip_sync",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::generate_lip_sync(video.clone(), audio.clone())
                });
                durability.persist(GenerateLipSyncInput { video, audio }, result)
            } else {
                durability.replay()
            }
        }

        fn list_voices(language: Option<String>) -> Result<Vec<VoiceInfo>, VideoError> {
            init_logging();
            let durability = Durability::<Vec<VoiceInfo>, VideoError>::new(
                "golem_video",
                "list_voices",
                DurableFunctionType::ReadRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::list_voices(language.clone())
                });
                durability.persist(ListVoicesInput { language }, result)
            } else {
                durability.replay()
            }
        }
    }

    impl<Impl: ExtendedVideoGenerationProvider> AdvancedVideoGenerationProvider for DurableVideo<Impl> {
        fn extend_video(options: ExtendVideoOptions) -> Result<String, VideoError> {
            init_logging();
            let durability = Durability::<String, VideoError>::new(
                "golem_video",
                "extend_video",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::extend_video(options.clone())
                });
                durability.persist(options, result)
            } else {
                durability.replay()
            }
        }

        fn upscale_video(input: BaseVideo) -> Result<String, VideoError> {
            init_logging();
            let durability = Durability::<String, VideoError>::new(
                "golem_video",
                "upscale_video",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::upscale_video(input.clone())
                });
                durability.persist(UpscaleVideoInput { input }, result)
            } else {
                durability.replay()
            }
        }

        fn generate_video_effects(
            options: GenerateVideoEffectsOptions,
        ) -> Result<String, VideoError> {
            init_logging();
            let durability = Durability::<String, VideoError>::new(
                "golem_video",
                "generate_video_effects",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::generate_video_effects(options.clone())
                });
                durability.persist(options, result)
            } else {
                durability.replay()
            }
        }

        fn multi_image_generation(
            options: MultImageGenerationOptions,
        ) -> Result<String, VideoError> {
            init_logging();
            let durability = Durability::<String, VideoError>::new(
                "golem_video",
                "multi_image_generation",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::multi_image_generation(options.clone())
                });
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
