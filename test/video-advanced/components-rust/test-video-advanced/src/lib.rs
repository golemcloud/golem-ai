#[allow(static_mut_refs)]
mod bindings;

use crate::bindings::exports::test::video_advanced_exports::test_video_api::*;
use crate::bindings::golem::video::types;
use crate::bindings::golem::video::{video_generation, advanced, lip_sync};
use std::fs;
use std::fs::File;
use std::io::Read;
use std::thread;
use std::time::Duration;
use std::sync::Mutex;

static JOB_ID_STORAGE: Mutex<String> = Mutex::new(String::new());

struct Component;

impl Guest for Component {

    /// Test1 - Image to video generation with first frame and last frame included (both inline images)
    fn test1() -> String {
        println!("Test1: Image to video with first frame and last frame");
        
        // Load test image for both first and last frame
        let (first_image_bytes, first_image_mime_type) = match load_file_bytes("/data/first.png") {
            Ok((bytes, mime_type)) => (bytes, mime_type),
            Err(err) => return format!("ERROR: {}", err),
        };

        let (last_image_bytes, last_image_mime_type) = match load_file_bytes("/data/last.png") {
            Ok((bytes, mime_type)) => (bytes, mime_type),
            Err(err) => return format!("ERROR: {}", err),
        };

        // Create configuration with lastframe
        let config = types::GenerationConfig {
            negative_prompt: Some("blurry, low quality, distorted".to_string()),
            seed: None,
            scheduler: None,
            guidance_scale: None,
            aspect_ratio: Some(types::AspectRatio::Square),
            model: None,
            duration_seconds: Some(5.0),
            resolution: Some(types::Resolution::Hd),
            enable_audio: Some(false),
            enhance_prompt: Some(true),
            provider_options: None,
            lastframe: Some(types::InputImage {
                data: types::MediaData::Bytes(types::RawBytes {
                    bytes: last_image_bytes.clone(),
                    mime_type: last_image_mime_type.clone(),
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
                    bytes: first_image_bytes,
                    mime_type: first_image_mime_type,
                }),
            },
            prompt: Some("A close up shot of eagle that slowly zooms into its eyes, and then it zooms out to a headshot of a majestic lion, smooth camera movement" .to_string()),
            role: Some(types::ImageRole::First),
        });

        println!("Sending first/last frame video generation request...");
        let job_id = match video_generation::generate(&media_input, &config) {
            Ok(id) => id.trim().to_string(),
            Err(error) => return format!("ERROR: Failed to generate video: {:?}", error),
        };

        poll_job_until_complete(&job_id, "test1")
    }

    /// Test2 - Image to video generation with advancedcamera control enum
    fn test2() -> String {
        println!("Test2: Image to video with advancedcamera control enum");

        // Load test image
        let (image_bytes, image_mime_type) = match load_file_bytes("/data/cameracontrol.jpeg") {
            Ok((bytes, mime_type)) => (bytes, mime_type),
            Err(err) => return format!("ERROR: {}", err),
        };

        // Create configuration with camera movement enum
        let config = types::GenerationConfig {
            negative_prompt: Some("static, boring, low quality".to_string()),
            seed: None,
            scheduler: None,
            guidance_scale: Some(7.5),
            aspect_ratio: Some(types::AspectRatio::Square),
            model: None,
            duration_seconds: Some(5.0),
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
            prompt: Some("The scally dragon slowly breaths embers and smoke, it eyes glow and spark, the flame make the dragon light up".to_string()),
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
                data: types::MediaData::Url("https://h2.inkwai.com/bs2/upload-ylab-stunt/ai_portal/1732888177/cOLNrShrSO/static_mask.png".to_string()),
            },
        };

        // Create dynamic mask with trajectory points and URL
        let dynamic_mask = types::DynamicMask {
            mask: types::InputImage {
                data: types::MediaData::Url("https://h2.inkwai.com/bs2/upload-ylab-stunt/ai_portal/1732888130/WU8spl23dA/dynamic_mask_1.png".to_string()),
            },
            trajectories: vec![
                types::Position { x: 279, y: 219 },
                types::Position { x: 417, y: 65 },
            ],
        };

        let config = types::GenerationConfig {
            negative_prompt: None,
            seed: None,
            scheduler: None,
            guidance_scale: None,
            aspect_ratio: Some(types::AspectRatio::Landscape),
            model: None,
            duration_seconds: Some(5.0),
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
                data: types::MediaData::Url("https://h2.inkwai.com/bs2/upload-ylab-stunt/se/ai_portal_queue_mmu_image_upscale_aiweb/3214b798-e1b4-4b00-b7af-72b5b0417420_raw_image_0.jpg".to_string()),
            },

           prompt: Some("The astronaut stood up and walked away".to_string()),
            role: None,
        });

        println!("Sending static and dynamic mask video generation request...");
        let job_id = match video_generation::generate(&media_input, &config) {
            Ok(id) => id.trim().to_string(),
            Err(error) => return format!("ERROR: Failed to generate video: {:?}", error),
        };

        // Store the job ID in the Mutex
        *JOB_ID_STORAGE.lock().unwrap() = job_id.clone();
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

                result
            }
            Err(error) => {
                format!("ERROR: Failed to list voices: {:?}", error)
            }
        }
    }

    /// Test5 - Lip-sync video generation using voice-id (inline raw bytes video input)
    fn test5() -> String {
        println!("Test5: Lip-sync with voice-id");

        let base_video = types::BaseVideo {
            data: types::MediaData::Url("https://v1-kling.klingai.com/kcdn/cdn-kcdn112452/kling-api-document/videos/sing-1.mp4".to_string()),
        };

        let lip_sync_video = types::LipSyncVideo::Video(base_video);

        let text_to_speech = types::TextToSpeech {
            text: "Hello, this is a test of Lip Sync functionality in golem video".to_string(),
            voice_id: "genshin_vindi2".to_string(),
            language: types::VoiceLanguage::En,
            speed: 100,
        };

        let audio_source = types::AudioSource::FromText(text_to_speech);

        println!("Sending lip-sync with voice-id request...");
        let job_id = match lip_sync::generate_lip_sync(&lip_sync_video, &audio_source) {
            Ok(id) => id.trim().to_string(),
            Err(error) => return format!("ERROR: Failed to generate lip-sync: {:?}", error),
        };

        poll_job_until_complete(&job_id, "test5")
    }

    /// Test6 - Lip-sync video generation using audio file (inline raw bytes audio input)
    fn test6() -> String {
        println!("Test6: Lip-sync with audio file");

        let (audio_bytes, audio_mime_type) = match load_file_bytes("/data/audio.wav") {
            Ok((bytes, mime_type)) => (bytes, mime_type),
            Err(err) => return format!("ERROR loading audio: {}", err),
        };

        let base_video = types::BaseVideo {
            data: types::MediaData::Url("https://v1-kling.klingai.com/kcdn/cdn-kcdn112452/kling-api-document/videos/sing-1.mp4".to_string()),
        };

        let lip_sync_video = types::LipSyncVideo::Video(base_video);

        let audio_source = types::AudioSource::FromAudio(types::Narration {
            data: types::MediaData::Bytes(types::RawBytes {
                bytes: audio_bytes,
                mime_type: audio_mime_type,
            }),
        });

        println!("Sending lip-sync with audio file request...");
        let job_id = match lip_sync::generate_lip_sync(&lip_sync_video, &audio_source) {
            Ok(id) => id.trim().to_string(),
            Err(error) => return format!("ERROR: Failed to generate lip-sync: {:?}", error),
        };

        poll_job_until_complete(&job_id, "test6")
    }

    /// Test7 - Video effects with single input image (inline raw bytes) amd effect boom
    fn test7() -> String {
        println!("Test7: Video effects with single image");

        let (image_bytes, image_mime_type) = match load_file_bytes("/data/single-effect.jpeg") {
            Ok((bytes, mime_type)) => (bytes, mime_type),
            Err(err) => return format!("ERROR: {}", err),
        };

        let input_image = types::InputImage {
            data: types::MediaData::Bytes(types::RawBytes {
                bytes: image_bytes,
                mime_type: image_mime_type,
            }),
        };

        let effect = types::EffectType::Single(types::SingleImageEffects::Fuzzyfuzzy);

        println!("Sending single image effect request...");
        let job_id = match advanced::generate_video_effects(
            &input_image,
            &effect,
            None,
            None,
            None,
        ) {
            Ok(id) => id.trim().to_string(),
            Err(error) => return format!("ERROR: Failed to generate video effects: {:?}", error),
        };

        poll_job_until_complete(&job_id, "test7")
    }

    /// Test8 - Video effects with two input images (URLs) and effect "hug"
    fn test8() -> String {
        println!("Test8: Video effects with two images");

        let input_image = types::InputImage {
            data: types::MediaData::Url("https://p2-kling.klingai.com/bs2/upload-ylab-stunt/c54e463c95816d959602f1f2541c62b2.png".to_string()),
        };

        let second_image = types::InputImage {
            data: types::MediaData::Url("https://p2-kling.klingai.com/bs2/upload-ylab-stunt/5eef15e03a70e1fa80732808a2f50f3f.png".to_string()),
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
            None,
            None,
            None,
        ) {
            Ok(id) => id.trim().to_string(),
            Err(error) => return format!("ERROR: Failed to generate video effects: {:?}", error),
        };

        poll_job_until_complete(&job_id, "test8")
    }

    /// Test9 - Extend video using job-id from test3
    /// Pre-requisite: Test3 must be run first and job-id must be stored in JOB_ID_STORAGE
    fn test9() -> String {
        println!("Test9: Extend video using job-id from test3");

        // Retrieve the stored job ID
        let job_id_from_test3 = JOB_ID_STORAGE.lock().unwrap().clone();
        if job_id_from_test3.is_empty() {
            return "ERROR: No job ID stored from test3".to_string();
        }

        println!("Attempting to extend video with job ID: {}", job_id_from_test3);
        
        match advanced::extend_video(
            &job_id_from_test3,
            Some("and the astronaut continues to walk away"),
            None,
            None,
            None,
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

    // Test 10 - Multi-image generation (2 URLs + 1 inline raw bytes), Supports max of 4 images
    fn testx() -> String {
        println!("Test10: Multi-image generation (2 URLs + 1 inline raw bytes)");

        // Load one image as inline bytes  
        let (image_bytes, image_mime_type) = match load_file_bytes("/data/multi-image.jpeg") {
            Ok((bytes, mime_type)) => (bytes, mime_type),
            Err(err) => return format!("ERROR: {}", err),
        };

        // Create a list of 3 images: 2 URLs and 1 inline bytes as specified
        let input_images = vec![
            // First image - URL
            types::InputImage {
                data: types::MediaData::Url("https://h2.inkwai.com/bs2/upload-ylab-stunt/se/ai_portal_queue_mmu_image_upscale_aiweb/3214b798-e1b4-4b00-b7af-72b5b0417420_raw_image_0.jpg".to_string()),
            },
            // Second image - URL  
            types::InputImage {
                data: types::MediaData::Url("https://p1-kling.klingai.com/kcdn/cdn-kcdn112452/kling-api-document/multi-image-unicorn.jpeg".to_string()),
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
            negative_prompt: None,
            seed: None,
            scheduler: None,
            guidance_scale: None,
            aspect_ratio: Some(types::AspectRatio::Landscape),
            model: None,
            duration_seconds: Some(5.0),
            resolution: Some(types::Resolution::Fhd),
            enable_audio: Some(false),
            enhance_prompt: Some(true),
            provider_options: None,
            lastframe: None,
            static_mask: None,
            dynamic_mask: None,
            camera_control: None,
        };

        let prompt: Option<&str> = Some("A girl riding a unicorn in the forest, cinematic realism style");

        println!("Sending multi-image generation request...");
        let job_id = match advanced::multi_image_generation(&input_images, prompt, &config) {
            Ok(id) => id.trim().to_string(),
            Err(error) => return format!("ERROR: Failed to generate multi-image video: {:?}", error),
        };

        poll_job_until_complete(&job_id, "test10")
    }


}

// Helper function to save video result
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

// Helper function to load file bytes
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
                Some("jpeg") => "image/jpeg".to_string(),
                Some("wav") => "audio/wav".to_string(),
                _ => "application/octet-stream".to_string(), // Default or unknown
            };
            Ok((buffer, mime_type))
        }
        Err(err) => Err(format!("Failed to read {}: {}", path, err)),
    }
}

// Polling function happens here
fn poll_job_until_complete(job_id: &str, test_name: &str) -> String {
    println!("Polling for {} results with job ID: {}", test_name, job_id);

    // Wait 5 seconds after job creation before starting polling
    println!("Waiting 5 seconds for job initialization...");
    thread::sleep(Duration::from_secs(5));

    // Poll every 5 seconds until completion (Kling generation takes few minutes)
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
        
        // Wait 20 seconds before polling again
        thread::sleep(Duration::from_secs(5));
    }
}

bindings::export!(Component with_types_in bindings);
