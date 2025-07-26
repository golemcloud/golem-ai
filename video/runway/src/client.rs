use golem_video::error::{from_reqwest_error, video_error_from_status};
use golem_video::exports::golem::video_generation::types::VideoError;
use log::trace;
use reqwest::{Client, Method, Response};
use serde::{Deserialize, Serialize};

const BASE_URL: &str = "https://api.dev.runwayml.com";
const API_VERSION: &str = "2024-11-06";

/// The Runway API client for image-to-video generation
pub struct RunwayApi {
    pub api_key: String,
    pub client: Client,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationResponse {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextToImageResponse {
    pub id: String,
}

#[derive(Debug, Clone)]
pub enum PollResponse {
    Processing,
    Complete {
        video_data: Option<Vec<u8>>,
        mime_type: String,
        uri: String,
        generation_id: String,
    },
}

#[derive(Debug, Clone)]
pub enum ImagePollResponse {
    Processing,
    Complete { image_url: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResponse {
    pub id: String,
    pub status: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    pub output: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageTaskResponse {
    pub id: String,
    pub status: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    pub output: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PromptImage {
    pub uri: String,
    pub position: String, // "first" or "last"
}

#[derive(Debug, Clone, Serialize)]
pub struct ContentModeration {
    #[serde(rename = "publicFigureThreshold")]
    pub public_figure_threshold: String, // "auto" or "low"
}

#[derive(Debug, Clone, Serialize)]
pub struct VideoUpscaleRequest {
    #[serde(rename = "videoUri")]
    pub video_uri: String,
    pub model: String, // Must be "upscale_v1"
}

#[derive(Debug, Clone, Serialize)]
pub struct ImageToVideoRequest {
    #[serde(rename = "promptImage")]
    pub prompt_image: Vec<PromptImage>,
    pub model: String,
    pub ratio: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
    #[serde(rename = "promptText", skip_serializing_if = "Option::is_none")]
    pub prompt_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<u32>,
    #[serde(rename = "contentModeration", skip_serializing_if = "Option::is_none")]
    pub content_moderation: Option<ContentModeration>,
}

impl RunwayApi {
    pub fn new(api_key: String) -> Self {
        let client = Client::builder()
            .default_headers(reqwest::header::HeaderMap::new())
            .build()
            .expect("Failed to initialize HTTP client");
        Self { api_key, client }
    }

    pub fn generate_video(
        &self,
        request: ImageToVideoRequest,
    ) -> Result<GenerationResponse, VideoError> {
        trace!("Sending image-to-video request to Runway API");

        let response: Response = self
            .client
            .request(Method::POST, format!("{BASE_URL}/v1/image_to_video"))
            .header("Authorization", format!("Bearer {}", &self.api_key))
            .header("X-Runway-Version", API_VERSION)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .map_err(|err| from_reqwest_error("Request failed", err))?;

        parse_response(response)
    }

    pub fn poll_generation(&self, task_id: &str) -> Result<PollResponse, VideoError> {
        trace!("Polling generation status for ID: {task_id}");

        let response: Response = self
            .client
            .request(Method::GET, format!("{BASE_URL}/v1/tasks/{task_id}"))
            .header("Authorization", format!("Bearer {}", &self.api_key))
            .header("X-Runway-Version", API_VERSION)
            .send()
            .map_err(|err| from_reqwest_error("Poll request failed", err))?;

        let status = response.status();

        if status.is_success() {
            let task_response: TaskResponse = response
                .json()
                .map_err(|err| from_reqwest_error("Failed to parse task response", err))?;

            match task_response.status.as_str() {
                "PENDING" | "RUNNING" => Ok(PollResponse::Processing),
                "SUCCEEDED" => {
                    if let Some(output) = task_response.output {
                        if let Some(video_url) = output.first() {
                            Ok(PollResponse::Complete {
                                video_data: None,
                                mime_type: "video/mp4".to_string(),
                                uri: video_url.clone(),
                                generation_id: task_response.id.clone(),
                            })
                        } else {
                            Err(VideoError::InternalError(
                                "No output URL in successful task".to_string(),
                            ))
                        }
                    } else {
                        Err(VideoError::InternalError(
                            "No output in successful task".to_string(),
                        ))
                    }
                }
                "FAILED" | "CANCELED" => Err(VideoError::GenerationFailed(
                    "Task failed or was canceled".to_string(),
                )),
                _ => Err(VideoError::InternalError(format!(
                    "Unknown task status: {}",
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

    pub fn cancel_task(&self, task_id: &str) -> Result<(), VideoError> {
        trace!("Canceling task: {task_id}");

        let response: Response = self
            .client
            .request(Method::DELETE, format!("{BASE_URL}/v1/tasks/{task_id}"))
            .header("Authorization", format!("Bearer {}", &self.api_key))
            .header("X-Runway-Version", API_VERSION)
            .send()
            .map_err(|err| from_reqwest_error("Cancel request failed", err))?;

        if response.status().is_success() {
            Ok(())
        } else {
            let status = response.status();
            let error_body = response
                .text()
                .map_err(|err| from_reqwest_error("Failed to read error response", err))?;

            Err(video_error_from_status(status, error_body))
        }
    }

    pub fn upscale_video(
        &self,
        request: VideoUpscaleRequest,
    ) -> Result<GenerationResponse, VideoError> {
        trace!("Sending video upscale request to Runway API");

        let response: Response = self
            .client
            .request(Method::POST, format!("{BASE_URL}/v1/video_upscale"))
            .header("Authorization", format!("Bearer {}", &self.api_key))
            .header("X-Runway-Version", API_VERSION)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .map_err(|err| from_reqwest_error("Upscale request failed", err))?;

        parse_response(response)
    }

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

fn parse_response<T: serde::de::DeserializeOwned>(response: Response) -> Result<T, VideoError> {
    let status = response.status();
    if status.is_success() {
        response
            .json::<T>()
            .map_err(|err| from_reqwest_error("Failed to decode response body", err))
    } else {
        let error_body = response
            .text()
            .map_err(|err| from_reqwest_error("Failed to receive error response body", err))?;

        let error_message = format!("Request failed with {status}: {error_body}");
        Err(video_error_from_status(status, error_message))
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
