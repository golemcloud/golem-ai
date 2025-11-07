#[cfg(not(feature = "durability"))]
mod passthrough_impl {
    use crate::{
        durability::DurableTTS,
        golem::tts::{
            advanced::TtsError,
            voices::{Guest, LanguageInfo, TtsError, Voice, Voice, VoiceFilter},
        },
    };

    impl<Impl: Guest> Guest for DurableTTS<Impl> {
        type Voice = Impl::Voice;

        #[doc = " List available voices with filtering and pagination"]
        fn list_voices(filter: Option<VoiceFilter>) -> Result<Vec<Voice>, TtsError> {
            Impl::list_voices(filter)
        }

        #[doc = " Get specific voice by ID"]
        fn get_voice(voice_id: String) -> Result<Voice, TtsError> {
            Impl::get_voice(voice_id)
        }

        #[doc = " Get supported languages"]
        fn list_languages() -> Result<Vec<LanguageInfo>, TtsError> {
            Impl::list_languages()
        }
    }
}
mod durability_impl {

    use std::fmt::Debug;

    use golem_rust::{
        bindings::golem::durability::durability::DurableFunctionType, durability::Durability,
        with_persistence_level, FromValueAndType, IntoValue, PersistenceLevel,
    };
    use log::trace;

    use crate::{
        durability::DurableTTS,
        golem::tts::voices::{Guest, LanguageInfo, TtsError, Voice, VoiceFilter},
        init_logging,
    };

    impl<Impl: Guest> Guest for DurableTTS<Impl> {
        #[doc = " List available voices with filtering and pagination"]
        fn list_voices(filter: Option<VoiceFilter>) -> Result<Vec<Voice>, TtsError> {
            init_logging();

            let durability = Durability::<Vec<Voice>, TtsError>::new(
                "golem-tts",
                "list_voices",
                DurableFunctionType::ReadLocal,
            );

            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::list_voices(filter.clone())
                });
                let _ = durability.persist(ListVoicesInput { filter }, result.clone());
                result
            } else {
                durability.replay()
            }
        }

        #[doc = " Get specific voice by ID"]
        fn get_voice(voice_id: String) -> Result<Voice, TtsError> {
            init_logging();

            let durability = Durability::<Voice, TtsError>::new(
                "golem-tts",
                "get_voice",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                trace!("[LIVE] get_voice");
                let voice = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::get_voice(voice_id.clone())
                });
                let _ = durability.persist(GetVoiceInput { voice_id }, Clone::clone(&voice));

                voice
            } else {
                trace!("[REPLAY] get_voice");
                durability.replay()
            }
        }

        #[doc = " Get supported languages"]
        fn list_languages() -> Result<Vec<LanguageInfo>, TtsError> {
            init_logging();

            let durability = Durability::<Vec<LanguageInfo>, TtsError>::new(
                "golem-tts",
                "list_languages",
                DurableFunctionType::ReadLocal,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::list_languages()
                });

                let _ = durability.persist(NoInput, result.clone());

                result
            } else {
                durability.replay()
            }
        }
    }

    #[derive(Debug, Clone, IntoValue, FromValueAndType)]
    struct NoInput;

    #[derive(Debug, Clone, IntoValue, FromValueAndType)]
    struct ListVoicesInput {
        filter: Option<VoiceFilter>,
    }

    #[derive(Debug, Clone, IntoValue, FromValueAndType)]
    struct GetVoiceInput {
        voice_id: String,
    }
}
