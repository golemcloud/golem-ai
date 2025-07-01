use crate::client::{
    CameraConfigRequest, CameraControlRequest, DynamicMaskRequest, ImageListItem,
    ImageToVideoRequest, KlingApi, LipSyncInput, LipSyncRequest, MultiImageToVideoRequest,
    PollResponse, TextToVideoRequest, TrajectoryPoint,
};
use golem_video::error::invalid_input;
use golem_video::exports::golem::video::types::{
    AspectRatio, AudioSource, CameraControl, CameraMovement, GenerationConfig, ImageRole,
    JobStatus, MediaData, MediaInput, Resolution, Video, VideoError, VideoResult,
};
use log::trace;
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
    client: &KlingApi,
    video: golem_video::exports::golem::video::types::BaseVideo,
    audio: golem_video::exports::golem::video::types::AudioSource,
) -> Result<String, VideoError> {
    use crate::voices::is_valid_voice_id;

    trace!("Generating lip-sync video with Kling API");

    // Convert video data to required format
    let (video_id, video_url) = match &video.data {
        MediaData::Url(url) => (None, Some(url.clone())),
        MediaData::Bytes(_) => {
            return Err(invalid_input(
                "Lip-sync requires video URL or video_id from Kling API. Base64 video data is not supported.",
            ));
        }
    };

    // Convert audio source to request format
    let (mode, text, voice_id, voice_language, voice_speed, audio_type, audio_file, audio_url) =
        match audio {
            AudioSource::FromText(tts) => {
                // Text-to-video mode
                let voice_id = tts.voice_id.as_ref().ok_or_else(|| {
                    invalid_input("voice_id is required for text-to-speech lip-sync")
                })?;

                // Determine language from voice_id
                let language = if is_valid_voice_id(voice_id, "zh") {
                    "zh"
                } else if is_valid_voice_id(voice_id, "en") {
                    "en"
                } else {
                    return Err(invalid_input(format!("Invalid voice_id: {voice_id}")));
                };

                // Convert speed from u32 to f32 and validate range
                let speed = tts.speed as f32 / 100.0; // Convert from percentage to decimal
                let voice_speed = speed.clamp(0.8, 2.0);

                (
                    "text2video".to_string(),
                    Some(tts.text.clone()),
                    Some(voice_id.clone()),
                    Some(language.to_string()),
                    Some(voice_speed),
                    None,
                    None,
                    None,
                )
            }
            AudioSource::FromAudio(narration) => {
                // Audio-to-video mode
                match &narration.data {
                    MediaData::Url(url) => (
                        "audio2video".to_string(),
                        None,
                        None,
                        None,
                        None,
                        Some("url".to_string()),
                        None,
                        Some(url.clone()),
                    ),
                    MediaData::Bytes(bytes) => {
                        // Convert to base64
                        use base64::Engine;
                        let audio_base64 = base64::engine::general_purpose::STANDARD.encode(bytes);
                        (
                            "audio2video".to_string(),
                            None,
                            None,
                            None,
                            None,
                            Some("file".to_string()),
                            Some(audio_base64),
                            None,
                        )
                    }
                }
            }
        };

    let input = LipSyncInput {
        video_id,
        video_url,
        mode,
        text,
        voice_id,
        voice_language,
        voice_speed,
        audio_type,
        audio_file,
        audio_url,
    };

    let request = LipSyncRequest {
        input,
        callback_url: None,
    };

    let response = client.generate_lip_sync(request)?;
    if response.code == 0 {
        Ok(response.data.task_id)
    } else {
        Err(VideoError::GenerationFailed(format!(
            "API error {}: {}",
            response.code, response.message
        )))
    }
}

pub fn list_available_voices(
    _client: &KlingApi,
    language: Option<String>,
) -> Result<Vec<golem_video::exports::golem::video::types::VoiceInfo>, VideoError> {
    use crate::voices::get_voices;

    trace!("Listing available voices for language: {language:?}");

    let voices = get_voices(language);
    Ok(voices)
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
    client: &KlingApi,
    input: golem_video::exports::golem::video::types::InputImage,
    effect: golem_video::exports::golem::video::types::EffectType,
    model: Option<String>,
    duration: Option<f32>,
    mode: Option<String>,
) -> Result<String, VideoError> {
    use crate::client::{VideoEffectsInput, VideoEffectsRequest};
    use golem_video::exports::golem::video::types::{
        DualImageEffects, EffectType, SingleImageEffects,
    };

    trace!("Generating video effects with Kling API");

    // Convert input image to string (Base64 or URL)
    let input_image_data = convert_media_data_to_string(&input.data)?;

    // Determine effect scene and build request based on effect type
    let (effect_scene, request_input) = match effect {
        EffectType::Single(single_effect) => {
            // Single image effects
            let scene_name = match single_effect {
                SingleImageEffects::Bloombloom => "bloombloom",
                SingleImageEffects::Dizzydizzy => "dizzydizzy",
                SingleImageEffects::Fuzzyfuzzy => "fuzzyfuzzy",
                SingleImageEffects::Squish => "squish",
                SingleImageEffects::Expansion => "expansion",
            };

            // For single image effects, model_name is required to be "kling-v1-6"
            let model_name = Some("kling-v1-6".to_string());

            // Duration for single image effects is fixed to "5"
            let duration_str = "5".to_string();

            // Single image effects don't support mode parameter
            if mode.is_some() {
                log::warn!(
                    "Mode parameter is not supported for single image effects and will be ignored"
                );
            }

            let input = VideoEffectsInput {
                model_name,
                mode: None, // Single image effects don't support mode
                image: Some(input_image_data),
                images: None,
                duration: duration_str,
            };

            (scene_name.to_string(), input)
        }
        EffectType::Dual(dual_effect) => {
            // Dual character effects
            let scene_name = match dual_effect.effect {
                DualImageEffects::Hug => "hug",
                DualImageEffects::Kiss => "kiss",
                DualImageEffects::HeartGesture => "heart_gesture",
            };

            // Convert second image to string
            let second_image_data = convert_media_data_to_string(&dual_effect.second_image.data)?;

            // Build images array with first and second image
            let images = vec![input_image_data, second_image_data];

            // For dual effects, model validation
            let model_name = if let Some(ref m) = model {
                if !matches!(m.as_str(), "kling-v1" | "kling-v1-5" | "kling-v1-6") {
                    return Err(invalid_input(
                        "Model must be one of: kling-v1, kling-v1-5, kling-v1-6 for dual effects",
                    ));
                }
                Some(m.clone())
            } else {
                Some("kling-v1".to_string()) // Default for dual effects
            };

            // Mode validation
            let mode_val = if let Some(ref m) = mode {
                if !matches!(m.as_str(), "std" | "pro") {
                    return Err(invalid_input("Mode must be 'std' or 'pro'"));
                }
                Some(m.clone())
            } else {
                Some("std".to_string()) // Default mode
            };

            // Duration handling - convert from seconds to string
            let duration_str = if let Some(dur) = duration {
                if dur <= 5.0 {
                    "5".to_string()
                } else {
                    "10".to_string()
                }
            } else {
                "5".to_string() // Default duration
            };

            let input = VideoEffectsInput {
                model_name,
                mode: mode_val,
                image: None, // For dual effects, use images array instead
                images: Some(images),
                duration: duration_str,
            };

            (scene_name.to_string(), input)
        }
    };

    let request = VideoEffectsRequest {
        effect_scene,
        input: request_input,
        callback_url: None,
        external_task_id: None,
    };

    let response = client.generate_video_effects(request)?;
    if response.code == 0 {
        Ok(response.data.task_id)
    } else {
        Err(VideoError::GenerationFailed(format!(
            "API error {}: {}",
            response.code, response.message
        )))
    }
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
