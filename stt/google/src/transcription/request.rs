use bytes::Bytes;

#[allow(unused)]
#[derive(Debug, Clone, PartialEq)]
pub enum AudioFormat {
    LinearPcm,
    Flac,
    Mp3,
    OggOpus,
    WebmOpus,
    AmrNb,
    AmrWb,
    Wav,
    Mp4,
    M4a,
    Mov,
}

impl std::fmt::Display for AudioFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AudioFormat::LinearPcm => write!(f, "LINEAR16"),
            AudioFormat::Flac => write!(f, "FLAC"),
            AudioFormat::Mp3 => write!(f, "MP3"),
            AudioFormat::OggOpus => write!(f, "OGG_OPUS"),
            AudioFormat::WebmOpus => write!(f, "WEBM_OPUS"),
            AudioFormat::AmrNb => write!(f, "AMR"),
            AudioFormat::AmrWb => write!(f, "AMR_WB"),
            AudioFormat::Wav => write!(f, "WAV_LINEAR16"),
            AudioFormat::Mp4 => write!(f, "MP4_AAC"),
            AudioFormat::M4a => write!(f, "M4A_AAC"),
            AudioFormat::Mov => write!(f, "MOV_AAC"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AudioConfig {
    pub format: AudioFormat,
    pub sample_rate_hertz: Option<u32>,
    pub channels: Option<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TranscriptionConfig {
    pub language_codes: Option<Vec<String>>,
    pub model: Option<String>,
    pub enable_profanity_filter: bool,
    pub diarization: Option<DiarizationConfig>,
    pub enable_multi_channel: bool,
    pub phrases: Vec<Phrase>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiarizationConfig {
    pub enabled: bool,
    pub min_speaker_count: Option<i32>,
    pub max_speaker_count: Option<i32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Phrase {
    pub value: String,
    pub boost: Option<f32>,
}

pub struct TranscriptionRequest {
    pub request_id: String,
    pub audio: Bytes,
    pub audio_config: AudioConfig,
    pub transcription_config: Option<TranscriptionConfig>,
}
