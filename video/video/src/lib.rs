pub mod config;
pub mod durability;
pub mod error;
pub mod model;
pub mod utils;

use crate::model::advanced::{
    BaseVideo, ExtendVideoOptions, GenerateVideoEffectsOptions, MultImageGenerationOptions,
};
use crate::model::lip_sync::{AudioSource, LipSyncVideo, VoiceInfo};
use crate::model::video_generation::{GenerationConfig, MediaInput, VideoError, VideoResult};
use std::cell::RefCell;
use std::str::FromStr;

pub trait VideoGenerationProvider {
    /// Provider-specific configuration (API keys, base URLs, etc.) that the
    /// caller resolves once and passes in. Each provider crate defines its
    /// own concrete config type; see e.g. `golem_ai_video_runway::RunwayConfig`.
    type ProviderConfig: Clone + 'static;

    fn generate(
        provider_config: Self::ProviderConfig,
        input: MediaInput,
        config: GenerationConfig,
    ) -> Result<String, VideoError>;
    fn poll(
        provider_config: Self::ProviderConfig,
        job_id: String,
    ) -> Result<VideoResult, VideoError>;
    fn cancel(provider_config: Self::ProviderConfig, job_id: String) -> Result<String, VideoError>;
}

pub trait LipSyncProvider {
    /// Provider-specific configuration; see [`VideoGenerationProvider::ProviderConfig`].
    type ProviderConfig: Clone + 'static;

    fn generate_lip_sync(
        provider_config: Self::ProviderConfig,
        video: LipSyncVideo,
        audio: AudioSource,
    ) -> Result<String, model::lip_sync::VideoError>;
    fn list_voices(
        provider_config: Self::ProviderConfig,
        language: Option<String>,
    ) -> Result<Vec<VoiceInfo>, model::lip_sync::VideoError>;
}

pub trait AdvancedVideoGenerationProvider {
    /// Provider-specific configuration; see [`VideoGenerationProvider::ProviderConfig`].
    type ProviderConfig: Clone + 'static;

    fn extend_video(
        provider_config: Self::ProviderConfig,
        options: ExtendVideoOptions,
    ) -> Result<String, model::advanced::VideoError>;
    fn upscale_video(
        provider_config: Self::ProviderConfig,
        input: BaseVideo,
    ) -> Result<String, model::advanced::VideoError>;
    fn generate_video_effects(
        provider_config: Self::ProviderConfig,
        options: GenerateVideoEffectsOptions,
    ) -> Result<String, model::advanced::VideoError>;
    fn multi_image_generation(
        provider_config: Self::ProviderConfig,
        options: MultImageGenerationOptions,
    ) -> Result<String, model::advanced::VideoError>;
}

pub struct LoggingState {
    logging_initialized: bool,
}

impl LoggingState {
    /// Initializes WASI logging based on the `GOLEM_VIDEO_LOG` environment variable.
    fn init(&mut self) {
        if !self.logging_initialized {
            let _ = wasi_logger::Logger::install();
            let max_level: log::LevelFilter =
                log::LevelFilter::from_str(&std::env::var("GOLEM_VIDEO_LOG").unwrap_or_default())
                    .unwrap_or(log::LevelFilter::Info);
            log::set_max_level(max_level);
            self.logging_initialized = true;
        }
    }
}

thread_local! {
    /// This holds the state of our application.
    static LOGGING_STATE: RefCell<LoggingState> = const { RefCell::new(LoggingState {
        logging_initialized: false,
    }) };
}

pub fn init_logging() {
    LOGGING_STATE.with_borrow_mut(|state| state.init());
}
