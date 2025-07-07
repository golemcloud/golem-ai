#[allow(static_mut_refs)]
mod bindings;

use crate::bindings::exports::test::video_exports::test_video_api::*;
use crate::bindings::golem::video::types;
use crate::bindings::golem::video::video_generation;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::thread;
use std::time::Duration;

struct Component;

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

///job_id to test stability: 939104de411db613f610b6193259df171e7a5bbd555db55f2310009ad06bfae
///because stability polling fails

/// kling job_id 767103052582096949
/// incase

//// google projects/golem-test-463802/locations/us-central1/publishers/google/models/veo-2.0-generate-001/operations/6013adea-df6a-465a-ae73-21dbf73a0b1f
impl Guest for Component {
    /// test1 demonstrates a simple video generation using a binary image input.
    fn test1() -> String {
        // VEO image-to-video job_id for testing: 6013adea-df6a-465a-ae73-21dbf73a0b1f
        // let job_id = "projects/golem-test-463802/locations/us-central1/publishers/google/models/veo-2.0-generate-001/operations/6013adea-df6a-465a-ae73-21dbf73a0b1f".to_string();
        
        println!("Reading image from Initial File System...");
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
            aspect_ratio: None, // Will be determined by input image dimensions
            model: None,
            duration_seconds: None,
            resolution: None, // Will be determined by input image dimensions  
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

        // Create media input with the image data
        let media_input = types::MediaInput::Image(types::Reference {
            data: types::InputImage {
                data: types::MediaData::Bytes(types::RawBytes {
                    bytes: image_bytes,
                    mime_type: image_mime_type,
                }),
            },
            prompt: Some("An Old smiling man, and waving his hand".to_string()),
            role: None,
        });

        println!("Sending video generation request...");
        let job_id = match video_generation::generate(&media_input, &config) {
            Ok(id) => {
                println!("Generated job ID: '{}'", id);
                println!("Job ID length: {}", id.len());
                println!("Job ID bytes: {:?}", id.as_bytes());
                id.trim().to_string() // Trim whitespace to fix stringification issues
            }
            Err(error) => {
                return format!("ERROR: Failed to generate video: {:?}", error);
            }
        };

        // Wait 5 seconds after job creation before starting polling
        println!("Waiting 5 seconds for job initialization...");
        thread::sleep(Duration::from_secs(5));

        println!("Polling for video generation results with job ID: {}", job_id);

        // Poll every 9 seconds until completion
        loop {
            match video_generation::poll(&job_id) {
                Ok(video_result) => {
                    match video_result.status {
                        types::JobStatus::Pending => {
                            println!("Video generation is pending...");
                        }
                        types::JobStatus::Running => {
                            println!("Video generation is running...");
                        }
                        types::JobStatus::Succeeded => {
                            println!("Video generation completed successfully!");
                            let file_path = save_video_result(&video_result, &job_id);
                            return format!("Video generated successfully. Saved to: {}", file_path);
                        }
                        types::JobStatus::Failed(error_msg) => {
                            return format!("Video generation failed: {}", error_msg);
                        }
                    }
                }
                Err(error) => {
                    return format!("Error polling video generation: {:?}", error);
                }
            }
            
            // Wait 9 seconds before polling again
            thread::sleep(Duration::from_secs(9));
        }
    }

    /// test2 demonstrates text-to-video generation with a creative prompt.
    fn test2() -> String {
        println!("Starting text-to-video generation...");

        // VEO text-to-video job_id for testing: a8b50c2f-3726-48f7-9d81-6f4c6038e1e7
        // let job_id = "projects/golem-test-463802/locations/us-central1/publishers/google/models/veo-2.0-generate-001/operations/a8b50c2f-3726-48f7-9d81-6f4c6038e1e7".to_string();
        
        // Create video generation configuration
        let config = types::GenerationConfig {
            negative_prompt: Some("blurry, low quality, distorted, ugly".to_string()),
            seed: None,
            scheduler: None,
            guidance_scale: None,
            aspect_ratio: None,
            model: None,
            duration_seconds: Some(5.0),
            resolution: None,
            enable_audio: Some(false),
            enhance_prompt: Some(true),
            provider_options: Some(vec![
                types::Kv {
                    key: "mode".to_string(),
                    value: "std".to_string(),
                }
            ]),
            lastframe: None,
            static_mask: None,
            dynamic_mask: None,
            camera_control: None,
        };

        // Create text prompt for video generation
        let creative_prompt = "Create a joyful, cartoon-style scene of a playful snow leopard cub with big, expressive eyes prancing through a whimsical winter forest. The cub leaps over snowdrifts, chases falling snowflakes, and slides playfully down a hill. Use bright, cheerful colors, rounded trees dusted with snow, and gentle sunlight filtering through the branches for a heartwarming, upbeat mood".to_string();
        
        let media_input = types::MediaInput::Text(creative_prompt.clone());

        println!("Sending text-to-video generation request with prompt: {}", creative_prompt);
        let job_id = match video_generation::generate(&media_input, &config) {
            Ok(id) => {
                println!("Generated job ID: '{}'", id);
                println!("Job ID length: {}", id.len());
                println!("Job ID bytes: {:?}", id.as_bytes());
                id.trim().to_string() // Trim whitespace to fix stringification issues
            }
            Err(error) => {
                return format!("ERROR: Failed to generate video: {:?}", error);
            }
        };
        
        // Wait 5 seconds after job creation before starting polling
        println!("Waiting 5 seconds for job initialization...");
        thread::sleep(Duration::from_secs(5));

        // let job_id = "projects/golem-test-463802/locations/us-central1/publishers/google/models/veo-2.0-generate-001/operations/8dae1743-e1b7-4f38-b3da-af7feff2e8ca".to_string();
        // let job_id = "projects/golem-test-463802/locations/us-central1/publishers/google/models/veo-2.0-generate-001/operations/8ff4ffc3-3885-4d4c-ac53-5f8dc0672620".to_string();
        println!("Polling for video generation results with job ID: {}", job_id);

        // Poll every 9 seconds until completion
        loop {
            match video_generation::poll(&job_id) {
                Ok(video_result) => {
                    match video_result.status {
                        types::JobStatus::Pending => {
                            println!("Text-to-video generation is pending...");
                        }
                        types::JobStatus::Running => {
                            println!("Text-to-video generation is running...");
                        }
                        types::JobStatus::Succeeded => {
                            println!("Text-to-video generation completed successfully!");
                            let file_path = save_video_result(&video_result, &job_id);
                            return format!("Text-to-video generated successfully. Saved to: {}", file_path);
                        }
                        types::JobStatus::Failed(error_msg) => {
                            return format!("Text-to-video generation failed: {}", error_msg);
                        }
                    }
                }
                Err(error) => {
                    return format!("Error polling text-to-video generation: {:?}", error);
                }
            }
            
            // Wait 9 seconds before polling again
            thread::sleep(Duration::from_secs(9));
        }
    }

    fn test3() -> String {
        return "test3".to_string();
    }

    fn test4() -> String {
        return "test4".to_string();
    }
 
    fn test5() -> String {
        return "test5".to_string();
    }

}

bindings::export!(Component with_types_in bindings);