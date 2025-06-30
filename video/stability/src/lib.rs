mod client;
mod conversion;

use crate::client::StabilityApi;
use crate::conversion::{
    cancel_video_generation, generate_lip_sync_video, generate_video, list_available_voices,
    poll_video_generation,
};
use golem_video::config::with_config_key;
use golem_video::durability::{DurableVideo, ExtendedGuest};
use golem_video::exports::golem::video::video::{
    AudioSource, BaseVideo, GenerationConfig, MediaInput, VideoError, VoiceInfo,
};
use golem_video::exports::golem::video::video::{Guest, VideoResult};
use golem_video::LOGGING_STATE;

struct StabilityComponent;

impl StabilityComponent {
    const ENV_VAR_NAME: &'static str = "STABILITY_API_KEY";
}

impl Guest for StabilityComponent {
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

impl ExtendedGuest for StabilityComponent {}

type DurableStabilityComponent = DurableVideo<StabilityComponent>;

golem_video::export_video!(DurableStabilityComponent with_types_in golem_video);
