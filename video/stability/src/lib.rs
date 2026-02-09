mod client;
mod conversion;

use crate::client::StabilityApi;
use crate::conversion::{
    cancel_video_generation, extend_video, generate_lip_sync_video, generate_video,
    generate_video_effects, list_available_voices, multi_image_generation, poll_video_generation,
    upscale_video,
};
use golem_ai_video::config::with_config_key;
use golem_ai_video::durability::{DurableVideo, ExtendedVideoGenerationProvider};
use golem_ai_video::model::advanced::{
    ExtendVideoOptions, GenerateVideoEffectsOptions, MultImageGenerationOptions,
};
use golem_ai_video::model::types::{
    AudioSource, BaseVideo, GenerationConfig, LipSyncVideo, MediaInput, VideoError, VideoResult,
    VoiceInfo,
};
use golem_ai_video::{AdvancedVideoGenerationProvider, LipSyncProvider, VideoGenerationProvider};

pub struct Stability;

impl Stability {
    const ENV_VAR_NAME: &'static str = "STABILITY_API_KEY";
}

impl VideoGenerationProvider for Stability {
    fn generate(input: MediaInput, config: GenerationConfig) -> Result<String, VideoError> {
        with_config_key(Self::ENV_VAR_NAME, Err, |api_key| {
            let client = StabilityApi::new(api_key);
            generate_video(&client, input, config)
        })
    }

    fn poll(job_id: String) -> Result<VideoResult, VideoError> {
        with_config_key(Self::ENV_VAR_NAME, Err, |api_key| {
            let client = StabilityApi::new(api_key);
            poll_video_generation(&client, job_id)
        })
    }

    fn cancel(job_id: String) -> Result<String, VideoError> {
        cancel_video_generation(job_id)
    }
}

impl LipSyncProvider for Stability {
    fn generate_lip_sync(video: LipSyncVideo, audio: AudioSource) -> Result<String, VideoError> {
        with_config_key(Self::ENV_VAR_NAME, Err, |api_key| {
            let client = StabilityApi::new(api_key);
            generate_lip_sync_video(&client, video, audio)
        })
    }

    fn list_voices(language: Option<String>) -> Result<Vec<VoiceInfo>, VideoError> {
        with_config_key(Self::ENV_VAR_NAME, Err, |api_key| {
            let client = StabilityApi::new(api_key);
            list_available_voices(&client, language)
        })
    }
}

impl AdvancedVideoGenerationProvider for Stability {
    fn extend_video(options: ExtendVideoOptions) -> Result<String, VideoError> {
        with_config_key(Self::ENV_VAR_NAME, Err, |api_key| {
            let client = StabilityApi::new(api_key);
            extend_video(
                &client,
                options.video_id,
                options.prompt,
                options.negative_prompt,
                options.cfg_scale,
                options.provider_options,
            )
        })
    }

    fn upscale_video(input: BaseVideo) -> Result<String, VideoError> {
        with_config_key(Self::ENV_VAR_NAME, Err, |api_key| {
            let client = StabilityApi::new(api_key);
            upscale_video(&client, input)
        })
    }

    fn generate_video_effects(options: GenerateVideoEffectsOptions) -> Result<String, VideoError> {
        with_config_key(Self::ENV_VAR_NAME, Err, |api_key| {
            let client = StabilityApi::new(api_key);
            generate_video_effects(
                &client,
                options.input,
                options.effect,
                options.model,
                options.duration,
                options.mode,
            )
        })
    }

    fn multi_image_generation(options: MultImageGenerationOptions) -> Result<String, VideoError> {
        with_config_key(Self::ENV_VAR_NAME, Err, |api_key| {
            let client = StabilityApi::new(api_key);
            multi_image_generation(
                &client,
                options.input_images,
                options.prompt,
                options.config,
            )
        })
    }
}

impl ExtendedVideoGenerationProvider for Stability {}

pub type DurableStability = DurableVideo<Stability>;
