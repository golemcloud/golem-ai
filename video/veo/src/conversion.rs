use crate::client::{
    ImageData, ImageToVideoInstance, ImageToVideoRequest, PollResponse, TextToVideoInstance,
    TextToVideoRequest, VeoApi, VideoData, VideoParameters,
};
use golem_video::error::invalid_input;
use golem_video::exports::golem::video_generation::types::{
    AspectRatio, GenerationConfig, JobStatus, MediaData, MediaInput, Resolution, Video, VideoError,
    VideoResult,
};
use golem_video::utils::{download_image_from_url, download_video_from_url};
use std::collections::HashMap;

type RequestTuple = (
    Option<TextToVideoRequest>,
    Option<ImageToVideoRequest>,
    Option<String>,
);

pub fn media_input_to_request(
    input: MediaInput,
    config: GenerationConfig,
) -> Result<RequestTuple, VideoError> {
    // Parse provider options
    let options: HashMap<String, String> = config
        .provider_options
        .as_ref()
        .map(|po| {
            po.iter()
                .map(|kv| (kv.key.clone(), kv.value.clone()))
                .collect()
        })
        .unwrap_or_default();

    // Storage URI from provider options
    let storage_uri = options.get("storage_uri").cloned();

    // Determine model - default to veo-2.0-generate-001
    let model_id = config
        .model
        .clone()
        .or_else(|| Some("veo-2.0-generate-001".to_string()));

    // Validate model if provided, only warn
    if let Some(ref model) = model_id {
        if !matches!(
            model.as_str(),
            "veo-2.0-generate-001" | "veo-3.0-generate-preview" | "veo-3.0-fast-generate-preview"
        ) {
            log::warn!(
                "Model '{model}' is not officially supported. Supported models are: veo-2.0-generate-001, veo-3.0-generate-preview, veo-3.0-fast-generate-preview"
            );
        }
    }

    // Determine aspect ratio
    let aspect_ratio = determine_aspect_ratio(config.aspect_ratio, config.resolution)?;

    // Duration support, 5 or 8 seconds, 8 seconds is only supported by veo-2.0
    let duration_seconds = match config.duration_seconds {
        Some(d) => {
            let duration = d.round() as u32;
            duration.clamp(5, 8)
        }
        None => 5, // Default to 5 seconds
    };

    // Generate audio support, only supported by veo-3.0
    let generate_audio = config.enable_audio;

    // Person generation setting
    let person_generation = options
        .get("person_generation")
        .cloned()
        .or_else(|| Some("allow_adult".to_string()));
    if let Some(ref setting) = person_generation {
        if !matches!(setting.as_str(), "allow_adult" | "dont_allow") {
            return Err(invalid_input(
                "person_generation must be 'allow_adult' or 'dont_allow'",
            ));
        }
    }

    // Sample count (1-4 videos)
    let sample_count = options
        .get("sample_count")
        .and_then(|s| s.parse::<u32>().ok())
        .map(|c| c.clamp(1, 4));

    let parameters = VideoParameters {
        aspect_ratio: Some(aspect_ratio),
        duration_seconds,
        enhance_prompt: config.enhance_prompt,
        generate_audio,
        negative_prompt: config.negative_prompt.clone(),
        person_generation,
        sample_count,
        seed: config.seed.map(|s| s as u32),
        storage_uri,
    };

    match input {
        MediaInput::Video(ref_video) => {
            // Check if model supports video input - only veo-2.0-generate-001 supports video
            let model_str = model_id.as_deref().unwrap_or("veo-2.0-generate-001");
            if model_str != "veo-2.0-generate-001" {
                return Err(golem_video::error::unsupported_feature(
                    "Video-to-video is only supported by veo-2.0-generate-001 model",
                ));
            }

            // Extract video data from BaseVideo structure
            let video_data = match ref_video.data {
                MediaData::Url(url) => {
                    if url.starts_with("gs://") {
                        // Use as gcsUri - default to video/mp4 for GCS URIs
                        VideoData {
                            bytes_base64_encoded: None,
                            mime_type: "video/mp4".to_string(),
                            gcs_uri: Some(url),
                        }
                    } else {
                        // Download video from URL and convert to base64
                        let raw_bytes = download_video_from_url(&url)?;
                        VideoData {
                            bytes_base64_encoded: Some(base64::Engine::encode(
                                &base64::engine::general_purpose::STANDARD,
                                &raw_bytes.bytes,
                            )),
                            mime_type: raw_bytes.mime_type.clone(),
                            gcs_uri: None,
                        }
                    }
                }
                MediaData::Bytes(raw_bytes) => {
                    // Use the mime type from the raw bytes, or determine from bytes if not available
                    VideoData {
                        bytes_base64_encoded: Some(base64::Engine::encode(
                            &base64::engine::general_purpose::STANDARD,
                            &raw_bytes.bytes,
                        )),
                        mime_type: raw_bytes.mime_type.clone(),
                        gcs_uri: None,
                    }
                }
            };

            // Use a default prompt for video-to-video since BaseVideo doesn't have a prompt field
            let prompt = "Extend this video".to_string();

            let instances = vec![ImageToVideoInstance {
                prompt,
                image: None,
                last_frame: None,
                video: Some(video_data),
            }];
            let request = ImageToVideoRequest {
                instances,
                parameters,
            };

            // Log warnings for unsupported options
            log_unsupported_options(&config, &options);

            Ok((None, Some(request), model_id))
        }
        MediaInput::Text(prompt) => {
            let instances = vec![TextToVideoInstance { prompt }];
            let request = TextToVideoRequest {
                instances,
                parameters,
            };

            // Log warnings for unsupported options
            log_unsupported_options(&config, &options);

            Ok((Some(request), None, model_id))
        }
        MediaInput::Image(ref_image) => {
            // Extract image data from Reference structure
            let image_data = match ref_image.data.data {
                MediaData::Url(url) => {
                    if url.starts_with("gs://") {
                        // Use as gcsUri - default to image/jpeg for GCS URIs
                        ImageData {
                            bytes_base64_encoded: None,
                            mime_type: "image/jpg".to_string(),
                            gcs_uri: Some(url),
                        }
                    } else {
                        // Download image from URL and convert to base64
                        let raw_bytes = download_image_from_url(&url)?;
                        ImageData {
                            bytes_base64_encoded: Some(base64::Engine::encode(
                                &base64::engine::general_purpose::STANDARD,
                                &raw_bytes.bytes,
                            )),
                            mime_type: raw_bytes.mime_type.clone(),
                            gcs_uri: None,
                        }
                    }
                }
                MediaData::Bytes(raw_bytes) => {
                    // Use the mime type from the raw bytes, or determine from bytes if not available
                    ImageData {
                        bytes_base64_encoded: Some(base64::Engine::encode(
                            &base64::engine::general_purpose::STANDARD,
                            &raw_bytes.bytes,
                        )),
                        mime_type: raw_bytes.mime_type.clone(),
                        gcs_uri: None,
                    }
                }
            };

            // Use prompt from the reference image, or default
            let prompt = ref_image
                .prompt
                .clone()
                .unwrap_or_else(|| "Generate a video from this image".to_string());

            // Check if role is specified and log warning
            if ref_image.role.is_some() {
                log::warn!("Image role is not supported by Veo API and will be ignored");
            }

            // Handle lastframe from config if available
            let last_frame_data = if let Some(lastframe_config) = &config.lastframe {
                match &lastframe_config.data {
                    MediaData::Url(url) => {
                        if url.starts_with("gs://") {
                            Some(ImageData {
                                bytes_base64_encoded: None,
                                mime_type: "image/jpg".to_string(),
                                gcs_uri: Some(url.clone()),
                            })
                        } else {
                            let raw_bytes = download_image_from_url(url)?;
                            Some(ImageData {
                                bytes_base64_encoded: Some(base64::Engine::encode(
                                    &base64::engine::general_purpose::STANDARD,
                                    &raw_bytes.bytes,
                                )),
                                mime_type: raw_bytes.mime_type.clone(),
                                gcs_uri: None,
                            })
                        }
                    }
                    MediaData::Bytes(raw_bytes) => Some(ImageData {
                        bytes_base64_encoded: Some(base64::Engine::encode(
                            &base64::engine::general_purpose::STANDARD,
                            &raw_bytes.bytes,
                        )),
                        mime_type: raw_bytes.mime_type.clone(),
                        gcs_uri: None,
                    }),
                }
            } else {
                None
            };

            let instances = vec![ImageToVideoInstance {
                prompt,
                image: Some(image_data),
                last_frame: last_frame_data,
                video: None,
            }];
            let request = ImageToVideoRequest {
                instances,
                parameters,
            };

            // Log warnings for unsupported options
            log_unsupported_options(&config, &options);

            Ok((None, Some(request), model_id))
        }
    }
}

fn determine_aspect_ratio(
    aspect_ratio: Option<AspectRatio>,
    _resolution: Option<Resolution>,
) -> Result<String, VideoError> {
    let target_aspect = aspect_ratio.unwrap_or(AspectRatio::Landscape);

    match target_aspect {
        AspectRatio::Landscape => Ok("16:9".to_string()),
        AspectRatio::Portrait => Ok("9:16".to_string()),
        AspectRatio::Square => {
            log::warn!("Square aspect ratio not supported by Veo, using 16:9");
            Ok("16:9".to_string())
        }
        AspectRatio::Cinema => {
            log::warn!("Cinema aspect ratio not directly supported by Veo, using 16:9");
            Ok("16:9".to_string())
        }
    }
}

fn log_unsupported_options(config: &GenerationConfig, options: &HashMap<String, String>) {
    if config.scheduler.is_some() {
        log::warn!("scheduler is not supported by Veo API and will be ignored");
    }
    if config.guidance_scale.is_some() {
        log::warn!("guidance_scale is not supported by Veo API and will be ignored");
    }
    if config.static_mask.is_some() {
        log::warn!("static_mask is not supported by Veo API and will be ignored");
    }
    if config.dynamic_mask.is_some() {
        log::warn!("dynamic_mask is not supported by Veo API and will be ignored");
    }
    if config.camera_control.is_some() {
        log::warn!("camera_control is not supported by Veo API and will be ignored");
    }

    // Log unused provider options
    for key in options.keys() {
        if !matches!(
            key.as_str(),
            "person_generation" | "sample_count" | "storage_uri"
        ) {
            log::warn!("Provider option '{key}' is not supported by Veo API");
        }
    }
}

pub fn generate_video(
    client: &VeoApi,
    input: MediaInput,
    config: GenerationConfig,
) -> Result<String, VideoError> {
    let (text_request, image_request, model_id) = media_input_to_request(input, config)?;

    if let Some(request) = text_request {
        let response = client.generate_text_to_video(request, model_id)?;
        Ok(response.name)
    } else if let Some(request) = image_request {
        let response = client.generate_image_to_video(request, model_id)?;
        Ok(response.name)
    } else {
        Err(VideoError::InternalError(
            "No valid request generated".to_string(),
        ))
    }
}

pub fn poll_video_generation(
    client: &VeoApi,
    operation_name: String,
) -> Result<VideoResult, VideoError> {
    match client.poll_generation(&operation_name) {
        Ok(PollResponse::Processing) => Ok(VideoResult {
            status: JobStatus::Running,
            videos: None,
        }),
        Ok(PollResponse::Complete(video_results)) => {
            let videos: Vec<Video> = video_results
                .into_iter()
                .map(|result| Video {
                    uri: result.gcs_uri,
                    base64_bytes: if result.video_data.is_empty() {
                        None
                    } else {
                        Some(result.video_data)
                    },
                    mime_type: result.mime_type,
                    width: None,
                    height: None,
                    fps: None,
                    duration_seconds: None,
                    generation_id: None,
                })
                .collect();

            Ok(VideoResult {
                status: JobStatus::Succeeded,
                videos: Some(videos),
            })
        }
        Err(error) => Err(error),
    }
}

pub fn cancel_video_generation(
    _client: &VeoApi,
    operation_name: String,
) -> Result<String, VideoError> {
    Err(VideoError::UnsupportedFeature(format!(
        "Cancellation is not supported by Veo API for operation {operation_name}"
    )))
}

pub fn generate_lip_sync_video(
    _client: &VeoApi,
    _video: golem_video::exports::golem::video_generation::types::LipSyncVideo,
    _audio: golem_video::exports::golem::video_generation::types::AudioSource,
) -> Result<String, VideoError> {
    Err(VideoError::UnsupportedFeature(
        "Lip sync is not supported by Veo API".to_string(),
    ))
}

pub fn list_available_voices(
    _client: &VeoApi,
    _language: Option<String>,
) -> Result<Vec<golem_video::exports::golem::video_generation::types::VoiceInfo>, VideoError> {
    Err(VideoError::UnsupportedFeature(
        "Voice listing is not supported by Veo API".to_string(),
    ))
}

pub fn extend_video(
    _client: &VeoApi,
    _video_id: String,
    _prompt: Option<String>,
    _negative_prompt: Option<String>,
    _cfg_scale: Option<f32>,
    _provider_options: Option<Vec<golem_video::exports::golem::video_generation::types::Kv>>,
) -> Result<String, VideoError> {
    Err(VideoError::UnsupportedFeature(
        "Video extension is not supported by Veo API".to_string(),
    ))
}

pub fn upscale_video(
    _client: &VeoApi,
    _input: golem_video::exports::golem::video_generation::types::BaseVideo,
) -> Result<String, VideoError> {
    Err(VideoError::UnsupportedFeature(
        "Video upscaling is not supported by Veo API".to_string(),
    ))
}

pub fn generate_video_effects(
    _client: &VeoApi,
    _input: golem_video::exports::golem::video_generation::types::InputImage,
    _effect: golem_video::exports::golem::video_generation::types::EffectType,
    _model: Option<String>,
    _duration: Option<f32>,
    _mode: Option<String>,
) -> Result<String, VideoError> {
    Err(VideoError::UnsupportedFeature(
        "Video effects generation is not supported by Veo API".to_string(),
    ))
}

pub fn multi_image_generation(
    _client: &VeoApi,
    _input_images: Vec<golem_video::exports::golem::video_generation::types::InputImage>,
    _prompt: Option<String>,
    _config: GenerationConfig,
) -> Result<String, VideoError> {
    Err(VideoError::UnsupportedFeature(
        "Multi-image generation is not supported by Veo API".to_string(),
    ))
}
