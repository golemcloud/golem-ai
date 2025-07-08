#[allow(static_mut_refs)]
mod bindings;

use golem_rust::atomically;
use crate::bindings::exports::test::video_exports::test_video_api::*;
use crate::bindings::golem::video::types;
use crate::bindings::golem::video::video_generation;
use crate::bindings::test::helper_client::test_helper_client::TestHelperApi;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::thread;
use std::time::Duration;

struct Component;

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

///job_id to test stability: 939104de411db613f610b6193259df171e7a5bbd555db55f2310009ad06bfae
///because stability polling fails

/// kling job_id 767103052582096949
/// incase

//// google projects/golem-test-463802/locations/us-central1/publishers/google/models/veo-2.0-generate-001/operations/6013adea-df6a-465a-ae73-21dbf73a0b1f
impl Guest for Component {
    /// test1 demonstrates text-to-video generation with a simple prompt
    fn test1() -> String {
        println!("Test1: Text to video generation");

        // Create video generation configuration
        let config = types::GenerationConfig {
            negative_prompt: None,
            seed: None,
            scheduler: None,
            guidance_scale: None,
            aspect_ratio: None,
            model: None,
            duration_seconds: None,
            resolution: None,
            enable_audio: Some(false),
            enhance_prompt: Some(false),
            provider_options: Some(vec![
                types::Kv {
                    key: "motion_bucket_id".to_string(),
                    value: "127".to_string(),
                }
            ]),
            lastframe: None,
            static_mask: None,
            dynamic_mask: None,
            camera_control: None,
        };

        // Create text prompt for video generation
        let media_input = types::MediaInput::Text("A beautiful sunset over the ocean".to_string());

        println!("Sending text-to-video generation request...");
        let job_id = match video_generation::generate(&media_input, &config) {
            Ok(id) => id.trim().to_string(),
            Err(error) => return format!("ERROR: Failed to generate video: {:?}", error),
        };

        poll_job_until_complete(&job_id, "test1")
    }

    /// test2 demonstrates image-to-video generation with durability testing
    fn test2() -> String {
        println!("Test2: Image to video with durability test");
        
        // Load test image as inline raw bytes
        let (image_bytes, image_mime_type) = match load_file_bytes("/data/old.png") {
            Ok((bytes, mime_type)) => (bytes, mime_type),
            Err(err) => return format!("ERROR: Failed to open old.png: {}", err),
        };

        // Create video generation configuration
        let config = types::GenerationConfig {
            negative_prompt: None,
            seed: None,
            scheduler: None,
            guidance_scale: None,
            aspect_ratio: None,
            model: None,
            duration_seconds: None,
            resolution: None,
            enable_audio: Some(false),
            enhance_prompt: Some(false),
            provider_options: None,
            lastframe: None,
            static_mask: None,
            dynamic_mask: None,
            camera_control: None,
        };

        // Create media input with image data and 'none' role
        let media_input = types::MediaInput::Image(types::Reference {
            data: types::InputImage {
                data: types::MediaData::Bytes(types::RawBytes {
                    bytes: image_bytes,
                    mime_type: image_mime_type,
                }),
            },
            prompt: Some("A simple motion effect".to_string()),
            role: None,  // Role set to 'none' as specified
        });

        println!("Sending image-to-video generation request...");
        let job_id = match video_generation::generate(&media_input, &config) {
            Ok(id) => id.trim().to_string(),
            Err(error) => return format!("ERROR: Failed to generate video: {:?}", error),
        };

        poll_job_until_complete_with_durability(&job_id, "test2")
    }

    fn test3() -> String {
        println!("Test3: Image to video with 'last' role and URL");
        
        // Create video generation configuration
        let config = types::GenerationConfig {
            negative_prompt: Some("blurry, distorted".to_string()),
            seed: None,
            scheduler: None,
            guidance_scale: None,
            aspect_ratio: None,
            model: None,
            duration_seconds: Some(3.0),
            resolution: None,
            enable_audio: Some(false),
            enhance_prompt: Some(false),
            provider_options: None,
            lastframe: None,
            static_mask: None,
            dynamic_mask: None,
            camera_control: None,
        };

        // Create media input with image URL and 'last' role
        let media_input = types::MediaInput::Image(types::Reference {
            data: types::InputImage {
                data: types::MediaData::Url("https://example.com/test-image.png".to_string()),
            },
            prompt: Some("A serene landscape transforming with gentle motion".to_string()),
            role: Some(types::ImageRole::Last),  // Set to 'last' as specified
        });

        println!("Sending image-to-video generation request with 'last' role...");
        let job_id = match video_generation::generate(&media_input, &config) {
            Ok(id) => id.trim().to_string(),
            Err(error) => return format!("ERROR: Failed to generate video: {:?}", error),
        };

        poll_job_until_complete(&job_id, "test3")
    }

    fn test4() -> String {
        println!("Test4: Video to video generation (VEO only)");
        
        // Load the output from test1 as input video (inline raw bytes)
        let (video_bytes, video_mime_type) = match load_file_bytes("/output/video-test1-0.mp4") {
            Ok((bytes, mime_type)) => (bytes, mime_type),
            Err(_) => {
                // Fallback message if test1 output not available
                return "Test4: VEO video-to-video transformation (requires test1 output)".to_string();
            }
        };

        let config = types::GenerationConfig {
            negative_prompt: Some("artifacts, glitches".to_string()),
            seed: None,
            scheduler: None,
            guidance_scale: None,
            aspect_ratio: None,
            model: Some("veo".to_string()), // VEO only as specified
            duration_seconds: Some(4.0),
            resolution: None,
            enable_audio: Some(false),
            enhance_prompt: Some(true),
            provider_options: None,
            lastframe: None,
            static_mask: None,
            dynamic_mask: None,
            camera_control: None,
        };

        // Create media input with video data (inline raw bytes)
        let media_input = types::MediaInput::Video(types::BaseVideo {
            data: types::MediaData::Bytes(types::RawBytes {
                bytes: video_bytes,
                mime_type: video_mime_type,
            }),
        });

        println!("Sending video-to-video generation request...");
        let job_id = match video_generation::generate(&media_input, &config) {
            Ok(id) => id.trim().to_string(),
            Err(error) => return format!("ERROR: Failed to generate video: {:?}", error),
        };

        poll_job_until_complete(&job_id, "test4")
    }

    fn test5() -> String {
        use crate::bindings::golem::video::advanced;
        
        println!("Test5: Video upscale (Runway only)");
        
        // Load the output from test1 as input video
        let (video_bytes, video_mime_type) = match load_file_bytes("/output/video-test1-0.mp4") {
            Ok((bytes, mime_type)) => (bytes, mime_type),
            Err(_) => {
                // Fallback message if test1 output not available
                return "Test5: Runway video upscaling (requires test1 output)".to_string();
            }
        };

        let base_video = types::BaseVideo {
            data: types::MediaData::Bytes(types::RawBytes {
                bytes: video_bytes,
                mime_type: video_mime_type,
            }),
        };

        println!("Sending video upscale request...");
        let job_id = match advanced::upscale_video(&base_video) {
            Ok(id) => id.trim().to_string(),
            Err(error) => return format!("ERROR: Failed to upscale video: {:?}", error),
        };

        poll_job_until_complete(&job_id, "test5")
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

fn poll_job_until_complete_with_durability(job_id: &str, test_name: &str) -> String {
    println!("Polling for {} results with job ID: {} (with durability test)", test_name, job_id);

    // Wait 5 seconds after job creation before starting polling
    println!("Waiting 5 seconds for job initialization...");
    thread::sleep(Duration::from_secs(5));

    let name = std::env::var("GOLEM_WORKER_NAME").unwrap();
    let mut round = 0;

    // Poll every 9 seconds until completion
    loop {
        match video_generation::poll(&job_id) {
            Ok(video_result) => {
                match video_result.status {
                    types::JobStatus::Pending => {
                        println!("{} is pending... (round {})", test_name, round);
                    }
                    types::JobStatus::Running => {
                        println!("{} is running... (round {})", test_name, round);
                    }
                    types::JobStatus::Succeeded => {
                        println!("{} completed successfully after {} rounds!", test_name, round);
                        let file_path = save_video_result(&video_result, test_name);
                        return format!("{} generated successfully. Saved to: {} (durability test passed)", test_name, file_path);
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
        
        // Durability test simulation: simulate a crash during polling, but only first time
        // After automatic recovery it will continue and finish the request successfully
        if round == 2 {
            atomically(|| {
                let client = TestHelperApi::new(&name);
                let answer = client.blocking_inc_and_get();
                if answer == 1 {
                    panic!("Simulating crash during durability test")
                }
            });
        }

        round += 1;
        
        // Wait 9 seconds before polling again
        thread::sleep(Duration::from_secs(9));
    }
}

bindings::export!(Component with_types_in bindings);