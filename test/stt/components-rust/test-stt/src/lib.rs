use std::fs;
use std::io::Read;

#[allow(static_mut_refs)]
mod bindings;

use crate::bindings::exports::test::stt_exports::test_stt_api::*;
use crate::bindings::golem::stt::languages::list_languages;
use crate::bindings::golem::stt::transcription::{
    transcribe, transcribe_many, TranscriptionRequest as WitTranscriptionRequest,
};
use crate::bindings::golem::stt::types::{
    AudioConfig as WitAudioConfig, AudioFormat as WitAudioFormat,
};

struct Component;

impl Guest for Component {
    fn test_transcribe() -> Result<String, String> {
        let file_path = "/samples/jfk.mp3";

        let audio_bytes = read_file_to_bytes(file_path).expect("Should work");

        let wit_transcription_request = WitTranscriptionRequest {
            request_id: "transcribe-jfk-mp3".to_string(),
            audio: audio_bytes,
            config: WitAudioConfig {
                format: WitAudioFormat::Mp3,
                sample_rate: None,
                channels: None,
            },
            options: None,
        };

        match transcribe(&wit_transcription_request) {
            Ok(res) => Ok(format!("{res:?}")),
            Err(err) => Err(format!("error: {err:?}")),
        }
    }

    fn test_transcribe_many() -> Result<String, String> {
        let file_path_1 = "/samples/jfk.mp3";
        let file_path_2 = "/samples/mm1.wav";

        let audio_bytes_1 = read_file_to_bytes(file_path_1).expect("Should work");
        let audio_bytes_2 = read_file_to_bytes(file_path_2).expect("Should work");

        let wit_transcription_request_1 = WitTranscriptionRequest {
            request_id: "transcribe-jfk-mp3".to_string(),
            audio: audio_bytes_1,
            config: WitAudioConfig {
                format: WitAudioFormat::Mp3,
                sample_rate: None,
                channels: None,
            },
            options: None,
        };

        let wit_transcription_request_2 = WitTranscriptionRequest {
            request_id: "transcribe-mm1-wav".to_string(),
            audio: audio_bytes_2,
            config: WitAudioConfig {
                format: WitAudioFormat::Wav,
                sample_rate: None,
                channels: None,
            },
            options: None,
        };

        match transcribe_many(&vec![
            wit_transcription_request_1,
            wit_transcription_request_2,
        ]) {
            Ok(res) => {
                let successes: Vec<_> = res.successes.iter().map(|tr| format!("{tr:?}")).collect();
                let failures: Vec<_> = res.failures.iter().map(|tr| format!("{tr:?}")).collect();

                Ok(format!("successes = {successes:?}, failures {failures:?}"))
            }
            Err(err) => Err(format!("multi transcription error: {err:?}")),
        }
    }

    fn test_list_supported_languages() -> Result<String, String> {
        match list_languages() {
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

bindings::export!(Component with_types_in bindings);
