#[allow(static_mut_refs)]
mod bindings;

use crate::bindings::exports::test::video_exports::test_video_api::*;
use crate::bindings::golem::video::types;
use crate::bindings::golem::video::{video_generation, advanced, lip_sync};
use std::fs;
use std::fs::File;
use std::io::Read;
use std::thread;
use std::time::Duration;

struct Component;

impl Guest for Component {

    /// Test1 - Image to video generation with first role and lastframe (both inline raw bytes)
    fn test1() -> String {
        println!("Test1: Image to video with first role and lastframe");
        
        // Load test image for both first and last frame
        let (image_bytes, image_mime_type) = match load_file_bytes("/data/test.png") {
            Ok((bytes, mime_type)) => (bytes, mime_type),
            Err(err) => return format!("ERROR: {}", err),
        };

        // Create configuration with lastframe
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
            provider_options: None,
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
            prompt: Some("A person walking through a beautiful garden".to_string()),
            role: Some(types::ImageRole::First),
        });

        println!("Sending first/last frame video generation request...");
        let job_id = match video_generation::generate(&media_input, &config) {
            Ok(id) => id.trim().to_string(),
            Err(error) => return format!("ERROR: Failed to generate video: {:?}", error),
        };

        poll_job_until_complete(&job_id, "test1")
    }

    /// Test2 - Image to video generation with camera control enum
    fn test2() -> String {
        println!("Test2: Image to video with camera control enum");

        // Load test image
        let (image_bytes, image_mime_type) = match load_file_bytes("/data/test.png") {
            Ok((bytes, mime_type)) => (bytes, mime_type),
            Err(err) => return format!("ERROR: {}", err),
        };

        // Create configuration with camera movement enum
        let config = types::GenerationConfig {
            negative_prompt: Some("static, boring, low quality".to_string()),
            seed: None,
            scheduler: None,
            guidance_scale: Some(7.5),
            aspect_ratio: Some(types::AspectRatio::Cinema),
            model: Some("kling".to_string()),
            duration_seconds: Some(8.0),
            resolution: Some(types::Resolution::Fhd),
            enable_audio: Some(false),
            enhance_prompt: Some(true),
            provider_options: None,
            lastframe: None,
            static_mask: None,
            dynamic_mask: None,
            camera_control: Some(types::CameraControl::Movement(types::CameraMovement::ForwardUp)),
        };

        let media_input = types::MediaInput::Image(types::Reference {
            data: types::InputImage {
                data: types::MediaData::Bytes(types::RawBytes {
                    bytes: image_bytes,
                    mime_type: image_mime_type,
                }),
            },
            prompt: Some("A majestic eagle soaring through mountain peaks".to_string()),
            role: None,
        });

        println!("Sending image-to-video with camera control request...");
        let job_id = match video_generation::generate(&media_input, &config) {
            Ok(id) => id.trim().to_string(),
            Err(error) => return format!("ERROR: Failed to generate video: {:?}", error),
        };

        poll_job_until_complete(&job_id, "test2")
    }

    /// Test3 - Image to video generation with static and dynamic mask (URL input, save job-id for test9)
    fn test3() -> String {
        println!("Test3: Image to video with static and dynamic mask");

        // Create static mask using URL
        let static_mask = types::StaticMask {
            mask: types::InputImage {
                data: types::MediaData::Url("https://example.com/mask.png".to_string()),
            },
        };

        // Create dynamic mask with trajectory points and URL
        let dynamic_mask = types::DynamicMask {
            mask: types::InputImage {
                data: types::MediaData::Url("https://example.com/dynamic-mask.png".to_string()),
            },
            trajectories: vec![
                types::Position { x: 100, y: 100 },
                types::Position { x: 150, y: 120 },
                types::Position { x: 200, y: 150 },
                types::Position { x: 250, y: 180 },
            ],
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
            provider_options: None,
            lastframe: None,
            static_mask: Some(static_mask),
            dynamic_mask: Some(dynamic_mask),
            camera_control: None,
        };

        let media_input = types::MediaInput::Image(types::Reference {
            data: types::InputImage {
                data: types::MediaData::Url("https://example.com/test-image.png".to_string()),
            },
            prompt: Some("Apply motion effects while preserving masked areas".to_string()),
            role: None,
        });

        println!("Sending static and dynamic mask video generation request...");
        let job_id = match video_generation::generate(&media_input, &config) {
            Ok(id) => id.trim().to_string(),
            Err(error) => return format!("ERROR: Failed to generate video: {:?}", error),
        };

        // TODO: Store job_id for test9 - in real implementation this would be persisted
        println!("Job ID for test9: {}", job_id);
        poll_job_until_complete(&job_id, "test3")
    }

    /// Test4 - List voice IDs and their information
    fn test4() -> String {
        println!("Test4: List voice IDs");

        // List all available voices
        match lip_sync::list_voices(None) {
            Ok(voices) => {
                let mut result = String::new();
                result.push_str("Available voices:\n");
                
                for voice in voices {
                    result.push_str(&format!(
                        "Voice ID: {}, Name: {}, Language: {:?}\n",
                        voice.voice_id, voice.name, voice.language
                    ));
                }
                
                if result.len() > 20 {
                    result
                } else {
                    "No voices found".to_string()
                }
            }
            Err(error) => {
                format!("ERROR: Failed to list voices: {:?}", error)
            }
        }
    }

    /// Test5 - Lip-sync video generation using voice-id (inline raw bytes video input)
    fn test5() -> String {
        println!("Test5: Lip-sync with voice-id");

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
            text: "Hello, this is a test of lip-sync functionality".to_string(),
            voice_id: "example_voice_id".to_string(),
            language: types::VoiceLanguage::En,
            speed: 100,
        };

        let audio_source = types::AudioSource::FromText(text_to_speech);

        println!("Sending lip-sync with voice-id request...");
        let job_id = match lip_sync::generate_lip_sync(&base_video, &audio_source) {
            Ok(id) => id.trim().to_string(),
            Err(error) => return format!("ERROR: Failed to generate lip-sync: {:?}", error),
        };

        poll_job_until_complete(&job_id, "test5")
    }

    /// Test6 - Lip-sync video generation using audio file (inline raw bytes audio input)
    fn test6() -> String {
        println!("Test6: Lip-sync with audio file");

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

        poll_job_until_complete(&job_id, "test6")
    }

    /// Test7 - Video effects with single input image (inline raw bytes)
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

        poll_job_until_complete(&job_id, "test7")
    }

    /// Test8 - Video effects with two input images (URLs)
    fn test8() -> String {
        println!("Test8: Video effects with two images");

        let input_image = types::InputImage {
            data: types::MediaData::Url("https://example.com/image1.png".to_string()),
        };

        let second_image = types::InputImage {
            data: types::MediaData::Url("https://example.com/image2.png".to_string()),
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

        poll_job_until_complete(&job_id, "test8")
    }

    /// Test9 - Extend video using job-id from test3
    fn test9() -> String {
        println!("Test9: Extend video using job-id from test3");

        // In a real scenario, you would use the actual job_id from test3
        // For this test, we'll use a placeholder that should be replaced with actual job-id
        let job_id_from_test3 = "test3_job_id_placeholder".to_string();

        println!("Attempting to extend video with job ID: {}", job_id_from_test3);
        
        let provider_options = vec![
            types::Kv {
                key: "extend_duration".to_string(),
                value: "3.0".to_string(),
            },
        ];

        match advanced::extend_video(
            &job_id_from_test3,
            Some("Continue the motion smoothly"),
            Some("abrupt changes"),
            Some(7.5),
            &provider_options,
        ) {
            Ok(extend_job_id) => {
                let extend_job_id = extend_job_id.trim().to_string();
                poll_job_until_complete(&extend_job_id, "test9")
            }
            Err(error) => {
                format!("ERROR: Failed to extend video: {:?}", error)
            }
        }
    }

    fn test10() -> String {
        println!("Test10: Multi-image generation (2 URLs + 1 inline raw bytes)");

        // Load one image as inline bytes  
        let (image_bytes, image_mime_type) = match load_file_bytes("/data/test.png") {
            Ok((bytes, mime_type)) => (bytes, mime_type),
            Err(err) => return format!("ERROR: {}", err),
        };

        // Create a list of 3 images: 2 URLs and 1 inline bytes as specified
        let input_images = vec![
            // First image - URL
            types::InputImage {
                data: types::MediaData::Url("https://example.com/image1.png".to_string()),
            },
            // Second image - URL  
            types::InputImage {
                data: types::MediaData::Url("https://example.com/image2.png".to_string()),
            },
            // Third image - inline raw bytes
            types::InputImage {
                data: types::MediaData::Bytes(types::RawBytes {
                    bytes: image_bytes,
                    mime_type: image_mime_type,
                }),
            },
        ];

        let config = types::GenerationConfig {
            negative_prompt: Some("inconsistent style, poor quality".to_string()),
            seed: None,
            scheduler: None,
            guidance_scale: Some(8.0),
            aspect_ratio: Some(types::AspectRatio::Landscape),
            model: Some("kling".to_string()),
            duration_seconds: Some(8.0),
            resolution: Some(types::Resolution::Fhd),
            enable_audio: Some(false),
            enhance_prompt: Some(true),
            provider_options: Some(vec![
                types::Kv {
                    key: "transition_style".to_string(),
                    value: "smooth".to_string(),
                }
            ]),
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

        poll_job_until_complete(&job_id, "test10")
    }


}

fn save_video_result(video_result: &types::VideoResult, test_name: &str) -> String {
    if let Some(videos) = &video_result.videos {
        for (i, video_data) in videos.iter().enumerate() {
            let filename = format!("/output/video-{}-{}.mp4", test_name, i);
            
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

fn poll_job_until_complete(job_id: &str, test_name: &str) -> String {
    println!("Polling for {} results with job ID: {}", test_name, job_id);

    // Wait 5 seconds after job creation before starting polling
    println!("Waiting 5 seconds for job initialization...");
    thread::sleep(Duration::from_secs(5));

    // Poll every 9 seconds until completion
    loop {
        match video_generation::poll(&job_id) {
            Ok(video_result) => {
                match video_result.status {
                    types::JobStatus::Pending => {
                        println!("{} is pending...", test_name);
                    }
                    types::JobStatus::Running => {
                        println!("{} is running...", test_name);
                    }
                    types::JobStatus::Succeeded => {
                        println!("{} completed successfully!", test_name);
                        let file_path = save_video_result(&video_result, test_name);
                        return format!("{} generated successfully. Saved to: {}", test_name, file_path);
                    }
                    types::JobStatus::Failed(error_msg) => {
                        return format!("{} failed: {}", test_name, error_msg);
                    }
                }
            }
            Err(error) => {
                return format!("Error polling {}: {:?}", test_name, error);
            }
        }
        
        // Wait 9 seconds before polling again
        thread::sleep(Duration::from_secs(9));
    }
}

bindings::export!(Component with_types_in bindings);
