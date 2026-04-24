use golem_ai_video::model::types::*;
use golem_ai_video::{AdvancedVideoGenerationProvider, VideoGenerationProvider};
use golem_rust::{agent_definition, agent_implementation, mark_atomic_operation};
use std::fs;
use std::fs::File;
use std::io::Read;
use std::thread;
use std::time::Duration;

#[cfg(feature = "stability")]
type Provider = golem_ai_video_stability::DurableStability;
#[cfg(feature = "runway")]
type Provider = golem_ai_video_runway::DurableRunway;
#[cfg(feature = "kling")]
type Provider = golem_ai_video_kling::DurableKling;
#[cfg(feature = "veo")]
type Provider = golem_ai_video_veo::DurableVeo;

const POLLING_SLEEP_SECONDS: u64 = 5;

#[agent_definition]
pub trait TestHelper {
    fn new(name: String) -> Self;
    fn inc_and_get(&mut self) -> u64;
}

struct TestHelperImpl {
    _name: String,
    total: u64,
}

#[agent_implementation]
impl TestHelper for TestHelperImpl {
    fn new(name: String) -> Self {
        Self {
            _name: name,
            total: 0,
        }
    }

    fn inc_and_get(&mut self) -> u64 {
        self.total += 1;
        self.total
    }
}

#[agent_definition]
pub trait VideoTest {
    fn new(name: String) -> Self;
    fn test1(&self) -> String;
    async fn test2(&self) -> String;
    fn test3(&self) -> String;
    fn test4(&self) -> String;
    fn test5(&self) -> String;
}

struct VideoTestImpl {
    _name: String,
}

#[agent_implementation]
impl VideoTest for VideoTestImpl {
    fn new(name: String) -> Self {
        Self { _name: name }
    }

    fn test1(&self) -> String {
        println!("Test1: Text to video generation");

        let config = GenerationConfig {
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

        let media_input =
            MediaInput::Text("A beautiful sunset over the ocean, orange and red hues".to_string());

        println!("Sending text-to-video generation request...");
        let job_id = match Provider::generate(media_input, config) {
            Ok(id) => id.trim().to_string(),
            Err(error) => return format!("ERROR: Failed to generate video: {:?}", error),
        };

        poll_job_until_complete(&job_id, "test1")
    }

    async fn test2(&self) -> String {
        println!("Test2: Image to video with durability test");

        let (image_bytes, image_mime_type) = match load_file_bytes("/data/old.png") {
            Ok((bytes, mime_type)) => (bytes, mime_type),
            Err(err) => return format!("ERROR: Failed to open old.png: {}", err),
        };

        let config = GenerationConfig {
            negative_prompt: None,
            seed: None,
            scheduler: None,
            guidance_scale: None,
            aspect_ratio: Some(AspectRatio::Square),
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

        let media_input = MediaInput::Image(Reference {
            data: InputImage {
                data: MediaData::Bytes(RawBytes {
                    bytes: image_bytes,
                    mime_type: image_mime_type,
                }),
            },
            prompt: Some("Video of a snowy night landscape with pine trees, vivid aurora borealis dancing in the sky, gentle snowfall, and a peaceful, photorealistic atmosphere.".to_string()),
            role: None,
        });

        println!("Sending image-to-video generation request...");
        let job_id = match Provider::generate(media_input, config) {
            Ok(id) => id.trim().to_string(),
            Err(error) => return format!("ERROR: Failed to generate video: {:?}", error),
        };

        println!(
            "Polling for test2 results with job ID: {} (with durability test)",
            job_id
        );

        println!("Waiting 5 seconds for job initialization...");
        thread::sleep(Duration::from_secs(5));

        let name = std::env::var("GOLEM_WORKER_NAME").unwrap();
        let mut round = 0;

        loop {
            match Provider::poll(job_id.clone()) {
                Ok(video_result) => match video_result.status {
                    JobStatus::Pending => {
                        println!("test2 is pending... (round {})", round);
                    }
                    JobStatus::Running => {
                        println!("test2 is running... (round {})", round);
                    }
                    JobStatus::Succeeded => {
                        println!("test2 completed successfully after {} rounds!", round);
                        let file_path = save_video_result(&video_result, "test2");
                        return format!(
                            "test2 generated successfully. Saved to: {} (durability test passed)",
                            file_path
                        );
                    }
                    JobStatus::Failed(error_msg) => {
                        return format!("test2 failed: {}", error_msg);
                    }
                },
                Err(error) => {
                    return format!("Error polling test2: {:?}", error);
                }
            }

            if round == 1 {
                let _guard = mark_atomic_operation();
                let mut client = TestHelperClient::get(name.clone());
                let answer = client.inc_and_get().await;
                if answer == 1 {
                    panic!("Simulating crash during durability test")
                }
            }

            round += 1;

            println!("Sleeping for {} seconds", POLLING_SLEEP_SECONDS);
            thread::sleep(Duration::from_secs(POLLING_SLEEP_SECONDS));
        }
    }

    fn test3(&self) -> String {
        println!("Test3: Image to video with 'last' role and URL");

        let config = GenerationConfig {
            negative_prompt: Some("blurry, distorted".to_string()),
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

        let media_input = MediaInput::Image(Reference {
            data: InputImage {
                data: MediaData::Url(
                    "https://wallpapercave.com/wp/wp12088891.jpg".to_string(),
                ),
            },
            prompt: Some(
                "A serene landscape transforming with gentle motion".to_string(),
            ),
            role: Some(ImageRole::Last),
        });

        println!("Sending image-to-video generation request with 'last' role...");
        let job_id = match Provider::generate(media_input, config) {
            Ok(id) => id.trim().to_string(),
            Err(error) => return format!("ERROR: Failed to generate video: {:?}", error),
        };

        poll_job_until_complete(&job_id, "test3")
    }

    fn test4(&self) -> String {
        println!("Test4: Video to video generation (VEO only)");

        let (video_bytes, video_mime_type) = match load_file_bytes("/output/video-test1-0.mp4") {
            Ok((bytes, mime_type)) => (bytes, mime_type),
            Err(_) => {
                return "Test4: VEO video-to-video transformation (requires test1 output)"
                    .to_string();
            }
        };

        let config = GenerationConfig {
            negative_prompt: Some("artifacts, glitches".to_string()),
            seed: None,
            scheduler: None,
            guidance_scale: None,
            aspect_ratio: None,
            model: None,
            duration_seconds: None,
            resolution: None,
            enable_audio: Some(false),
            enhance_prompt: Some(true),
            provider_options: Some(vec![Kv {
                key: "storage_uri".to_string(),
                value: "gs://golem-video-test-bucket/test".to_string(),
            }]),
            lastframe: None,
            static_mask: None,
            dynamic_mask: None,
            camera_control: None,
        };

        let media_input = MediaInput::Video(BaseVideo {
            data: MediaData::Bytes(RawBytes {
                bytes: video_bytes,
                mime_type: video_mime_type,
            }),
        });

        println!("Sending video-to-video generation request...");
        let job_id = match Provider::generate(media_input, config) {
            Ok(id) => id.trim().to_string(),
            Err(error) => return format!("ERROR: Failed to generate video: {:?}", error),
        };

        poll_job_until_complete(&job_id, "test4")
    }

    fn test5(&self) -> String {
        println!("Test5: Video upscale (Runway only)");

        let base_video = BaseVideo {
            data: MediaData::Url("https://v1-kling.kechuangai.com/kcdn/cdn-kcdn112452/kling-api-document/videos/a-girl-on-unicorn.mp4".to_string()),
        };

        println!("Sending video upscale request...");
        let job_id = match Provider::upscale_video(base_video) {
            Ok(id) => id.trim().to_string(),
            Err(error) => return format!("ERROR: Failed to upscale video: {:?}", error),
        };

        poll_job_until_complete(&job_id, "test5")
    }
}

fn save_video_result(video_result: &VideoResult, test_name: &str) -> String {
    if let Some(videos) = &video_result.videos {
        if videos.is_empty() {
            return "No videos in result".to_string();
        }
        let mut results = Vec::new();
        for (i, video_data) in videos.iter().enumerate() {
            if let Some(video_bytes) = &video_data.base64_bytes {
                let filename = format!("/output/video-{}-{}.mp4", test_name, i);

                if let Err(err) = fs::create_dir_all("/output") {
                    return format!("Failed to create output directory: {}", err);
                }

                match fs::write(&filename, video_bytes) {
                    Ok(_) => {
                        results.push(filename);
                    }
                    Err(err) => {
                        return format!("Failed to save video to {}: {}", filename, err);
                    }
                }
            } else if let Some(uri) = &video_data.uri {
                results.push(format!(
                    "Video {}-{} available at URI: {}",
                    test_name, i, uri
                ));
            } else {
                results.push(format!(
                    "No video data or URI available for video {}-{}",
                    test_name, i
                ));
            }
        }
        results.join("\n")
    } else {
        "No videos in result".to_string()
    }
}

fn load_file_bytes(path: &str) -> Result<(Vec<u8>, String), String> {
    println!("Reading file from: {}", path);
    let mut file =
        File::open(path).map_err(|err| format!("Failed to open {}: {}", path, err))?;

    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .map_err(|err| format!("Failed to read {}: {}", path, err))?;

    println!("Successfully read {} bytes from {}", buffer.len(), path);
    let mime_type = match path.rsplit('.').next() {
        Some("png") => "image/png".to_string(),
        _ => "application/octet-stream".to_string(),
    };
    Ok((buffer, mime_type))
}

fn poll_job_until_complete(job_id: &str, test_name: &str) -> String {
    println!(
        "Polling for {} results with job ID: {}",
        test_name, job_id
    );

    println!("Waiting 5 seconds for job initialization...");
    thread::sleep(Duration::from_secs(5));

    loop {
        match Provider::poll(job_id.to_string()) {
            Ok(video_result) => match video_result.status {
                JobStatus::Pending => {
                    println!("{} is pending...", test_name);
                }
                JobStatus::Running => {
                    println!("{} is running...", test_name);
                }
                JobStatus::Succeeded => {
                    println!("{} completed successfully!", test_name);
                    let file_path = save_video_result(&video_result, test_name);
                    return format!(
                        "{} generated successfully. Saved to: {}",
                        test_name, file_path
                    );
                }
                JobStatus::Failed(error_msg) => {
                    return format!("{} failed: {}", test_name, error_msg);
                }
            },
            Err(error) => {
                return format!("Error polling {}: {:?}", test_name, error);
            }
        }

        thread::sleep(Duration::from_secs(POLLING_SLEEP_SECONDS));
    }
}


