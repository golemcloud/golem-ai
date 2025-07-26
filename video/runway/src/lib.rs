mod client;
mod conversion;

use crate::client::RunwayApi;
use crate::conversion::{
    cancel_video_generation, extend_video, generate_lip_sync_video, generate_video,
    generate_video_effects, list_available_voices, multi_image_generation, poll_video_generation,
    upscale_video,
};
use golem_video::config::with_config_key;
use golem_video::durability::{DurableVideo, ExtendedGuest};
use golem_video::exports::golem::video_generation::advanced::Guest as AdvancedGuest;
use golem_video::exports::golem::video_generation::lip_sync::Guest as LipSyncGuest;
use golem_video::exports::golem::video_generation::types::{
    AudioSource, BaseVideo, EffectType, GenerationConfig, InputImage, Kv, LipSyncVideo, MediaInput,
    VideoError, VideoResult, VoiceInfo,
};
use golem_video::exports::golem::video_generation::video_generation::Guest as VideoGenerationGuest;

struct RunwayComponent;

impl RunwayComponent {
    const ENV_VAR_NAME: &'static str = "RUNWAY_API_KEY";
}

impl VideoGenerationGuest for RunwayComponent {
    fn generate(input: MediaInput, config: GenerationConfig) -> Result<String, VideoError> {
        with_config_key(Self::ENV_VAR_NAME, Err, |api_key| {
            let client = RunwayApi::new(api_key);
            generate_video(&client, input, config)
        })
    }

    fn poll(job_id: String) -> Result<VideoResult, VideoError> {
        with_config_key(Self::ENV_VAR_NAME, Err, |api_key| {
            let client = RunwayApi::new(api_key);
            poll_video_generation(&client, job_id)
        })
    }

    fn cancel(job_id: String) -> Result<String, VideoError> {
        with_config_key(Self::ENV_VAR_NAME, Err, |api_key| {
            let client = RunwayApi::new(api_key);
            cancel_video_generation(&client, job_id)
        })
    }
}

impl LipSyncGuest for RunwayComponent {
    fn generate_lip_sync(video: LipSyncVideo, audio: AudioSource) -> Result<String, VideoError> {
        with_config_key(Self::ENV_VAR_NAME, Err, |api_key| {
            let client = RunwayApi::new(api_key);
            generate_lip_sync_video(&client, video, audio)
        })
    }

    fn list_voices(language: Option<String>) -> Result<Vec<VoiceInfo>, VideoError> {
        with_config_key(Self::ENV_VAR_NAME, Err, |api_key| {
            let client = RunwayApi::new(api_key);
            list_available_voices(&client, language)
        })
    }
}

impl AdvancedGuest for RunwayComponent {
    fn extend_video(
        video_id: String,
        prompt: Option<String>,
        negative_prompt: Option<String>,
        cfg_scale: Option<f32>,
        provider_options: Option<Vec<Kv>>,
    ) -> Result<String, VideoError> {
        with_config_key(Self::ENV_VAR_NAME, Err, |api_key| {
            let client = RunwayApi::new(api_key);
            extend_video(
                &client,
                video_id,
                prompt,
                negative_prompt,
                cfg_scale,
                provider_options,
            )
        })
    }

    fn upscale_video(input: BaseVideo) -> Result<String, VideoError> {
        with_config_key(Self::ENV_VAR_NAME, Err, |api_key| {
            let client = RunwayApi::new(api_key);
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
        with_config_key(Self::ENV_VAR_NAME, Err, |api_key| {
            let client = RunwayApi::new(api_key);
            generate_video_effects(&client, input, effect, model, duration, mode)
        })
    }

    fn multi_image_generation(
        input_images: Vec<InputImage>,
        prompt: Option<String>,
        config: GenerationConfig,
    ) -> Result<String, VideoError> {
        with_config_key(Self::ENV_VAR_NAME, Err, |api_key| {
            let client = RunwayApi::new(api_key);
            multi_image_generation(&client, input_images, prompt, config)
        })
    }
}

impl ExtendedGuest for RunwayComponent {}

type DurableRunwayComponent = DurableVideo<RunwayComponent>;

golem_video::export_video!(DurableRunwayComponent with_types_in golem_video);
