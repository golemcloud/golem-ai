use std::cell::RefCell;

use bytes::Bytes;
use golem_rust::{FromValueAndType, IntoValue};
use golem_tts::client::TtsClient;
use golem_tts::golem::tts::advanced::{
    GuestLongFormOperation, GuestPronunciationLexicon, LongFormResult, OperationStatus, TtsError,
};
use golem_tts::golem::tts::types::LanguageCode;
use http::Request;
use reqwest::{header::{HeaderMap, HeaderName, HeaderValue}, Method};
use serde::{Deserialize, Serialize};

use crate::{
    error::{from_http_error, unsupported},
    polly::Polly,
    types::{AwsLexicon, GetSpeechSynthesisTaskResponse, LexiconAttributes, SynthesisTask},
};

#[derive(Serialize, Deserialize, Debug, Clone, IntoValue, FromValueAndType)]
pub struct VoiceResponse {
    #[serde(rename = "AdditionalLanguageCodes")]
    pub additional_language_codes: Option<Vec<String>>,
    #[serde(rename = "Gender")]
    pub gender: String,
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "LanguageCode")]
    pub language_code: String,
    #[serde(rename = "LanguageName")]
    pub language_name: String,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "SupportedEngines")]
    pub supported_engines: Option<Vec<String>>,
}



pub struct AwsPronunciationLexicon {
    lexicon: RefCell<AwsLexicon>,
    language_code: RefCell<String>,
    entry_count: RefCell<u32>,
    _lexicon_attributes: RefCell<LexiconAttributes>,
}

impl AwsPronunciationLexicon {
    pub fn new(
        lexicon: AwsLexicon,
        language_code: String,
        lexicon_attributes: LexiconAttributes,
    ) -> Self {
        Self {
            lexicon: RefCell::new(lexicon),
            language_code: RefCell::new(language_code),
            entry_count: RefCell::new(0),
            _lexicon_attributes: RefCell::new(lexicon_attributes),
        }
    }
}

impl GuestPronunciationLexicon for AwsPronunciationLexicon {
    #[doc = " Get lexicon name"]
    fn get_name(&self) -> String {
        self.lexicon.borrow().name.clone()
    }

    #[doc = " Get supported language"]
    fn get_language(&self) -> LanguageCode {
        self.language_code.borrow().clone()
    }

    #[doc = " Get number of entries"]
    fn get_entry_count(&self) -> u32 {
        *self.entry_count.borrow()
    }

    #[doc = " Add pronunciation entry"]
    fn add_entry(&self, _word: String, _pronunciation: String) -> Result<(), TtsError> {
        unsupported(
            "Adding entries to existing lexicon not supported. Create a new lexicon instead.",
        )
    }

    #[doc = " Export lexicon content"]
    fn export_content(&self) -> Result<String, TtsError> {
        Ok(self.lexicon.borrow().content.clone())
    }

    #[doc = " Remove entry by word"]
    fn remove_entry(&self, _word: String) -> Result<(), TtsError> {
        unsupported("Removing entries from lexicon not supported by AWS Polly")
    }
}

pub struct AwsLongFormOperation {
    task: RefCell<SynthesisTask>,
    output_location: RefCell<String>,
}

impl AwsLongFormOperation {
    pub fn new(task: SynthesisTask, output_location: String) -> Self {
        Self {
            task: RefCell::new(task),
            output_location: RefCell::new(output_location),
        }
    }

    fn map_task_status(status: &str) -> OperationStatus {
        match status {
            "scheduled" => OperationStatus::Pending,
            "inProgress" => OperationStatus::Processing,
            "completed" => OperationStatus::Completed,
            "failed" => OperationStatus::Failed,
            _ => OperationStatus::Failed,
        }
    }
}

impl GuestLongFormOperation for AwsLongFormOperation {
    #[doc = " Get operation status"]
    fn get_status(&self) -> OperationStatus {
        // Refresh task status from AWS
        let task_id = self.task.borrow().task_id.clone();

        match Polly::new() {
            Ok(polly) => {
                let path = format!("/v1/synthesisTasks/{}", task_id);
                let full_uri = format!("{}{}", polly.base_url, path);

                let request = Request::builder()
                    .method("GET")
                    .uri(full_uri)
                    .body(Bytes::new());

                if let Ok(req) = request {
                    if let Ok(signed_request) = polly.signer.sign_request(req) {
                        let mut headers = HeaderMap::new();
                        for (key, value) in signed_request.headers().iter() {
                            if let Ok(key) = HeaderName::from_bytes(key.as_str().as_bytes()) {
                                if let Ok(value) = HeaderValue::from_bytes(value.as_bytes()) {
                                    headers.insert(key, value);
                                }
                            }
                        }

                        let response = polly.client.make_request::<GetSpeechSynthesisTaskResponse, (), (), _>(
                            Method::GET,
                            &path,
                            (),
                            None::<&()>,
                            Some(&headers),
                            from_http_error,
                        );

                        if let Ok(resp) = response {
                            *self.task.borrow_mut() = resp.synthesis_task.clone();
                            return Self::map_task_status(&resp.synthesis_task.task_status);
                        }
                    }
                }
            }
            Err(_) =>return OperationStatus::Failed,
        }

        // If we can't fetch status, return current status
        let task = self.task.borrow();
        Self::map_task_status(&task.task_status)
    }

    #[doc = " Get completion percentage (0-100)"]
    fn get_progress(&self) -> f32 {
        let task = self.task.borrow();
        match task.task_status.as_str() {
            "scheduled" => 0.0,
            "inProgress" => 50.0,
            "completed" => 100.0,
            "failed" => 100.0,
            _ => 0.0,
        }
    }

    #[doc = " Get result when operation is complete"]
    fn get_result(&self) -> Result<LongFormResult, TtsError> {
        let task = self.task.borrow();
        let output_location = self.output_location.borrow();

        if task.task_status != "completed" {
            return Err(TtsError::InternalError(format!(
                "Task not completed. Current status: {}",
                task.task_status
            )));
        }

        // Estimate duration based on character count (rough estimate: ~150 chars per minute)
        let duration_seconds = (task.request_characters as f32) / 150.0 * 60.0;

        Ok(LongFormResult {
            output_location: output_location.clone(),
            total_duration: duration_seconds,
            chapter_durations: None,
            metadata: golem_tts::golem::tts::types::SynthesisMetadata {
                duration_seconds,
                character_count: task.request_characters,
                word_count: task.request_characters / 5, // Rough estimate
                audio_size_bytes: 0, // Not available from Polly API
                request_id: task.task_id.clone(),
                provider_info: Some(format!("AWS Polly - Engine: {}", task.engine)),
            },
        })
    }

    #[doc = " Cancel the operation"]
    fn cancel(&self) -> Result<(), TtsError> {
        unsupported("AWS Polly does not support canceling synthesis tasks")
    }
}
