mod authentication;
mod client;
mod conversion;

use crate::client::VeoApi;
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

pub struct Veo;

impl Veo {
    const PROJECT_ID_ENV_VAR: &'static str = "VEO_PROJECT_ID";
    const CLIENT_EMAIL_ENV_VAR: &'static str = "VEO_CLIENT_EMAIL";
    const PRIVATE_KEY_ENV_VAR: &'static str = "VEO_PRIVATE_KEY";
}

impl VideoGenerationProvider for Veo {
    fn generate(input: MediaInput, config: GenerationConfig) -> Result<String, VideoError> {
        with_config_key(Self::PROJECT_ID_ENV_VAR, Err, |project_id| {
            with_config_key(Self::CLIENT_EMAIL_ENV_VAR, Err, |client_email| {
                with_config_key(Self::PRIVATE_KEY_ENV_VAR, Err, |private_key| {
                    let client = VeoApi::new(project_id, client_email, private_key);
                    generate_video(&client, input, config)
                })
            })
        })
    }

    fn poll(job_id: String) -> Result<VideoResult, VideoError> {
        with_config_key(Self::PROJECT_ID_ENV_VAR, Err, |project_id| {
            with_config_key(Self::CLIENT_EMAIL_ENV_VAR, Err, |client_email| {
                with_config_key(Self::PRIVATE_KEY_ENV_VAR, Err, |private_key| {
                    let client = VeoApi::new(project_id, client_email, private_key);
                    poll_video_generation(&client, job_id)
                })
            })
        })
    }

    fn cancel(job_id: String) -> Result<String, VideoError> {
        with_config_key(Self::PROJECT_ID_ENV_VAR, Err, |project_id| {
            with_config_key(Self::CLIENT_EMAIL_ENV_VAR, Err, |client_email| {
                with_config_key(Self::PRIVATE_KEY_ENV_VAR, Err, |private_key| {
                    let client = VeoApi::new(project_id, client_email, private_key);
                    cancel_video_generation(&client, job_id)
                })
            })
        })
    }
}

impl LipSyncProvider for Veo {
    fn generate_lip_sync(video: LipSyncVideo, audio: AudioSource) -> Result<String, VideoError> {
        with_config_key(Self::PROJECT_ID_ENV_VAR, Err, |project_id| {
            with_config_key(Self::CLIENT_EMAIL_ENV_VAR, Err, |client_email| {
                with_config_key(Self::PRIVATE_KEY_ENV_VAR, Err, |private_key| {
                    let client = VeoApi::new(project_id, client_email, private_key);
                    generate_lip_sync_video(&client, video, audio)
                })
            })
        })
    }

    fn list_voices(language: Option<String>) -> Result<Vec<VoiceInfo>, VideoError> {
        with_config_key(Self::PROJECT_ID_ENV_VAR, Err, |project_id| {
            with_config_key(Self::CLIENT_EMAIL_ENV_VAR, Err, |client_email| {
                with_config_key(Self::PRIVATE_KEY_ENV_VAR, Err, |private_key| {
                    let client = VeoApi::new(project_id, client_email, private_key);
                    list_available_voices(&client, language)
                })
            })
        })
    }
}

impl AdvancedVideoGenerationProvider for Veo {
    fn extend_video(options: ExtendVideoOptions) -> Result<String, VideoError> {
        with_config_key(Self::PROJECT_ID_ENV_VAR, Err, |project_id| {
            with_config_key(Self::CLIENT_EMAIL_ENV_VAR, Err, |client_email| {
                with_config_key(Self::PRIVATE_KEY_ENV_VAR, Err, |private_key| {
                    let client = VeoApi::new(project_id, client_email, private_key);
                    extend_video(
                        &client,
                        options.video_id,
                        options.prompt,
                        options.negative_prompt,
                        options.cfg_scale,
                        options.provider_options,
                    )
                })
            })
        })
    }

    fn upscale_video(input: BaseVideo) -> Result<String, VideoError> {
        with_config_key(Self::PROJECT_ID_ENV_VAR, Err, |project_id| {
            with_config_key(Self::CLIENT_EMAIL_ENV_VAR, Err, |client_email| {
                with_config_key(Self::PRIVATE_KEY_ENV_VAR, Err, |private_key| {
                    let client = VeoApi::new(project_id, client_email, private_key);
                    upscale_video(&client, input)
                })
            })
        })
    }

    fn generate_video_effects(options: GenerateVideoEffectsOptions) -> Result<String, VideoError> {
        with_config_key(Self::PROJECT_ID_ENV_VAR, Err, |project_id| {
            with_config_key(Self::CLIENT_EMAIL_ENV_VAR, Err, |client_email| {
                with_config_key(Self::PRIVATE_KEY_ENV_VAR, Err, |private_key| {
                    let client = VeoApi::new(project_id, client_email, private_key);
                    generate_video_effects(
                        &client,
                        options.input,
                        options.effect,
                        options.model,
                        options.duration,
                        options.mode,
                    )
                })
            })
        })
    }

    fn multi_image_generation(options: MultImageGenerationOptions) -> Result<String, VideoError> {
        with_config_key(Self::PROJECT_ID_ENV_VAR, Err, |project_id| {
            with_config_key(Self::CLIENT_EMAIL_ENV_VAR, Err, |client_email| {
                with_config_key(Self::PRIVATE_KEY_ENV_VAR, Err, |private_key| {
                    let client = VeoApi::new(project_id, client_email, private_key);
                    multi_image_generation(
                        &client,
                        options.input_images,
                        options.prompt,
                        options.config,
                    )
                })
            })
        })
    }
}

impl ExtendedVideoGenerationProvider for Veo {}

pub type DurableVeo = DurableVideo<Veo>;
