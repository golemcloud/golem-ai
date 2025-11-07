#[cfg(not(feature = "durability"))]
mod passthrough_impl {
    use crate::{
        durability::DurableTTS,
        golem::tts::{
            advanced::{
                AudioSample, Guest, LanguageCode, LongFormOperation, PronunciationEntry,
                PronunciationLexicon, TtsError, Voice, VoiceDesignParams,
            },
            voices::Voice,
        },
    };

    impl<Impl: TtsClient> Guest for DurableTTS<Impl> {
        type PronunciationLexicon = Impl::PronunciationLexicon;

        type LongFormOperation = Impl::LongFormOperation;

        #[doc = " Voice cloning and creation (removed async)"]
        fn create_voice_clone(
            name: String,
            audio_samples: Vec<AudioSample>,
            description: Option<String>,
        ) -> Result<Voice, TtsError> {
            let client = Impl::new()?;
            client.create_voice_clone(name, audio_samples, description)
        }

        #[doc = " Design synthetic voice (removed async)"]
        fn design_voice(
            name: String,
            characteristics: VoiceDesignParams,
        ) -> Result<Voice, TtsError> {
            let client = Impl::new()?;
            client.design_voice(name, characteristics)
        }

        #[doc = " Voice-to-voice conversion (removed async)"]
        fn convert_voice(
            input_audio: Vec<u8>,
            target_voice: Voice,
            preserve_timing: Option<bool>,
        ) -> Result<Vec<u8>, TtsError> {
            let client = Impl::new()?;
            let target_voice_id = target_voice.id.clone();
            client.convert_voice(input_audio, target_voice_id, preserve_timing)
        }

        #[doc = " Generate sound effects from text description (removed async)"]
        fn generate_sound_effect(
            description: String,
            duration_seconds: Option<f32>,
            style_influence: Option<f32>,
        ) -> Result<Vec<u8>, TtsError> {
            let client = Impl::new()?;
            client.generate_sound_effect(description, duration_seconds, style_influence)
        }

        #[doc = " Create custom pronunciation lexicon"]
        fn create_lexicon(
            name: String,
            language: LanguageCode,
            entries: Option<Vec<PronunciationEntry>>,
        ) -> Result<PronunciationLexicon, TtsError> {
            let client = Impl::new()?;
            client.create_lexicon(name, language, entries)
        }

        #[doc = " Long-form content synthesis with optimization (removed async)"]
        fn synthesize_long_form(
            content: String,
            voice: Voice,
            chapter_breaks: Option<Vec<u32>>,
        ) -> Result<LongFormOperation, TtsError> {
            let client = Impl::new()?;
            let voice_id = voice.id.clone();
            client.synthesize_long_form(content, voice_id, chapter_breaks)
        }
    }
}

#[cfg(feature = "durability")]
mod durability_impl {
    use std::cell::RefCell;

    use golem_rust::{
        bindings::golem::durability::durability::DurableFunctionType, durability::Durability,
        with_persistence_level, FromValueAndType, IntoValue, PersistenceLevel,
    };

    use crate::{
        durability::{DurableTTS, ExtendedAdvancedTrait},
        golem::tts::advanced::{
            AudioSample, Guest, GuestLongFormOperation, GuestPronunciationLexicon, LanguageCode,
            LongFormOperation, LongFormResult, OperationStatus, PronunciationEntry,
            PronunciationLexicon, TtsError, Voice, VoiceDesignParams,
        },
    };

    #[derive(Debug, IntoValue)]
    struct NoInput;

    #[derive(Debug, Clone, FromValueAndType, IntoValue)]
    struct NoOutput;

    enum DurablePronunciationLexiconState<Impl: Guest> {
        Live { lexicon: Impl::PronunciationLexicon },
        Replay,
    }

    pub struct DurablePronunciationLexicon<Impl: Guest> {
        state: RefCell<Option<DurablePronunciationLexiconState<Impl>>>,
    }

    impl<Impl: Guest> DurablePronunciationLexicon<Impl> {
        fn live(lexicon: Impl::PronunciationLexicon) -> Self {
            Self {
                state: RefCell::new(Some(DurablePronunciationLexiconState::Live { lexicon })),
            }
        }
        fn replay() -> Self {
            Self {
                state: RefCell::new(Some(DurablePronunciationLexiconState::Replay)),
            }
        }
    }

    impl<Impl: Guest + 'static> GuestPronunciationLexicon for DurablePronunciationLexicon<Impl> {
        fn get_name(&self) -> String {
            let durability = Durability::<String, TtsError>::new(
                "golem-tts",
                "get_name",
                DurableFunctionType::ReadLocal,
            );
            if durability.is_live() {
                let state = self.state.borrow_mut();
                match &*state {
                    Some(DurablePronunciationLexiconState::Live { lexicon }) => {
                        let result =
                            with_persistence_level(PersistenceLevel::PersistNothing, || {
                                lexicon.get_name()
                            });
                        let _ = durability.persist_infallible(NoInput, result.clone());
                        result
                    }

                    _ => {
                        unreachable!()
                    }
                }
            } else {
                durability.replay_infallible()
            }
        }

        fn get_language(&self) -> LanguageCode {
            let durability = Durability::<String, TtsError>::new(
                "golem-tts",
                "get_language",
                DurableFunctionType::ReadLocal,
            );
            if durability.is_live() {
                let state = self.state.borrow_mut();
                match &*state {
                    Some(DurablePronunciationLexiconState::Live { lexicon }) => {
                        let result =
                            with_persistence_level(PersistenceLevel::PersistNothing, || {
                                lexicon.get_language()
                            });
                        let _ = durability.persist_infallible(NoInput, result.clone());
                        result
                    }

                    _ => {
                        unreachable!()
                    }
                }
            } else {
                durability.replay_infallible()
            }
        }

        fn get_entry_count(&self) -> u32 {
            let durability = Durability::<u32, TtsError>::new(
                "golem-tts",
                "get_entry_count",
                DurableFunctionType::ReadLocal,
            );
            if durability.is_live() {
                let state = self.state.borrow_mut();
                match &*state {
                    Some(DurablePronunciationLexiconState::Live { lexicon }) => {
                        let result =
                            with_persistence_level(PersistenceLevel::PersistNothing, || {
                                lexicon.get_entry_count()
                            });
                        let _ = durability.persist_infallible(NoInput, result);

                        result
                    }

                    _ => {
                        unreachable!()
                    }
                }
            } else {
                durability.replay_infallible()
            }
        }

        #[doc = " Add pronunciation rule"]
        fn add_entry(&self, word: String, pronunciation: String) -> Result<(), TtsError> {
            let durability = Durability::<NoOutput, TtsError>::new(
                "golem-tts",
                "add_entry",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let state = self.state.borrow_mut();
                match &*state {
                    Some(DurablePronunciationLexiconState::Live { lexicon }) => {
                        with_persistence_level(PersistenceLevel::PersistNothing, || {
                            lexicon.add_entry(word, pronunciation)
                        })?;
                        let _ = durability.persist_infallible(NoInput, NoOutput);
                        Ok(())
                    }

                    _ => {
                        unreachable!()
                    }
                }
            } else {
                let _: NoOutput = durability.replay_infallible();
                Ok(())
            }
        }

        #[doc = " Remove pronunciation rule"]
        fn remove_entry(&self, word: String) -> Result<(), TtsError> {
            let durability = Durability::<NoOutput, TtsError>::new(
                "golem-tts",
                "remove_entry",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let state = self.state.borrow_mut();
                match &*state {
                    Some(DurablePronunciationLexiconState::Live { lexicon }) => {
                        with_persistence_level(PersistenceLevel::PersistNothing, || {
                            lexicon.remove_entry(word.clone())
                        })?;
                        let _ = durability.persist_infallible(NoInput, NoOutput);
                        Ok(())
                    }

                    _ => {
                        unreachable!()
                    }
                }
            } else {
                let _: NoOutput = durability.replay_infallible();
                Ok(())
            }
        }

        #[doc = " Export lexicon content"]
        fn export_content(&self) -> Result<String, TtsError> {
            let durability = Durability::<String, TtsError>::new(
                "golem-tts",
                "export_content",
                DurableFunctionType::ReadLocal,
            );
            if durability.is_live() {
                let state = self.state.borrow_mut();
                match &*state {
                    Some(DurablePronunciationLexiconState::Live { lexicon }) => {
                        let result =
                            with_persistence_level(PersistenceLevel::PersistNothing, || {
                                lexicon.export_content()
                            });
                        let _ = durability.persist(NoInput, result.clone());
                        result
                    }

                    _ => {
                        unreachable!()
                    }
                }
            } else {
                durability.replay()
            }
        }
    }

    enum DurableLongFormOperationState<Impl: Guest> {
        Live { operation: Impl::LongFormOperation },
        Replay,
    }
    pub struct DurableLongFormOperation<Impl: Guest> {
        state: RefCell<Option<DurableLongFormOperationState<Impl>>>,
    }
    impl<Impl: Guest> DurableLongFormOperation<Impl> {
        fn live(operation: Impl::LongFormOperation) -> Self {
            Self {
                state: RefCell::new(Some(DurableLongFormOperationState::Live { operation })),
            }
        }
        fn replay() -> Self {
            Self {
                state: RefCell::new(Some(DurableLongFormOperationState::Replay)),
            }
        }
    }

    impl<Impl: Guest + 'static> GuestLongFormOperation for DurableLongFormOperation<Impl> {
        fn get_status(&self) -> OperationStatus {
            let durability = Durability::<OperationStatus, TtsError>::new(
                "golem-tts",
                "get_status",
                DurableFunctionType::ReadLocal,
            );
            if durability.is_live() {
                let state = self.state.borrow_mut();
                match &*state {
                    Some(DurableLongFormOperationState::Live { operation }) => {
                        let result =
                            with_persistence_level(PersistenceLevel::PersistNothing, || {
                                operation.get_status()
                            });
                        let _ = durability.persist_infallible(NoInput, result);
                        result
                    }

                    _ => {
                        unreachable!()
                    }
                }
            } else {
                durability.replay_infallible()
            }
        }

        fn get_progress(&self) -> f32 {
            let durability = Durability::<f32, TtsError>::new(
                "golem-tts",
                "get_progress",
                DurableFunctionType::ReadLocal,
            );
            if durability.is_live() {
                let state = self.state.borrow_mut();
                match &*state {
                    Some(DurableLongFormOperationState::Live { operation }) => {
                        let result =
                            with_persistence_level(PersistenceLevel::PersistNothing, || {
                                operation.get_progress()
                            });
                        let _ = durability.persist_infallible(NoInput, result);
                        result
                    }

                    _ => {
                        unreachable!()
                    }
                }
            } else {
                durability.replay_infallible()
            }
        }

        fn cancel(&self) -> Result<(), TtsError> {
            let durability = Durability::<NoOutput, TtsError>::new(
                "golem-tts",
                "cancel",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let state = self.state.borrow_mut();
                match &*state {
                    Some(DurableLongFormOperationState::Live { operation }) => {
                        with_persistence_level(PersistenceLevel::PersistNothing, || {
                            operation.cancel()
                        })?;
                        let _ = durability.persist_infallible(NoInput, NoOutput);
                        Ok(())
                    }

                    _ => {
                        unreachable!()
                    }
                }
            } else {
                let _: NoOutput = durability.replay_infallible();
                Ok(())
            }
        }

        fn get_result(&self) -> Result<LongFormResult, TtsError> {
            let durability = Durability::<LongFormResult, TtsError>::new(
                "golem-tts",
                "get_result",
                DurableFunctionType::ReadLocal,
            );
            if durability.is_live() {
                let state = self.state.borrow_mut();
                match &*state {
                    Some(DurableLongFormOperationState::Live { operation }) => {
                        let result =
                            with_persistence_level(PersistenceLevel::PersistNothing, || {
                                operation.get_result()
                            });
                        durability.persist(NoInput, result.clone())
                    }

                    _ => {
                        unreachable!()
                    }
                }
            } else {
                durability.replay()
            }
        }
    }

    impl<Impl: ExtendedAdvancedTrait> Guest for DurableTTS<Impl> {
        type PronunciationLexicon = DurablePronunciationLexicon<Impl>;

        type LongFormOperation = DurableLongFormOperation<Impl>;

        #[doc = " Voice cloning and creation (removed async)"]
        fn create_voice_clone(
            name: String,
            audio_samples: Vec<AudioSample>,
            description: Option<String>,
        ) -> Result<Voice, TtsError> {
            let durability = Durability::<Voice, TtsError>::new(
                "golem-tts",
                "create_voice_clone",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::create_voice_clone(
                        name.clone(),
                        audio_samples.clone(),
                        description.clone(),
                    )
                });
                durability.persist(NoInput, result)
            } else {
                durability.replay()
            }
        }

        #[doc = " Design synthetic voice (removed async)"]
        fn design_voice(
            name: String,
            characteristics: VoiceDesignParams,
        ) -> Result<Voice, TtsError> {
            let durability = Durability::<Voice, TtsError>::new(
                "golem-tts",
                "create_voice_clone",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::design_voice(name, characteristics)
                });
                durability.persist(NoInput, result)
            } else {
                durability.replay()
            }
        }

        #[doc = " Voice-to-voice conversion (removed async)"]
        fn convert_voice(
            input_audio: Vec<u8>,
            target_voice: Voice,
            preserve_timing: Option<bool>,
        ) -> Result<Vec<u8>, TtsError> {
            let durability = Durability::<Vec<u8>, TtsError>::new(
                "golem-tts",
                "create_voice_clone",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::convert_voice(input_audio, target_voice, preserve_timing)
                });
                durability.persist(NoInput, result)
            } else {
                durability.replay()
            }
        }

        #[doc = " Generate sound effects from text description (removed async)"]
        fn generate_sound_effect(
            description: String,
            duration_seconds: Option<f32>,
            style_influence: Option<f32>,
        ) -> Result<Vec<u8>, TtsError> {
            let durability = Durability::<Vec<u8>, TtsError>::new(
                "golem-tts",
                "create_voice_clone",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::generate_sound_effect(description, duration_seconds, style_influence)
                });
                durability.persist(NoInput, result)
            } else {
                durability.replay()
            }
        }

        #[doc = " Create custom pronunciation lexicon"]
        fn create_lexicon(
            name: String,
            language: LanguageCode,
            entries: Option<Vec<PronunciationEntry>>,
        ) -> Result<PronunciationLexicon, TtsError> {
            let durability = Durability::<NoOutput, TtsError>::new(
                "golem-tts",
                "create_voice_clone",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    let lexicon = Impl::unwrappered_created_lexicon(name, language, entries)?;
                    Ok(PronunciationLexicon::new(
                        DurablePronunciationLexicon::<Impl>::live(lexicon),
                    ))
                });
                let _ = durability.persist_infallible(NoInput, NoOutput);
                result
            } else {
                let _: NoOutput = durability.replay_infallible();
                Ok(PronunciationLexicon::new(
                    DurablePronunciationLexicon::<Impl>::replay(),
                ))
            }
        }

        #[doc = " Long-form content synthesis with optimization (removed async)"]
        fn synthesize_long_form(
            content: String,
            voice: Voice,
            chapter_breaks: Option<Vec<u32>>,
        ) -> Result<LongFormOperation, TtsError> {
            let durability = Durability::<NoOutput, TtsError>::new(
                "golem-tts",
                "create_voice_clone",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    let longform_operation = Impl::unwrappered_synthesize_long_form(
                        content,
                        voice,
                        chapter_breaks,
                    )?;
                    Ok(LongFormOperation::new(
                        DurableLongFormOperation::<Impl>::live(longform_operation),
                    ))
                });
                let _ = durability.persist_infallible(NoInput, NoOutput);
                result
            } else {
                let _: NoOutput = durability.replay_infallible();
                Ok(LongFormOperation::new(
                    DurableLongFormOperation::<Impl>::replay(),
                ))
            }
        }
    }
}
