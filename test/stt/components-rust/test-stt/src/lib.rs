use std::fs;
use std::io::Read;

use golem_ai_stt::model::transcription::TranscriptionRequest;
use golem_ai_stt::model::types::{AudioConfig, AudioFormat};
use golem_ai_stt::{LanguageProvider, TranscriptionProvider};
use golem_rust::{agent_definition, agent_implementation};

#[cfg(feature = "whisper")]
type Provider = golem_ai_stt_whisper::DurableWhisperStt;
#[cfg(feature = "aws")]
type Provider = golem_ai_stt_aws::DurableAwsStt;
#[cfg(feature = "azure")]
type Provider = golem_ai_stt_azure::DurableAzureStt;
#[cfg(feature = "deepgram")]
type Provider = golem_ai_stt_deepgram::DurableDeepgramStt;
#[cfg(feature = "google")]
type Provider = golem_ai_stt_google::DurableGoogleStt;

#[agent_definition]
pub trait SttTest {
    fn new(name: String) -> Self;
    fn test_transcribe(&self) -> Result<String, String>;
    fn test_transcribe_many(&self) -> Result<String, String>;
    fn test_list_supported_languages(&self) -> Result<String, String>;
}

struct SttTestImpl {
    _name: String,
}

#[agent_implementation]
impl SttTest for SttTestImpl {
    fn new(name: String) -> Self {
        Self { _name: name }
    }

    fn test_transcribe(&self) -> Result<String, String> {
        let file_path = "/samples/jfk.mp3";
        let audio_bytes = read_file_to_bytes(file_path).expect("Should work");

        let request = TranscriptionRequest {
            request_id: "transcribe-jfk-mp3".to_string(),
            audio: audio_bytes,
            config: AudioConfig {
                format: AudioFormat::Mp3,
                sample_rate: None,
                channels: None,
            },
            options: None,
        };

        match Provider::transcribe(request) {
            Ok(res) => Ok(format!("{res:?}")),
            Err(err) => Err(format!("error: {err:?}")),
        }
    }

    fn test_transcribe_many(&self) -> Result<String, String> {
        let file_path_1 = "/samples/jfk.mp3";
        let file_path_2 = "/samples/mm1.wav";

        let audio_bytes_1 = read_file_to_bytes(file_path_1).expect("Should work");
        let audio_bytes_2 = read_file_to_bytes(file_path_2).expect("Should work");

        let request_1 = TranscriptionRequest {
            request_id: "transcribe-jfk-mp3".to_string(),
            audio: audio_bytes_1,
            config: AudioConfig {
                format: AudioFormat::Mp3,
                sample_rate: None,
                channels: None,
            },
            options: None,
        };

        let request_2 = TranscriptionRequest {
            request_id: "transcribe-mm1-wav".to_string(),
            audio: audio_bytes_2,
            config: AudioConfig {
                format: AudioFormat::Wav,
                sample_rate: None,
                channels: None,
            },
            options: None,
        };

        match Provider::transcribe_many(vec![request_1, request_2]) {
            Ok(res) => {
                let successes: Vec<_> = res.successes.iter().map(|tr| format!("{tr:?}")).collect();
                let failures: Vec<_> = res.failures.iter().map(|tr| format!("{tr:?}")).collect();
                Ok(format!("successes = {successes:?}, failures {failures:?}"))
            }
            Err(err) => Err(format!("multi transcription error: {err:?}")),
        }
    }

    fn test_list_supported_languages(&self) -> Result<String, String> {
        match Provider::list_languages() {
            Ok(languages) => Ok(format!("{languages:?}")),
            Err(err) => Err(format!("error: {err:?}")),
        }
    }
}

fn read_file_to_bytes(path: &str) -> std::io::Result<Vec<u8>> {
    let mut file = fs::File::open(path)?;
    let metadata = file.metadata()?;
    let file_size = metadata.len() as usize;
    let mut buffer = Vec::with_capacity(file_size);
    file.read_to_end(&mut buffer)?;
    Ok(buffer)
}
