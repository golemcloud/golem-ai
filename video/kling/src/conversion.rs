use crate::client::{
    CameraConfigRequest, CameraControlRequest, DynamicMaskRequest, ImageListItem,
    ImageToVideoRequest, KlingApi, LipSyncInput, LipSyncRequest, MultiImageToVideoRequest,
    PollResponse, TextToVideoRequest, TrajectoryPoint, VideoExtendRequest,
};
use crate::voices::get_voices;
use golem_video::error::invalid_input;
use golem_video::exports::golem::video_generation::types::{
    AspectRatio, AudioSource, CameraMovement, GenerationConfig, JobStatus, MediaData, MediaInput,
    Resolution, Video, VideoError, VideoResult, VoiceLanguage,
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
        .as_ref()
        .map(|po| {
            po.iter()
                .map(|kv| (kv.key.clone(), kv.value.clone()))
                .collect()
        })
        .unwrap_or_default();

    // Determine model - default to kling-v1
    let model_name = config
        .model
        .clone()
        .or_else(|| Some("kling-v1".to_string()));

    // Validate model if provided, Only warn
    if let Some(ref model) = model_name {
        if !matches!(
            model.as_str(),
            "kling-v1" | "kling-v1-6" | "kling-v2" | "kling-v2-1" | "kling-v1-5"
        ) {
            log::warn!(
                "Model '{model}' is not officially supported. Supported models are: kling-v1, kling-v1-6, kling-v2, kling-v2-1, kling-v1-5"
            );
        }
    }

    // Determine aspect ratio
    let aspect_ratio = determine_aspect_ratio(config.aspect_ratio, config.resolution)?;

    // Duration support - Kling supports 5 and 10 seconds
    let duration = config.duration_seconds.map(|d| {
        if d <= 10.0 {
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
        MediaInput::Video(_) => Err(golem_video::error::unsupported_feature(
            "Video-to-video is not supported by Kling API",
        )),
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
            // Extract image data from InputImage structure - always use as start frame
            let image_data = {
                let result = convert_media_data_to_string(&ref_image.data.data);
                result?
            };

            // Handle lastframe as image_tail if provided
            let image_tail = if let Some(ref lastframe) = config.lastframe {
                Some(convert_media_data_to_string(&lastframe.data)?)
            } else {
                None
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

            // Validate API constraints: image_tail, dynamic_masks/static_mask, and camera_control cannot be used together
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
                image: image_data,
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
        MediaData::Bytes(raw_bytes) => {
            // Convert bytes to base64 string
            use base64::Engine;
            Ok(base64::engine::general_purpose::STANDARD.encode(&raw_bytes.bytes))
        }
    }
}

fn convert_camera_control(
    camera_movement: &CameraMovement,
) -> Result<CameraControlRequest, VideoError> {
    match camera_movement {
        CameraMovement::Simple(config) => {
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
                horizontal: config.horizontal,
                vertical: config.vertical,
                pan: config.pan,
                tilt: config.tilt,
                roll: config.roll,
                zoom: config.zoom,
            };

            Ok(CameraControlRequest {
                movement_type: "simple".to_string(),
                config: Some(config_req),
            })
        }
        CameraMovement::DownBack => Ok(CameraControlRequest {
            movement_type: "down_back".to_string(),
            config: None,
        }),
        CameraMovement::ForwardUp => Ok(CameraControlRequest {
            movement_type: "forward_up".to_string(),
            config: None,
        }),
        CameraMovement::RightTurnForward => Ok(CameraControlRequest {
            movement_type: "right_turn_forward".to_string(),
            config: None,
        }),
        CameraMovement::LeftTurnForward => Ok(CameraControlRequest {
            movement_type: "left_turn_forward".to_string(),
            config: None,
        }),
    }
}

fn convert_dynamic_mask(
    dynamic_mask: &golem_video::exports::golem::video_generation::types::DynamicMask,
) -> Result<Vec<DynamicMaskRequest>, VideoError> {
    // Validate trajectory length
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
        if !matches!(key.as_str(), "mode") {
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
        if !matches!(key.as_str(), "mode") {
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
    trace!("Polling video generation for task ID: {task_id}");

    match client.poll_generation(&task_id) {
        Ok(PollResponse::Processing) => {
            log::info!("Task {task_id} is still processing");
            Ok(VideoResult {
                status: JobStatus::Running,
                videos: None,
            })
        }
        Ok(PollResponse::Complete {
            video_data,
            mime_type,
            duration,
            uri,
            generation_id,
        }) => {
            log::info!("Task {task_id} completed successfully");
            let duration_seconds = parse_duration_string(&duration);

            let video = Video {
                uri: Some(uri),
                base64_bytes: video_data,
                mime_type,
                width: None,
                height: None,
                fps: None,
                duration_seconds,
                generation_id: Some(generation_id),
            };

            Ok(VideoResult {
                status: JobStatus::Succeeded,
                videos: Some(vec![video]),
            })
        }
        Err(error) => {
            log::error!("Task {task_id} failed: {error:?}");
            Err(error)
        }
    }
}

fn parse_duration_string(duration_str: &str) -> Option<f32> {
    // Try to parse duration string like "5" or "10" to float
    duration_str.parse::<f32>().ok()
}

fn validate_voice_id_and_language(voice_id: &str, language: &VoiceLanguage) {
    let language_str = match language {
        VoiceLanguage::En => "en",
        VoiceLanguage::Zh => "zh",
    };

    let available_voices = get_voices(Some(language_str.to_string()));
    let voice_exists = available_voices
        .iter()
        .any(|voice| voice.voice_id == voice_id);
    // Just warn in case in voice-id are added in the future
    if !voice_exists {
        log::warn!(
            "Voice ID '{voice_id}' is not valid for language '{language_str}'. Please use a valid voice ID from the available voices."
        );
    }
}

pub fn cancel_video_generation(_client: &KlingApi, task_id: String) -> Result<String, VideoError> {
    // Kling API does not support cancellation
    Err(VideoError::UnsupportedFeature(format!(
        "Cancellation is not supported by Kling API for task {task_id}"
    )))
}

pub fn generate_lip_sync_video(
    client: &KlingApi,
    video: golem_video::exports::golem::video_generation::types::LipSyncVideo,
    audio: golem_video::exports::golem::video_generation::types::AudioSource,
) -> Result<String, VideoError> {
    trace!("Generating lip-sync video with Kling API");

    // Convert video data to required format
    // Supports both video_id and video_url from Kling API
    let (video_id, video_url) = match &video {
        golem_video::exports::golem::video_generation::types::LipSyncVideo::VideoId(id) => {
            (Some(id.clone()), None)
        }
        golem_video::exports::golem::video_generation::types::LipSyncVideo::Video(base_video) => {
            match &base_video.data {
                MediaData::Url(url) => (None, Some(url.clone())),
                MediaData::Bytes(_) => {
                    return Err(invalid_input(
                        "Lip-sync requires video URL. Base64 video data is not supported.",
                    ));
                }
            }
        }
    };

    // Convert audio source to request format
    let (mode, text, voice_id, voice_language, voice_speed, audio_type, audio_file, audio_url) =
        match audio {
            AudioSource::FromText(tts) => {
                // Text-to-video mode
                let voice_id = &tts.voice_id;

                // Validate voice_id and language combination, only warn
                validate_voice_id_and_language(voice_id, &tts.language);

                // Use the language from the TTS object
                let language = match tts.language {
                    golem_video::exports::golem::video_generation::types::VoiceLanguage::En => "en",
                    golem_video::exports::golem::video_generation::types::VoiceLanguage::Zh => "zh",
                };

                let speed = tts.speed;
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
                    MediaData::Bytes(raw_bytes) => {
                        // Convert to base64
                        use base64::Engine;
                        let audio_base64 =
                            base64::engine::general_purpose::STANDARD.encode(&raw_bytes.bytes);
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
) -> Result<Vec<golem_video::exports::golem::video_generation::types::VoiceInfo>, VideoError> {
    trace!("Listing available voices for language: {language:?}");

    let voices = get_voices(language);
    Ok(voices)
}

pub fn extend_video(
    client: &KlingApi,
    video_id: String,
    prompt: Option<String>,
    negative_prompt: Option<String>,
    cfg_scale: Option<f32>,
    provider_options: Option<Vec<golem_video::exports::golem::video_generation::types::Kv>>,
) -> Result<String, VideoError> {
    trace!("Extending video with ID: {video_id}");

    // Parse provider options
    let options: HashMap<String, String> = provider_options
        .as_ref()
        .map(|po| {
            po.iter()
                .map(|kv| (kv.key.clone(), kv.value.clone()))
                .collect()
        })
        .unwrap_or_default();

    // Validate prompt length (max 2500 characters)
    if let Some(ref p) = prompt {
        if p.len() > 2500 {
            return Err(invalid_input("Prompt cannot exceed 2500 characters"));
        }
    }

    // Validate negative prompt length (max 2500 characters)
    if let Some(ref np) = negative_prompt {
        if np.len() > 2500 {
            return Err(invalid_input(
                "Negative prompt cannot exceed 2500 characters",
            ));
        }
    }

    // Validate cfg_scale range [0, 1]
    if let Some(scale) = cfg_scale {
        if !(0.0..=1.0).contains(&scale) {
            return Err(invalid_input("cfg_scale must be between 0.0 and 1.0"));
        }
    }

    // Log warnings for unsupported provider options
    for key in options.keys() {
        log::warn!("Provider option '{key}' is not supported by Kling video extension API");
    }

    let request = VideoExtendRequest {
        video_id,
        prompt,
        negative_prompt,
        cfg_scale,
        callback_url: None,
    };

    let response = client.extend_video(request)?;
    if response.code == 0 {
        Ok(response.data.task_id)
    } else {
        Err(VideoError::GenerationFailed(format!(
            "API error {}: {}",
            response.code, response.message
        )))
    }
}

pub fn upscale_video(
    _client: &KlingApi,
    _input: golem_video::exports::golem::video_generation::types::BaseVideo,
) -> Result<String, VideoError> {
    Err(VideoError::UnsupportedFeature(
        "Video upscaling is not supported by Kling API".to_string(),
    ))
}

pub fn generate_video_effects(
    client: &KlingApi,
    input: golem_video::exports::golem::video_generation::types::InputImage,
    effect: golem_video::exports::golem::video_generation::types::EffectType,
    model: Option<String>,
    duration: Option<f32>,
    mode: Option<String>,
) -> Result<String, VideoError> {
    use crate::client::{VideoEffectsInput, VideoEffectsRequest};
    use golem_video::exports::golem::video_generation::types::{
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
                SingleImageEffects::AnimeFigure => "anime_figure",
                SingleImageEffects::Rocketrocket => "rocketrocket",
                // anime_figure and rocketrocket are newly added, in their chinese documentations
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
                if dur <= 10.0 {
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
    input_images: Vec<golem_video::exports::golem::video_generation::types::InputImage>,
    prompt: Option<String>,
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
        .as_ref()
        .map(|po| {
            po.iter()
                .map(|kv| (kv.key.clone(), kv.value.clone()))
                .collect()
        })
        .unwrap_or_default();

    // Determine model - for multi-image, default to kling-v1-6 as per API docs
    let model_name = config
        .model
        .clone()
        .or_else(|| Some("kling-v1-6".to_string()));

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
    let prompt = prompt.unwrap_or_else(|| "Generate a video from these images".to_string());

    // Determine aspect ratio
    let aspect_ratio = determine_aspect_ratio(config.aspect_ratio, config.resolution)?;

    // Duration support - Kling supports 5 and 10 seconds
    let duration = config.duration_seconds.map(|d| {
        if d <= 10.0 {
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
