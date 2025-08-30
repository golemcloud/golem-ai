use crate::exports::golem::video_generation::advanced::Guest as AdvancedGuest;
use crate::exports::golem::video_generation::lip_sync::Guest as LipSyncGuest;
#[allow(unused_imports)]
use crate::exports::golem::video_generation::types::{
    AudioSource, BaseVideo, EffectType, GenerationConfig, InputImage, Kv, LipSyncVideo, MediaInput,
    VideoError, VideoResult, VoiceInfo,
};
use crate::exports::golem::video_generation::video_generation::Guest as VideoGenerationGuest;
use std::marker::PhantomData;

/// Wraps a Video implementation with custom durability
pub struct DurableVideo<Impl> {
    phantom: PhantomData<Impl>,
}

/// Trait to be implemented in addition to the Video `Guest` traits when wrapping it with `DurableVideo`.
pub trait ExtendedGuest: VideoGenerationGuest + LipSyncGuest + AdvancedGuest + 'static {}

/// When the durability feature flag is off, wrapping with `DurableVideo` is just a passthrough
#[cfg(not(feature = "durability"))]
mod passthrough_impl {
    use crate::durability::{DurableVideo, ExtendedGuest};
    use crate::exports::golem::video_generation::advanced::Guest as AdvancedGuest;
    use crate::exports::golem::video_generation::lip_sync::Guest as LipSyncGuest;
    use crate::exports::golem::video_generation::types::{
        AudioSource, BaseVideo, EffectType, GenerationConfig, InputImage, Kv, LipSyncVideo,
        MediaInput, VideoError, VideoResult, VoiceInfo,
    };
    use crate::exports::golem::video_generation::video_generation::Guest as VideoGenerationGuest;

    impl<Impl: ExtendedGuest> VideoGenerationGuest for DurableVideo<Impl> {
        fn generate(input: MediaInput, config: GenerationConfig) -> Result<String, VideoError> {
            Impl::generate(input, config)
        }

        fn poll(job_id: String) -> Result<VideoResult, VideoError> {
            Impl::poll(job_id)
        }

        fn cancel(job_id: String) -> Result<String, VideoError> {
            Impl::cancel(job_id)
        }
    }

    impl<Impl: ExtendedGuest> LipSyncGuest for DurableVideo<Impl> {
        fn generate_lip_sync(
            video: LipSyncVideo,
            audio: AudioSource,
        ) -> Result<String, VideoError> {
            Impl::generate_lip_sync(video, audio)
        }

        fn list_voices(language: Option<String>) -> Result<Vec<VoiceInfo>, VideoError> {
            Impl::list_voices(language)
        }
    }

    impl<Impl: ExtendedGuest> AdvancedGuest for DurableVideo<Impl> {
        fn extend_video(
            video_id: String,
            prompt: Option<String>,
            negative_prompt: Option<String>,
            cfg_scale: Option<f32>,
            provider_options: Option<Vec<Kv>>,
        ) -> Result<String, VideoError> {
            Impl::extend_video(
                video_id,
                prompt,
                negative_prompt,
                cfg_scale,
                provider_options,
            )
        }

        fn upscale_video(input: BaseVideo) -> Result<String, VideoError> {
            Impl::upscale_video(input)
        }

        fn generate_video_effects(
            input: InputImage,
            effect: EffectType,
            model: Option<String>,
            duration: Option<f32>,
            mode: Option<String>,
        ) -> Result<String, VideoError> {
            Impl::generate_video_effects(input, effect, model, duration, mode)
        }

        fn multi_image_generation(
            input_images: Vec<InputImage>,
            prompt: Option<String>,
            config: GenerationConfig,
        ) -> Result<String, VideoError> {
            Impl::multi_image_generation(input_images, prompt, config)
        }
    }
}

/// When the durability feature flag is on, wrapping with `DurableVideo` adds custom durability
/// on top of the provider-specific Video implementation using Golem's special host functions and
/// the `golem-rust` helper library.
///
/// There will be custom durability entries saved in the oplog, with the full Video request and configuration
/// stored as input, and the full response stored as output. To serialize these in a way it is
/// observable by oplog consumers, each relevant data type has to be converted to/from `ValueAndType`
/// which is implemented using the type classes and builder in the `golem-rust` library.
#[cfg(feature = "durability")]
mod durable_impl {
    use crate::durability::{DurableVideo, ExtendedGuest};
    use crate::exports::golem::video_generation::advanced::Guest as AdvancedGuest;
    use crate::exports::golem::video_generation::lip_sync::Guest as LipSyncGuest;
    use crate::exports::golem::video_generation::types::{
        AudioSource, BaseVideo, EffectType, GenerationConfig, InputImage, Kv, LipSyncVideo,
        MediaInput, VideoError, VideoResult, VoiceInfo,
    };
    use crate::exports::golem::video_generation::video_generation::Guest as VideoGenerationGuest;
    use crate::init_logging;
    use golem_rust::bindings::golem::durability::durability::DurableFunctionType;
    use golem_rust::durability::Durability;
    use golem_rust::{with_persistence_level, FromValueAndType, IntoValue, PersistenceLevel};
    use std::fmt::{Display, Formatter};

    impl<Impl: ExtendedGuest> VideoGenerationGuest for DurableVideo<Impl> {
        fn generate(input: MediaInput, config: GenerationConfig) -> Result<String, VideoError> {
            init_logging();
            let durability = Durability::<String, VideoError>::new(
                "golem_video",
                "generate",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::generate(input.clone(), config.clone())
                });
                durability.persist(GenerateInput { input, config }, result)
            } else {
                durability.replay()
            }
        }

        fn poll(job_id: String) -> Result<VideoResult, VideoError> {
            init_logging();
            let durability = Durability::<VideoResult, VideoError>::new(
                "golem_video",
                "poll",
                DurableFunctionType::ReadRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::poll(job_id.clone())
                });
                durability.persist(PollInput { job_id }, result)
            } else {
                durability.replay()
            }
        }

        fn cancel(job_id: String) -> Result<String, VideoError> {
            init_logging();
            let durability = Durability::<String, VideoError>::new(
                "golem_video",
                "cancel",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::cancel(job_id.clone())
                });
                durability.persist(CancelInput { job_id }, result)
            } else {
                durability.replay()
            }
        }
    }

    impl<Impl: ExtendedGuest> LipSyncGuest for DurableVideo<Impl> {
        fn generate_lip_sync(
            video: LipSyncVideo,
            audio: AudioSource,
        ) -> Result<String, VideoError> {
            init_logging();
            let durability = Durability::<String, VideoError>::new(
                "golem_video",
                "generate_lip_sync",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::generate_lip_sync(video.clone(), audio.clone())
                });
                durability.persist(GenerateLipSyncInput { video, audio }, result)
            } else {
                durability.replay()
            }
        }

        fn list_voices(language: Option<String>) -> Result<Vec<VoiceInfo>, VideoError> {
            init_logging();
            let durability = Durability::<Vec<VoiceInfo>, VideoError>::new(
                "golem_video",
                "list_voices",
                DurableFunctionType::ReadRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::list_voices(language.clone())
                });
                durability.persist(ListVoicesInput { language }, result)
            } else {
                durability.replay()
            }
        }
    }

    impl<Impl: ExtendedGuest> AdvancedGuest for DurableVideo<Impl> {
        fn extend_video(
            video_id: String,
            prompt: Option<String>,
            negative_prompt: Option<String>,
            cfg_scale: Option<f32>,
            provider_options: Option<Vec<Kv>>,
        ) -> Result<String, VideoError> {
            init_logging();
            let durability = Durability::<String, VideoError>::new(
                "golem_video",
                "extend_video",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::extend_video(
                        video_id.clone(),
                        prompt.clone(),
                        negative_prompt.clone(),
                        cfg_scale,
                        provider_options.clone(),
                    )
                });
                durability.persist(
                    ExtendVideoInput {
                        video_id,
                        prompt,
                        negative_prompt,
                        cfg_scale,
                        provider_options,
                    },
                    result,
                )
            } else {
                durability.replay()
            }
        }

        fn upscale_video(input: BaseVideo) -> Result<String, VideoError> {
            init_logging();
            let durability = Durability::<String, VideoError>::new(
                "golem_video",
                "upscale_video",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::upscale_video(input.clone())
                });
                durability.persist(UpscaleVideoInput { input }, result)
            } else {
                durability.replay()
            }
        }

        fn generate_video_effects(
            input: InputImage,
            effect: EffectType,
            model: Option<String>,
            duration: Option<f32>,
            mode: Option<String>,
        ) -> Result<String, VideoError> {
            init_logging();
            let durability = Durability::<String, VideoError>::new(
                "golem_video",
                "generate_video_effects",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::generate_video_effects(
                        input.clone(),
                        effect.clone(),
                        model.clone(),
                        duration,
                        mode.clone(),
                    )
                });
                durability.persist(
                    GenerateVideoEffectsInput {
                        input,
                        effect,
                        model,
                        duration,
                        mode,
                    },
                    result,
                )
            } else {
                durability.replay()
            }
        }

        fn multi_image_generation(
            input_images: Vec<InputImage>,
            prompt: Option<String>,
            config: GenerationConfig,
        ) -> Result<String, VideoError> {
            init_logging();
            let durability = Durability::<String, VideoError>::new(
                "golem_video",
                "multi_image_generation",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::multi_image_generation(
                        input_images.clone(),
                        prompt.clone(),
                        config.clone(),
                    )
                });
                durability.persist(
                    MultiImageGenerationInput {
                        input_images,
                        prompt,
                        config,
                    },
                    result,
                )
            } else {
                durability.replay()
            }
        }
    }

    #[derive(Debug, Clone, PartialEq, IntoValue, FromValueAndType)]
    struct GenerateInput {
        input: MediaInput,
        config: GenerationConfig,
    }

    #[derive(Debug, Clone, PartialEq, IntoValue, FromValueAndType)]
    struct PollInput {
        job_id: String,
    }

    #[derive(Debug, Clone, PartialEq, IntoValue, FromValueAndType)]
    struct CancelInput {
        job_id: String,
    }

    #[derive(Debug, Clone, PartialEq, IntoValue, FromValueAndType)]
    struct GenerateLipSyncInput {
        video: LipSyncVideo,
        audio: AudioSource,
    }

    #[derive(Debug, Clone, PartialEq, IntoValue, FromValueAndType)]
    struct ListVoicesInput {
        language: Option<String>,
    }

    #[derive(Debug, Clone, PartialEq, IntoValue, FromValueAndType)]
    struct ExtendVideoInput {
        video_id: String,
        prompt: Option<String>,
        negative_prompt: Option<String>,
        cfg_scale: Option<f32>,
        provider_options: Option<Vec<Kv>>,
    }

    #[derive(Debug, Clone, PartialEq, IntoValue, FromValueAndType)]
    struct UpscaleVideoInput {
        input: BaseVideo,
    }

    #[derive(Debug, Clone, PartialEq, IntoValue, FromValueAndType)]
    struct GenerateVideoEffectsInput {
        input: InputImage,
        effect: EffectType,
        model: Option<String>,
        duration: Option<f32>,
        mode: Option<String>,
    }

    #[derive(Debug, Clone, PartialEq, IntoValue, FromValueAndType)]
    struct MultiImageGenerationInput {
        input_images: Vec<InputImage>,
        prompt: Option<String>,
        config: GenerationConfig,
    }

    #[derive(Debug, FromValueAndType, IntoValue)]
    struct UnusedError;

    impl Display for UnusedError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "UnusedError")
        }
    }

    impl From<&VideoError> for VideoError {
        fn from(error: &VideoError) -> Self {
            error.clone()
        }
    }

    #[cfg(test)]
    mod tests {
        use crate::durability::durable_impl::{
            CancelInput, ExtendVideoInput, GenerateInput, GenerateLipSyncInput,
            GenerateVideoEffectsInput, ListVoicesInput, MultiImageGenerationInput, PollInput,
            UpscaleVideoInput,
        };
        use crate::exports::golem::video_generation::types::{
            AspectRatio, AudioSource, BaseVideo, DualEffect, DualImageEffects, EffectType,
            GenerationConfig, InputImage, Kv, LipSyncVideo, MediaData, MediaInput, Narration,
            RawBytes, Reference, Resolution, SingleImageEffects, TextToSpeech,
        };
        use golem_rust::value_and_type::{FromValueAndType, IntoValueAndType};
        use std::fmt::Debug;

        fn roundtrip_test<T: Debug + Clone + PartialEq + IntoValueAndType + FromValueAndType>(
            value: T,
        ) {
            let vnt = value.clone().into_value_and_type();
            let extracted = T::from_value_and_type(vnt).unwrap();
            assert_eq!(value, extracted);
        }

        #[test]
        fn generate_input_roundtrip() {
            let input = GenerateInput {
                input: MediaInput::Text("Generate a video of a cat".to_string()),
                config: GenerationConfig {
                    negative_prompt: Some("blurry, low quality".to_string()),
                    seed: Some(12345),
                    scheduler: Some("ddim".to_string()),
                    guidance_scale: Some(7.5),
                    aspect_ratio: Some(AspectRatio::Landscape),
                    duration_seconds: Some(5.0),
                    resolution: Some(Resolution::Hd),
                    model: Some("runway-gen3".to_string()),
                    enable_audio: Some(true),
                    enhance_prompt: Some(false),
                    provider_options: Some(vec![Kv {
                        key: "model".to_string(),
                        value: "runway-gen3".to_string(),
                    }]),
                    lastframe: None,
                    static_mask: None,
                    dynamic_mask: None,
                    camera_control: None,
                },
            };
            roundtrip_test(input);
        }

        #[test]
        fn generate_input_with_image_roundtrip() {
            let input = GenerateInput {
                input: MediaInput::Image(Reference {
                    data: InputImage {
                        data: MediaData::Bytes(RawBytes {
                            bytes: vec![0, 1, 2, 3, 4, 5],
                            mime_type: "image/jpeg".to_string(),
                        }),
                    },
                    prompt: Some("Animate this image".to_string()),
                    role: None,
                }),
                config: GenerationConfig {
                    negative_prompt: None,
                    seed: None,
                    scheduler: None,
                    guidance_scale: None,
                    aspect_ratio: Some(AspectRatio::Square),
                    duration_seconds: Some(3.0),
                    resolution: Some(Resolution::Sd),
                    model: None,
                    enable_audio: Some(false),
                    enhance_prompt: None,
                    provider_options: None,
                    lastframe: None,
                    static_mask: None,
                    dynamic_mask: None,
                    camera_control: None,
                },
            };
            roundtrip_test(input);
        }

        #[test]
        fn poll_input_roundtrip() {
            let input = PollInput {
                job_id: "job_12345".to_string(),
            };
            roundtrip_test(input);
        }

        #[test]
        fn cancel_input_roundtrip() {
            let input = CancelInput {
                job_id: "job_67890".to_string(),
            };
            roundtrip_test(input);
        }

        #[test]
        fn generate_lip_sync_input_roundtrip() {
            let input = GenerateLipSyncInput {
                video: LipSyncVideo::Video(BaseVideo {
                    data: MediaData::Bytes(RawBytes {
                        bytes: vec![0, 1, 2, 3, 4, 5],
                        mime_type: "video/mp4".to_string(),
                    }),
                }),
                audio: AudioSource::FromText(TextToSpeech {
                    text: "Hello world".to_string(),
                    voice_id: "voice_123".to_string(),
                    language: crate::exports::golem::video_generation::types::VoiceLanguage::En,
                    speed: 1.0,
                }),
            };
            roundtrip_test(input);
        }

        #[test]
        fn generate_lip_sync_input_with_audio_roundtrip() {
            let input = GenerateLipSyncInput {
                video: LipSyncVideo::VideoId("video_123".to_string()),
                audio: AudioSource::FromAudio(Narration {
                    data: MediaData::Bytes(RawBytes {
                        bytes: vec![1, 2, 3, 4, 5, 6],
                        mime_type: "audio/mpeg".to_string(),
                    }),
                }),
            };
            roundtrip_test(input);
        }

        #[test]
        fn list_voices_input_roundtrip() {
            let input = ListVoicesInput {
                language: Some("en".to_string()),
            };
            roundtrip_test(input);
        }

        #[test]
        fn list_voices_input_no_language_roundtrip() {
            let input = ListVoicesInput { language: None };
            roundtrip_test(input);
        }

        #[test]
        fn extend_video_input_roundtrip() {
            let input = ExtendVideoInput {
                video_id: "video_123".to_string(),
                prompt: Some("extend this video".to_string()),
                negative_prompt: Some("static".to_string()),
                cfg_scale: Some(8.0),
                provider_options: Some(vec![Kv {
                    key: "quality".to_string(),
                    value: "high".to_string(),
                }]),
            };
            roundtrip_test(input);
        }

        #[test]
        fn upscale_video_input_roundtrip() {
            let input = UpscaleVideoInput {
                input: BaseVideo {
                    data: MediaData::Bytes(RawBytes {
                        bytes: vec![10, 20, 30, 40, 50],
                        mime_type: "video/mp4".to_string(),
                    }),
                },
            };
            roundtrip_test(input);
        }

        #[test]
        fn generate_video_effects_input_roundtrip() {
            let input = GenerateVideoEffectsInput {
                input: InputImage {
                    data: MediaData::Bytes(RawBytes {
                        bytes: vec![100, 200, 255],
                        mime_type: "image/jpeg".to_string(),
                    }),
                },
                effect: EffectType::Single(SingleImageEffects::Bloombloom),
                model: Some("kling-effects".to_string()),
                duration: Some(3.5),
                mode: Some("fast".to_string()),
            };
            roundtrip_test(input);
        }

        #[test]
        fn generate_video_effects_dual_input_roundtrip() {
            let input = GenerateVideoEffectsInput {
                input: InputImage {
                    data: MediaData::Url("https://example.com/image1.jpg".to_string()),
                },
                effect: EffectType::Dual(DualEffect {
                    effect: DualImageEffects::Hug,
                    second_image: InputImage {
                        data: MediaData::Url("https://example.com/image2.jpg".to_string()),
                    },
                }),
                model: None,
                duration: Some(2.0),
                mode: None,
            };
            roundtrip_test(input);
        }

        #[test]
        fn multi_image_generation_input_roundtrip() {
            let input = MultiImageGenerationInput {
                input_images: vec![
                    InputImage {
                        data: MediaData::Bytes(RawBytes {
                            bytes: vec![1, 2, 3],
                            mime_type: "image/jpeg".to_string(),
                        }),
                    },
                    InputImage {
                        data: MediaData::Url("https://example.com/image.png".to_string()),
                    },
                ],
                prompt: Some("Beautiful prompts".to_string()),
                config: GenerationConfig {
                    negative_prompt: Some("blurry".to_string()),
                    seed: None,
                    scheduler: Some("euler".to_string()),
                    guidance_scale: None,
                    aspect_ratio: Some(AspectRatio::Portrait),
                    duration_seconds: Some(4.0),
                    resolution: Some(Resolution::Uhd),
                    model: Some("kling-multi".to_string()),
                    enable_audio: Some(true),
                    enhance_prompt: Some(false),
                    provider_options: Some(vec![Kv {
                        key: "quality".to_string(),
                        value: "high".to_string(),
                    }]),
                    lastframe: None,
                    static_mask: None,
                    dynamic_mask: None,
                    camera_control: None,
                },
            };
            roundtrip_test(input);
        }
    }
}
