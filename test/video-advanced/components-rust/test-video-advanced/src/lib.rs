#[allow(static_mut_refs)]
mod bindings;

use crate::bindings::exports::test::video_advanced_exports::test_video_advanced_api::*;
use crate::bindings::golem::video::types;
use crate::bindings::golem::video::{video_generation, advanced, lip_sync};
use std::fs;
use std::fs::File;
use std::io::Read;
use std::thread;
use std::time::Duration;

struct Component;

impl Guest for Component {

    /// Test1 - Image to video generation using first and last frame from kling video module
    fn test1() -> String {
        println!("Test1: Image to video with first and last frame");
        
        // Load test image
        let (image_bytes, image_mime_type) = match load_file_bytes("/data/test.png") {
            Ok((bytes, mime_type)) => (bytes, mime_type),
            Err(err) => return format!("ERROR: {}", err),
        };

        // Create configuration with first and last frame setup
        let config = types::GenerationConfig {
            negative_prompt: Some("blurry, low quality, distorted".to_string()),
            seed: None,
            scheduler: None,
            guidance_scale: None,
            aspect_ratio: Some(types::AspectRatio::Landscape),
            model: Some("kling".to_string()),
            duration_seconds: Some(5.0),
            resolution: Some(types::Resolution::Hd),
            enable_audio: Some(false),
            enhance_prompt: Some(true),
            provider_options: vec![],
            lastframe: Some(types::InputImage {
                data: types::MediaData::Bytes(types::RawBytes {
                    bytes: image_bytes.clone(),
                    mime_type: image_mime_type.clone(),
                }),
            }),
            static_mask: None,
            dynamic_mask: None,
            camera_control: None,
        };

        // Create media input with first frame image
        let media_input = types::MediaInput::Image(types::Reference {
            data: types::InputImage {
                data: types::MediaData::Bytes(types::RawBytes {
                    bytes: image_bytes,
                    mime_type: image_mime_type,
                }),
            },
            prompt: Some("A person walking through a beautiful garden, smooth motion".to_string()),
            role: Some(types::ImageRole::First),
        });

        println!("Sending first/last frame video generation request...");
        let job_id = match video_generation::generate(&media_input, &config) {
            Ok(id) => id.trim().to_string(),
            Err(error) => return format!("ERROR: Failed to generate video: {:?}", error),
        };

        poll_job_until_complete(&job_id, "First/Last frame video generation")
    }

    /// Test2 - Text to video generation using text-to-speech and advanced camera controls
    fn test2() -> String {
        println!("Test2: Text to video with TTS and advanced camera controls");

        // Create advanced camera configuration
        let camera_control = types::CameraControl::Config(types::CameraConfig {
            horizontal: 0.1,
            vertical: 0.05,
            pan: 15.0,
            tilt: 10.0,
            zoom: 1.2,
            roll: 0.0,
        });

        let config = types::GenerationConfig {
            negative_prompt: Some("static, boring, low quality".to_string()),
            seed: None,
            scheduler: None,
            guidance_scale: Some(7.5),
            aspect_ratio: Some(types::AspectRatio::Cinema),
            model: Some("kling".to_string()),
            duration_seconds: Some(8.0),
            resolution: Some(types::Resolution::Fhd),
            enable_audio: Some(true),
            enhance_prompt: Some(true),
            provider_options: vec![
                types::Kv {
                    key: "use_tts".to_string(),
                    value: "true".to_string(),
                },
                types::Kv {
                    key: "voice_id".to_string(),
                    value: "example_voice".to_string(),
                }
            ],
            lastframe: None,
            static_mask: None,
            dynamic_mask: None,
            camera_control: Some(camera_control),
        };

        let media_input = types::MediaInput::Text(
            "A majestic eagle soaring through mountain peaks at sunset, with dynamic camera movement following its flight path".to_string()
        );

        println!("Sending text-to-video with TTS and camera controls request...");
        let job_id = match video_generation::generate(&media_input, &config) {
            Ok(id) => id.trim().to_string(),
            Err(error) => return format!("ERROR: Failed to generate video: {:?}", error),
        };

        poll_job_until_complete(&job_id, "Text-to-video with TTS and camera controls")
    }

    /// Test3 - Image to video generation using static mask image
    fn test3() -> String {
        println!("Test3: Image to video with static mask");

        // Load test image and mask
        let (image_bytes, image_mime_type) = match load_file_bytes("/data/test.png") {
            Ok((bytes, mime_type)) => (bytes, mime_type),
            Err(err) => return format!("ERROR: {}", err),
        };

        let (mask_bytes, mask_mime_type) = match load_file_bytes("/data/mask.png") {
            Ok((bytes, mime_type)) => (bytes, mime_type),
            Err(err) => return format!("ERROR: {}", err),
        };

        let static_mask = types::StaticMask {
            mask: types::InputImage {
                data: types::MediaData::Bytes(types::RawBytes {
                    bytes: mask_bytes,
                    mime_type: mask_mime_type,
                }),
            },
        };

        let config = types::GenerationConfig {
            negative_prompt: Some("artifacts, distortion".to_string()),
            seed: None,
            scheduler: None,
            guidance_scale: None,
            aspect_ratio: Some(types::AspectRatio::Landscape),
            model: Some("kling".to_string()),
            duration_seconds: Some(4.0),
            resolution: Some(types::Resolution::Hd),
            enable_audio: Some(false),
            enhance_prompt: Some(false),
            provider_options: vec![],
            lastframe: None,
            static_mask: Some(static_mask),
            dynamic_mask: None,
            camera_control: None,
        };

        let media_input = types::MediaInput::Image(types::Reference {
            data: types::InputImage {
                data: types::MediaData::Bytes(types::RawBytes {
                    bytes: image_bytes,
                    mime_type: image_mime_type,
                }),
            },
            prompt: Some("Apply motion effects while preserving masked areas".to_string()),
            role: None,
        });

        println!("Sending static mask video generation request...");
        let job_id = match video_generation::generate(&media_input, &config) {
            Ok(id) => id.trim().to_string(),
            Err(error) => return format!("ERROR: Failed to generate video: {:?}", error),
        };

        poll_job_until_complete(&job_id, "Static mask video generation")
    }

    /// Test4 - Image to video generation using dynamic mask image
    fn test4() -> String {
        println!("Test4: Image to video with dynamic mask");

        // Load test image and mask
        let (image_bytes, image_mime_type) = match load_file_bytes("/data/test.png") {
            Ok((bytes, mime_type)) => (bytes, mime_type),
            Err(err) => return format!("ERROR: {}", err),
        };

        let (mask_bytes, mask_mime_type) = match load_file_bytes("/data/mask.png") {
            Ok((bytes, mime_type)) => (bytes, mime_type),
            Err(err) => return format!("ERROR: {}", err),
        };

        // Create dynamic mask with trajectory points
        let dynamic_mask = types::DynamicMask {
            mask: types::InputImage {
                data: types::MediaData::Bytes(types::RawBytes {
                    bytes: mask_bytes,
                    mime_type: mask_mime_type,
                }),
            },
            trajectories: vec![
                types::Position { x: 100, y: 100 },
                types::Position { x: 150, y: 120 },
                types::Position { x: 200, y: 150 },
                types::Position { x: 250, y: 180 },
            ],
        };

        let config = types::GenerationConfig {
            negative_prompt: Some("static, rigid movement".to_string()),
            seed: None,
            scheduler: None,
            guidance_scale: None,
            aspect_ratio: Some(types::AspectRatio::Landscape),
            model: Some("kling".to_string()),
            duration_seconds: Some(6.0),
            resolution: Some(types::Resolution::Hd),
            enable_audio: Some(false),
            enhance_prompt: Some(true),
            provider_options: vec![],
            lastframe: None,
            static_mask: None,
            dynamic_mask: Some(dynamic_mask),
            camera_control: None,
        };

        let media_input = types::MediaInput::Image(types::Reference {
            data: types::InputImage {
                data: types::MediaData::Bytes(types::RawBytes {
                    bytes: image_bytes,
                    mime_type: image_mime_type,
                }),
            },
            prompt: Some("Dynamic motion following the trajectory path".to_string()),
            role: None,
        });

        println!("Sending dynamic mask video generation request...");
        let job_id = match video_generation::generate(&media_input, &config) {
            Ok(id) => id.trim().to_string(),
            Err(error) => return format!("ERROR: Failed to generate video: {:?}", error),
        };

        poll_job_until_complete(&job_id, "Dynamic mask video generation")
    }

    /// Test5 - Lip-sync video generation using audio file
    fn test5() -> String {
        println!("Test5: Lip-sync with audio file");

        // Load base video and audio file
        let (video_bytes, video_mime_type) = match load_file_bytes("/data/video.mp4") {
            Ok((bytes, mime_type)) => (bytes, mime_type),
            Err(err) => return format!("ERROR loading video: {}", err),
        };

        let (audio_bytes, audio_mime_type) = match load_file_bytes("/data/audio.mp3") {
            Ok((bytes, mime_type)) => (bytes, mime_type),
            Err(err) => return format!("ERROR loading audio: {}", err),
        };

        let base_video = types::BaseVideo {
            data: types::MediaData::Bytes(types::RawBytes {
                bytes: video_bytes,
                mime_type: video_mime_type,
            }),
        };

        let audio_source = types::AudioSource::FromAudio(types::Narration {
            data: types::MediaData::Bytes(types::RawBytes {
                bytes: audio_bytes,
                mime_type: audio_mime_type,
            }),
        });

        println!("Sending lip-sync with audio file request...");
        let job_id = match lip_sync::generate_lip_sync(&base_video, &audio_source) {
            Ok(id) => id.trim().to_string(),
            Err(error) => return format!("ERROR: Failed to generate lip-sync: {:?}", error),
        };

        poll_job_until_complete(&job_id, "Lip-sync with audio file")
    }

    /// Test6 - Lip-sync video generation using voice-id and text
    fn test6() -> String {
        println!("Test6: Lip-sync with voice-id and text");

        // Load base video
        let (video_bytes, video_mime_type) = match load_file_bytes("/data/video.mp4") {
            Ok((bytes, mime_type)) => (bytes, mime_type),
            Err(err) => return format!("ERROR loading video: {}", err),
        };

        let base_video = types::BaseVideo {
            data: types::MediaData::Bytes(types::RawBytes {
                bytes: video_bytes,
                mime_type: video_mime_type,
            }),
        };

        let text_to_speech = types::TextToSpeech {
            text: "Hello, this is a test of lip-sync functionality with generated speech".to_string(),
            voice_id: "example_voice_id".to_string(),
            language: types::VoiceLanguage::En,
            speed: 100,
        };

        let audio_source = types::AudioSource::FromText(text_to_speech);

        println!("Sending lip-sync with TTS request...");
        let job_id = match lip_sync::generate_lip_sync(&base_video, &audio_source) {
            Ok(id) => id.trim().to_string(),
            Err(error) => return format!("ERROR: Failed to generate lip-sync: {:?}", error),
        };

        poll_job_until_complete(&job_id, "Lip-sync with TTS")
    }

    /// Test7 - Video effects with single input image
    fn test7() -> String {
        println!("Test7: Video effects with single image");

        let (image_bytes, image_mime_type) = match load_file_bytes("/data/test.png") {
            Ok((bytes, mime_type)) => (bytes, mime_type),
            Err(err) => return format!("ERROR: {}", err),
        };

        let input_image = types::InputImage {
            data: types::MediaData::Bytes(types::RawBytes {
                bytes: image_bytes,
                mime_type: image_mime_type,
            }),
        };

        let effect = types::EffectType::Single(types::SingleImageEffects::Bloombloom);

        println!("Sending single image effect request...");
        let job_id = match advanced::generate_video_effects(
            &input_image,
            &effect,
            Some("kling"),
            Some(3.0),
            Some("creative")
        ) {
            Ok(id) => id.trim().to_string(),
            Err(error) => return format!("ERROR: Failed to generate video effects: {:?}", error),
        };

        poll_job_until_complete(&job_id, "Single image video effects")
    }

    /// Test8 - Video effects with two input images
    fn test8() -> String {
        println!("Test8: Video effects with two images");

        let (image_bytes, image_mime_type) = match load_file_bytes("/data/test.png") {
            Ok((bytes, mime_type)) => (bytes, mime_type),
            Err(err) => return format!("ERROR loading first image: {}", err),
        };

        // Use the same image as second image for demo purposes
        let (second_image_bytes, second_image_mime_type) = match load_file_bytes("/data/test.png") {
            Ok((bytes, mime_type)) => (bytes, mime_type),
            Err(err) => return format!("ERROR loading second image: {}", err),
        };

        let input_image = types::InputImage {
            data: types::MediaData::Bytes(types::RawBytes {
                bytes: image_bytes,
                mime_type: image_mime_type,
            }),
        };

        let second_image = types::InputImage {
            data: types::MediaData::Bytes(types::RawBytes {
                bytes: second_image_bytes,
                mime_type: second_image_mime_type,
            }),
        };

        let dual_effect = types::DualEffect {
            effect: types::DualImageEffects::Hug,
            second_image,
        };

        let effect = types::EffectType::Dual(dual_effect);

        println!("Sending dual image effect request...");
        let job_id = match advanced::generate_video_effects(
            &input_image,
            &effect,
            Some("kling"),
            Some(4.0),
            Some("interaction")
        ) {
            Ok(id) => id.trim().to_string(),
            Err(error) => return format!("ERROR: Failed to generate video effects: {:?}", error),
        };

        poll_job_until_complete(&job_id, "Dual image video effects")
    }

    /// Test9 - Multi image to video generation
    fn test9() -> String {
        println!("Test9: Multi-image to video generation");

        // Load multiple images (using same image multiple times for demo)
        let (image_bytes, image_mime_type) = match load_file_bytes("/data/test.png") {
            Ok((bytes, mime_type)) => (bytes, mime_type),
            Err(err) => return format!("ERROR: {}", err),
        };

        let input_images = vec![
            types::InputImage {
                data: types::MediaData::Bytes(types::RawBytes {
                    bytes: image_bytes.clone(),
                    mime_type: image_mime_type.clone(),
                }),
            },
            types::InputImage {
                data: types::MediaData::Bytes(types::RawBytes {
                    bytes: image_bytes.clone(),
                    mime_type: image_mime_type.clone(),
                }),
            },
            types::InputImage {
                data: types::MediaData::Bytes(types::RawBytes {
                    bytes: image_bytes,
                    mime_type: image_mime_type,
                }),
            },
        ];

        let config = types::GenerationConfig {
            negative_prompt: Some("inconsistent, jarring transitions".to_string()),
            seed: None,
            scheduler: None,
            guidance_scale: Some(8.0),
            aspect_ratio: Some(types::AspectRatio::Landscape),
            model: Some("kling".to_string()),
            duration_seconds: Some(10.0),
            resolution: Some(types::Resolution::Fhd),
            enable_audio: Some(false),
            enhance_prompt: Some(true),
            provider_options: vec![
                types::Kv {
                    key: "transition_style".to_string(),
                    value: "smooth".to_string(),
                }
            ],
            lastframe: None,
            static_mask: None,
            dynamic_mask: None,
            camera_control: None,
        };

        println!("Sending multi-image generation request...");
        let job_id = match advanced::multi_image_generation(&input_images, &config) {
            Ok(id) => id.trim().to_string(),
            Err(error) => return format!("ERROR: Failed to generate multi-image video: {:?}", error),
        };

        poll_job_until_complete(&job_id, "Multi-image video generation")
    }
}

fn save_video_result(video_result: &types::VideoResult, _job_id: &str) -> String {
    if let Some(videos) = &video_result.videos {
        for (i, video_data) in videos.iter().enumerate() {
            let filename = format!("/output/video-{}.mp4", i);
            
            // Create output directory if it doesn't exist
            if let Err(err) = fs::create_dir_all("/output") {
                return format!("Failed to create output directory: {}", err);
            }
            
            // Save the video data
            match &video_data.base64_bytes {
                Some(video_bytes) => {
                    match fs::write(&filename, video_bytes) {
                        Ok(_) => {
                            return filename;
                        }
                        Err(err) => {
                            return format!("Failed to save video to {}: {}", filename, err);
                        }
                    }
                }
                None => {
                    if let Some(uri) = &video_data.uri {
                        return format!("Video available at URI: {}", uri);
                    } else {
                        return "No video data or URI available".to_string();
                    }
                }
            }
        }
        "No videos in result".to_string()
    } else {
        "No videos in result".to_string()
    }
}

fn load_file_bytes(path: &str) -> Result<(Vec<u8>, String), String> {
    println!("Reading file from: {}", path);
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(err) => return Err(format!("Failed to open {}: {}", path, err)),
    };

    let mut buffer = Vec::new();
    match file.read_to_end(&mut buffer) {
        Ok(_) => {
            println!("Successfully read {} bytes from {}", buffer.len(), path);
            let mime_type = match path.rsplit('.').next() {
                Some("png") => "image/png".to_string(),
                Some("mp4") => "video/mp4".to_string(),
                Some("mp3") => "audio/mpeg".to_string(),
                _ => "application/octet-stream".to_string(), // Default or unknown
            };
            Ok((buffer, mime_type))
        }
        Err(err) => Err(format!("Failed to read {}: {}", path, err)),
    }
}

fn poll_job_until_complete(job_id: &str, operation_name: &str) -> String {
    println!("Polling for {} results with job ID: {}", operation_name, job_id);

    // Wait 5 seconds after job creation before starting polling
    println!("Waiting 5 seconds for job initialization...");
    thread::sleep(Duration::from_secs(5));

    // Poll every 9 seconds until completion
    loop {
        match video_generation::poll(&job_id) {
            Ok(video_result) => {
                match video_result.status {
                    types::JobStatus::Pending => {
                        println!("{} is pending...", operation_name);
                    }
                    types::JobStatus::Running => {
                        println!("{} is running...", operation_name);
                    }
                    types::JobStatus::Succeeded => {
                        println!("{} completed successfully!", operation_name);
                        let file_path = save_video_result(&video_result, &job_id);
                        return format!("{} generated successfully. Saved to: {}", operation_name, file_path);
                    }
                    types::JobStatus::Failed(error_msg) => {
                        return format!("{} failed: {}", operation_name, error_msg);
                    }
                }
            }
            Err(error) => {
                return format!("Error polling {}: {:?}", operation_name, error);
            }
        }
        
        // Wait 9 seconds before polling again
        thread::sleep(Duration::from_secs(9));
    }
}

bindings::export!(Component with_types_in bindings);
