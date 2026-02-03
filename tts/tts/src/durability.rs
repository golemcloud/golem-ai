use std::marker::PhantomData;

use crate::golem::tts::advanced::{AudioSample, LongFormResult, VoiceDesignParams};
use crate::golem::tts::streaming::{Guest as StreamingGuest, SynthesisOptions};
use crate::golem::tts::synthesis::{
    Guest as SynthesisGuest, SynthesisOptions as WitSynthesisOptions, ValidationResult,
};
use crate::golem::tts::types::{AudioChunk, SynthesisResult, TextInput, TimingInfo, TtsError};
use crate::golem::tts::voices::{Guest as VoicesGuest, VoiceFilter, VoiceInfo};
use crate::guest::{StreamRequest, SynthesisRequest, TtsGuest};
use crate::LOGGING_STATE;

pub struct DurableTts<Impl> {
    phantom: PhantomData<Impl>,
}

pub trait ExtendedGuest: TtsGuest + 'static {}

#[cfg(not(feature = "durability"))]
mod passthrough_impl {
    use super::*;

    impl<Impl: ExtendedGuest> VoicesGuest for DurableTts<Impl> {
        type Voice = crate::voices::VoiceResource;

        fn list_voices(filter: Option<VoiceFilter>) -> Result<Vec<VoiceInfo>, TtsError> {
            LOGGING_STATE.with_borrow_mut(|state| state.init());
            Impl::list_voices(filter)
        }

        fn get_voice(voice_id: String) -> Result<VoiceInfo, TtsError> {
            LOGGING_STATE.with_borrow_mut(|state| state.init());
            Impl::get_voice(voice_id)
        }

        fn search_voices(
            query: String,
            filter: Option<VoiceFilter>,
        ) -> Result<Vec<VoiceInfo>, TtsError> {
            LOGGING_STATE.with_borrow_mut(|state| state.init());
            Impl::search_voices(query, filter)
        }

        fn list_languages() -> Result<Vec<String>, TtsError> {
            LOGGING_STATE.with_borrow_mut(|state| state.init());
            Impl::list_languages()
        }
    }

    impl<Impl: ExtendedGuest> SynthesisGuest for DurableTts<Impl> {
        fn synthesize(
            input: TextInput,
            voice: String,
            options: Option<WitSynthesisOptions>,
        ) -> Result<SynthesisResult, TtsError> {
            LOGGING_STATE.with_borrow_mut(|state| state.init());
            Impl::synthesize(SynthesisRequest {
                input,
                voice_id: voice,
                options,
            })
        }

        fn synthesize_batch(
            inputs: Vec<TextInput>,
            voice: String,
            options: Option<WitSynthesisOptions>,
        ) -> Result<Vec<SynthesisResult>, TtsError> {
            LOGGING_STATE.with_borrow_mut(|state| state.init());
            Impl::synthesize_batch(
                inputs
                    .into_iter()
                    .map(|input| SynthesisRequest {
                        input,
                        voice_id: voice.clone(),
                        options: options.clone(),
                    })
                    .collect(),
            )
        }

        fn get_timing_marks(
            input: TextInput,
            voice: String,
        ) -> Result<Vec<TimingInfo>, TtsError> {
            LOGGING_STATE.with_borrow_mut(|state| state.init());
            Impl::get_timing_marks(input, voice)
        }

        fn validate_input(
            input: TextInput,
            voice: String,
        ) -> Result<ValidationResult, TtsError> {
            LOGGING_STATE.with_borrow_mut(|state| state.init());
            Impl::validate_input(input, voice)
        }
    }

    impl<Impl: ExtendedGuest> StreamingGuest for DurableTts<Impl> {
        fn create_stream(
            voice: String,
            options: Option<SynthesisOptions>,
        ) -> Result<Vec<AudioChunk>, TtsError> {
            LOGGING_STATE.with_borrow_mut(|state| state.init());
            let stream = Impl::create_stream(StreamRequest { voice_id: voice, options })?;
            if let Some(chunk) = stream.receive_chunk()? {
                Ok(vec![chunk])
            } else {
                Ok(Vec::new())
            }
        }
    }

    impl<Impl: ExtendedGuest> crate::golem::tts::advanced::Guest for DurableTts<Impl> {
        fn create_voice_clone(
            name: String,
            audio_samples: Vec<AudioSample>,
            description: Option<String>,
        ) -> Result<String, TtsError> {
            LOGGING_STATE.with_borrow_mut(|state| state.init());
            Impl::create_voice_clone(name, audio_samples, description)
        }

        fn design_voice(name: String, characteristics: VoiceDesignParams) -> Result<String, TtsError> {
            LOGGING_STATE.with_borrow_mut(|state| state.init());
            Impl::design_voice(name, characteristics)
        }

        fn convert_voice(
            input_audio: Vec<u8>,
            target_voice: String,
            preserve_timing: Option<bool>,
        ) -> Result<SynthesisResult, TtsError> {
            LOGGING_STATE.with_borrow_mut(|state| state.init());
            Impl::convert_voice(input_audio, target_voice, preserve_timing)
        }

        fn generate_sound_effect(
            description: String,
            duration_seconds: Option<f32>,
            style_influence: Option<f32>,
        ) -> Result<SynthesisResult, TtsError> {
            LOGGING_STATE.with_borrow_mut(|state| state.init());
            Impl::generate_sound_effect(description, duration_seconds, style_influence)
        }

        // create_lexicon removed from WIT, no implementation needed

        fn synthesize_long_form(
            content: String,
            voice: String,
            output_location: String,
            chapter_breaks: Option<Vec<u32>>,
        ) -> Result<LongFormResult, TtsError> {
            LOGGING_STATE.with_borrow_mut(|state| state.init());
            Impl::synthesize_long_form(content, voice, output_location, chapter_breaks)
        }
    }
}

#[cfg(feature = "durability")]
mod durable_impl {
    use super::*;
    use crate::guest::TtsStreamGuest;
    use golem_rust::bindings::golem::durability::durability::DurableFunctionType;
    use golem_rust::durability::Durability;
    use golem_rust::{with_persistence_level, FromValueAndType, IntoValue, PersistenceLevel};
    #[derive(Debug, Clone, PartialEq, IntoValue, FromValueAndType)]
    struct VoicesInput {
        filter: Option<VoiceFilter>,
    }

    #[derive(Debug, Clone, PartialEq, IntoValue, FromValueAndType)]
    struct VoiceInput {
        voice_id: String,
    }

    #[derive(Debug, Clone, PartialEq, IntoValue, FromValueAndType)]
    struct SearchVoiceInput {
        query: String,
        filter: Option<VoiceFilter>,
    }

    #[derive(Debug, Clone, PartialEq, IntoValue, FromValueAndType)]
    struct SynthesizeInput {
        input: TextInput,
        voice_id: String,
        options: Option<WitSynthesisOptions>,
    }

    #[derive(Debug, Clone, PartialEq, IntoValue, FromValueAndType)]
    struct BatchInput {
        requests: Vec<SynthesizeInput>,
    }

    #[derive(Debug, Clone, PartialEq, IntoValue, FromValueAndType)]
    struct TimingInput {
        input: TextInput,
        voice_id: String,
    }

    #[derive(Debug, Clone, PartialEq, IntoValue, FromValueAndType)]
    struct StreamInput {
        voice_id: String,
        options: Option<SynthesisOptions>,
    }

    impl From<&TtsError> for TtsError {
        fn from(err: &TtsError) -> Self {
            err.clone()
        }
    }

    impl<Impl: ExtendedGuest> VoicesGuest for DurableTts<Impl> {
        type Voice = crate::voices::VoiceResource;

        fn list_voices(filter: Option<VoiceFilter>) -> Result<Vec<VoiceInfo>, TtsError> {
            LOGGING_STATE.with_borrow_mut(|state| state.init());
            let durability = Durability::<Vec<VoiceInfo>, TtsError>::new(
                "golem_tts",
                "list_voices",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::list_voices(filter.clone())
                });
                durability.persist(VoicesInput { filter }, result)
            } else {
                durability.replay()
            }
        }

        fn get_voice(voice_id: String) -> Result<VoiceInfo, TtsError> {
            LOGGING_STATE.with_borrow_mut(|state| state.init());
            let durability = Durability::<VoiceInfo, TtsError>::new(
                "golem_tts",
                "get_voice",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::get_voice(voice_id.clone())
                });
                durability.persist(VoiceInput { voice_id }, result)
            } else {
                durability.replay()
            }
        }

        fn search_voices(
            query: String,
            filter: Option<VoiceFilter>,
        ) -> Result<Vec<VoiceInfo>, TtsError> {
            LOGGING_STATE.with_borrow_mut(|state| state.init());
            let durability = Durability::<Vec<VoiceInfo>, TtsError>::new(
                "golem_tts",
                "search_voices",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::search_voices(query.clone(), filter.clone())
                });
                durability.persist(SearchVoiceInput { query, filter }, result)
            } else {
                durability.replay()
            }
        }

        fn list_languages() -> Result<Vec<String>, TtsError> {
            LOGGING_STATE.with_borrow_mut(|state| state.init());
            let durability = Durability::<Vec<String>, TtsError>::new(
                "golem_tts",
                "list_languages",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::list_languages()
                });
                durability.persist(Vec::<String>::new(), result)
            } else {
                durability.replay()
            }
        }
    }

    impl<Impl: ExtendedGuest> SynthesisGuest for DurableTts<Impl> {
        fn synthesize(
            input: TextInput,
            voice: String,
            options: Option<WitSynthesisOptions>,
        ) -> Result<SynthesisResult, TtsError> {
            LOGGING_STATE.with_borrow_mut(|state| state.init());
            let durability = Durability::<SynthesisResult, TtsError>::new(
                "golem_tts",
                "synthesize",
                DurableFunctionType::WriteRemote,
            );
            let voice_id = voice;
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::synthesize(SynthesisRequest {
                        input: input.clone(),
                        voice_id: voice_id.clone(),
                        options: options.clone(),
                    })
                });
                durability.persist(
                    SynthesizeInput {
                        input,
                        voice_id,
                        options,
                    },
                    result,
                )
            } else {
                durability.replay()
            }
        }

        fn synthesize_batch(
            inputs: Vec<TextInput>,
            voice: String,
            options: Option<WitSynthesisOptions>,
        ) -> Result<Vec<SynthesisResult>, TtsError> {
            LOGGING_STATE.with_borrow_mut(|state| state.init());
            let durability = Durability::<Vec<SynthesisResult>, TtsError>::new(
                "golem_tts",
                "synthesize_batch",
                DurableFunctionType::WriteRemote,
            );
            let voice_id = voice;
            let requests: Vec<SynthesizeInput> = inputs
                .into_iter()
                .map(|input| SynthesizeInput {
                    input,
                    voice_id: voice_id.clone(),
                    options: options.clone(),
                })
                .collect();
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::synthesize_batch(
                        requests
                            .iter()
                            .map(|req| SynthesisRequest {
                                input: req.input.clone(),
                                voice_id: req.voice_id.clone(),
                                options: req.options.clone(),
                            })
                            .collect(),
                    )
                });
                durability.persist(BatchInput { requests }, result)
            } else {
                durability.replay()
            }
        }

        fn get_timing_marks(
            input: TextInput,
            voice: String,
        ) -> Result<Vec<TimingInfo>, TtsError> {
            LOGGING_STATE.with_borrow_mut(|state| state.init());
            let durability = Durability::<Vec<TimingInfo>, TtsError>::new(
                "golem_tts",
                "get_timing_marks",
                DurableFunctionType::WriteRemote,
            );
            let voice_id = voice;
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::get_timing_marks(input.clone(), voice_id.clone())
                });
                durability.persist(TimingInput { input, voice_id }, result)
            } else {
                durability.replay()
            }
        }

        fn validate_input(
            input: TextInput,
            voice: String,
        ) -> Result<ValidationResult, TtsError> {
            LOGGING_STATE.with_borrow_mut(|state| state.init());
            let durability = Durability::<ValidationResult, TtsError>::new(
                "golem_tts",
                "validate_input",
                DurableFunctionType::WriteRemote,
            );
            let voice_id = voice;
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::validate_input(input.clone(), voice_id.clone())
                });
                durability.persist(TimingInput { input, voice_id }, result)
            } else {
                durability.replay()
            }
        }
    }


    impl<Impl: ExtendedGuest> StreamingGuest for DurableTts<Impl> {
        fn create_stream(
            voice: String,
            options: Option<SynthesisOptions>,
        ) -> Result<Vec<AudioChunk>, TtsError> {
            LOGGING_STATE.with_borrow_mut(|state| state.init());
            let durability = Durability::<StreamInput, TtsError>::new(
                "golem_tts",
                "create_stream",
                DurableFunctionType::WriteRemote,
            );
            let voice_id = voice;
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::create_stream(StreamRequest {
                        voice_id: voice_id.clone(),
                        options: options.clone(),
                    })
                });
                match result {
                    Ok(stream) => {
                        let _ = durability.persist(
                            StreamInput {
                                voice_id,
                                options,
                            },
                            Ok(StreamInput {
                                voice_id: "".to_string(),
                                options: None,
                            }),
                        );
                        if let Some(chunk) = stream.receive_chunk()? {
                            Ok(vec![chunk])
                        } else {
                            Ok(Vec::new())
                        }
                    }
                    Err(error) => {
                        let _ = durability.persist(
                            StreamInput {
                                voice_id,
                                options,
                            },
                            Err(error.clone()),
                        );
                        Err(error)
                    }
                }
            } else {
                let _ = durability.replay::<StreamInput, TtsError>()?;
                Err(TtsError::UnsupportedOperation(
                    "Streaming replay not supported".to_string(),
                ))
            }
        }
    }

    impl<Impl: ExtendedGuest> crate::golem::tts::advanced::Guest for DurableTts<Impl> {
        fn create_voice_clone(
            name: String,
            audio_samples: Vec<AudioSample>,
            description: Option<String>,
        ) -> Result<String, TtsError> {
            LOGGING_STATE.with_borrow_mut(|state| state.init());
            let durability = Durability::<String, TtsError>::new(
                "golem_tts",
                "create_voice_clone",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::create_voice_clone(name.clone(), audio_samples.clone(), description.clone())
                });
                durability.persist((name, audio_samples, description), result)
            } else {
                durability.replay()
            }
        }

        fn design_voice(name: String, characteristics: VoiceDesignParams) -> Result<String, TtsError> {
            LOGGING_STATE.with_borrow_mut(|state| state.init());
            let durability = Durability::<String, TtsError>::new(
                "golem_tts",
                "design_voice",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::design_voice(name.clone(), characteristics.clone())
                });
                durability.persist((name, characteristics), result)
            } else {
                durability.replay()
            }
        }

        fn convert_voice(
            input_audio: Vec<u8>,
            target_voice: String,
            preserve_timing: Option<bool>,
        ) -> Result<SynthesisResult, TtsError> {
            LOGGING_STATE.with_borrow_mut(|state| state.init());
            let durability = Durability::<SynthesisResult, TtsError>::new(
                "golem_tts",
                "convert_voice",
                DurableFunctionType::WriteRemote,
            );
            let voice_id = target_voice;
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::convert_voice(input_audio.clone(), voice_id.clone(), preserve_timing)
                });
                durability.persist((input_audio, voice_id, preserve_timing), result)
            } else {
                durability.replay()
            }
        }

        fn generate_sound_effect(
            description: String,
            duration_seconds: Option<f32>,
            style_influence: Option<f32>,
        ) -> Result<SynthesisResult, TtsError> {
            LOGGING_STATE.with_borrow_mut(|state| state.init());
            let durability = Durability::<SynthesisResult, TtsError>::new(
                "golem_tts",
                "generate_sound_effect",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::generate_sound_effect(
                        description.clone(),
                        duration_seconds,
                        style_influence,
                    )
                });
                durability.persist((description, duration_seconds, style_influence), result)
            } else {
                durability.replay()
            }
        }

        // create_lexicon removed from WIT, no implementation needed

        fn synthesize_long_form(
            content: String,
            voice: String,
            output_location: String,
            chapter_breaks: Option<Vec<u32>>,
        ) -> Result<LongFormResult, TtsError> {
            LOGGING_STATE.with_borrow_mut(|state| state.init());
            let durability = Durability::<LongFormResult, TtsError>::new(
                "golem_tts",
                "synthesize_long_form",
                DurableFunctionType::WriteRemote,
            );
            let voice_id = voice;
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::synthesize_long_form(
                        content.clone(),
                        voice_id.clone(),
                        output_location.clone(),
                        chapter_breaks.clone(),
                    )
                });
                durability.persist((content, voice_id, output_location, chapter_breaks), result)
            } else {
                durability.replay()
            }
        }
    }

}
