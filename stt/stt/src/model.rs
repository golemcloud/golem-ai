pub mod types {
    pub type LanguageCode = String;

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub enum SttError {
        InvalidAudio(String),
        UnsupportedFormat(String),
        UnsupportedLanguage(String),
        TranscriptionFailed(String),
        Unauthorized(String),
        AccessDenied(String),
        RateLimited(String),
        InsufficientCredits,
        UnsupportedOperation(String),
        ServiceUnavailable(String),
        NetworkError(String),
        InternalError(String),
    }

    impl core::fmt::Display for SttError {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                SttError::InvalidAudio(e) => write!(f, "Invalid audio: {e}"),
                SttError::UnsupportedFormat(e) => write!(f, "Unsupported format: {e}"),
                SttError::UnsupportedLanguage(e) => {
                    write!(f, "Unsupported language: {e}")
                }
                SttError::TranscriptionFailed(e) => {
                    write!(f, "Transcription failed: {e}")
                }
                SttError::Unauthorized(e) => write!(f, "Unauthorized: {e}"),
                SttError::AccessDenied(e) => write!(f, "Access denied: {e}"),
                SttError::RateLimited(e) => write!(f, "Rate limited: {e}"),
                SttError::InsufficientCredits => write!(f, "Insufficient credits"),
                SttError::UnsupportedOperation(e) => {
                    write!(f, "Unsupported operation: {e}")
                }
                SttError::ServiceUnavailable(e) => {
                    write!(f, "Service unavailable: {e}")
                }
                SttError::NetworkError(e) => write!(f, "Network error: {e}"),
                SttError::InternalError(e) => write!(f, "Internal error: {e}"),
            }
        }
    }

    impl std::error::Error for SttError {}

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
    pub enum AudioFormat {
        Wav,
        Mp3,
        Flac,
        Ogg,
        Aac,
        Pcm,
    }

    #[derive(
        Clone, Copy, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue,
    )]
    pub struct AudioConfig {
        pub format: AudioFormat,
        pub sample_rate: Option<u32>,
        pub channels: Option<u8>,
    }

    #[derive(
        Clone, Copy, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue,
    )]
    pub struct TimingInfo {
        pub start_time_seconds: f32,
        pub end_time_seconds: f32,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct WordSegment {
        pub text: String,
        pub timing_info: Option<TimingInfo>,
        pub confidence: Option<f32>,
        pub speaker_id: Option<String>,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct TranscriptionMetadata {
        pub duration_seconds: f32,
        pub audio_size_bytes: u32,
        pub request_id: String,
        pub model: Option<String>,
        pub language: LanguageCode,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct TranscriptionSegment {
        pub transcript: String,
        pub timing_info: Option<TimingInfo>,
        pub speaker_id: Option<String>,
        pub words: Vec<WordSegment>,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct TranscriptionChannel {
        pub id: String,
        pub transcript: String,
        pub segments: Vec<TranscriptionSegment>,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct TranscriptionResult {
        pub transcript_metadata: TranscriptionMetadata,
        pub channels: Vec<TranscriptionChannel>,
    }
}

pub mod languages {
    pub type LanguageCode = super::types::LanguageCode;
    pub type SttError = super::types::SttError;

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct LanguageInfo {
        pub code: LanguageCode,
        pub name: String,
        pub native_name: String,
    }
}

pub mod transcription {
    pub type AudioConfig = super::types::AudioConfig;
    pub type TranscriptionResult = super::types::TranscriptionResult;
    pub type SttError = super::types::SttError;
    pub type LanguageCode = super::types::LanguageCode;

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct Phrase {
        pub value: String,
        pub boost: Option<f32>,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct Vocabulary {
        pub phrases: Vec<Phrase>,
    }

    #[derive(
        Clone, Copy, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue,
    )]
    pub struct DiarizationOptions {
        pub enabled: bool,
        pub min_speaker_count: Option<u32>,
        pub max_speaker_count: Option<u32>,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct TranscribeOptions {
        pub language: Option<LanguageCode>,
        pub model: Option<String>,
        pub profanity_filter: Option<bool>,
        pub vocabulary: Option<Vocabulary>,
        pub diarization: Option<DiarizationOptions>,
        pub enable_multi_channel: Option<bool>,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct TranscriptionRequest {
        pub request_id: String,
        pub audio: Vec<u8>,
        pub config: AudioConfig,
        pub options: Option<TranscribeOptions>,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct FailedTranscription {
        pub request_id: String,
        pub error: SttError,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct MultiTranscriptionResult {
        pub successes: Vec<TranscriptionResult>,
        pub failures: Vec<FailedTranscription>,
    }
}
