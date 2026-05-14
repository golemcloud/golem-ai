mod authentication;
mod client;
pub mod config;
mod conversion;
mod voices;

use crate::client::KlingApi;
use crate::conversion::{
    cancel_video_generation, extend_video, generate_lip_sync_video, generate_video,
    generate_video_effects, list_available_voices, multi_image_generation, poll_video_generation,
    upscale_video,
};
use golem_ai_video::durability::{DurableVideo, ExtendedVideoGenerationProvider};
use golem_ai_video::model::advanced::{
    ExtendVideoOptions, GenerateVideoEffectsOptions, MultImageGenerationOptions,
};
use golem_ai_video::model::types::{
    AudioSource, BaseVideo, GenerationConfig, LipSyncVideo, MediaInput, VideoError, VideoResult,
    VoiceInfo,
};
use golem_ai_video::{AdvancedVideoGenerationProvider, LipSyncProvider, VideoGenerationProvider};

pub use config::KlingConfig;
#[cfg(feature = "golem")]
pub use config::KlingHostConfig;

pub struct Kling;

impl VideoGenerationProvider for Kling {
    type ProviderConfig = KlingConfig;

    fn generate(
        provider_config: Self::ProviderConfig,
        input: MediaInput,
        config: GenerationConfig,
    ) -> Result<String, VideoError> {
        let client = KlingApi::new(&provider_config);
        generate_video(&client, input, config)
    }

    fn poll(
        provider_config: Self::ProviderConfig,
        job_id: String,
    ) -> Result<VideoResult, VideoError> {
        let client = KlingApi::new(&provider_config);
        poll_video_generation(&client, job_id)
    }

    fn cancel(
        provider_config: Self::ProviderConfig,
        job_id: String,
    ) -> Result<String, VideoError> {
        let client = KlingApi::new(&provider_config);
        cancel_video_generation(&client, job_id)
    }
}

impl LipSyncProvider for Kling {
    type ProviderConfig = KlingConfig;

    fn generate_lip_sync(
        provider_config: Self::ProviderConfig,
        video: LipSyncVideo,
        audio: AudioSource,
    ) -> Result<String, VideoError> {
        let client = KlingApi::new(&provider_config);
        generate_lip_sync_video(&client, video, audio)
    }

    fn list_voices(
        provider_config: Self::ProviderConfig,
        language: Option<String>,
    ) -> Result<Vec<VoiceInfo>, VideoError> {
        let client = KlingApi::new(&provider_config);
        list_available_voices(&client, language)
    }
}

impl AdvancedVideoGenerationProvider for Kling {
    type ProviderConfig = KlingConfig;

    fn extend_video(
        provider_config: Self::ProviderConfig,
        options: ExtendVideoOptions,
    ) -> Result<String, VideoError> {
        let client = KlingApi::new(&provider_config);
        extend_video(
            &client,
            options.video_id,
            options.prompt,
            options.negative_prompt,
            options.cfg_scale,
            options.provider_options,
        )
    }

    fn upscale_video(
        provider_config: Self::ProviderConfig,
        input: BaseVideo,
    ) -> Result<String, VideoError> {
        let client = KlingApi::new(&provider_config);
        upscale_video(&client, input)
    }

    fn generate_video_effects(
        provider_config: Self::ProviderConfig,
        options: GenerateVideoEffectsOptions,
    ) -> Result<String, VideoError> {
        let client = KlingApi::new(&provider_config);
        generate_video_effects(
            &client,
            options.input,
            options.effect,
            options.model,
            options.duration,
            options.mode,
        )
    }

    fn multi_image_generation(
        provider_config: Self::ProviderConfig,
        options: MultImageGenerationOptions,
    ) -> Result<String, VideoError> {
        let client = KlingApi::new(&provider_config);
        multi_image_generation(
            &client,
            options.input_images,
            options.prompt,
            options.config,
        )
    }
}

impl ExtendedVideoGenerationProvider for Kling {}

pub type DurableKling = DurableVideo<Kling>;
