pub mod types {
    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub enum VideoError {
        InvalidInput(String),
        UnsupportedFeature(String),
        QuotaExceeded,
        GenerationFailed(String),
        Cancelled,
        InternalError(String),
    }

    impl core::fmt::Display for VideoError {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            write!(f, "{:?}", self)
        }
    }

    impl std::error::Error for VideoError {}

    #[repr(u8)]
    #[derive(
        Clone,
        Copy,
        Debug,
        Eq,
        Ord,
        PartialEq,
        PartialOrd,
        golem_rust::FromValueAndType,
        golem_rust::IntoValue,
    )]
    pub enum ImageRole {
        First,
        Last,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct RawBytes {
        pub bytes: Vec<u8>,
        pub mime_type: String,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub enum MediaData {
        Url(String),
        Bytes(RawBytes),
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct InputImage {
        pub data: MediaData,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct Reference {
        pub data: InputImage,
        pub prompt: Option<String>,
        pub role: Option<ImageRole>,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct BaseVideo {
        pub data: MediaData,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub enum MediaInput {
        Text(String),
        Image(Reference),
        Video(BaseVideo),
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct Narration {
        pub data: MediaData,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct StaticMask {
        pub mask: InputImage,
    }

    #[derive(
        Clone, Copy, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue,
    )]
    pub struct Position {
        pub x: u32,
        pub y: u32,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct DynamicMask {
        pub mask: InputImage,
        pub trajectories: Vec<Position>,
    }

    #[derive(
        Clone, Copy, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue,
    )]
    pub struct CameraConfig {
        pub horizontal: f32,
        pub vertical: f32,
        pub pan: f32,
        pub tilt: f32,
        pub zoom: f32,
        pub roll: f32,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub enum CameraMovement {
        Simple(CameraConfig),
        DownBack,
        ForwardUp,
        RightTurnForward,
        LeftTurnForward,
    }

    #[repr(u8)]
    #[derive(
        Clone,
        Copy,
        Debug,
        Eq,
        Ord,
        PartialEq,
        PartialOrd,
        golem_rust::FromValueAndType,
        golem_rust::IntoValue,
    )]
    pub enum AspectRatio {
        Square,
        Portrait,
        Landscape,
        Cinema,
    }

    #[repr(u8)]
    #[derive(
        Clone,
        Copy,
        Debug,
        Eq,
        Ord,
        PartialEq,
        PartialOrd,
        golem_rust::FromValueAndType,
        golem_rust::IntoValue,
    )]
    pub enum Resolution {
        Sd,
        Hd,
        Fhd,
        Uhd,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct Kv {
        pub key: String,
        pub value: String,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct GenerationConfig {
        pub negative_prompt: Option<String>,
        pub seed: Option<u64>,
        pub scheduler: Option<String>,
        pub guidance_scale: Option<f32>,
        pub aspect_ratio: Option<AspectRatio>,
        pub duration_seconds: Option<f32>,
        pub resolution: Option<Resolution>,
        pub model: Option<String>,
        pub enable_audio: Option<bool>,
        pub enhance_prompt: Option<bool>,
        pub provider_options: Option<Vec<Kv>>,
        pub lastframe: Option<InputImage>,
        pub static_mask: Option<StaticMask>,
        pub dynamic_mask: Option<DynamicMask>,
        pub camera_control: Option<CameraMovement>,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct Video {
        pub uri: Option<String>,
        pub base64_bytes: Option<Vec<u8>>,
        pub mime_type: String,
        pub width: Option<u32>,
        pub height: Option<u32>,
        pub fps: Option<f32>,
        pub duration_seconds: Option<f32>,
        pub generation_id: Option<String>,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub enum JobStatus {
        Pending,
        Running,
        Succeeded,
        Failed(String),
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct VideoResult {
        pub status: JobStatus,
        pub videos: Option<Vec<Video>>,
    }

    #[repr(u8)]
    #[derive(
        Clone,
        Copy,
        Debug,
        Eq,
        Ord,
        PartialEq,
        PartialOrd,
        golem_rust::FromValueAndType,
        golem_rust::IntoValue,
    )]
    pub enum VoiceLanguage {
        En,
        Zh,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct TextToSpeech {
        pub text: String,
        pub voice_id: String,
        pub language: VoiceLanguage,
        pub speed: f32,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub enum AudioSource {
        FromText(TextToSpeech),
        FromAudio(Narration),
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct VoiceInfo {
        pub voice_id: String,
        pub name: String,
        pub language: VoiceLanguage,
        pub preview_url: Option<String>,
    }

    #[repr(u8)]
    #[derive(
        Clone,
        Copy,
        Debug,
        Eq,
        Ord,
        PartialEq,
        PartialOrd,
        golem_rust::FromValueAndType,
        golem_rust::IntoValue,
    )]
    pub enum SingleImageEffects {
        Bloombloom,
        Dizzydizzy,
        Fuzzyfuzzy,
        Squish,
        Expansion,
        AnimeFigure,
        Rocketrocket,
    }

    #[repr(u8)]
    #[derive(
        Clone,
        Copy,
        Debug,
        Eq,
        Ord,
        PartialEq,
        PartialOrd,
        golem_rust::FromValueAndType,
        golem_rust::IntoValue,
    )]
    pub enum DualImageEffects {
        Hug,
        Kiss,
        HeartGesture,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct DualEffect {
        pub effect: DualImageEffects,
        pub second_image: InputImage,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub enum EffectType {
        Single(SingleImageEffects),
        Dual(DualEffect),
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub enum LipSyncVideo {
        Video(BaseVideo),
        VideoId(String),
    }
}

pub mod video_generation {
    pub type MediaInput = super::types::MediaInput;
    pub type GenerationConfig = super::types::GenerationConfig;
    pub type VideoResult = super::types::VideoResult;
    pub type VideoError = super::types::VideoError;
}

pub mod lip_sync {
    pub type BaseVideo = super::types::BaseVideo;
    pub type AudioSource = super::types::AudioSource;
    pub type VideoError = super::types::VideoError;
    pub type VoiceInfo = super::types::VoiceInfo;
    pub type LipSyncVideo = super::types::LipSyncVideo;
}

pub mod advanced {
    pub type VideoError = super::types::VideoError;
    pub type Kv = super::types::Kv;
    pub type BaseVideo = super::types::BaseVideo;
    pub type GenerationConfig = super::types::GenerationConfig;
    pub type InputImage = super::types::InputImage;
    pub type EffectType = super::types::EffectType;

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct ExtendVideoOptions {
        pub video_id: String,
        pub prompt: Option<String>,
        pub negative_prompt: Option<String>,
        pub cfg_scale: Option<f32>,
        pub provider_options: Option<Vec<Kv>>,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct GenerateVideoEffectsOptions {
        pub input: InputImage,
        pub effect: EffectType,
        pub model: Option<String>,
        pub duration: Option<f32>,
        pub mode: Option<String>,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct MultImageGenerationOptions {
        pub input_images: Vec<InputImage>,
        pub prompt: Option<String>,
        pub config: GenerationConfig,
    }
}
