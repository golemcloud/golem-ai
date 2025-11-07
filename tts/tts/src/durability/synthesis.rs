#[cfg(not(feature = "durability"))]
mod passthrough_impl {
    use crate::{
        durability::DurableTTS,
        golem::tts::{
            synthesis::{Guest, SynthesisOptions, TextInput, TimingInfo, ValidationResult},
            types::SynthesisResult,
            voices::TtsError,
        },
    };

    impl<Impl: Guest> Guest for DurableTTS<Impl> {
        #[doc = " Convert text to speech (removed async)"]
        fn synthesize(
            input: TextInput,
            voice: Voice,
            options: Option<SynthesisOptions>,
        ) -> Result<SynthesisResult, TtsError> {
            Impl::synthesize(input, voice, options)
        }

        #[doc = " Batch synthesis for multiple inputs (removed async)"]
        fn synthesize_batch(
            inputs: Vec<TextInput>,
            voice: Voice,
            options: Option<SynthesisOptions>,
        ) -> Result<Vec<SynthesisResult>, TtsError> {
            Impl::synthesize_batch(inputs, voice, options)
        }

        #[doc = " Get timing information without audio synthesis"]
        fn get_timing_marks(input: TextInput, voice: Voice) -> Result<Vec<TimingInfo>, TtsError> {
            Impl::get_timing_marks(input, voice)
        }

        #[doc = " Validate text before synthesis"]
        fn validate_input(input: TextInput, voice: Voice) -> Result<ValidationResult, TtsError> {
            Impl::validate_input(input, voice)
        }
    }
}

#[cfg(feature = "durability")]
mod durability_impl {
    use golem_rust::{
        bindings::golem::durability::durability::DurableFunctionType, durability::Durability,
        with_persistence_level, FromValueAndType, IntoValue, PersistenceLevel,
    };
    use log::trace;

    use crate::{
        durability::DurableTTS,
        golem::tts::{
            synthesis::{Guest, SynthesisOptions, TextInput, TimingInfo, ValidationResult},
            types::SynthesisResult,
            voices::{TtsError, Voice},
        },
        init_logging,
    };
    #[derive(Debug, Clone, IntoValue, FromValueAndType)]
    struct NoInput;

    impl<Impl: Guest> Guest for DurableTTS<Impl> {
        #[doc = " Convert text to speech (removed async)"]
        fn synthesize(
            input: TextInput,
            voice: Voice,
            options: Option<SynthesisOptions>,
        ) -> Result<SynthesisResult, TtsError> {
            init_logging();

            let durability = Durability::<SynthesisResult, TtsError>::new(
                "golem-tts",
                "synthesize",
                DurableFunctionType::WriteRemote,
            );

            if durability.is_live() {
                trace!("[LIVE] synthesize");

                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::synthesize(input.clone(), voice, options.clone())
                });
                durability.persist(NoInput, result)
            } else {
                trace!("[REPLAY] synthesize");

                durability.replay()
            }
        }

        #[doc = " Batch synthesis for multiple inputs (removed async)"]
        fn synthesize_batch(
            inputs: Vec<TextInput>,
            voice: Voice,
            options: Option<SynthesisOptions>,
        ) -> Result<Vec<SynthesisResult>, TtsError> {
            init_logging();

            let durability = Durability::<Vec<SynthesisResult>, TtsError>::new(
                "golem-tts",
                "synthesize_batch",
                DurableFunctionType::WriteRemote,
            );

            if durability.is_live() {
                trace!("[LIVE] synthesize_batch");

                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::synthesize_batch(inputs.clone(), voice, options.clone())
                });

                durability.persist(NoInput, result)
            } else {
                trace!("[REPLAY] synthesize_batch");

                durability.replay()
            }
        }

        #[doc = " Get timing information without audio synthesis"]
        fn get_timing_marks(input: TextInput, voice: Voice) -> Result<Vec<TimingInfo>, TtsError> {
            init_logging();

            let durability = Durability::<Vec<TimingInfo>, TtsError>::new(
                "golem-tts",
                "get_timing_marks",
                DurableFunctionType::WriteRemote,
            );

            if durability.is_live() {
                trace!("[LIVE] get_timing_marks");

                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::get_timing_marks(input.clone(), voice)
                });

                durability.persist(NoInput, result)
            } else {
                trace!("[REPLAY] get_timing_marks");

                durability.replay()
            }
        }

        #[doc = " Validate text before synthesis"]
        fn validate_input(input: TextInput, voice: Voice) -> Result<ValidationResult, TtsError> {
            init_logging();

            let durability = Durability::<ValidationResult, TtsError>::new(
                "golem-tts",
                "validate_input",
                DurableFunctionType::WriteRemote,
            );

            if durability.is_live() {
                trace!("[LIVE] validate_input");

                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::validate_input(input.clone(), voice)
                });

                durability.persist(NoInput, result)
            } else {
                trace!("[REPLAY] validate_input");

                durability.replay()
            }
        }
    }
}
