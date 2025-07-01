use golem_video::error::{from_reqwest_error, video_error_from_status};
use golem_video::exports::golem::video::types::VideoError;
use log::trace;
use reqwest::{Method, Response};
use serde::{Deserialize, Serialize};

const BASE_URL: &str = "https://api.dev.runwayml.com";
const API_VERSION: &str = "2024-11-06";

#[derive(Debug, Clone, Serialize)]
pub struct TextToImageRequest {
    #[serde(rename = "promptText")]
    pub prompt_text: String,
    pub ratio: String,
    pub model: String, // Must be "gen4_image"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
    #[serde(rename = "contentModeration", skip_serializing_if = "Option::is_none")]
    pub content_moderation: Option<ContentModeration>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContentModeration {
    #[serde(rename = "publicFigureThreshold")]
    pub public_figure_threshold: String, // "auto" or "low"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextToImageResponse {
    pub id: String,
}

#[derive(Debug, Clone)]
pub enum ImagePollResponse {
    Processing,
    Complete { image_url: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageTaskResponse {
    pub id: String,
    pub status: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    pub output: Option<Vec<String>>,
}

impl crate::client::RunwayApi {
    pub fn generate_text_to_image(
        &self,
        request: TextToImageRequest,
    ) -> Result<TextToImageResponse, VideoError> {
        trace!("Sending text-to-image request to Runway API");

        let response: Response = self
            .client
            .request(Method::POST, format!("{BASE_URL}/v1/text_to_image"))
            .header("Authorization", format!("Bearer {}", &self.api_key))
            .header("X-Runway-Version", API_VERSION)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .map_err(|err| from_reqwest_error("Text-to-image request failed", err))?;

        parse_text_to_image_response(response)
    }

    pub fn poll_text_to_image(&self, task_id: &str) -> Result<ImagePollResponse, VideoError> {
        trace!("Polling text-to-image status for ID: {task_id}");

        let response: Response = self
            .client
            .request(Method::GET, format!("{BASE_URL}/v1/tasks/{task_id}"))
            .header("Authorization", format!("Bearer {}", &self.api_key))
            .header("X-Runway-Version", API_VERSION)
            .send()
            .map_err(|err| from_reqwest_error("Text-to-image poll request failed", err))?;

        let status = response.status();

        if status.is_success() {
            let task_response: ImageTaskResponse = response
                .json()
                .map_err(|err| from_reqwest_error("Failed to parse image task response", err))?;

            match task_response.status.as_str() {
                "PENDING" | "RUNNING" => Ok(ImagePollResponse::Processing),
                "SUCCEEDED" => {
                    if let Some(output) = task_response.output {
                        if let Some(image_url) = output.first() {
                            Ok(ImagePollResponse::Complete {
                                image_url: image_url.clone(),
                            })
                        } else {
                            Err(VideoError::InternalError(
                                "No output URL in successful image task".to_string(),
                            ))
                        }
                    } else {
                        Err(VideoError::InternalError(
                            "No output in successful image task".to_string(),
                        ))
                    }
                }
                "FAILED" | "CANCELED" => Err(VideoError::GenerationFailed(
                    "Image generation task failed or was canceled".to_string(),
                )),
                _ => Err(VideoError::InternalError(format!(
                    "Unknown image task status: {}",
                    task_response.status
                ))),
            }
        } else {
            let error_body = response
                .text()
                .map_err(|err| from_reqwest_error("Failed to read error response", err))?;

            Err(video_error_from_status(status, error_body))
        }
    }
}

fn parse_text_to_image_response(response: Response) -> Result<TextToImageResponse, VideoError> {
    let status = response.status();
    if status.is_success() {
        response
            .json::<TextToImageResponse>()
            .map_err(|err| from_reqwest_error("Failed to decode text-to-image response body", err))
    } else {
        let error_body = response
            .text()
            .map_err(|err| from_reqwest_error("Failed to receive error response body", err))?;

        let error_message = format!("Text-to-image request failed with {status}: {error_body}");
        Err(video_error_from_status(status, error_message))
    }
}
