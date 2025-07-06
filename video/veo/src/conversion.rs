use crate::client::{
    ImageData, ImageToVideoInstance, ImageToVideoRequest, PollResponse, TextToVideoInstance,
    TextToVideoRequest, VeoApi, VideoParameters,
};
use golem_video::error::invalid_input;
use golem_video::exports::golem::video::types::{
    AspectRatio, GenerationConfig, ImageRole, JobStatus, MediaData, MediaInput, Resolution, Video,
    VideoError, VideoResult,
};
use golem_video::utils::download_image_from_url;
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
        .iter()
        .map(|kv| (kv.key.clone(), kv.value.clone()))
        .collect();

    // Determine model - default to veo-2.0-generate-001, can be overridden
    let model_id = config.model.clone().or_else(|| {
        options
            .get("model")
            .cloned()
            .or_else(|| Some("veo-2.0-generate-001".to_string()))
    });

    // Validate model if provided
    if let Some(ref model) = model_id {
        if !matches!(
            model.as_str(),
            "veo-2.0-generate-001" | "veo-3.0-generate-preview"
        ) {
            return Err(invalid_input(
                "Model must be one of: veo-2.0-generate-001, veo-3.0-generate-preview",
            ));
        }
    }

    // Determine aspect ratio
    let aspect_ratio = determine_aspect_ratio(config.aspect_ratio, config.resolution)?;

    // Duration support - Veo supports 5-8 seconds for veo-2.0, 8 seconds for veo-3.0
    let duration_seconds = match config.duration_seconds {
        Some(d) => {
            let duration = d.round() as u32;
            if model_id.as_deref() == Some("veo-3.0-generate-preview") {
                8 // veo-3.0 only supports 8 seconds
            } else {
                duration.clamp(5, 8) // veo-2.0 supports 5-8 seconds
            }
        }
        None => 8, // Default to 8 seconds
    };

    // Generate audio support (required for veo-3.0)
    let generate_audio = if model_id.as_deref() == Some("veo-3.0-generate-preview") {
        Some(config.enable_audio.unwrap_or(false))
    } else {
        None // Not supported by veo-2.0
    };

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

    // Storage URI for output
    let storage_uri = options.get("storage_uri").cloned();

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
            // Extract image data from new InputImage structure
            let image_data = match ref_image.data.data {
                MediaData::Url(url) => {
                    // Download image from URL and convert to base64
                    let raw_bytes = download_image_from_url(&url)?;
                    let mime_type = if !raw_bytes.mime_type.is_empty() {
                        raw_bytes.mime_type.clone()
                    } else {
                        determine_image_mime_type(&url, &raw_bytes.bytes)?
                    };

                    ImageData {
                        bytes_base64_encoded: base64::Engine::encode(
                            &base64::engine::general_purpose::STANDARD,
                            &raw_bytes.bytes,
                        ),
                        mime_type,
                    }
                }
                MediaData::Bytes(raw_bytes) => {
                    // Use the mime type from the raw bytes, or determine from bytes if not available
                    let mime_type = if !raw_bytes.mime_type.is_empty() {
                        raw_bytes.mime_type.clone()
                    } else if raw_bytes.bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
                        "image/png".to_string()
                    } else {
                        "image/jpeg".to_string()
                    };

                    ImageData {
                        bytes_base64_encoded: base64::Engine::encode(
                            &base64::engine::general_purpose::STANDARD,
                            &raw_bytes.bytes,
                        ),
                        mime_type,
                    }
                }
            };

            // Use prompt from the reference image, or default
            let prompt = ref_image
                .prompt
                .clone()
                .unwrap_or_else(|| "Generate a video from this image".to_string());

            // Handle image role and lastframe
            let image_role = ref_image.role.as_ref();

            // Handle lastframe - check if model supports it
            let last_frame_data = if let Some(lastframe) = &config.lastframe {
                // Check if we're using a model that supports lastFrame
                let model_id = model_id.as_deref().unwrap_or("veo-2.0-generate-001");
                if model_id != "veo-2.0-generate-001" {
                    log::warn!("lastFrame is only supported by veo-2.0-generate-001 model, ignoring for {model_id}");
                    None
                } else {
                    let lastframe_image_data = match lastframe.data {
                        MediaData::Url(ref url) => {
                            let raw_bytes = download_image_from_url(url)?;
                            let mime_type = if !raw_bytes.mime_type.is_empty() {
                                raw_bytes.mime_type.clone()
                            } else {
                                determine_image_mime_type(url, &raw_bytes.bytes)?
                            };
                            ImageData {
                                bytes_base64_encoded: base64::Engine::encode(
                                    &base64::engine::general_purpose::STANDARD,
                                    &raw_bytes.bytes,
                                ),
                                mime_type,
                            }
                        }
                        MediaData::Bytes(ref raw_bytes) => {
                            let mime_type = if !raw_bytes.mime_type.is_empty() {
                                raw_bytes.mime_type.clone()
                            } else if raw_bytes.bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
                                "image/png".to_string()
                            } else {
                                "image/jpeg".to_string()
                            };
                            ImageData {
                                bytes_base64_encoded: base64::Engine::encode(
                                    &base64::engine::general_purpose::STANDARD,
                                    &raw_bytes.bytes,
                                ),
                                mime_type,
                            }
                        }
                    };
                    Some(lastframe_image_data)
                }
            } else {
                None
            };

            // Handle image role for positioning
            let (image_for_first, last_frame_final) = match image_role {
                Some(ImageRole::Last) => {
                    // If image role is "last", use it as lastFrame instead
                    (None, Some(image_data))
                }
                Some(ImageRole::First) | None => {
                    // Use as first frame (default behavior)
                    (Some(image_data), last_frame_data)
                }
            };

            // Ensure we have at least one image
            let final_image = image_for_first.unwrap_or_else(|| {
                // If we don't have a first frame but have a last frame, create a dummy first frame
                log::warn!("No first frame provided, using a placeholder. Consider providing both first and last frames.");
                ImageData {
                    bytes_base64_encoded: String::new(),
                    mime_type: "image/jpeg".to_string(),
                }
            });

            let instances = vec![ImageToVideoInstance {
                prompt,
                image: final_image,
                last_frame: last_frame_final,
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

fn determine_image_mime_type(url: &str, bytes: &[u8]) -> Result<String, VideoError> {
    // Check file extension first
    if url.to_lowercase().ends_with(".png") {
        return Ok("image/png".to_string());
    }
    if url.to_lowercase().ends_with(".jpg") || url.to_lowercase().ends_with(".jpeg") {
        return Ok("image/jpeg".to_string());
    }

    // Check magic bytes
    if bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
        Ok("image/png".to_string())
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Ok("image/jpeg".to_string())
    } else {
        // Default to JPEG
        log::warn!("Could not determine image type, defaulting to JPEG");
        Ok("image/jpeg".to_string())
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
            "model" | "person_generation" | "sample_count" | "storage_uri"
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
            metadata: None,
        }),
        Ok(PollResponse::Complete(video_results)) => {
            let videos: Vec<Video> = video_results
                .into_iter()
                .map(|result| Video {
                    uri: None,
                    base64_bytes: Some(result.video_data),
                    mime_type: result.mime_type,
                    width: None,
                    height: None,
                    fps: None,
                    duration_seconds: None, // Veo doesn't provide duration in response
                })
                .collect();

            Ok(VideoResult {
                status: JobStatus::Succeeded,
                videos: Some(videos),
                metadata: None,
            })
        }
        Err(error) => Err(error),
    }
}

pub fn cancel_video_generation(
    _client: &VeoApi,
    operation_name: String,
) -> Result<String, VideoError> {
    // Veo API does not support cancellation according to requirements
    Err(VideoError::UnsupportedFeature(format!(
        "Cancellation is not supported by Veo API for operation {operation_name}"
    )))
}

pub fn generate_lip_sync_video(
    _client: &VeoApi,
    _video: golem_video::exports::golem::video::types::BaseVideo,
    _audio: golem_video::exports::golem::video::types::AudioSource,
) -> Result<String, VideoError> {
    Err(VideoError::UnsupportedFeature(
        "Lip sync is not supported by Veo API".to_string(),
    ))
}

pub fn list_available_voices(
    _client: &VeoApi,
    _language: Option<String>,
) -> Result<Vec<golem_video::exports::golem::video::types::VoiceInfo>, VideoError> {
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
    _provider_options: Vec<golem_video::exports::golem::video::types::Kv>,
) -> Result<String, VideoError> {
    Err(VideoError::UnsupportedFeature(
        "Video extension is not supported by Veo API".to_string(),
    ))
}

pub fn upscale_video(
    _client: &VeoApi,
    _input: golem_video::exports::golem::video::types::BaseVideo,
) -> Result<String, VideoError> {
    Err(VideoError::UnsupportedFeature(
        "Video upscaling is not supported by Veo API".to_string(),
    ))
}

pub fn generate_video_effects(
    _client: &VeoApi,
    _input: golem_video::exports::golem::video::types::InputImage,
    _effect: golem_video::exports::golem::video::types::EffectType,
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
    _input_images: Vec<golem_video::exports::golem::video::types::InputImage>,
    _config: GenerationConfig,
) -> Result<String, VideoError> {
    Err(VideoError::UnsupportedFeature(
        "Multi-image generation is not supported by Veo API".to_string(),
    ))
}
