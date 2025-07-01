use crate::client::{
    CameraConfigRequest, CameraControlRequest, DynamicMaskRequest, ImageListItem,
    ImageToVideoRequest, KlingApi, MultiImageToVideoRequest, PollResponse, TextToVideoRequest,
    TrajectoryPoint,
};
use golem_video::error::invalid_input;
use golem_video::exports::golem::video::types::{
    AspectRatio, CameraControl, CameraMovement, GenerationConfig, ImageRole, JobStatus, MediaData,
    MediaInput, Resolution, Video, VideoError, VideoResult,
};
use std::collections::HashMap;

pub fn media_input_to_request(
    input: MediaInput,
    config: GenerationConfig,
) -> Result<(Option<TextToVideoRequest>, Option<ImageToVideoRequest>), VideoError> {
    // Parse provider options
    let options: HashMap<String, String> = config
        .provider_options
        .iter()
        .map(|kv| (kv.key.clone(), kv.value.clone()))
        .collect();

    // Determine model - default to kling-v1, can be overridden
    let model_name = config.model.clone().or_else(|| {
        options
            .get("model")
            .cloned()
            .or_else(|| Some("kling-v1".to_string()))
    });

    // Validate model if provided
    if let Some(ref model) = model_name {
        if !matches!(
            model.as_str(),
            "kling-v1" | "kling-v1-6" | "kling-v2-master" | "kling-v2-1-master"
        ) {
            return Err(invalid_input(
                "Model must be one of: kling-v1, kling-v1-6, kling-v2-master, kling-v2-1-master",
            ));
        }
    }

    // Determine aspect ratio
    let aspect_ratio = determine_aspect_ratio(config.aspect_ratio, config.resolution)?;

    // Duration support - Kling supports 5 and 10 seconds
    let duration = config.duration_seconds.map(|d| {
        if d <= 5.0 {
            "5".to_string()
        } else {
            "10".to_string()
        }
    });

    // Mode support - std or pro
    let mode = options
        .get("mode")
        .cloned()
        .or_else(|| Some("std".to_string()));
    if let Some(ref mode_val) = mode {
        if !matches!(mode_val.as_str(), "std" | "pro") {
            return Err(invalid_input("Mode must be 'std' or 'pro'"));
        }
    }

    // CFG scale support (0.0 to 1.0)
    let cfg_scale = config
        .guidance_scale
        .map(|scale| (scale / 10.0).clamp(0.0, 1.0));

    // Camera control support
    let camera_control = config
        .camera_control
        .as_ref()
        .map(convert_camera_control)
        .transpose()?;

    // Clone negative_prompt before moving values
    let negative_prompt = config.negative_prompt.clone();

    match input {
        MediaInput::Text(prompt) => {
            let request = TextToVideoRequest {
                model_name,
                prompt,
                negative_prompt,
                cfg_scale,
                mode,
                camera_control,
                aspect_ratio: Some(aspect_ratio),
                duration,
                callback_url: None,
                external_task_id: None,
            };

            // Log warnings for unsupported options
            log_unsupported_options(&config, &options);

            Ok((Some(request), None))
        }
        MediaInput::Image(ref_image) => {
            // Handle role and lastframe logic
            let image_role = ref_image.role.as_ref();

            // Check for conflict: both role=last and lastframe provided
            if matches!(image_role, Some(ImageRole::Last)) && config.lastframe.is_some() {
                log::warn!("Both image role=last and lastframe provided. Using lastframe only as specified.");
            }

            // Extract image data from InputImage structure
            let image_data = convert_media_data_to_string(&ref_image.data.data)?;

            // Handle lastframe - either from role=last or explicit lastframe
            let image_tail = if matches!(image_role, Some(ImageRole::Last)) {
                Some(image_data.clone())
            } else if let Some(ref lastframe) = config.lastframe {
                Some(convert_media_data_to_string(&lastframe.data)?)
            } else {
                None
            };

            // Set image based on role - if role=last, use None for main image, otherwise use the image
            let main_image = if matches!(image_role, Some(ImageRole::Last)) {
                None
            } else {
                Some(image_data)
            };

            // Static mask support
            let static_mask = config
                .static_mask
                .as_ref()
                .map(|sm| convert_media_data_to_string(&sm.mask.data))
                .transpose()?;

            // Dynamic mask support
            let dynamic_masks = config
                .dynamic_mask
                .as_ref()
                .map(convert_dynamic_mask)
                .transpose()?;

            // Validate API constraints: image+image_tail, dynamic_masks/static_mask, and camera_control cannot be used together
            let has_image_tail = image_tail.is_some();
            let has_masks = static_mask.is_some() || dynamic_masks.is_some();
            let has_camera_control = camera_control.is_some();

            if has_image_tail && has_masks {
                return Err(invalid_input(
                    "image_tail (lastframe) cannot be used together with static_mask or dynamic_masks",
                ));
            }
            if has_image_tail && has_camera_control {
                return Err(invalid_input(
                    "image_tail (lastframe) cannot be used together with camera_control",
                ));
            }
            if has_masks && has_camera_control {
                return Err(invalid_input(
                    "static_mask/dynamic_masks cannot be used together with camera_control",
                ));
            }

            // Validate that at least one image (image or image_tail) is provided
            if main_image.is_none() && image_tail.is_none() {
                return Err(invalid_input(
                    "At least one of image or image_tail must be provided",
                ));
            }

            // Use prompt from the reference image, or default
            let prompt = ref_image
                .prompt
                .clone()
                .unwrap_or_else(|| "Generate a video from this image".to_string());

            let request = ImageToVideoRequest {
                model_name,
                prompt,
                negative_prompt,
                cfg_scale,
                mode,
                aspect_ratio: Some(aspect_ratio),
                duration,
                image: main_image,
                image_tail,
                static_mask,
                dynamic_masks,
                camera_control,
                callback_url: None,
                external_task_id: None,
            };

            // Log warnings for unsupported options
            log_unsupported_options(&config, &options);

            Ok((None, Some(request)))
        }
    }
}

fn convert_media_data_to_string(media_data: &MediaData) -> Result<String, VideoError> {
    match media_data {
        MediaData::Url(url) => Ok(url.clone()),
        MediaData::Bytes(bytes) => {
            // Convert bytes to base64 string
            use base64::Engine;
            Ok(base64::engine::general_purpose::STANDARD.encode(bytes))
        }
    }
}

fn convert_camera_control(
    camera_control: &CameraControl,
) -> Result<CameraControlRequest, VideoError> {
    match camera_control {
        CameraControl::Movement(movement) => {
            let movement_type = match movement {
                CameraMovement::Simple => "simple".to_string(),
                CameraMovement::DownBack => "down_back".to_string(),
                CameraMovement::ForwardUp => "forward_up".to_string(),
                CameraMovement::RightTurnForward => "right_turn_forward".to_string(),
                CameraMovement::LeftTurnForward => "left_turn_forward".to_string(),
            };

            Ok(CameraControlRequest {
                movement_type,
                config: None,
            })
        }
        CameraControl::Config(config) => {
            // For simple camera control with custom config
            // Validate that only one parameter is non-zero
            let non_zero_count = [
                config.horizontal,
                config.vertical,
                config.pan,
                config.tilt,
                config.roll,
                config.zoom,
            ]
            .iter()
            .filter(|&&val| val != 0.0)
            .count();

            if non_zero_count != 1 {
                return Err(invalid_input(
                    "Camera config must have exactly one non-zero parameter",
                ));
            }

            // Validate range [-10, 10]
            for &val in &[
                config.horizontal,
                config.vertical,
                config.pan,
                config.tilt,
                config.roll,
                config.zoom,
            ] {
                if !(-10.0..=10.0).contains(&val) {
                    return Err(invalid_input(
                        "Camera config values must be in range [-10, 10]",
                    ));
                }
            }

            let config_req = CameraConfigRequest {
                horizontal: if config.horizontal != 0.0 {
                    Some(config.horizontal)
                } else {
                    None
                },
                vertical: if config.vertical != 0.0 {
                    Some(config.vertical)
                } else {
                    None
                },
                pan: if config.pan != 0.0 {
                    Some(config.pan)
                } else {
                    None
                },
                tilt: if config.tilt != 0.0 {
                    Some(config.tilt)
                } else {
                    None
                },
                roll: if config.roll != 0.0 {
                    Some(config.roll)
                } else {
                    None
                },
                zoom: if config.zoom != 0.0 {
                    Some(config.zoom)
                } else {
                    None
                },
            };

            Ok(CameraControlRequest {
                movement_type: "simple".to_string(),
                config: Some(config_req),
            })
        }
    }
}

fn convert_dynamic_mask(
    dynamic_mask: &golem_video::exports::golem::video::types::DynamicMask,
) -> Result<Vec<DynamicMaskRequest>, VideoError> {
    // Validate trajectory length (max 77 for 5s video)
    if dynamic_mask.trajectories.len() < 2 {
        return Err(invalid_input(
            "Dynamic mask must have at least 2 trajectory points",
        ));
    }
    if dynamic_mask.trajectories.len() > 77 {
        return Err(invalid_input(
            "Dynamic mask cannot have more than 77 trajectory points",
        ));
    }

    let mask_data = convert_media_data_to_string(&dynamic_mask.mask.data)?;
    let trajectories: Vec<TrajectoryPoint> = dynamic_mask
        .trajectories
        .iter()
        .map(|pos| TrajectoryPoint { x: pos.x, y: pos.y })
        .collect();

    Ok(vec![DynamicMaskRequest {
        mask: mask_data,
        trajectories,
    }])
}

fn determine_aspect_ratio(
    aspect_ratio: Option<AspectRatio>,
    _resolution: Option<Resolution>,
) -> Result<String, VideoError> {
    let target_aspect = aspect_ratio.unwrap_or(AspectRatio::Landscape);

    match target_aspect {
        AspectRatio::Landscape => Ok("16:9".to_string()),
        AspectRatio::Portrait => Ok("9:16".to_string()),
        AspectRatio::Square => Ok("1:1".to_string()),
        AspectRatio::Cinema => {
            log::warn!("Cinema aspect ratio not directly supported, using 16:9");
            Ok("16:9".to_string())
        }
    }
}

fn log_unsupported_options(config: &GenerationConfig, options: &HashMap<String, String>) {
    if config.scheduler.is_some() {
        log::warn!("scheduler is not supported by Kling API and will be ignored");
    }
    if config.enable_audio.is_some() {
        log::warn!("enable_audio is not supported by Kling API and will be ignored");
    }
    if config.enhance_prompt.is_some() {
        log::warn!("enhance_prompt is not supported by Kling API and will be ignored");
    }

    // Log unused provider options
    for key in options.keys() {
        if !matches!(key.as_str(), "model" | "mode") {
            log::warn!("Provider option '{key}' is not supported by Kling API");
        }
    }
}

fn log_multi_image_unsupported_options(
    config: &GenerationConfig,
    options: &HashMap<String, String>,
) {
    // Multi-image generation has additional restrictions
    if config.scheduler.is_some() {
        log::warn!("scheduler is not supported by Kling multi-image API and will be ignored");
    }
    if config.enable_audio.is_some() {
        log::warn!("enable_audio is not supported by Kling multi-image API and will be ignored");
    }
    if config.enhance_prompt.is_some() {
        log::warn!("enhance_prompt is not supported by Kling multi-image API and will be ignored");
    }
    if config.guidance_scale.is_some() {
        log::warn!("guidance_scale (cfg_scale) is not supported by Kling multi-image API and will be ignored");
    }
    if config.lastframe.is_some() {
        log::warn!("lastframe is not supported by Kling multi-image API and will be ignored");
    }
    if config.static_mask.is_some() {
        log::warn!("static_mask is not supported by Kling multi-image API and will be ignored");
    }
    if config.dynamic_mask.is_some() {
        log::warn!("dynamic_mask is not supported by Kling multi-image API and will be ignored");
    }
    if config.camera_control.is_some() {
        log::warn!("camera_control is not supported by Kling multi-image API and will be ignored");
    }

    // Log unused provider options
    for key in options.keys() {
        if !matches!(key.as_str(), "model" | "mode") {
            log::warn!("Provider option '{key}' is not supported by Kling multi-image API");
        }
    }
}

pub fn generate_video(
    client: &KlingApi,
    input: MediaInput,
    config: GenerationConfig,
) -> Result<String, VideoError> {
    let (text_request, image_request) = media_input_to_request(input, config)?;

    if let Some(request) = text_request {
        let response = client.generate_text_to_video(request)?;
        if response.code == 0 {
            Ok(response.data.task_id)
        } else {
            Err(VideoError::GenerationFailed(format!(
                "API error {}: {}",
                response.code, response.message
            )))
        }
    } else if let Some(request) = image_request {
        let response = client.generate_image_to_video(request)?;
        if response.code == 0 {
            Ok(response.data.task_id)
        } else {
            Err(VideoError::GenerationFailed(format!(
                "API error {}: {}",
                response.code, response.message
            )))
        }
    } else {
        Err(VideoError::InternalError(
            "No valid request generated".to_string(),
        ))
    }
}

pub fn poll_video_generation(
    client: &KlingApi,
    task_id: String,
) -> Result<VideoResult, VideoError> {
    match client.poll_generation(&task_id) {
        Ok(PollResponse::Processing) => Ok(VideoResult {
            status: JobStatus::Running,
            videos: None,
            metadata: None,
        }),
        Ok(PollResponse::Complete {
            video_data,
            mime_type,
            duration,
        }) => {
            // Parse duration to extract seconds if possible
            let duration_seconds = parse_duration_string(&duration);

            let video = Video {
                uri: None,
                base64_bytes: Some(video_data),
                mime_type,
                width: None,
                height: None,
                fps: None,
                duration_seconds,
            };

            Ok(VideoResult {
                status: JobStatus::Succeeded,
                videos: Some(vec![video]),
                metadata: None,
            })
        }
        Err(error) => Err(error),
    }
}

fn parse_duration_string(duration_str: &str) -> Option<f32> {
    // Try to parse duration string like "5" or "10" to float
    duration_str.parse::<f32>().ok()
}

pub fn cancel_video_generation(_client: &KlingApi, task_id: String) -> Result<String, VideoError> {
    // Kling API does not support cancellation according to requirements
    Err(VideoError::UnsupportedFeature(format!(
        "Cancellation is not supported by Kling API for task {task_id}"
    )))
}

pub fn generate_lip_sync_video(
    _client: &KlingApi,
    _video: golem_video::exports::golem::video::types::BaseVideo,
    _audio: golem_video::exports::golem::video::types::AudioSource,
) -> Result<String, VideoError> {
    Err(VideoError::UnsupportedFeature(
        "Lip sync is not supported by Kling API".to_string(),
    ))
}

pub fn list_available_voices(
    _client: &KlingApi,
    _language: Option<String>,
) -> Result<Vec<golem_video::exports::golem::video::types::VoiceInfo>, VideoError> {
    Err(VideoError::UnsupportedFeature(
        "Voice listing is not supported by Kling API".to_string(),
    ))
}

pub fn extend_video(
    _client: &KlingApi,
    _input: golem_video::exports::golem::video::types::BaseVideo,
    _config: GenerationConfig,
) -> Result<String, VideoError> {
    Err(VideoError::UnsupportedFeature(
        "Video extension is not supported by Kling API".to_string(),
    ))
}

pub fn upscale_video(
    _client: &KlingApi,
    _input: golem_video::exports::golem::video::types::BaseVideo,
) -> Result<String, VideoError> {
    Err(VideoError::UnsupportedFeature(
        "Video upscaling is not supported by Kling API".to_string(),
    ))
}

pub fn generate_video_effects(
    _client: &KlingApi,
    _input: golem_video::exports::golem::video::types::InputImage,
    _effect: golem_video::exports::golem::video::types::EffectType,
    _model: Option<String>,
    _duration: Option<f32>,
    _mode: Option<String>,
) -> Result<String, VideoError> {
    Err(VideoError::UnsupportedFeature(
        "Video effects generation is not supported by Kling API".to_string(),
    ))
}

pub fn multi_image_generation(
    client: &KlingApi,
    input_images: Vec<golem_video::exports::golem::video::types::InputImage>,
    config: GenerationConfig,
) -> Result<String, VideoError> {
    // Validate input: 1 to 4 images supported
    if input_images.is_empty() {
        return Err(invalid_input(
            "At least 1 image is required for multi-image generation",
        ));
    }
    if input_images.len() > 4 {
        return Err(invalid_input(
            "Multi-image generation supports at most 4 images",
        ));
    }

    // Parse provider options
    let options: HashMap<String, String> = config
        .provider_options
        .iter()
        .map(|kv| (kv.key.clone(), kv.value.clone()))
        .collect();

    // Determine model - for multi-image, default to kling-v1-6 as per API docs
    let model_name = config.model.clone().or_else(|| {
        options
            .get("model")
            .cloned()
            .or_else(|| Some("kling-v1-6".to_string()))
    });

    // Validate model if provided (multi-image endpoint only supports kling-v1-6 according to docs)
    if let Some(ref model) = model_name {
        if model != "kling-v1-6" {
            log::warn!("Multi-image generation only supports kling-v1-6 model. Using kling-v1-6.");
        }
    }

    // Convert input images to image_list format
    let mut image_list = Vec::new();
    for input_image in &input_images {
        let image_data = convert_media_data_to_string(&input_image.data)?;
        image_list.push(ImageListItem { image: image_data });
    }

    // Build prompt - use the first image's prompt if available, or create a default
    let prompt = input_images
        .first()
        .and({
            // InputImage doesn't have a prompt field directly, so we'll use a default
            None::<String>
        })
        .unwrap_or_else(|| "Generate a video from these images".to_string());

    // Determine aspect ratio
    let aspect_ratio = determine_aspect_ratio(config.aspect_ratio, config.resolution)?;

    // Duration support - Kling supports 5 and 10 seconds
    let duration = config.duration_seconds.map(|d| {
        if d <= 5.0 {
            "5".to_string()
        } else {
            "10".to_string()
        }
    });

    // Mode support - std or pro
    let mode = options
        .get("mode")
        .cloned()
        .or_else(|| Some("std".to_string()));
    if let Some(ref mode_val) = mode {
        if !matches!(mode_val.as_str(), "std" | "pro") {
            return Err(invalid_input("Mode must be 'std' or 'pro'"));
        }
    }

    let request = MultiImageToVideoRequest {
        model_name: Some("kling-v1-6".to_string()), // Force kling-v1-6 for multi-image
        image_list,
        prompt: Some(prompt),
        negative_prompt: config.negative_prompt.clone(),
        mode,
        duration,
        aspect_ratio: Some(aspect_ratio),
        callback_url: None,
        external_task_id: None,
    };

    // Log warnings for unsupported options specific to multi-image
    log_multi_image_unsupported_options(&config, &options);

    let response = client.generate_multi_image_to_video(request)?;
    if response.code == 0 {
        Ok(response.data.task_id)
    } else {
        Err(VideoError::GenerationFailed(format!(
            "API error {}: {}",
            response.code, response.message
        )))
    }
}
