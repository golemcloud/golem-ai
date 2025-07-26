use crate::authentication::generate_jwt_token;
use golem_video::error::{from_reqwest_error, video_error_from_status};
use golem_video::exports::golem::video_generation::types::VideoError;
use log::trace;
use reqwest::{Client, Method, Response};
use serde::{Deserialize, Serialize};

// For older users, the api endpoint is https://api.klingai.com
const BASE_URL: &str = "https://api-singapore.klingai.com";

/// The Kling API client for video generation
pub struct KlingApi {
    access_key: String,
    secret_key: String,
    client: Client,
}

impl KlingApi {
    pub fn new(access_key: String, secret_key: String) -> Self {
        let client = Client::builder()
            .default_headers(reqwest::header::HeaderMap::new())
            .build()
            .expect("Failed to initialize HTTP client");
        Self {
            access_key,
            secret_key,
            client,
        }
    }

    fn get_auth_header(&self) -> Result<String, VideoError> {
        let token = generate_jwt_token(&self.access_key, &self.secret_key)
            .map_err(|e| VideoError::InternalError(format!("JWT token generation failed: {e}")))?;
        Ok(format!("Bearer {token}"))
    }

    pub fn generate_text_to_video(
        &self,
        request: TextToVideoRequest,
    ) -> Result<GenerationResponse, VideoError> {
        trace!("Sending text-to-video request to Kling API");

        let auth_header = self.get_auth_header()?;

        let response: Response = self
            .client
            .request(Method::POST, format!("{BASE_URL}/v1/videos/text2video"))
            .header("Authorization", auth_header)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .map_err(|err| from_reqwest_error("Request failed", err))?;

        parse_response(response)
    }

    pub fn generate_image_to_video(
        &self,
        request: ImageToVideoRequest,
    ) -> Result<GenerationResponse, VideoError> {
        trace!("Sending image-to-video request to Kling API");

        let auth_header = self.get_auth_header()?;

        let response: Response = self
            .client
            .request(Method::POST, format!("{BASE_URL}/v1/videos/image2video"))
            .header("Authorization", auth_header)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .map_err(|err| from_reqwest_error("Request failed", err))?;

        trace!("Received response status: {}", response.status());
        parse_response(response)
    }

    pub fn generate_multi_image_to_video(
        &self,
        request: MultiImageToVideoRequest,
    ) -> Result<GenerationResponse, VideoError> {
        trace!("Sending multi-image-to-video request to Kling API");

        let auth_header = self.get_auth_header()?;

        let response: Response = self
            .client
            .request(
                Method::POST,
                format!("{BASE_URL}/v1/videos/multi-image2video"),
            )
            .header("Authorization", auth_header)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .map_err(|err| from_reqwest_error("Request failed", err))?;

        parse_response(response)
    }

    // Kling has individual endpoints for each generation type
    // We use polling in text2video, polling for any type works on all endpoints
    pub fn poll_generation(&self, task_id: &str) -> Result<PollResponse, VideoError> {
        trace!("Polling generation status for ID: {task_id}");

        let auth_header = self.get_auth_header()?;

        let response: Response = self
            .client
            .request(
                Method::GET,
                format!("{BASE_URL}/v1/videos/text2video/{task_id}"),
            )
            .header("Authorization", auth_header)
            .send()
            .map_err(|err| from_reqwest_error("Poll request failed", err))?;

        let status = response.status();

        if status.is_success() {
            let task_response: TaskResponse = parse_response(response)?;

            if task_response.code != 0 {
                return Err(VideoError::GenerationFailed(format!(
                    "API error {}: {}",
                    task_response.code, task_response.message
                )));
            }

            match task_response.data.task_status.as_str() {
                "submitted" | "processing" => {
                    trace!("Task {task_id} is still processing");
                    Ok(PollResponse::Processing)
                }
                "succeed" => {
                    if let Some(task_result) = task_response.data.task_result {
                        if let Some(videos) = task_result.videos {
                            if let Some(video) = videos.first() {
                                Ok(PollResponse::Complete {
                                    video_data: None,
                                    mime_type: "video/mp4".to_string(),
                                    duration: video.duration.clone(),
                                    uri: video.url.clone(),
                                    generation_id: video.id.clone(),
                                })
                            } else {
                                Err(VideoError::InternalError(
                                    "No video in successful task".to_string(),
                                ))
                            }
                        } else {
                            Err(VideoError::InternalError(
                                "No videos in successful task".to_string(),
                            ))
                        }
                    } else {
                        Err(VideoError::InternalError(
                            "No task result in successful task".to_string(),
                        ))
                    }
                }
                "failed" => {
                    let error_msg = task_response
                        .data
                        .task_status_msg
                        .unwrap_or_else(|| "Task failed".to_string());
                    Err(VideoError::GenerationFailed(error_msg))
                }
                _ => Err(VideoError::InternalError(format!(
                    "Unknown task status: {}",
                    task_response.data.task_status
                ))),
            }
        } else {
            let error_body = response
                .text()
                .map_err(|err| from_reqwest_error("Failed to read error response", err))?;

            Err(video_error_from_status(status, error_body))
        }
    }

    pub fn generate_lip_sync(
        &self,
        request: LipSyncRequest,
    ) -> Result<GenerationResponse, VideoError> {
        trace!("Sending lip-sync request to Kling API");

        let auth_header = self.get_auth_header()?;

        let response: Response = self
            .client
            .request(Method::POST, format!("{BASE_URL}/v1/videos/lip-sync"))
            .header("Authorization", auth_header)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .map_err(|err| from_reqwest_error("Lip-sync request failed", err))?;

        parse_response(response)
    }

    pub fn generate_video_effects(
        &self,
        request: VideoEffectsRequest,
    ) -> Result<GenerationResponse, VideoError> {
        trace!("Sending video effects request to Kling API");

        let auth_header = self.get_auth_header()?;

        let response: Response = self
            .client
            .request(Method::POST, format!("{BASE_URL}/v1/videos/effects"))
            .header("Authorization", auth_header)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .map_err(|err| from_reqwest_error("Video effects request failed", err))?;

        parse_response(response)
    }

    pub fn extend_video(
        &self,
        request: VideoExtendRequest,
    ) -> Result<GenerationResponse, VideoError> {
        trace!("Sending video extend request to Kling API");

        let auth_header = self.get_auth_header()?;

        let response: Response = self
            .client
            .request(Method::POST, format!("{BASE_URL}/v1/videos/video-extend"))
            .header("Authorization", auth_header)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .map_err(|err| from_reqwest_error("Video extend request failed", err))?;

        parse_response(response)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TextToVideoRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_name: Option<String>,
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub negative_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cfg_scale: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub camera_control: Option<CameraControlRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aspect_ratio: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callback_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_task_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImageToVideoRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_name: Option<String>,
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub negative_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cfg_scale: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aspect_ratio: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<String>,
    pub image: String, // Base64 encoded image or URL (start frame)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_tail: Option<String>, // Base64 encoded image or URL (end frame)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub static_mask: Option<String>, // Base64 encoded image or URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dynamic_masks: Option<Vec<DynamicMaskRequest>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub camera_control: Option<CameraControlRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callback_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_task_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DynamicMaskRequest {
    pub mask: String, // Base64 encoded image or URL
    pub trajectories: Vec<TrajectoryPoint>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TrajectoryPoint {
    pub x: u32,
    pub y: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct CameraControlRequest {
    #[serde(rename = "type")]
    pub movement_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<CameraConfigRequest>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CameraConfigRequest {
    pub horizontal: f32,
    pub vertical: f32,
    pub pan: f32,
    pub tilt: f32,
    pub roll: f32,
    pub zoom: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationResponse {
    pub code: i32,
    pub message: String,
    pub request_id: String,
    pub data: GenerationResponseData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationResponseData {
    pub task_id: String,
    pub task_status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_info: Option<TaskInfo>,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_task_id: Option<String>,
}

#[derive(Debug, Clone)]
pub enum PollResponse {
    Processing,
    Complete {
        video_data: Option<Vec<u8>>,
        mime_type: String,
        duration: String,
        uri: String,
        generation_id: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResponse {
    pub code: i32,
    pub message: String,
    pub request_id: String,
    pub data: TaskResponseData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResponseData {
    pub task_id: String,
    pub task_status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_status_msg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_info: Option<TaskInfo>,
    pub created_at: u64,
    pub updated_at: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_result: Option<TaskResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub videos: Option<Vec<VideoResult>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoResult {
    pub id: String,
    pub url: String,
    pub duration: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MultiImageToVideoRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_name: Option<String>,
    pub image_list: Vec<ImageListItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub negative_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aspect_ratio: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callback_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_task_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImageListItem {
    pub image: String, // Base64 encoded image or URL
}

#[derive(Debug, Clone, Serialize)]
pub struct LipSyncRequest {
    pub input: LipSyncInput,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callback_url: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LipSyncInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_url: Option<String>,
    pub mode: String,
    // Text2Video mode fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>, //Text here is limited to 200 characters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_speed: Option<f32>,
    // Audio2Video mode fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_type: Option<String>, //supports audio syncing upto 60 seconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_file: Option<String>, // Base64 encoded
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_url: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VideoEffectsRequest {
    pub effect_scene: String,
    pub input: VideoEffectsInput,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callback_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_task_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VideoEffectsInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>, // Base64 encoded image or URL (for single image effects)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<String>>, // Array of Base64 encoded images or URLs (for dual effects)
    pub duration: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct VideoExtendRequest {
    pub video_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub negative_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cfg_scale: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callback_url: Option<String>,
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
