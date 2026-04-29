use std::marker::PhantomData;

use crate::guest::SttTranscriptionProvider;
use crate::LanguageProvider;

pub struct DurableStt<Impl> {
    phantom: PhantomData<Impl>,
}

pub trait ExtendedSttProvider: SttTranscriptionProvider + LanguageProvider + 'static {}

#[cfg(not(feature = "durability"))]
mod passthrough_impl {
    use bytes::Bytes;

    use crate::model::golem::stt::languages::{
        Guest as WitLanguageGuest, LanguageInfo as WitLanguageInfo,
    };

    use crate::durability::{DurableStt, ExtendedSttProvider};
    use crate::model::golem::stt::transcription::{
        Guest as WitTranscriptionGuest, MultiTranscriptionResult as WitMultiTranscriptionResult,
        TranscriptionRequest as WitTranscriptionRequest,
    };

    use crate::model::golem::stt::types::{
        SttError, TranscriptionResult as WitTranscriptionResult,
    };

    use crate::guest::SttTranscriptionRequest;
    use crate::model::types::SttError;
    use crate::LOGGING_STATE;
    use golem_rust::{FromValueAndType, IntoValue};
    use wstd::runtime::block_on;

    // When used as a standalone WASM component (no `durability` feature), the WIT `Guest`
    // exports are synchronous. This is the only safe place to introduce a `block_on` — a
    // component exported this way is not running inside an outer async executor.
    impl<Impl: ExtendedSttProvider> WitTranscriptionGuest for DurableStt<Impl> {
        fn transcribe(
            request: WitTranscriptionRequest,
        ) -> Result<WitTranscriptionResult, SttError> {
            LOGGING_STATE.with_borrow_mut(|state| state.init());

            let request = SttTranscriptionRequest {
                request_id: request.request_id,
                audio: Bytes::from(request.audio),
                config: request.config,
                options: request.options,
            };

            block_on(Impl::transcribe(request))
        }

        fn transcribe_many(
            requests: Vec<WitTranscriptionRequest>,
        ) -> Result<WitMultiTranscriptionResult, SttError> {
            LOGGING_STATE.with_borrow_mut(|state| state.init());

            let stt_requests: Vec<SttTranscriptionRequest> = requests
                .into_iter()
                .map(|req| SttTranscriptionRequest {
                    request_id: req.request_id,
                    audio: Bytes::from(req.audio),
                    config: req.config,
                    options: req.options,
                })
                .collect();

            block_on(Impl::transcribe_many(stt_requests))
        }
    }

    impl<Impl: ExtendedSttProvider> WitLanguageGuest for DurableStt<Impl> {
        fn list_languages() -> Result<Vec<WitLanguageInfo>, SttError> {
            Impl::list_languages()
        }
    }

    #[derive(Debug, Clone, PartialEq, IntoValue, FromValueAndType)]
    struct TranscribeInput {
        request: WitTranscriptionRequest,
    }

    #[derive(Debug, Clone, PartialEq, IntoValue, FromValueAndType)]
    struct TranscribeManyInput {
        requests: Vec<WitTranscriptionRequest>,
    }

    impl From<&SttError> for SttError {
        fn from(error: &SttError) -> Self {
            error.clone()
        }
    }
}

#[cfg(feature = "durability")]
mod durable_impl {
    use bytes::Bytes;
    use golem_rust::bindings::golem::durability::durability::DurableFunctionType;
    use golem_rust::durability::Durability;

    use crate::durability::{DurableStt, ExtendedSttProvider};

    use crate::guest::SttTranscriptionRequest;
    use crate::model::languages::LanguageInfo;
    use crate::model::transcription::{
        MultiTranscriptionResult, TranscriptionRequest, TranscriptionResult,
    };
    use crate::model::types::SttError;
    use crate::{LanguageProvider, TranscriptionProvider, LOGGING_STATE};
    use golem_rust::{
        with_persistence_level_async, FromValueAndType, IntoValue, PersistenceLevel,
    };

    impl<Impl: ExtendedSttProvider> TranscriptionProvider for DurableStt<Impl> {
        async fn transcribe(
            request: TranscriptionRequest,
        ) -> Result<TranscriptionResult, SttError> {
            LOGGING_STATE.with_borrow_mut(|state| state.init());
            let durability = Durability::<TranscriptionResult, SttError>::new(
                "golem_ai_stt",
                "transcribe",
                DurableFunctionType::WriteRemote,
            );

            let audio_bytes = Bytes::from(request.audio);
            let request_id = request.request_id;
            let config = request.config;
            let options = request.options;

            if durability.is_live() {
                let result = with_persistence_level_async(PersistenceLevel::PersistNothing, || {
                    let request = SttTranscriptionRequest {
                        request_id: request_id.clone(),
                        audio: audio_bytes.clone(),
                        config,
                        options: options.clone(),
                    };

                    async move { Impl::transcribe(request).await }
                })
                .await;

                // Reconstruct original request for persistence
                let orig_request_copy = TranscriptionRequest {
                    request_id,
                    audio: audio_bytes.to_vec(),
                    config,
                    options,
                };

                durability.persist(
                    TranscribeInput {
                        request: orig_request_copy,
                    },
                    result,
                )
            } else {
                durability.replay()
            }
        }

        async fn transcribe_many(
            requests: Vec<TranscriptionRequest>,
        ) -> Result<MultiTranscriptionResult, SttError> {
            LOGGING_STATE.with_borrow_mut(|state| state.init());
            let durability = Durability::<MultiTranscriptionResult, SttError>::new(
                "golem_ai_stt",
                "transcribe_many",
                DurableFunctionType::WriteRemote,
            );

            let requests_with_bytes: Vec<_> = requests
                .into_iter()
                .map(|req| {
                    (
                        Bytes::from(req.audio),
                        req.request_id,
                        req.config,
                        req.options,
                    )
                })
                .collect();

            if durability.is_live() {
                let stt_requests: Vec<SttTranscriptionRequest> = requests_with_bytes
                    .iter()
                    .map(
                        |(audio_bytes, request_id, config, options)| SttTranscriptionRequest {
                            request_id: request_id.clone(),
                            audio: audio_bytes.clone(),
                            config: *config,
                            options: options.clone(),
                        },
                    )
                    .collect();

                let result = with_persistence_level_async(
                    PersistenceLevel::PersistNothing,
                    || async move { Impl::transcribe_many(stt_requests).await },
                )
                .await;

                // Reconstruct original requests for persistence
                let orig_requests_copy: Vec<TranscriptionRequest> = requests_with_bytes
                    .into_iter()
                    .map(
                        |(audio_bytes, request_id, config, options)| TranscriptionRequest {
                            request_id,
                            audio: audio_bytes.to_vec(),
                            config,
                            options,
                        },
                    )
                    .collect();

                durability.persist(
                    TranscribeManyInput {
                        requests: orig_requests_copy,
                    },
                    result,
                )
            } else {
                durability.replay()
            }
        }
    }

    impl<Impl: ExtendedSttProvider> LanguageProvider for DurableStt<Impl> {
        fn list_languages() -> Result<Vec<LanguageInfo>, SttError> {
            Impl::list_languages()
        }
    }

    #[derive(Debug, Clone, PartialEq, IntoValue, FromValueAndType)]
    struct TranscribeInput {
        request: TranscriptionRequest,
    }

    #[derive(Debug, Clone, PartialEq, IntoValue, FromValueAndType)]
    struct TranscribeManyInput {
        requests: Vec<TranscriptionRequest>,
    }

    impl From<&SttError> for SttError {
        fn from(error: &SttError) -> Self {
            error.clone()
        }
    }
}
