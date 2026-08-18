use std::marker::PhantomData;

use crate::guest::SttTranscriptionProvider;
use crate::LanguageProvider;

pub struct DurableStt<Impl> {
    phantom: PhantomData<Impl>,
}

pub trait ExtendedSttProvider: SttTranscriptionProvider + LanguageProvider + 'static {}

/// When the `golem` feature flag is off, `DurableStt<Impl>` is a transparent wrapper that
/// forwards every call to the inner provider without any oplog persistence.
#[cfg(not(feature = "golem"))]
mod passthrough_impl {
    use bytes::Bytes;

    use crate::durability::{DurableStt, ExtendedSttProvider};
    use crate::guest::{SttTranscriptionProvider, SttTranscriptionRequest};
    use crate::model::languages::LanguageInfo;
    use crate::model::transcription::{
        MultiTranscriptionResult, TranscriptionRequest, TranscriptionResult,
    };
    use crate::model::types::SttError;
    use crate::{LanguageProvider, TranscriptionProvider, LOGGING_STATE};

    impl<Impl: ExtendedSttProvider> TranscriptionProvider for DurableStt<Impl> {
        type ProviderConfig = <Impl as SttTranscriptionProvider>::ProviderConfig;

        async fn transcribe(
            provider_config: Self::ProviderConfig,
            request: TranscriptionRequest,
        ) -> Result<TranscriptionResult, SttError> {
            LOGGING_STATE.with_borrow_mut(|state| state.init());

            let request = SttTranscriptionRequest {
                request_id: request.request_id,
                audio: Bytes::from(request.audio),
                config: request.config,
                options: request.options,
            };

            Impl::transcribe(provider_config, request).await
        }

        async fn transcribe_many(
            provider_config: Self::ProviderConfig,
            requests: Vec<TranscriptionRequest>,
        ) -> Result<MultiTranscriptionResult, SttError> {
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

            Impl::transcribe_many(provider_config, stt_requests).await
        }
    }

    impl<Impl: ExtendedSttProvider> LanguageProvider for DurableStt<Impl> {
        fn list_languages() -> Result<Vec<LanguageInfo>, SttError> {
            Impl::list_languages()
        }
    }
}

#[cfg(feature = "golem")]
mod durable_impl {
    use bytes::Bytes;
    use golem_rust::durability::{Durability, DurableFunctionType};

    use crate::durability::{DurableStt, ExtendedSttProvider};

    use crate::guest::{SttTranscriptionProvider, SttTranscriptionRequest};
    use crate::model::languages::LanguageInfo;
    use crate::model::transcription::{
        MultiTranscriptionResult, TranscriptionRequest, TranscriptionResult,
    };
    use crate::model::types::SttError;
    use crate::{LanguageProvider, TranscriptionProvider, LOGGING_STATE};
    use golem_rust::{FromSchema, IntoSchema};

    impl<Impl: ExtendedSttProvider> TranscriptionProvider for DurableStt<Impl> {
        type ProviderConfig = <Impl as SttTranscriptionProvider>::ProviderConfig;

        async fn transcribe(
            provider_config: Self::ProviderConfig,
            request: TranscriptionRequest,
        ) -> Result<TranscriptionResult, SttError> {
            LOGGING_STATE.with_borrow_mut(|state| state.init());
            let input = TranscribeInput {
                request: request.clone(),
            };
            let durability = Durability::<TranscriptionResult, SttError>::new(
                "golem_ai_stt",
                "transcribe",
                DurableFunctionType::WriteRemote,
                &input,
            );
            // NOTE: `provider_config` deliberately not included in the persisted input,
            // because it can carry secrets (API keys etc.).
            durability
                .run_async(|| {
                    let request = SttTranscriptionRequest {
                        request_id: request.request_id,
                        audio: Bytes::from(request.audio),
                        config: request.config,
                        options: request.options,
                    };
                    Impl::transcribe(provider_config, request)
                })
                .await
        }

        async fn transcribe_many(
            provider_config: Self::ProviderConfig,
            requests: Vec<TranscriptionRequest>,
        ) -> Result<MultiTranscriptionResult, SttError> {
            LOGGING_STATE.with_borrow_mut(|state| state.init());
            let input = TranscribeManyInput {
                requests: requests.clone(),
            };
            let durability = Durability::<MultiTranscriptionResult, SttError>::new(
                "golem_ai_stt",
                "transcribe_many",
                DurableFunctionType::WriteRemote,
                &input,
            );

            // NOTE: `provider_config` deliberately not included in the persisted input,
            // because it can carry secrets (API keys etc.).
            durability
                .run_async(|| {
                    let stt_requests: Vec<SttTranscriptionRequest> = requests
                        .into_iter()
                        .map(|request| SttTranscriptionRequest {
                            request_id: request.request_id,
                            audio: Bytes::from(request.audio),
                            config: request.config,
                            options: request.options,
                        })
                        .collect();
                    Impl::transcribe_many(provider_config, stt_requests)
                })
                .await
        }
    }

    impl<Impl: ExtendedSttProvider> LanguageProvider for DurableStt<Impl> {
        fn list_languages() -> Result<Vec<LanguageInfo>, SttError> {
            Impl::list_languages()
        }
    }

    #[derive(Debug, Clone, PartialEq, IntoSchema, FromSchema)]
    struct TranscribeInput {
        request: TranscriptionRequest,
    }

    #[derive(Debug, Clone, PartialEq, IntoSchema, FromSchema)]
    struct TranscribeManyInput {
        requests: Vec<TranscriptionRequest>,
    }

    impl From<&SttError> for SttError {
        fn from(error: &SttError) -> Self {
            error.clone()
        }
    }
}
