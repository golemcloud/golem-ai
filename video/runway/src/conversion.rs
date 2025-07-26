use crate::client::{
    ContentModeration, ImagePollResponse, ImageToVideoRequest, PollResponse, PromptImage,
    RunwayApi, TextToImageRequest, VideoUpscaleRequest,
};
use golem_video::error::{invalid_input, unsupported_feature};
use golem_video::exports::golem::video_generation::types::{
    AspectRatio, GenerationConfig, ImageRole, JobStatus, MediaData, MediaInput, Resolution, Video,
    VideoError, VideoResult,
};
use std::collections::HashMap;

pub fn media_input_to_request(
    input: MediaInput,
    config: GenerationConfig,
) -> Result<ImageToVideoRequest, VideoError> {
    match input {
        MediaInput::Text(_) => Err(unsupported_feature(
            "Text-to-video should be handled in generate_video function",
        )),
        MediaInput::Video(_) => Err(unsupported_feature(
            "Video-to-video error should be handled in generate_video function",
        )),
        MediaInput::Image(ref_image) => {
            // Extract image data from new InputImage structure
            let image_data = match ref_image.data.data {
                MediaData::Url(url) => url,
                MediaData::Bytes(raw_bytes) => {
                    // Convert bytes to data URI with proper mime type
                    use base64::Engine;
                    let base64_data =
                        base64::engine::general_purpose::STANDARD.encode(&raw_bytes.bytes);
                    let mime_type = if !raw_bytes.mime_type.is_empty() {
                        &raw_bytes.mime_type
                    } else {
                        "image/png"
                    };
                    format!("data:{mime_type};base64,{base64_data}")
                }
            };

            // Parse provider options
            let options: HashMap<String, String> = config
                .provider_options
                .map(|po| po.into_iter().map(|kv| (kv.key, kv.value)).collect())
                .unwrap_or_default();

            // Determine model - default to gen3a_turbo
            let model = config.model.unwrap_or_else(|| "gen3a_turbo".to_string());

            // Validate model
            if !matches!(model.as_str(), "gen3a_turbo" | "gen4_turbo") {
                return Err(invalid_input("Model must be 'gen3a_turbo' or 'gen4_turbo'"));
            }

            // Determine ratio based on aspect_ratio and resolution
            let ratio = determine_ratio(&model, config.aspect_ratio, config.resolution)?;

            // Duration support
            let duration = match config.duration_seconds {
                Some(d) => {
                    let dur = d as u32;
                    if dur < 10 {
                        Some(5) // Default to 5 if less than 10
                    } else {
                        Some(10) // Cap at 10 if 10 or above
                    }
                }
                None => Some(5), // Default to 5 if no duration provided
            };

            // Content moderation
            let content_moderation = options.get("publicFigureThreshold").map(|threshold| {
                let threshold_value = if threshold == "low" { "low" } else { "auto" };
                ContentModeration {
                    public_figure_threshold: threshold_value.to_string(),
                }
            });

            // Create prompt images based on role only
            let mut prompt_images = Vec::new();

            // Determine position from image role (default to "first")
            let position = match ref_image.role {
                Some(ImageRole::First) => "first",
                Some(ImageRole::Last) => "last",
                None => "first", // Default to first frame
            };

            prompt_images.push(PromptImage {
                uri: image_data,
                position: position.to_string(),
            });

            // Warn if lastframe is provided (not supported by Runway API)
            if config.lastframe.is_some() {
                log::warn!("lastframe is not supported by Runway API and will be ignored");
            }

            // Use prompt text from the image if available
            let prompt_text = ref_image.prompt;

            // Validate seed if provided
            if let Some(seed_val) = config.seed {
                if seed_val > 4294967295 {
                    return Err(invalid_input("Seed must be between 0 and 4294967295"));
                }
            }

            // Log warnings for unsupported built-in options
            if config.negative_prompt.is_some() {
                log::warn!("negative_prompt is not supported by Runway API and will be ignored");
            }
            if config.scheduler.is_some() {
                log::warn!("scheduler is not supported by Runway API and will be ignored");
            }
            if config.guidance_scale.is_some() {
                log::warn!("guidance_scale is not supported by Runway API and will be ignored");
            }
            if config.enable_audio.is_some() {
                log::warn!("enable_audio is not supported by Runway API and will be ignored");
            }
            if config.enhance_prompt.is_some() {
                log::warn!("enhance_prompt is not supported by Runway API and will be ignored");
            }
            if config.static_mask.is_some() {
                log::warn!("static_mask is not supported by Runway API and will be ignored");
            }
            if config.dynamic_mask.is_some() {
                log::warn!("dynamic_mask is not supported by Runway API and will be ignored");
            }
            if config.camera_control.is_some() {
                log::warn!("camera_control is not supported by Runway API and will be ignored");
            }

            Ok(ImageToVideoRequest {
                prompt_image: prompt_images,
                model,
                ratio,
                seed: config.seed,
                prompt_text,
                duration,
                content_moderation,
            })
        }
    }
}

fn determine_ratio(
    model: &str,
    aspect_ratio: Option<AspectRatio>,
    _resolution: Option<Resolution>,
) -> Result<String, VideoError> {
    // Default ratios by model
    let default_ratio = match model {
        "gen3a_turbo" => "1280:768",
        "gen4_turbo" => "1280:720",
        _ => return Err(invalid_input("Invalid model")),
    };

    // If no aspect ratio specified, use default
    let target_aspect = aspect_ratio.unwrap_or(AspectRatio::Landscape);

    match model {
        "gen3a_turbo" => match target_aspect {
            AspectRatio::Landscape => Ok("1280:768".to_string()),
            AspectRatio::Portrait => Ok("768:1280".to_string()),
            AspectRatio::Square | AspectRatio::Cinema => {
                log::warn!(
                    "Aspect ratio {target_aspect:?} not supported by gen3a_turbo, using landscape"
                );
                Ok("1280:768".to_string())
            }
        },
        "gen4_turbo" => match target_aspect {
            AspectRatio::Landscape => Ok("1280:720".to_string()),
            AspectRatio::Portrait => Ok("720:1280".to_string()),
            AspectRatio::Square => Ok("960:960".to_string()),
            AspectRatio::Cinema => Ok("1584:672".to_string()),
        },
        _ => Ok(default_ratio.to_string()),
    }
}

pub fn generate_video(
    client: &RunwayApi,
    input: MediaInput,
    config: GenerationConfig,
) -> Result<String, VideoError> {
    match input {
        MediaInput::Text(prompt) => {
            // For text input, first generate an image, then use that for video generation
            generate_text_to_video_via_image(client, prompt, config)
        }
        MediaInput::Image(_) => {
            // For image input, use existing flow
            let request = media_input_to_request(input, config)?;
            let response = client.generate_video(request)?;
            Ok(response.id)
        }
        MediaInput::Video(_) => Err(unsupported_feature(
            "Video-to-video is not supported by Runway API",
        )),
    }
}

fn generate_text_to_video_via_image(
    client: &RunwayApi,
    prompt: String,
    config: GenerationConfig,
) -> Result<String, VideoError> {
    // Step 1: Generate image from text
    let image_task_id = generate_text_to_image(client, prompt.clone(), &config)?;

    // Step 2: Poll for image completion (with timeout)
    let max_polls = 60; // 5 minutes with 5-second intervals
    let mut polls = 0;

    let image_url = loop {
        if polls >= max_polls {
            return Err(VideoError::GenerationFailed(
                "Text-to-image generation timed out".to_string(),
            ));
        }

        match poll_text_to_image_generation(client, &image_task_id)? {
            Some(url) => break url,
            None => {
                // Sleep for 5 seconds before next poll
                std::thread::sleep(std::time::Duration::from_secs(5));
                polls += 1;
            }
        }
    };

    // Step 3: Use the generated image URL for video generation
    let image_input = MediaInput::Image(
        golem_video::exports::golem::video_generation::types::Reference {
            data: golem_video::exports::golem::video_generation::types::InputImage {
                data: MediaData::Url(image_url),
            },
            prompt: Some(prompt),
            role: Some(golem_video::exports::golem::video_generation::types::ImageRole::First),
        },
    );

    let request = media_input_to_request(image_input, config)?;
    let response = client.generate_video(request)?;
    Ok(response.id)
}

pub fn poll_video_generation(
    client: &RunwayApi,
    task_id: String,
) -> Result<VideoResult, VideoError> {
    match client.poll_generation(&task_id) {
        Ok(PollResponse::Processing) => Ok(VideoResult {
            status: JobStatus::Running,
            videos: None,
        }),
        Ok(PollResponse::Complete {
            video_data,
            mime_type,
            uri,
            generation_id,
        }) => {
            let video = Video {
                uri: Some(uri),
                base64_bytes: video_data,
                mime_type,
                width: None,
                height: None,
                fps: None,
                duration_seconds: None,
                generation_id: Some(generation_id),
            };

            Ok(VideoResult {
                status: JobStatus::Succeeded,
                videos: Some(vec![video]),
            })
        }
        Err(error) => Err(error),
    }
}

pub fn cancel_video_generation(client: &RunwayApi, task_id: String) -> Result<String, VideoError> {
    client.cancel_task(&task_id)?;
    Ok(format!("Task {task_id} canceled successfully"))
}

// Text-to-Image functions for Runway
pub fn text_to_image_request(
    prompt: String,
    config: &GenerationConfig,
) -> Result<TextToImageRequest, VideoError> {
    // Parse provider options
    let options: std::collections::HashMap<String, String> = config
        .provider_options
        .as_ref()
        .map(|po| {
            po.iter()
                .map(|kv| (kv.key.clone(), kv.value.clone()))
                .collect()
        })
        .unwrap_or_default();

    // Determine ratio based on aspect_ratio and resolution
    let ratio = determine_text_to_image_ratio(config.aspect_ratio, config.resolution)?;

    // Content moderation
    let content_moderation = options.get("publicFigureThreshold").map(|threshold| {
        let threshold_value = if threshold == "low" { "low" } else { "auto" };
        crate::client::ContentModeration {
            public_figure_threshold: threshold_value.to_string(),
        }
    });

    // Validate seed if provided
    if let Some(seed_val) = config.seed {
        if seed_val > 4294967295 {
            return Err(invalid_input("Seed must be between 0 and 4294967295"));
        }
    }

    Ok(TextToImageRequest {
        prompt_text: prompt,
        ratio,
        model: "gen4_image".to_string(),
        seed: config.seed,
        content_moderation,
    })
}

fn determine_text_to_image_ratio(
    aspect_ratio: Option<AspectRatio>,
    _resolution: Option<Resolution>,
) -> Result<String, VideoError> {
    let target_aspect = aspect_ratio.unwrap_or(AspectRatio::Landscape);

    match target_aspect {
        AspectRatio::Landscape => Ok("1920:1080".to_string()),
        AspectRatio::Portrait => Ok("1080:1920".to_string()),
        AspectRatio::Square => Ok("1024:1024".to_string()),
        AspectRatio::Cinema => Ok("1808:768".to_string()),
    }
}

pub fn generate_text_to_image(
    client: &RunwayApi,
    prompt: String,
    config: &GenerationConfig,
) -> Result<String, VideoError> {
    let request = text_to_image_request(prompt, config)?;
    let response = client.generate_text_to_image(request)?;
    Ok(response.id)
}

pub fn poll_text_to_image_generation(
    client: &RunwayApi,
    task_id: &str,
) -> Result<Option<String>, VideoError> {
    match client.poll_text_to_image(task_id) {
        Ok(ImagePollResponse::Processing) => Ok(None),
        Ok(ImagePollResponse::Complete { image_url }) => Ok(Some(image_url)),
        Err(error) => Err(error),
    }
}

pub fn upscale_video(
    client: &RunwayApi,
    input: golem_video::exports::golem::video_generation::types::BaseVideo,
) -> Result<String, VideoError> {
    let video_uri = match input.data {
        MediaData::Url(url) => Ok(url),
        MediaData::Bytes(_) => Err(VideoError::UnsupportedFeature(
            "Video effects generation is not supported by Runway API".to_string(),
        )),
        // Convert bytes to data URI for video with proper mime type
        // Docs indicate they support bytes, but they aren't clear how
        // so this goes to unsupported for now
        // https://docs.dev.runwayml.com/api/#tag/Start-generating/paths/~1v1~1video_upscale/post
        // https://docs.dev.runwayml.com/assets/inputs/#data-uris-base64-encoded-images
        // below code results in 400 format error
        /*
            use base64::Engine;
            let base64_data = base64::engine::general_purpose::STANDARD.encode(&raw_bytes.bytes);
            let mime_type = if !raw_bytes.mime_type.is_empty() {
                &raw_bytes.mime_type
            } else {
                "video/mp4"
            };
            format!("data:{mime_type};base64,{base64_data}")
        */
    }?;

    let request = VideoUpscaleRequest {
        video_uri,
        model: "upscale_v1".to_string(),
    };

    let response = client.upscale_video(request)?;

    // Return the task ID directly from Runway API
    Ok(response.id)
}

// Unsupported features

pub fn generate_video_effects(
    _client: &RunwayApi,
    _input: golem_video::exports::golem::video_generation::types::InputImage,
    _effect: golem_video::exports::golem::video_generation::types::EffectType,
    _model: Option<String>,
    _duration: Option<f32>,
    _mode: Option<String>,
) -> Result<String, VideoError> {
    Err(VideoError::UnsupportedFeature(
        "Video effects generation is not supported by Runway API".to_string(),
    ))
}

pub fn multi_image_generation(
    _client: &RunwayApi,
    _input_images: Vec<golem_video::exports::golem::video_generation::types::InputImage>,
    _prompt: Option<String>,
    _config: GenerationConfig,
) -> Result<String, VideoError> {
    Err(VideoError::UnsupportedFeature(
        "Multi-image generation is not supported by Runway API".to_string(),
    ))
}

pub fn generate_lip_sync_video(
    _client: &RunwayApi,
    _video: golem_video::exports::golem::video_generation::types::LipSyncVideo,
    _audio: golem_video::exports::golem::video_generation::types::AudioSource,
) -> Result<String, VideoError> {
    Err(VideoError::UnsupportedFeature(
        "Lip sync is not supported by Runway API".to_string(),
    ))
}

pub fn list_available_voices(
    _client: &RunwayApi,
    _language: Option<String>,
) -> Result<Vec<golem_video::exports::golem::video_generation::types::VoiceInfo>, VideoError> {
    Err(VideoError::UnsupportedFeature(
        "Voice listing is not supported by Runway API".to_string(),
    ))
}

pub fn extend_video(
    _client: &RunwayApi,
    _video_id: String,
    _prompt: Option<String>,
    _negative_prompt: Option<String>,
    _cfg_scale: Option<f32>,
    _provider_options: Option<Vec<golem_video::exports::golem::video_generation::types::Kv>>,
) -> Result<String, VideoError> {
    Err(VideoError::UnsupportedFeature(
        "Video extension is not supported by Runway API".to_string(),
    ))
}
