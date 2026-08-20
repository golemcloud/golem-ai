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

        async fn generate(
            provider_config: Self::ProviderConfig,
            input: MediaInput,
            config: GenerationConfig,
        ) -> Result<String, VideoError> {
            Impl::generate(provider_config, input, config).await
        }

        async fn poll(
            provider_config: Self::ProviderConfig,
            job_id: String,
        ) -> Result<VideoResult, VideoError> {
            Impl::poll(provider_config, job_id).await
        }

        async fn cancel(
            provider_config: Self::ProviderConfig,
            job_id: String,
        ) -> Result<String, VideoError> {
            Impl::cancel(provider_config, job_id).await
        }
    }

    impl<Impl: ExtendedVideoGenerationProvider> LipSyncProvider for DurableVideo<Impl> {
        type ProviderConfig = <Impl as VideoGenerationProvider>::ProviderConfig;

        async fn generate_lip_sync(
            provider_config: Self::ProviderConfig,
            video: LipSyncVideo,
            audio: AudioSource,
        ) -> Result<String, VideoError> {
            Impl::generate_lip_sync(provider_config, video, audio).await
        }

        async fn list_voices(
            provider_config: Self::ProviderConfig,
            language: Option<String>,
        ) -> Result<Vec<VoiceInfo>, VideoError> {
            Impl::list_voices(provider_config, language).await
        }
    }

    impl<Impl: ExtendedVideoGenerationProvider> AdvancedVideoGenerationProvider for DurableVideo<Impl> {
        type ProviderConfig = <Impl as VideoGenerationProvider>::ProviderConfig;

        async fn extend_video(
            provider_config: Self::ProviderConfig,
            options: ExtendVideoOptions,
        ) -> Result<String, VideoError> {
            Impl::extend_video(provider_config, options).await
        }

        async fn upscale_video(
            provider_config: Self::ProviderConfig,
            input: BaseVideo,
        ) -> Result<String, VideoError> {
            Impl::upscale_video(provider_config, input).await
        }

        async fn generate_video_effects(
            provider_config: Self::ProviderConfig,
            options: GenerateVideoEffectsOptions,
        ) -> Result<String, VideoError> {
            Impl::generate_video_effects(provider_config, options).await
        }

        async fn multi_image_generation(
            provider_config: Self::ProviderConfig,
            options: MultImageGenerationOptions,
        ) -> Result<String, VideoError> {
            Impl::multi_image_generation(provider_config, options).await
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
    use golem_rust::durability::{Durability, DurableFunctionType};
    use golem_rust::{FromSchema, IntoSchema};
    use std::fmt::{Display, Formatter};

    impl<Impl: ExtendedVideoGenerationProvider> VideoGenerationProvider for DurableVideo<Impl> {
        type ProviderConfig = <Impl as VideoGenerationProvider>::ProviderConfig;

        async fn generate(
            provider_config: Self::ProviderConfig,
            input: MediaInput,
            config: GenerationConfig,
        ) -> Result<String, VideoError> {
            init_logging();
            let persisted_input = GenerateInput {
                input: input.clone(),
                config: config.clone(),
            };
            Durability::<String, VideoError>::new(
                "golem_ai_video",
                "generate",
                DurableFunctionType::WriteRemote,
                &persisted_input,
            )
            .run_async(|| Impl::generate(provider_config, input, config))
            .await
            // NOTE: `provider_config` deliberately not included in the persisted input,
            // because it can carry secrets (API keys etc.).
        }

        async fn poll(
            provider_config: Self::ProviderConfig,
            job_id: String,
        ) -> Result<VideoResult, VideoError> {
            init_logging();
            let input = PollInput {
                job_id: job_id.clone(),
            };
            Durability::<VideoResult, VideoError>::new(
                "golem_ai_video",
                "poll",
                DurableFunctionType::ReadRemote,
                &input,
            )
            .run_async(|| Impl::poll(provider_config, job_id))
            .await
            // NOTE: `provider_config` deliberately not included in the persisted input,
            // because it can carry secrets (API keys etc.).
        }

        async fn cancel(
            provider_config: Self::ProviderConfig,
            job_id: String,
        ) -> Result<String, VideoError> {
            init_logging();
            let input = CancelInput {
                job_id: job_id.clone(),
            };
            Durability::<String, VideoError>::new(
                "golem_ai_video",
                "cancel",
                DurableFunctionType::WriteRemote,
                &input,
            )
            .run_async(|| Impl::cancel(provider_config, job_id))
            .await
            // NOTE: `provider_config` deliberately not included in the persisted input,
            // because it can carry secrets (API keys etc.).
        }
    }

    impl<Impl: ExtendedVideoGenerationProvider> LipSyncProvider for DurableVideo<Impl> {
        type ProviderConfig = <Impl as VideoGenerationProvider>::ProviderConfig;

        async fn generate_lip_sync(
            provider_config: Self::ProviderConfig,
            video: LipSyncVideo,
            audio: AudioSource,
        ) -> Result<String, VideoError> {
            init_logging();
            let input = GenerateLipSyncInput {
                video: video.clone(),
                audio: audio.clone(),
            };
            Durability::<String, VideoError>::new(
                "golem_ai_video",
                "generate_lip_sync",
                DurableFunctionType::WriteRemote,
                &input,
            )
            .run_async(|| Impl::generate_lip_sync(provider_config, video, audio))
            .await
            // NOTE: `provider_config` deliberately not included in the persisted input,
            // because it can carry secrets (API keys etc.).
        }

        async fn list_voices(
            provider_config: Self::ProviderConfig,
            language: Option<String>,
        ) -> Result<Vec<VoiceInfo>, VideoError> {
            init_logging();
            let input = ListVoicesInput {
                language: language.clone(),
            };
            Durability::<Vec<VoiceInfo>, VideoError>::new(
                "golem_ai_video",
                "list_voices",
                DurableFunctionType::ReadRemote,
                &input,
            )
            .run_async(|| Impl::list_voices(provider_config, language))
            .await
            // NOTE: `provider_config` deliberately not included in the persisted input,
            // because it can carry secrets (API keys etc.).
        }
    }

    impl<Impl: ExtendedVideoGenerationProvider> AdvancedVideoGenerationProvider for DurableVideo<Impl> {
        type ProviderConfig = <Impl as VideoGenerationProvider>::ProviderConfig;

        async fn extend_video(
            provider_config: Self::ProviderConfig,
            options: ExtendVideoOptions,
        ) -> Result<String, VideoError> {
            init_logging();
            Durability::<String, VideoError>::new(
                "golem_ai_video",
                "extend_video",
                DurableFunctionType::WriteRemote,
                &options,
            )
            .run_async(|| Impl::extend_video(provider_config, options))
            .await
            // NOTE: `provider_config` deliberately not included in the persisted input,
            // because it can carry secrets (API keys etc.).
        }

        async fn upscale_video(
            provider_config: Self::ProviderConfig,
            input: BaseVideo,
        ) -> Result<String, VideoError> {
            init_logging();
            let persisted_input = UpscaleVideoInput {
                input: input.clone(),
            };
            Durability::<String, VideoError>::new(
                "golem_ai_video",
                "upscale_video",
                DurableFunctionType::WriteRemote,
                &persisted_input,
            )
            .run_async(|| Impl::upscale_video(provider_config, input))
            .await
            // NOTE: `provider_config` deliberately not included in the persisted input,
            // because it can carry secrets (API keys etc.).
        }

        async fn generate_video_effects(
            provider_config: Self::ProviderConfig,
            options: GenerateVideoEffectsOptions,
        ) -> Result<String, VideoError> {
            init_logging();
            Durability::<String, VideoError>::new(
                "golem_ai_video",
                "generate_video_effects",
                DurableFunctionType::WriteRemote,
                &options,
            )
            .run_async(|| Impl::generate_video_effects(provider_config, options))
            .await
            // NOTE: `provider_config` deliberately not included in the persisted input,
            // because it can carry secrets (API keys etc.).
        }

        async fn multi_image_generation(
            provider_config: Self::ProviderConfig,
            options: MultImageGenerationOptions,
        ) -> Result<String, VideoError> {
            init_logging();
            Durability::<String, VideoError>::new(
                "golem_ai_video",
                "multi_image_generation",
                DurableFunctionType::WriteRemote,
                &options,
            )
            .run_async(|| Impl::multi_image_generation(provider_config, options))
            .await
            // NOTE: `provider_config` deliberately not included in the persisted input,
            // because it can carry secrets (API keys etc.).
        }
    }

    #[derive(Debug, Clone, PartialEq, IntoSchema, FromSchema)]
    struct GenerateInput {
        input: MediaInput,
        config: GenerationConfig,
    }

    #[derive(Debug, Clone, PartialEq, IntoSchema, FromSchema)]
    struct PollInput {
        job_id: String,
    }

    #[derive(Debug, Clone, PartialEq, IntoSchema, FromSchema)]
    struct CancelInput {
        job_id: String,
    }

    #[derive(Debug, Clone, PartialEq, IntoSchema, FromSchema)]
    struct GenerateLipSyncInput {
        video: LipSyncVideo,
        audio: AudioSource,
    }

    #[derive(Debug, Clone, PartialEq, IntoSchema, FromSchema)]
    struct ListVoicesInput {
        language: Option<String>,
    }

    #[derive(Debug, Clone, PartialEq, IntoSchema, FromSchema)]
    struct UpscaleVideoInput {
        input: BaseVideo,
    }

    #[allow(dead_code)]
    #[derive(Debug, FromSchema, IntoSchema)]
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
