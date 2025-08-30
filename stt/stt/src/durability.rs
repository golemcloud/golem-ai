use std::marker::PhantomData;

use crate::exports::golem::stt::languages::Guest as WitLanguageGuest;
use crate::guest::SttTranscriptionGuest;

pub struct DurableStt<Impl> {
    phantom: PhantomData<Impl>,
}

pub trait ExtendedGuest: SttTranscriptionGuest + WitLanguageGuest + 'static {}

#[cfg(not(feature = "durability"))]
mod passthrough_impl {
    use bytes::Bytes;

    use crate::exports::golem::stt::languages::{
        Guest as WitLanguageGuest, LanguageInfo as WitLanguageInfo,
    };

    use crate::durability::{DurableStt, ExtendedGuest};
    use crate::exports::golem::stt::transcription::{
        Guest as WitTranscriptionGuest, MultiTranscriptionResult as WitMultiTranscriptionResult,
        TranscriptionRequest as WitTranscriptionRequest,
    };

    use crate::exports::golem::stt::types::{
        SttError as WitSttError, TranscriptionResult as WitTranscriptionResult,
    };

    use crate::guest::SttTranscriptionRequest;
    use crate::LOGGING_STATE;
    use golem_rust::{FromValueAndType, IntoValue};

    impl<Impl: ExtendedGuest> WitTranscriptionGuest for DurableStt<Impl> {
        fn transcribe(
            request: WitTranscriptionRequest,
        ) -> Result<WitTranscriptionResult, WitSttError> {
            LOGGING_STATE.with_borrow_mut(|state| state.init());

            let request = SttTranscriptionRequest {
                request_id: request.request_id,
                audio: Bytes::from(request.audio),
                config: request.config,
                options: request.options,
            };

            Impl::transcribe(request)
        }

        fn transcribe_many(
            requests: Vec<WitTranscriptionRequest>,
        ) -> Result<WitMultiTranscriptionResult, WitSttError> {
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

            Impl::transcribe_many(stt_requests)
        }
    }

    impl<Impl: ExtendedGuest> WitLanguageGuest for DurableStt<Impl> {
        fn list_languages() -> Result<Vec<WitLanguageInfo>, WitSttError> {
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

    impl From<&WitSttError> for WitSttError {
        fn from(error: &WitSttError) -> Self {
            error.clone()
        }
    }
}

#[cfg(feature = "durability")]
mod durable_impl {
    use bytes::Bytes;
    use golem_rust::bindings::golem::durability::durability::DurableFunctionType;
    use golem_rust::durability::Durability;

    use crate::exports::golem::stt::languages::{
        Guest as WitLanguageGuest, LanguageInfo as WitLanguageInfo,
    };

    use crate::durability::{DurableStt, ExtendedGuest};
    use crate::exports::golem::stt::transcription::{
        Guest as WitTranscriptionGuest, MultiTranscriptionResult as WitMultiTranscriptionResult,
        TranscriptionRequest as WitTranscriptionRequest,
    };

    use crate::exports::golem::stt::types::{
        SttError as WitSttError, TranscriptionResult as WitTranscriptionResult,
    };

    use crate::guest::SttTranscriptionRequest;
    use crate::LOGGING_STATE;
    use golem_rust::{with_persistence_level, FromValueAndType, IntoValue, PersistenceLevel};

    impl<Impl: ExtendedGuest> WitTranscriptionGuest for DurableStt<Impl> {
        fn transcribe(
            request: WitTranscriptionRequest,
        ) -> Result<WitTranscriptionResult, WitSttError> {
            LOGGING_STATE.with_borrow_mut(|state| state.init());
            let durability = Durability::<WitTranscriptionResult, WitSttError>::new(
                "golem_stt",
                "transcribe",
                DurableFunctionType::WriteRemote,
            );

            let audio_bytes = Bytes::from(request.audio);
            let request_id = request.request_id;
            let config = request.config;
            let options = request.options;

            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    let request = SttTranscriptionRequest {
                        request_id: request_id.clone(),
                        audio: audio_bytes.clone(),
                        config,
                        options: options.clone(),
                    };

                    Impl::transcribe(request)
                });

                // Reconstruct original request for persistence
                let orig_request_copy = WitTranscriptionRequest {
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

        fn transcribe_many(
            requests: Vec<WitTranscriptionRequest>,
        ) -> Result<WitMultiTranscriptionResult, WitSttError> {
            LOGGING_STATE.with_borrow_mut(|state| state.init());
            let durability = Durability::<WitMultiTranscriptionResult, WitSttError>::new(
                "golem_stt",
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
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
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

                    Impl::transcribe_many(stt_requests)
                });

                // Reconstruct original requests for persistence
                let orig_requests_copy: Vec<WitTranscriptionRequest> = requests_with_bytes
                    .into_iter()
                    .map(
                        |(audio_bytes, request_id, config, options)| WitTranscriptionRequest {
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

    impl<Impl: ExtendedGuest> WitLanguageGuest for DurableStt<Impl> {
        fn list_languages() -> Result<Vec<WitLanguageInfo>, WitSttError> {
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

    impl From<&WitSttError> for WitSttError {
        fn from(error: &WitSttError) -> Self {
            error.clone()
        }
    }

    #[cfg(test)]
    mod tests {
        use crate::durability::durable_impl::{TranscribeInput, TranscribeManyInput};
        use crate::exports::golem::stt::transcription::{
            DiarizationOptions, Phrase, TranscribeOptions,
            TranscriptionRequest as WitTranscriptionRequest, Vocabulary,
        };
        use crate::exports::golem::stt::types::{AudioConfig, AudioFormat};
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
        fn transcribe_input_roundtrip() {
            let input = TranscribeInput {
                request: WitTranscriptionRequest {
                    request_id: "req_12345".to_string(),
                    audio: vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
                    config: AudioConfig {
                        format: AudioFormat::Wav,
                        sample_rate: Some(44100),
                        channels: Some(2),
                    },
                    options: Some(TranscribeOptions {
                        language: Some("en-US".to_string()),
                        model: Some("whisper-large".to_string()),
                        profanity_filter: Some(true),
                        vocabulary: Some(Vocabulary {
                            phrases: vec![
                                Phrase {
                                    value: "Golem".to_string(),
                                    boost: Some(1.5),
                                },
                                Phrase {
                                    value: "transcription".to_string(),
                                    boost: Some(1.2),
                                },
                            ],
                        }),
                        diarization: Some(DiarizationOptions {
                            enabled: true,
                            min_speaker_count: Some(2),
                            max_speaker_count: Some(4),
                        }),
                        enable_multi_channel: Some(false),
                    }),
                },
            };
            roundtrip_test(input);
        }

        #[test]
        fn transcribe_input_minimal_roundtrip() {
            let input = TranscribeInput {
                request: WitTranscriptionRequest {
                    request_id: "req_minimal".to_string(),
                    audio: vec![255, 254, 253],
                    config: AudioConfig {
                        format: AudioFormat::Mp3,
                        sample_rate: None,
                        channels: None,
                    },
                    options: None,
                },
            };
            roundtrip_test(input);
        }

        #[test]
        fn transcribe_input_with_partial_options_roundtrip() {
            let input = TranscribeInput {
                request: WitTranscriptionRequest {
                    request_id: "req_partial".to_string(),
                    audio: vec![10, 20, 30, 40, 50],
                    config: AudioConfig {
                        format: AudioFormat::Flac,
                        sample_rate: Some(16000),
                        channels: Some(1),
                    },
                    options: Some(TranscribeOptions {
                        language: Some("es-ES".to_string()),
                        model: None,
                        profanity_filter: Some(false),
                        vocabulary: None,
                        diarization: None,
                        enable_multi_channel: Some(true),
                    }),
                },
            };
            roundtrip_test(input);
        }

        #[test]
        fn transcribe_many_input_roundtrip() {
            let input = TranscribeManyInput {
                requests: vec![
                    WitTranscriptionRequest {
                        request_id: "batch_req_1".to_string(),
                        audio: vec![1, 2, 3],
                        config: AudioConfig {
                            format: AudioFormat::Ogg,
                            sample_rate: Some(22050),
                            channels: Some(1),
                        },
                        options: Some(TranscribeOptions {
                            language: Some("fr-FR".to_string()),
                            model: Some("whisper-base".to_string()),
                            profanity_filter: None,
                            vocabulary: Some(Vocabulary {
                                phrases: vec![Phrase {
                                    value: "bonjour".to_string(),
                                    boost: Some(2.0),
                                }],
                            }),
                            diarization: Some(DiarizationOptions {
                                enabled: false,
                                min_speaker_count: None,
                                max_speaker_count: None,
                            }),
                            enable_multi_channel: None,
                        }),
                    },
                    WitTranscriptionRequest {
                        request_id: "batch_req_2".to_string(),
                        audio: vec![100, 101, 102, 103],
                        config: AudioConfig {
                            format: AudioFormat::Aac,
                            sample_rate: Some(48000),
                            channels: Some(2),
                        },
                        options: None,
                    },
                ],
            };
            roundtrip_test(input);
        }

        #[test]
        fn transcribe_many_input_empty_roundtrip() {
            let input = TranscribeManyInput { requests: vec![] };
            roundtrip_test(input);
        }

        #[test]
        fn transcribe_many_input_single_roundtrip() {
            let input = TranscribeManyInput {
                requests: vec![WitTranscriptionRequest {
                    request_id: "single_batch".to_string(),
                    audio: vec![42],
                    config: AudioConfig {
                        format: AudioFormat::Pcm,
                        sample_rate: Some(8000),
                        channels: Some(1),
                    },
                    options: Some(TranscribeOptions {
                        language: Some("de-DE".to_string()),
                        model: Some("whisper-tiny".to_string()),
                        profanity_filter: Some(true),
                        vocabulary: None,
                        diarization: Some(DiarizationOptions {
                            enabled: true,
                            min_speaker_count: Some(1),
                            max_speaker_count: Some(1),
                        }),
                        enable_multi_channel: Some(false),
                    }),
                }],
            };
            roundtrip_test(input);
        }
    }
}
