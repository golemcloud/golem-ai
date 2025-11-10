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
        golem::tts::{advanced::{
            AudioSample, Guest, GuestLongFormOperation, GuestPronunciationLexicon, LanguageCode,
            LongFormOperation, LongFormResult, OperationStatus, PronunciationEntry,
            PronunciationLexicon, TtsError, Voice, VoiceDesignParams,
        }, types::SynthesisMetadata},
        init_logging,
    };

    #[derive(Debug, IntoValue)]
    struct NoInput;

    #[derive(Debug, Clone, FromValueAndType, IntoValue)]
    struct NoOutput;

    enum DurablePronunciationLexiconState<Impl: Guest> {
        Live {
            lexicon: Impl::PronunciationLexicon,
        },
        Replay {
            name: String,
            language: LanguageCode,
            entry_count: u32,
            content: Option<String>,
        },
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
        fn replay(
            name: String,
            language: LanguageCode,
            entry_count: u32,
            content: Option<String>,
        ) -> Self {
            Self {
                state: RefCell::new(Some(DurablePronunciationLexiconState::Replay {
                    name,
                    language,
                    entry_count,
                    content,
                })),
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
            init_logging();
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
                    Some(DurablePronunciationLexiconState::Replay { name, .. }) => name.clone(),
                    _ => {
                        unreachable!()
                    }
                }
            } else {
                let replay: String = durability.replay_infallible();
                let mut state = self.state.borrow_mut();
                match &mut *state {
                    Some(DurablePronunciationLexiconState::Replay { name, .. }) => {
                        *name = replay.clone();
                    }
                    _ => {
                        unreachable!()
                    }
                }
                replay
            }
        }

        fn get_language(&self) -> LanguageCode {
            let durability = Durability::<String, TtsError>::new(
                "golem-tts",
                "get_language",
                DurableFunctionType::ReadLocal,
            );
            init_logging();
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
                    Some(DurablePronunciationLexiconState::Replay { language, .. }) => {
                        language.clone()
                    }
                    _ => {
                        unreachable!()
                    }
                }
            } else {
                let replay: LanguageCode = durability.replay_infallible();
                let mut state = self.state.borrow_mut();
                match &mut *state {
                    Some(DurablePronunciationLexiconState::Replay { language, .. }) => {
                        *language = replay.clone();
                    }
                    _ => {
                        unreachable!()
                    }
                }
                replay
            }
        }

        fn get_entry_count(&self) -> u32 {
            let durability = Durability::<u32, TtsError>::new(
                "golem-tts",
                "get_entry_count",
                DurableFunctionType::ReadLocal,
            );
            init_logging();
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
                    Some(DurablePronunciationLexiconState::Replay { entry_count, .. }) => {
                        *entry_count
                    }
                    _ => {
                        unreachable!()
                    }
                }
            } else {
                let replay: u32 = durability.replay_infallible();
                let mut state = self.state.borrow_mut();
                match &mut *state {
                    Some(DurablePronunciationLexiconState::Replay { entry_count, .. }) => {
                        *entry_count = replay;
                    }
                    _ => {
                        unreachable!()
                    }
                }
                replay
            }
        }

        #[doc = " Add pronunciation rule"]
        fn add_entry(&self, word: String, pronunciation: String) -> Result<(), TtsError> {
            let durability = Durability::<NoOutput, TtsError>::new(
                "golem-tts",
                "add_entry",
                DurableFunctionType::WriteRemote,
            );
            init_logging();
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
                    Some(DurablePronunciationLexiconState::Replay { .. }) => Ok(()),
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
            init_logging();
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
                    Some(DurablePronunciationLexiconState::Replay { .. }) => Ok(()),

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
            init_logging();
            if durability.is_live() {
                let state = self.state.borrow_mut();
                match &*state {
                    Some(DurablePronunciationLexiconState::Live { lexicon }) => {
                        let result =
                            with_persistence_level(PersistenceLevel::PersistNothing, || {
                                lexicon.export_content()
                            });
                        durability.persist(NoInput, result.clone())
                    }
                    Some(DurablePronunciationLexiconState::Replay { content, .. }) => {
                        // In replay we assign the replayed value so it should not panic
                        Ok(content.clone().unwrap())
                    }
                    _ => {
                        unreachable!()
                    }
                }
            } else {
                let replay: Result<String, TtsError> = durability.replay();
                let mut state = self.state.borrow_mut();
                match &mut *state {
                    Some(DurablePronunciationLexiconState::Replay { content, .. }) => {
                        *content = replay.clone().ok();
                    }
                    _ => {
                        unreachable!()
                    }
                }
                replay
            }
        }
    }

    enum DurableLongFormOperationState<Impl: Guest> {
        Live {
            operation: Impl::LongFormOperation,
        },
        Replay {
            status: OperationStatus,
            progress: f32,
            result: LongFormResult,
            task_id: String,
            input: LongformInput,
        },
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
        fn replay(
            status: OperationStatus,
            progress: f32,
            result: LongFormResult,
            task_id: String,
            input: LongformInput,
        ) -> Self {
            Self {
                state: RefCell::new(Some(DurableLongFormOperationState::Replay {
                    progress,
                    status,
                    result,
                    task_id,
                    input,
                })),
            }
        }
    }

    impl<Impl: ExtendedAdvancedTrait> GuestLongFormOperation for DurableLongFormOperation<Impl> {
        fn get_task_id(&self) -> Result<String, TtsError> {
            let durability = Durability::<String, TtsError>::new(
                "golem-tts",
                "get_task_id",
                DurableFunctionType::WriteRemote,
            );
            init_logging();
            if durability.is_live() {
                let state = self.state.borrow();
                let (task_id, new_longform_synthesis) = match &*state {
                    Some(DurableLongFormOperationState::Live { operation }) => {
                        let task_id =
                            with_persistence_level(PersistenceLevel::PersistNothing, || {
                                operation.get_task_id()
                            })?;
                        (task_id, None)
                    }
                    Some(DurableLongFormOperationState::Replay { task_id, input, .. }) => {
                        let new_longform_synthesis = Impl::unwrappered_synthesize_long_form(
                            input.content.clone(),
                            input.voice.clone(),
                            input.chapter_breaks.clone(),
                            Some(task_id.clone()),
                        )?;

                        (task_id.clone(), Some(new_longform_synthesis))
                    }
                    _ => unreachable!(),
                };

                if let Some(new_longform_synthesis) = new_longform_synthesis {
                    let mut state = self.state.borrow_mut();

                    *state = Some(DurableLongFormOperationState::Live {
                        operation: new_longform_synthesis,
                    });
                }

                let _ = durability.persist_infallible(NoInput, task_id.clone());
                Ok(task_id)
            } else {
                let replay: String = durability.replay_infallible();
                let mut state = self.state.borrow_mut();
                match &mut *state {
                    Some(DurableLongFormOperationState::Replay { task_id, .. }) => {
                        *task_id = replay.clone();
                    }
                    _ => unreachable!(),
                }

                Ok(replay)
            }
        }

        fn get_status(&self) -> Result<OperationStatus, TtsError> {
            let durability = Durability::<OperationStatus, TtsError>::new(
                "golem-tts",
                "get_status",
                DurableFunctionType::WriteRemote,
            );
            init_logging();
            if durability.is_live() {
                let (status, new_longform_synthesis) = {
                    let state = self.state.borrow();
                    match &*state {
                        Some(DurableLongFormOperationState::Live { operation, .. }) => {
                            let status =
                                with_persistence_level(PersistenceLevel::PersistNothing, || {
                                    operation.get_status()
                                })?;

                            (status, None)
                        }
                        Some(DurableLongFormOperationState::Replay {
                            status,
                            input,
                            task_id,
                            ..
                        }) => {
                            let new_longform_synthesis = Impl::unwrappered_synthesize_long_form(
                                input.content.clone(),
                                input.voice.clone(),
                                input.chapter_breaks.clone(),
                                Some(task_id.clone()),
                            )?;

                            (status.clone(), Some(new_longform_synthesis))
                        }
                        _ => unreachable!(),
                    }
                };

                if let Some(new_longform_synthesis) = new_longform_synthesis {
                    let mut state = self.state.borrow_mut();

                    *state = Some(DurableLongFormOperationState::Live {
                        operation: new_longform_synthesis,
                    });
                }

                let _ = durability.persist_infallible(NoInput, status.clone());
                Ok(status)
            } else {
                let replay: OperationStatus = durability.replay_infallible();
                let mut state = self.state.borrow_mut();
                match &mut *state {
                    Some(DurableLongFormOperationState::Replay { status, .. }) => {
                        *status = replay.clone();
                    }
                    _ => {
                        unreachable!()
                    }
                }
                Ok(replay)
            }
        }

        fn get_progress(&self) -> Result<f32, TtsError> {
            let durability = Durability::<ProgressOutput, TtsError>::new(
                "golem-tts",
                "get_progress",
                DurableFunctionType::WriteRemote,
            );
            init_logging();
            if durability.is_live() {
                let (progress, new_longform_synthesis) = {
                    let state = self.state.borrow();
                    match &*state {
                        Some(DurableLongFormOperationState::Live { operation, .. }) => {
                            let progress =
                                with_persistence_level(PersistenceLevel::PersistNothing, || {
                                    operation.get_progress()
                                })?;
                            (progress, None)
                        }
                        Some(DurableLongFormOperationState::Replay {
                            progress,
                            input,
                            task_id,
                            ..
                        }) => {
                            let new_longform_synthesis = Impl::unwrappered_synthesize_long_form(
                                input.content.clone(),
                                input.voice.clone(),
                                input.chapter_breaks.clone(),
                                Some(task_id.clone()),
                            )?;

                            (progress.clone(), Some(new_longform_synthesis))
                        }
                        _ => unreachable!(),
                    }
                };

                if let Some(new_longform_synthesis) = new_longform_synthesis {
                    let mut state = self.state.borrow_mut();

                    *state = Some(DurableLongFormOperationState::Live {
                        operation: new_longform_synthesis,
                    });
                }

                let _ = durability.persist_infallible(NoInput, ProgressOutput { progress });
                Ok(progress)
            } else {
                let replay: ProgressOutput = durability.replay_infallible();
                let mut state = self.state.borrow_mut();
                match &mut *state {
                    Some(DurableLongFormOperationState::Replay { progress, .. }) => {
                        *progress = replay.progress;
                    }
                    _ => {
                        unreachable!()
                    }
                }
                Ok(replay.progress)
            }
        }

        fn cancel(&self) -> Result<(), TtsError> {
            let durability = Durability::<NoOutput, TtsError>::new(
                "golem-tts",
                "cancel",
                DurableFunctionType::WriteRemote,
            );
            init_logging();
            if durability.is_live() {
                let new_longform_synthesis = {
                    let state = self.state.borrow();
                    match &*state {
                        Some(DurableLongFormOperationState::Live { operation, .. }) => {
                            with_persistence_level(PersistenceLevel::PersistNothing, || {
                                operation.cancel()
                            })?;
                            None
                        }
                        Some(DurableLongFormOperationState::Replay { input, task_id, .. }) => {
                            let new_longform_synthesis = Impl::unwrappered_synthesize_long_form(
                                input.content.clone(),
                                input.voice.clone(),
                                input.chapter_breaks.clone(),
                                Some(task_id.clone()),
                            )?;

                            Some(new_longform_synthesis)
                        }
                        _ => {
                            unreachable!()
                        }
                    }
                };
                if let Some(new_longform_synthesis) = new_longform_synthesis {
                    let mut state = self.state.borrow_mut();

                    *state = Some(DurableLongFormOperationState::Live {
                        operation: new_longform_synthesis,
                    });
                }
                let _ = durability.persist_infallible(NoInput, NoOutput);
                Ok(())
            } else {
                let _: NoOutput = durability.replay_infallible();
                Ok(())
            }
        }

        fn get_result(&self) -> Result<LongFormResult, TtsError> {
            let durability = Durability::<LongFormResult, TtsError>::new(
                "golem-tts",
                "get_result",
                DurableFunctionType::WriteRemote,
            );
            init_logging();
            if durability.is_live() {
                let (result, new_longform_synthesis) = {
                    let state = self.state.borrow();
                    match &*state {
                        Some(DurableLongFormOperationState::Live { operation, .. }) => {
                            let result =
                                with_persistence_level(PersistenceLevel::PersistNothing, || {
                                    operation.get_result()
                                })?;
                            (result, None)
                        }
                        Some(DurableLongFormOperationState::Replay {
                            result,
                            input,
                            task_id,
                            ..
                        }) => {
                            let new_longform_synthesis = Impl::unwrappered_synthesize_long_form(
                                input.content.clone(),
                                input.voice.clone(),
                                input.chapter_breaks.clone(),
                                Some(task_id.clone()),
                            )?;

                            (result.clone(), Some(new_longform_synthesis))
                        }
                        _ => {
                            unreachable!()
                        }
                    }
                };
                if let Some(new_longform_synthesis) = new_longform_synthesis {
                    let mut state = self.state.borrow_mut();

                    *state = Some(DurableLongFormOperationState::Live {
                        operation: new_longform_synthesis,
                    });
                }
                let _ = durability.persist_infallible(NoInput, result.clone());
                Ok(result)
            } else {
                let replay: LongFormResult = durability.replay_infallible();
                let mut state = self.state.borrow_mut();
                match &mut *state {
                    Some(DurableLongFormOperationState::Replay { result, .. }) => {
                        *result = replay.clone();
                    }
                    _ => {
                        unreachable!()
                    }
                }
                Ok(replay)
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
            init_logging();
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
            init_logging();
            let durability = Durability::<Voice, TtsError>::new(
                "golem-tts",
                "design_voice",
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
            init_logging();
            let durability = Durability::<Vec<u8>, TtsError>::new(
                "golem-tts",
                "convert_voice",
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
            init_logging();
            let durability = Durability::<Vec<u8>, TtsError>::new(
                "golem-tts",
                "generate_sound_effect",
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
                "create_lexicon",
                DurableFunctionType::WriteRemote,
            );
            init_logging();
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    let lexicon = Impl::unwrappered_created_lexicon(
                        name.clone(),
                        language.clone(),
                        entries.clone(),
                    )?;
                    Ok(PronunciationLexicon::new(
                        DurablePronunciationLexicon::<Impl>::live(lexicon),
                    ))
                });
                let _ = durability.persist_infallible(NoInput, NoOutput);
                result
            } else {
                let _: NoOutput = durability.replay_infallible();
                let entry_count = entries.as_ref().map(|e| e.len() as u32).unwrap_or(0);
                Ok(PronunciationLexicon::new(
                    DurablePronunciationLexicon::<Impl>::replay(name, language, entry_count, None),
                ))
            }
        }

        #[doc = " Long-form content synthesis with optimization (removed async)"]
        fn synthesize_long_form(
            content: String,
            voice: Voice,
            chapter_breaks: Option<Vec<u32>>,
        ) -> Result<LongFormOperation, TtsError> {
            let durability = Durability::<String, TtsError>::new(
                "golem-tts",
                "synthesize_long_form",
                DurableFunctionType::WriteLocal,
            );
            init_logging();
            if durability.is_live() {
                let longform_operation = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::unwrappered_synthesize_long_form(
                        content.clone(),
                        voice.clone(),
                        chapter_breaks.clone(),
                        None,
                    )
                })?;

                let task_id = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    longform_operation.get_task_id()
                })?;

                let _ = durability.persist_infallible(NoInput, task_id.clone());

                Ok(LongFormOperation::new(DurableLongFormOperation::<Impl>::live(
                    longform_operation,
                )))
            } else {
                let task_id: String = durability.replay_infallible();

                Ok(LongFormOperation::new(
                    DurableLongFormOperation::<Impl>::replay(
                        OperationStatus::Processing,
                        0.0,
                        LongFormResult {
                            output_location: "".to_string(),
                            total_duration: 0.0,
                            chapter_durations: None,
                            metadata: SynthesisMetadata {
                                duration_seconds: 0.0,
                                character_count: 0,
                                word_count: 0,
                                audio_size_bytes: 0,
                                request_id: "".to_string(),
                                provider_info: None,
                            },
                        },
                        task_id,
                        LongformInput {
                            content,
                            voice,
                            chapter_breaks,
                        },
                    ),
                ))
            }
        }
    }

    #[derive(Debug, Clone)]
    struct LongformInput {
        content: String,
        voice: Voice,
        chapter_breaks: Option<Vec<u32>>,
    }

    #[derive(Debug, Clone, FromValueAndType, IntoValue)]
    struct ProgressOutput {
        progress: f32,
    }
}
