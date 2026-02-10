use golem_ai_video::model::advanced::{
    ExtendVideoOptions, GenerateVideoEffectsOptions, MultImageGenerationOptions,
};
use golem_ai_video::model::types;
use golem_ai_video::{AdvancedVideoGenerationProvider, LipSyncProvider, VideoGenerationProvider};
use golem_rust::{agent_definition, agent_implementation, mark_atomic_operation};
use std::fs::File;
use std::io::Read;
use std::thread;
use std::time::Duration;

#[cfg(feature = "kling")]
type Provider = golem_ai_video_kling::DurableKling;
#[cfg(feature = "runway")]
type Provider = golem_ai_video_runway::DurableRunway;
#[cfg(feature = "stability")]
type Provider = golem_ai_video_stability::DurableStability;
#[cfg(feature = "veo")]
type Provider = golem_ai_video_veo::DurableVeo;

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
pub trait VideoAdvancedTest {
    fn new(name: String) -> Self;

    async fn test1(&self) -> String;
    fn test2(&self) -> String;
    fn test3(&self) -> String;
    fn test4(&self) -> String;
    fn test5(&self) -> String;
    fn test6(&self) -> String;
    fn test7(&self) -> String;
    fn test8(&self) -> String;
    fn test9(&self) -> String;
    fn testx(&self) -> String;
    async fn testy(&self) -> String;
}

struct VideoAdvancedTestImpl {
    _name: String,
}

#[agent_implementation]
impl VideoAdvancedTest for VideoAdvancedTestImpl {
    fn new(name: String) -> Self {
        Self { _name: name }
    }

    async fn test1(&self) -> String {
        println!("Test1: Image to video with first frame and last frame");

        let (first_image_bytes, first_image_mime_type) = match load_file_bytes("/data/first.png") {
            Ok((bytes, mime_type)) => (bytes, mime_type),
            Err(err) => return format!("ERROR: {}", err),
        };

        let (last_image_bytes, last_image_mime_type) = match load_file_bytes("/data/last.png") {
            Ok((bytes, mime_type)) => (bytes, mime_type),
            Err(err) => return format!("ERROR: {}", err),
        };

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

        let media_input = types::MediaInput::Image(types::Reference {
            data: types::InputImage {
                data: types::MediaData::Bytes(types::RawBytes {
                    bytes: first_image_bytes,
                    mime_type: first_image_mime_type,
                }),
            },
            prompt: Some("A close up shot of eagle that slowly zooms into its eyes, and then it zooms out to a headshot of a majestic lion, smooth camera movement".to_string()),
            role: Some(types::ImageRole::First),
        });

        println!("Sending first/last frame video generation request...");
        let job_id = match Provider::generate(media_input, config) {
            Ok(id) => id.trim().to_string(),
            Err(error) => return format!("ERROR: Failed to generate video: {:?}", error),
        };

        poll_job_until_complete_with_durability(&job_id, "test1").await
    }

    fn test2(&self) -> String {
        println!("Test2: Image to video with advancedcamera control enum");

        let (image_bytes, image_mime_type) = match load_file_bytes("/data/cameracontrol.jpeg") {
            Ok((bytes, mime_type)) => (bytes, mime_type),
            Err(err) => return format!("ERROR: {}", err),
        };

        let config = types::GenerationConfig {
            negative_prompt: Some("static, boring, low quality".to_string()),
            seed: None,
            scheduler: None,
            guidance_scale: Some(7.5),
            aspect_ratio: Some(types::AspectRatio::Square),
            model: Some("kling-v1-5".to_string()),
            duration_seconds: Some(5.0),
            resolution: Some(types::Resolution::Fhd),
            enable_audio: Some(false),
            enhance_prompt: Some(true),
            provider_options: Some(vec![types::Kv {
                key: "mode".to_string(),
                value: "pro".to_string(),
            }]),
            lastframe: None,
            static_mask: None,
            dynamic_mask: None,
            camera_control: Some(types::CameraMovement::Simple(types::CameraConfig {
                horizontal: 0.0,
                vertical: 0.0,
                pan: 0.0,
                tilt: 0.0,
                zoom: 5.0,
                roll: 0.0,
            })),
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
        let job_id = match Provider::generate(media_input, config) {
            Ok(id) => id.trim().to_string(),
            Err(error) => return format!("ERROR: Failed to generate video: {:?}", error),
        };

        poll_job_until_complete(&job_id, "test2")
    }

    fn test3(&self) -> String {
        println!("Test3: Image to video with static and dynamic mask");

        let static_mask = types::StaticMask {
            mask: types::InputImage {
                data: types::MediaData::Url("https://h2.inkwai.com/bs2/upload-ylab-stunt/ai_portal/1732888177/cOLNrShrSO/static_mask.png".to_string()),
            },
        };

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
        let job_id = match Provider::generate(media_input, config) {
            Ok(id) => id.trim().to_string(),
            Err(error) => return format!("ERROR: Failed to generate video: {:?}", error),
        };

        poll_job_until_complete(&job_id, "test3")
    }

    fn test4(&self) -> String {
        println!("Test4: List voice IDs");

        match Provider::list_voices(None) {
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

    fn test5(&self) -> String {
        println!("Test5: Lip-sync with voice-id");

        let base_video = types::BaseVideo {
            data: types::MediaData::Url("https://v1-kling.klingai.com/kcdn/cdn-kcdn112452/kling-api-document/videos/sing-1.mp4".to_string()),
        };

        let lip_sync_video = types::LipSyncVideo::Video(base_video);

        let text_to_speech = types::TextToSpeech {
            text: "Hello, this is a test of Lip Sync functionality in golem video".to_string(),
            voice_id: "genshin_vindi2".to_string(),
            language: types::VoiceLanguage::En,
            speed: 1.0,
        };

        let audio_source = types::AudioSource::FromText(text_to_speech);

        println!("Sending lip-sync with voice-id request...");
        let job_id = match Provider::generate_lip_sync(lip_sync_video, audio_source) {
            Ok(id) => id.trim().to_string(),
            Err(error) => return format!("ERROR: Failed to generate lip-sync: {:?}", error),
        };

        poll_job_until_complete(&job_id, "test5")
    }

    fn test6(&self) -> String {
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
        let job_id = match Provider::generate_lip_sync(lip_sync_video, audio_source) {
            Ok(id) => id.trim().to_string(),
            Err(error) => return format!("ERROR: Failed to generate lip-sync: {:?}", error),
        };

        poll_job_until_complete(&job_id, "test6")
    }

    fn test7(&self) -> String {
        println!("Test7: Video effects with single image");

        let (image_bytes, image_mime_type) = match load_file_bytes("/data/single-effect.jpeg") {
            Ok((bytes, mime_type)) => (bytes, mime_type),
            Err(err) => return format!("ERROR: {}", err),
        };

        let input = types::InputImage {
            data: types::MediaData::Bytes(types::RawBytes {
                bytes: image_bytes,
                mime_type: image_mime_type,
            }),
        };

        let effect = types::EffectType::Single(types::SingleImageEffects::Fuzzyfuzzy);

        println!("Sending single image effect request...");
        let job_id = match Provider::generate_video_effects(GenerateVideoEffectsOptions {
            input,
            effect,
            model: None,
            duration: None,
            mode: None,
        }) {
            Ok(id) => id.trim().to_string(),
            Err(error) => return format!("ERROR: Failed to generate video effects: {:?}", error),
        };

        poll_job_until_complete(&job_id, "test7")
    }

    fn test8(&self) -> String {
        println!("Test8: Video effects with two images");

        let input = types::InputImage {
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
        let job_id = match Provider::generate_video_effects(GenerateVideoEffectsOptions {
            input,
            effect,
            model: None,
            duration: None,
            mode: None,
        }) {
            Ok(id) => id.trim().to_string(),
            Err(error) => return format!("ERROR: Failed to generate video effects: {:?}", error),
        };

        poll_job_until_complete(&job_id, "test8")
    }

    fn test9(&self) -> String {
        println!("Test9: Extend video using generation-id from completed text-to-video");

        let media_input = types::MediaInput::Text("A beautiful sunset over tropical beach paradise, with blue water reflecting the orange red sunset".to_string());

        let config = types::GenerationConfig {
            negative_prompt: None,
            seed: None,
            scheduler: None,
            guidance_scale: None,
            aspect_ratio: Some(types::AspectRatio::Landscape),
            model: Some("kling-v1-6".to_string()),
            duration_seconds: None,
            resolution: None,
            enable_audio: None,
            enhance_prompt: None,
            provider_options: None,
            lastframe: None,
            static_mask: None,
            dynamic_mask: None,
            camera_control: None,
        };

        println!("Sending text-to-video generation request...");
        let job_id = match Provider::generate(media_input, config) {
            Ok(id) => id.trim().to_string(),
            Err(error) => return format!("ERROR: Failed to generate video: {:?}", error),
        };

        let _poll_result = poll_job_until_complete(&job_id, "test9_initial");

        match Provider::poll(job_id) {
            Ok(video_result) => {
                let generation_id = if let Some(videos) = video_result.videos {
                    if let Some(video) = videos.first() {
                        if let Some(gid) = &video.generation_id {
                            gid.clone()
                        } else {
                            return "ERROR: No generation-id in video result".to_string();
                        }
                    } else {
                        return "ERROR: No videos in result".to_string();
                    }
                } else {
                    return "ERROR: No videos in result".to_string();
                };

                println!(
                    "Attempting to extend video with generation ID: {}",
                    generation_id
                );

                match Provider::extend_video(ExtendVideoOptions {
                    video_id: generation_id,
                    prompt: Some("and the sunset fades into night".to_string()),
                    negative_prompt: None,
                    cfg_scale: None,
                    provider_options: None,
                }) {
                    Ok(extend_job_id) => {
                        let extend_job_id = extend_job_id.trim().to_string();
                        poll_job_until_complete(&extend_job_id, "test9_extended")
                    }
                    Err(error) => {
                        format!("ERROR: Failed to extend video: {:?}", error)
                    }
                }
            }
            Err(error) => {
                format!("ERROR: Failed to poll video result: {:?}", error)
            }
        }
    }

    fn testx(&self) -> String {
        println!("Test10: Multi-image generation (2 URLs + 1 inline raw bytes)");

        let (image_bytes, image_mime_type) = match load_file_bytes("/data/multi-image.jpeg") {
            Ok((bytes, mime_type)) => (bytes, mime_type),
            Err(err) => return format!("ERROR: {}", err),
        };

        let input_images = vec![
            types::InputImage {
                data: types::MediaData::Url("https://h2.inkwai.com/bs2/upload-ylab-stunt/se/ai_portal_queue_mmu_image_upscale_aiweb/3214b798-e1b4-4b00-b7af-72b5b0417420_raw_image_0.jpg".to_string()),
            },
            types::InputImage {
                data: types::MediaData::Url("https://p1-kling.klingai.com/kcdn/cdn-kcdn112452/kling-api-document/multi-image-unicorn.jpeg".to_string()),
            },
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

        let prompt = "A girl riding a unicorn in the forest, cinematic realism style";

        println!("Sending multi-image generation request...");
        let job_id = match Provider::multi_image_generation(MultImageGenerationOptions {
            input_images,
            prompt: Some(prompt.to_string()),
            config,
        }) {
            Ok(id) => id.trim().to_string(),
            Err(error) => {
                return format!("ERROR: Failed to generate multi-image video: {:?}", error)
            }
        };

        poll_job_until_complete(&job_id, "test10")
    }

    async fn testy(&self) -> String {
        println!("Test11: Text to video, extend video, lip sync");

        let media_input = types::MediaInput::Text("A professional, Front facing, lookig at the camera,Caucasian businesswoman with striking red hair, neatly tied back, sits confidently in a modern office. No camera movement".to_string());

        let config = types::GenerationConfig {
            negative_prompt: None,
            seed: None,
            scheduler: None,
            guidance_scale: None,
            aspect_ratio: Some(types::AspectRatio::Landscape),
            model: Some("kling-v1-6".to_string()),
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

        println!("Sending text-to-video generation request...");
        let job_id = match Provider::generate(media_input, config) {
            Ok(id) => id.trim().to_string(),
            Err(error) => return format!("ERROR: Failed to generate video: {:?}", error),
        };

        let initial_result = poll_job_until_complete(&job_id, "test11_text_to_video");
        if initial_result.starts_with("ERROR") {
            return initial_result;
        }

        let generation_id = match Provider::poll(job_id) {
            Ok(video_result) => {
                if let Some(videos) = video_result.videos {
                    if let Some(video) = videos.first() {
                        if let Some(gid) = &video.generation_id {
                            gid.clone()
                        } else {
                            return "ERROR: No generation-id in video result".to_string();
                        }
                    } else {
                        return "ERROR: No videos in result".to_string();
                    }
                } else {
                    return "ERROR: No videos in result".to_string();
                }
            }
            Err(error) => {
                return format!("ERROR: Failed to poll video result: {:?}", error);
            }
        };

        println!("Extending video with generation ID: {}", generation_id);
        let extend_job_id = match Provider::extend_video(ExtendVideoOptions {
            video_id: generation_id,
            prompt: Some("continue the video with a businesswoman with red hair, in a modern office, front facing, looking at the camera, no camera movement".to_string()),
            negative_prompt: None,
            cfg_scale: None,
            provider_options: None,
        }) {
            Ok(id) => id.trim().to_string(),
            Err(error) => return format!("ERROR: Failed to extend video: {:?}", error),
        };

        let extended_result = poll_job_until_complete(&extend_job_id, "test11_extended");
        if extended_result.starts_with("ERROR") {
            return extended_result;
        }

        let extended_generation_id = match Provider::poll(extend_job_id) {
            Ok(video_result) => {
                if let Some(videos) = video_result.videos {
                    if let Some(video) = videos.first() {
                        if let Some(gid) = &video.generation_id {
                            gid.clone()
                        } else {
                            return "ERROR: No generation-id in extended video result".to_string();
                        }
                    } else {
                        return "ERROR: No videos in extended result".to_string();
                    }
                } else {
                    return "ERROR: No videos in extended result".to_string();
                }
            }
            Err(error) => {
                return format!("ERROR: Failed to poll extended video result: {:?}", error);
            }
        };

        println!(
            "Performing lip-sync on video with generation ID: {}",
            extended_generation_id
        );
        let lip_sync_video = types::LipSyncVideo::VideoId(extended_generation_id);

        let text_to_speech = types::TextToSpeech {
            text: "Hello, Golem Cloud is a durable, serverless platform for running long-lived, stateful AI agents and workflows. Welcome to Golem Cloud".to_string(),
            voice_id: "chengshu_jiejie".to_string(),
            language: types::VoiceLanguage::En,
            speed: 1.0,
        };

        let audio_source = types::AudioSource::FromText(text_to_speech);

        println!("Sending lip-sync request...");
        let lip_sync_job_id = match Provider::generate_lip_sync(lip_sync_video, audio_source) {
            Ok(id) => id.trim().to_string(),
            Err(error) => return format!("ERROR: Failed to generate lip-sync: {:?}", error),
        };

        poll_job_until_complete(&lip_sync_job_id, "test11_lip_sync")
    }
}

fn save_video_result(video_result: &types::VideoResult, test_name: &str) -> String {
    if let Some(videos) = &video_result.videos {
        if videos.is_empty() {
            return "No videos in result".to_string();
        }
        let mut results = Vec::new();

        for (i, video_data) in videos.iter().enumerate() {
            if let Some(uri) = &video_data.uri {
                results.push(format!(
                    "Video {}-{} available at URI: {}",
                    test_name, i, uri
                ));
            } else {
                results.push(format!("No URI available for video {}-{}", test_name, i));
            }
        }
        results.join("\n")
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
                Some("jpeg") => "image/jpeg".to_string(),
                Some("wav") => "audio/wav".to_string(),
                _ => "application/octet-stream".to_string(),
            };
            Ok((buffer, mime_type))
        }
        Err(err) => Err(format!("Failed to read {}: {}", path, err)),
    }
}

fn poll_job_until_complete(job_id: &str, test_name: &str) -> String {
    println!("Polling for {} results with job ID: {}", test_name, job_id);

    println!("Waiting 5 seconds for job initialization...");
    thread::sleep(Duration::from_secs(5));

    loop {
        match Provider::poll(job_id.to_string()) {
            Ok(video_result) => match video_result.status {
                types::JobStatus::Pending => {
                    println!("{} is pending...", test_name);
                }
                types::JobStatus::Running => {
                    println!("{} is running...", test_name);
                }
                types::JobStatus::Succeeded => {
                    println!("{} completed successfully!", test_name);
                    let file_path = save_video_result(&video_result, test_name);
                    return format!(
                        "{} generated successfully. Saved to: {}",
                        test_name, file_path
                    );
                }
                types::JobStatus::Failed(error_msg) => {
                    return format!("{} failed: {}", test_name, error_msg);
                }
            },
            Err(error) => {
                return format!("Error polling {}: {:?}", test_name, error);
            }
        }

        thread::sleep(Duration::from_secs(5));
    }
}

async fn poll_job_until_complete_with_durability(job_id: &str, test_name: &str) -> String {
    println!(
        "Polling for {} results with job ID: {} (with durability test)",
        test_name, job_id
    );

    println!("Waiting 5 seconds for job initialization...");
    thread::sleep(Duration::from_secs(5));

    let name = std::env::var("GOLEM_WORKER_NAME").unwrap();
    let mut round = 0;

    loop {
        match Provider::poll(job_id.to_string()) {
            Ok(video_result) => match video_result.status {
                types::JobStatus::Pending => {
                    println!("{} is pending... (round {})", test_name, round);
                }
                types::JobStatus::Running => {
                    println!("{} is running... (round {})", test_name, round);
                }
                types::JobStatus::Succeeded => {
                    println!(
                        "{} completed successfully after {} rounds!",
                        test_name, round
                    );
                    let file_path = save_video_result(&video_result, test_name);
                    return format!(
                        "{} generated successfully. Saved to: {} (durability test passed)",
                        test_name, file_path
                    );
                }
                types::JobStatus::Failed(error_msg) => {
                    return format!("{} failed: {}", test_name, error_msg);
                }
            },
            Err(error) => {
                return format!("Error polling {}: {:?}", test_name, error);
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

        println!("Sleeping for 5 seconds");
        thread::sleep(Duration::from_secs(5));
    }
}
