mod client;
mod conversion;

use crate::client::StabilityApi;
use crate::conversion::{
    cancel_video_generation, extend_video, generate_lip_sync_video, generate_video,
    generate_video_effects, list_available_voices, multi_image_generation, poll_video_generation,
    upscale_video,
};
use golem_video::config::with_config_key;
use golem_video::durability::{DurableVideo, ExtendedGuest};
use golem_video::exports::golem::video::advanced::Guest as AdvancedGuest;
use golem_video::exports::golem::video::lip_sync::Guest as LipSyncGuest;
use golem_video::exports::golem::video::types::{
    AudioSource, BaseVideo, EffectType, GenerationConfig, InputImage, MediaInput, VideoError,
    VideoResult, VoiceInfo,
};
use golem_video::exports::golem::video::video_generation::Guest as VideoGenerationGuest;
use golem_video::LOGGING_STATE;

struct StabilityComponent;

impl StabilityComponent {
    const ENV_VAR_NAME: &'static str = "STABILITY_API_KEY";
}

impl VideoGenerationGuest for StabilityComponent {
    fn generate(input: MediaInput, config: GenerationConfig) -> Result<String, VideoError> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());
        with_config_key(
            Self::ENV_VAR_NAME,
            |err| {
                // Return the error from the config lookup
                Err(err)
            },
            |api_key| {
                let client = StabilityApi::new(api_key);
                generate_video(&client, input, config)
            },
        )
    }

    fn poll(job_id: String) -> Result<VideoResult, VideoError> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());
        with_config_key(Self::ENV_VAR_NAME, Err, |api_key| {
            let client = StabilityApi::new(api_key);
            poll_video_generation(&client, job_id)
        })
    }

    fn cancel(job_id: String) -> Result<String, VideoError> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());
        cancel_video_generation(job_id)
    }
}

impl LipSyncGuest for StabilityComponent {
    fn generate_lip_sync(video: BaseVideo, audio: AudioSource) -> Result<String, VideoError> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());
        with_config_key(
            Self::ENV_VAR_NAME,
            |err| {
                // Return the error from the config lookup
                Err(err)
            },
            |api_key| {
                let client = StabilityApi::new(api_key);
                generate_lip_sync_video(&client, video, audio)
            },
        )
    }

    fn list_voices(language: Option<String>) -> Result<Vec<VoiceInfo>, VideoError> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());
        with_config_key(Self::ENV_VAR_NAME, Err, |api_key| {
            let client = StabilityApi::new(api_key);
            list_available_voices(&client, language)
        })
    }
}

impl AdvancedGuest for StabilityComponent {
    fn extend_video(input: BaseVideo, config: GenerationConfig) -> Result<String, VideoError> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());
        with_config_key(Self::ENV_VAR_NAME, Err, |api_key| {
            let client = StabilityApi::new(api_key);
            extend_video(&client, input, config)
        })
    }

    fn upscale_video(input: BaseVideo) -> Result<String, VideoError> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());
        with_config_key(Self::ENV_VAR_NAME, Err, |api_key| {
            let client = StabilityApi::new(api_key);
            upscale_video(&client, input)
        })
    }

    fn generate_video_effects(
        input: InputImage,
        effect: EffectType,
        model: Option<String>,
        duration: Option<f32>,
        mode: Option<String>,
    ) -> Result<String, VideoError> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());
        with_config_key(Self::ENV_VAR_NAME, Err, |api_key| {
            let client = StabilityApi::new(api_key);
            generate_video_effects(&client, input, effect, model, duration, mode)
        })
    }

    fn multi_image_generation(
        input_images: Vec<InputImage>,
        config: GenerationConfig,
    ) -> Result<String, VideoError> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());
        with_config_key(Self::ENV_VAR_NAME, Err, |api_key| {
            let client = StabilityApi::new(api_key);
            multi_image_generation(&client, input_images, config)
        })
    }
}

impl ExtendedGuest for StabilityComponent {}

type DurableStabilityComponent = DurableVideo<StabilityComponent>;

golem_video::export_video!(DurableStabilityComponent with_types_in golem_video);
