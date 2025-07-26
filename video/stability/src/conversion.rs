use crate::client::{ImageToVideoRequest, PollResponse, StabilityApi, TextToImageRequest};
use golem_video::error::{internal_error, invalid_input, unsupported_feature};
use golem_video::exports::golem::video_generation::types::{
    AspectRatio, GenerationConfig, JobStatus, MediaData, MediaInput, Video, VideoError, VideoResult,
};
use golem_video::utils::download_image_from_url;
use image::ImageFormat;
use std::collections::HashMap;
use std::io::Cursor;

/// Stability API supported dimensions
/// Stability only accepts these resolutions
/// We resize the image to the resolution before sending it to the API
/// Other providers do this at their end
#[derive(Debug, Clone, Copy)]
struct StabilityDimensions {
    width: u32,
    height: u32,
}

impl StabilityDimensions {
    const LANDSCAPE: Self = Self {
        width: 1024,
        height: 576,
    }; // 16:9
    const PORTRAIT: Self = Self {
        width: 576,
        height: 1024,
    }; // 9:16
    const SQUARE: Self = Self {
        width: 768,
        height: 768,
    }; // 1:1
}

/// Helper function to determine target dimensions based on aspect ratio configuration
fn determine_target_dimensions(aspect_ratio: Option<AspectRatio>) -> StabilityDimensions {
    match aspect_ratio {
        Some(AspectRatio::Square) => StabilityDimensions::SQUARE,
        Some(AspectRatio::Portrait) => StabilityDimensions::PORTRAIT,
        Some(AspectRatio::Landscape) | Some(AspectRatio::Cinema) | None => {
            // Default to landscape, cinema maps to 16:9
            if matches!(aspect_ratio, Some(AspectRatio::Cinema)) {
                log::warn!("Cinema aspect ratio mapped to 16:9 landscape for Stability API");
            }
            StabilityDimensions::LANDSCAPE
        }
    }
}

/// Helper function to process image data to meet Stability's dimension requirements
fn process_image_for_stability(
    image_data: &[u8],
    target_dims: StabilityDimensions,
) -> Result<Vec<u8>, VideoError> {
    // Load image from bytes
    let img = image::load_from_memory(image_data)
        .map_err(|e| invalid_input(format!("Failed to decode image: {e}")))?;

    log::debug!(
        "Original image dimensions: {}x{}",
        img.width(),
        img.height()
    );
    log::debug!(
        "Target dimensions: {}x{}",
        target_dims.width,
        target_dims.height
    );

    // Calculate target aspect ratio
    let target_aspect = target_dims.width as f32 / target_dims.height as f32;
    let current_aspect = img.width() as f32 / img.height() as f32;

    // Determine crop dimensions to match target aspect ratio
    let (crop_width, crop_height) = if current_aspect > target_aspect {
        // Image is wider than target, crop width
        let new_width = (img.height() as f32 * target_aspect) as u32;
        (new_width, img.height())
    } else {
        // Image is taller than target, crop height
        let new_height = (img.width() as f32 / target_aspect) as u32;
        (img.width(), new_height)
    };

    // Calculate crop position for center crop
    let crop_x = (img.width().saturating_sub(crop_width)) / 2;
    let crop_y = (img.height().saturating_sub(crop_height)) / 2;

    log::debug!("Cropping to {crop_width}x{crop_height} at ({crop_x}, {crop_y})");

    // Perform center crop
    let cropped = img.crop_imm(crop_x, crop_y, crop_width, crop_height);

    // Resize to target dimensions
    let resized = cropped.resize_exact(
        target_dims.width,
        target_dims.height,
        image::imageops::FilterType::Lanczos3,
    );

    log::debug!(
        "Final processed dimensions: {}x{}",
        resized.width(),
        resized.height()
    );

    // Convert back to bytes (PNG format)
    let mut output = Vec::new();
    let mut cursor = Cursor::new(&mut output);

    resized
        .write_to(&mut cursor, ImageFormat::Png)
        .map_err(|e| internal_error(format!("Failed to encode processed image: {e}")))?;

    Ok(output)
}

/// Helper function to map WIT aspect ratio to Stability text-to-image API format
fn map_aspect_ratio_to_stability_t2i(aspect_ratio: Option<AspectRatio>) -> Option<String> {
    match aspect_ratio {
        Some(AspectRatio::Square) => Some("1:1".to_string()),
        Some(AspectRatio::Portrait) => Some("9:16".to_string()),
        Some(AspectRatio::Landscape) => Some("16:9".to_string()),
        Some(AspectRatio::Cinema) => Some("21:9".to_string()),
        None => None, // Let API use default
    }
}

/// Generate image from text using Stability's text-to-image API
fn generate_image_from_text(
    client: &StabilityApi,
    prompt: String,
    config: &GenerationConfig,
) -> Result<Vec<u8>, VideoError> {
    log::debug!("Generating image from text: {prompt}");

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

    // Get style preset from provider options
    let style_preset = options.get("style_preset").cloned();

    // Validate seed range for text-to-image (same as video generation)
    if let Some(seed_val) = config.seed {
        if seed_val > 4294967294 {
            return Err(invalid_input("Seed must be between 0 and 4294967294"));
        }
    }

    let t2i_request = TextToImageRequest {
        prompt,
        aspect_ratio: map_aspect_ratio_to_stability_t2i(config.aspect_ratio),
        negative_prompt: config.negative_prompt.clone(),
        seed: config.seed,
        style_preset,
        output_format: "png".to_string(),
    };

    match client.generate_text_to_image(t2i_request) {
        Ok(response) => {
            log::debug!("Successfully generated image from text");
            if let Some(seed) = response.seed {
                log::debug!("Text-to-image used seed: {seed}");
            }
            if let Some(finish_reason) = response.finish_reason {
                log::debug!("Text-to-image finish reason: {finish_reason}");
            }
            Ok(response.image_data)
        }
        Err(err) => {
            log::error!("Failed to generate image from text: {err:?}");
            Err(internal_error(format!(
                "Text-to-image generation failed: {err}"
            )))
        }
    }
}

// Make request for video generation
pub fn media_input_to_request(
    input: MediaInput,
    config: GenerationConfig,
) -> Result<ImageToVideoRequest, VideoError> {
    match input {
        MediaInput::Text(_) => Err(internal_error(
            "Text processing should be handled in generate_video function",
        )),
        MediaInput::Video(_) => Err(unsupported_feature(
            "Video processing error should be handled in generate_video function",
        )),
        MediaInput::Image(ref_image) => {
            // Determine target dimensions based on aspect ratio config
            let target_dims = determine_target_dimensions(config.aspect_ratio);

            // Extract and process image data
            let processed_image_data = match ref_image.data.data {
                MediaData::Url(url) => {
                    // Download the image from the URL and process it
                    let raw_bytes = download_image_from_url(&url)?;
                    process_image_for_stability(&raw_bytes.bytes, target_dims)?
                }
                MediaData::Bytes(raw_bytes) => {
                    // Process the image bytes directly
                    process_image_for_stability(&raw_bytes.bytes, target_dims)?
                }
            };

            // Note: Stability doesn't support prompts with images and image roles, so we ignore prompt and role
            if ref_image.role.is_some() {
                log::warn!("image role positioning (first/last) is not supported by Stability API and will be ignored");
            }

            // Parse provider options - only for parameters not directly supported in WIT
            let options: HashMap<String, String> = config
                .provider_options
                .map(|po| po.into_iter().map(|kv| (kv.key, kv.value)).collect())
                .unwrap_or_default();

            // Use built-in config fields directly
            let seed = config.seed;
            let cfg_scale = config.guidance_scale;

            // motion_bucket_id is only available via provider options since it's Stability-specific
            let motion_bucket_id = options
                .get("motion_bucket_id")
                .and_then(|s| s.parse::<u32>().ok());

            // Validate parameter ranges according to Stability API
            if let Some(seed_val) = seed {
                if seed_val > 4294967294 {
                    return Err(invalid_input("Seed must be between 0 and 4294967294"));
                }
            }

            if let Some(cfg_val) = cfg_scale {
                if !(0.0..=10.0).contains(&cfg_val) {
                    return Err(invalid_input(
                        "CFG scale (guidance_scale) must be between 0.0 and 10.0",
                    ));
                }
            }

            if let Some(bucket_val) = motion_bucket_id {
                if !(1..=255).contains(&bucket_val) {
                    return Err(invalid_input("Motion bucket ID must be between 1 and 255"));
                }
            }

            // Log warnings for unsupported built-in options
            if config.model.is_some() {
                log::warn!("model is not supported by Stability API and will be ignored");
            }
            if config.negative_prompt.is_some() {
                log::warn!("negative_prompt is not supported by Stability API and will be ignored");
            }
            if config.scheduler.is_some() {
                log::warn!("scheduler is not supported by Stability API and will be ignored");
            }
            if config.aspect_ratio.is_some() {
                log::info!(
                    "aspect_ratio processed and mapped to Stability dimensions: {}x{}",
                    target_dims.width,
                    target_dims.height
                );
            }
            if config.duration_seconds.is_some() {
                log::warn!(
                    "duration_seconds is not supported by Stability API and will be ignored"
                );
            }
            if config.resolution.is_some() {
                log::warn!("resolution is handled by aspect ratio mapping to Stability dimensions");
            }
            if config.enable_audio.is_some() {
                log::warn!("enable_audio is not supported by Stability API and will be ignored");
            }
            if config.enhance_prompt.is_some() {
                log::warn!("enhance_prompt is not supported by Stability API and will be ignored");
            }
            if config.lastframe.is_some() {
                log::warn!("lastframe is not supported by Stability API and will be ignored");
            }
            if config.static_mask.is_some() {
                log::warn!("static_mask is not supported by Stability API and will be ignored");
            }
            if config.dynamic_mask.is_some() {
                log::warn!("dynamic_mask is not supported by Stability API and will be ignored");
            }
            if config.camera_control.is_some() {
                log::warn!("camera_control is not supported by Stability API and will be ignored");
            }

            Ok(ImageToVideoRequest {
                image_data: processed_image_data,
                seed,
                cfg_scale,
                motion_bucket_id,
            })
        }
    }
}

// Generate video from text or image, text->image->video, from video is unsupported
pub fn generate_video(
    client: &StabilityApi,
    input: MediaInput,
    config: GenerationConfig,
) -> Result<String, VideoError> {
    match input {
        MediaInput::Text(prompt) => {
            log::info!("Processing text-to-video request via text-to-image + image-to-video");

            // First generate image from text
            let image_data = generate_image_from_text(client, prompt, &config)?;

            // Create a new MediaInput with the generated image
            let image_input = MediaInput::Image(
                golem_video::exports::golem::video_generation::types::Reference {
                    data: golem_video::exports::golem::video_generation::types::InputImage {
                        data: MediaData::Bytes(
                            golem_video::exports::golem::video_generation::types::RawBytes {
                                bytes: image_data,
                                mime_type: "image/png".to_string(),
                            },
                        ),
                    },
                    prompt: None,
                    role: None,
                },
            );

            // Now generate video from the image
            let request = media_input_to_request(image_input, config)?;
            let response = client.generate_video(request)?;

            log::info!("Successfully initiated text-to-video generation");
            Ok(response.id)
        }
        MediaInput::Image(_) => {
            let request = media_input_to_request(input, config)?;
            let response = client.generate_video(request)?;
            Ok(response.id)
        }
        MediaInput::Video(_) => Err(unsupported_feature(
            "Video-to-video is not supported by Stability API",
        )),
    }
}

// Poll for video generation status
pub fn poll_video_generation(
    client: &StabilityApi,
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
        }) => {
            let video = Video {
                uri: None,
                base64_bytes: Some(video_data),
                mime_type,
                width: None,
                height: None,
                fps: None,
                duration_seconds: None,
                generation_id: None,
            };

            Ok(VideoResult {
                status: JobStatus::Succeeded,
                videos: Some(vec![video]),
            })
        }
        Err(error) => Err(error),
    }
}

// Unsupported features

pub fn cancel_video_generation(_task_id: String) -> Result<String, VideoError> {
    Err(unsupported_feature(
        "Video generation cancellation is not supported by Stability API",
    ))
}

pub fn generate_lip_sync_video(
    _client: &StabilityApi,
    _video: golem_video::exports::golem::video_generation::types::LipSyncVideo,
    _audio: golem_video::exports::golem::video_generation::types::AudioSource,
) -> Result<String, VideoError> {
    Err(VideoError::UnsupportedFeature(
        "Lip sync is not supported by Stability API".to_string(),
    ))
}

pub fn list_available_voices(
    _client: &StabilityApi,
    _language: Option<String>,
) -> Result<Vec<golem_video::exports::golem::video_generation::types::VoiceInfo>, VideoError> {
    Err(VideoError::UnsupportedFeature(
        "Voice listing is not supported by Stability API".to_string(),
    ))
}

pub fn extend_video(
    _client: &StabilityApi,
    _video_id: String,
    _prompt: Option<String>,
    _negative_prompt: Option<String>,
    _cfg_scale: Option<f32>,
    _provider_options: Option<Vec<golem_video::exports::golem::video_generation::types::Kv>>,
) -> Result<String, VideoError> {
    Err(VideoError::UnsupportedFeature(
        "Video extension is not supported by Stability API".to_string(),
    ))
}

pub fn upscale_video(
    _client: &StabilityApi,
    _input: golem_video::exports::golem::video_generation::types::BaseVideo,
) -> Result<String, VideoError> {
    Err(VideoError::UnsupportedFeature(
        "Video upscaling is not supported by Stability API".to_string(),
    ))
}

pub fn generate_video_effects(
    _client: &StabilityApi,
    _input: golem_video::exports::golem::video_generation::types::InputImage,
    _effect: golem_video::exports::golem::video_generation::types::EffectType,
    _model: Option<String>,
    _duration: Option<f32>,
    _mode: Option<String>,
) -> Result<String, VideoError> {
    Err(VideoError::UnsupportedFeature(
        "Video effects generation is not supported by Stability API".to_string(),
    ))
}

pub fn multi_image_generation(
    _client: &StabilityApi,
    _input_images: Vec<golem_video::exports::golem::video_generation::types::InputImage>,
    _prompt: Option<String>,
    _config: GenerationConfig,
) -> Result<String, VideoError> {
    Err(VideoError::UnsupportedFeature(
        "Multi-image generation is not supported by Stability API".to_string(),
    ))
}
