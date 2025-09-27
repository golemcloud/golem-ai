use golem_stt::durability::{DurableStt, ExtendedGuest};
use golem_stt::golem::stt::languages::{
    Guest as WitLanguageGuest, LanguageInfo as WitLanguageInfo,
};

use golem_stt::error::Error as SttError;

use golem_stt::golem::stt::transcription::{
    FailedTranscription as WitFailedTranscription,
    MultiTranscriptionResult as WitMultiTranscriptionResult,
    TranscribeOptions as WitTranscribeOptions,
};

use golem_stt::golem::stt::types::{
    AudioFormat as WitAudioFormat, SttError as WitSttError, TimingInfo as WitTimingInfo,
    TranscriptionChannel as WitTranscriptionChannel,
    TranscriptionMetadata as WitTranscriptionMetadata,
    TranscriptionResult as WitTranscriptionResult, TranscriptionSegment as WitTranscriptionSegment,
    WordSegment as WitWordSegment,
};
use golem_stt::guest::{SttTranscriptionGuest, SttTranscriptionRequest};
use golem_stt::http::WstdHttpClient;
use golem_stt::runtime::WasiAsyncRuntime;
use golem_stt::transcription::SttProviderClient;
use golem_stt::LOGGING_STATE;
use log::trace;
use transcription::api::{TranscribeApi, TranscriptionResponse};
use transcription::request::{
    AudioConfig, AudioFormat, DiarizationConfig, TranscriptionConfig, TranscriptionRequest,
};

use futures_concurrency::future::Join;
use itertools::Itertools;
use once_cell::sync::OnceCell;
use wstd::runtime::block_on;

use crate::transcription::{S3Client, TranscribeClient};

mod transcription;

static API_CLIENT: OnceCell<
    TranscribeApi<S3Client<WstdHttpClient>, TranscribeClient<WstdHttpClient, WasiAsyncRuntime>>,
> = OnceCell::new();

#[allow(unused)]
struct SttComponent;

impl SttComponent {
    fn create_or_get_client() -> Result<
        &'static TranscribeApi<
            S3Client<WstdHttpClient>,
            TranscribeClient<WstdHttpClient, WasiAsyncRuntime>,
        >,
        SttError,
    > {
        API_CLIENT.get_or_try_init(|| {
            let region = std::env::var("AWS_REGION").map_err(|err| {
                SttError::EnvVariablesNotSet(format!("Failed to load AWS_REGION: {err}"))
            })?;

            let access_key = std::env::var("AWS_ACCESS_KEY").map_err(|err| {
                SttError::EnvVariablesNotSet(format!("Failed to load AWS_ACCESS_KEY: {err}"))
            })?;

            let secret_key = std::env::var("AWS_SECRET_KEY").map_err(|err| {
                SttError::EnvVariablesNotSet(format!("Failed to load AWS_SECRET_KEY: {err}"))
            })?;

            let bucket_name = std::env::var("AWS_BUCKET_NAME").map_err(|err| {
                SttError::EnvVariablesNotSet(format!("Failed to load AWS_BUCKET_NAME: {err}"))
            })?;

            let api_client = TranscribeApi::live(bucket_name, access_key, secret_key, region);

            Ok(api_client)
        })
    }
}

impl WitLanguageGuest for SttComponent {
    fn list_languages() -> Result<Vec<WitLanguageInfo>, WitSttError> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());

        let supported_languages = transcription::api::get_supported_languages();
        Ok(supported_languages
            .iter()
            .map(|lang| WitLanguageInfo {
                code: lang.code.to_string(),
                name: lang.name.to_string(),
                native_name: lang.native_name.to_string(),
            })
            .collect())
    }
}

impl SttTranscriptionGuest for SttComponent {
    fn transcribe(req: SttTranscriptionRequest) -> Result<WitTranscriptionResult, WitSttError> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());

        block_on(async {
            let api_client = Self::create_or_get_client()?;

            let api_response = api_client.transcribe_audio(req.try_into()?).await?;

            Ok(api_response.into())
        })
    }

    fn transcribe_many(
        wit_requests: Vec<SttTranscriptionRequest>,
    ) -> Result<WitMultiTranscriptionResult, WitSttError> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());

        block_on(async {
            let api_client = Self::create_or_get_client()?;

            let mut successes: Vec<WitTranscriptionResult> = Vec::new();
            let mut failures: Vec<WitFailedTranscription> = Vec::new();

            let requests: Vec<_> = wit_requests
                .into_iter()
                .map(|wr| (wr.request_id.clone(), TranscriptionRequest::try_from(wr)))
                .filter_map(|(id, res)| match res {
                    Ok(req) => Some(req),
                    Err(err) => {
                        failures.push(WitFailedTranscription {
                            request_id: id,
                            error: err,
                        });
                        None
                    }
                })
                .collect();

            // Might need to enable this if https://github.com/golemcloud/golem/issues/1865 does not get fixed
            // for request in requests {
            //     let res = api_client.transcribe_audio(request).await; // returns a Result<TranscriptionResponse, TranscriptionError>
            //     match res {
            //         Ok(resp) => successes.push(resp.into()),
            //         Err(err) => {
            //             trace!("transcription request failed, error {err}");
            //             failures.push(WitFailedTranscription {
            //                 request_id: err.request_id().to_string(),
            //                 error: WitSttError::from(err),
            //             });
            //         }
            //     }
            // }

            for chunk in requests.into_iter().chunks(32).into_iter() {
                let req_vec: Vec<_> = chunk.collect();

                let futures = req_vec
                    .into_iter()
                    .map(|request| api_client.transcribe_audio(request))
                    .collect::<Vec<_>>();

                trace!("waiting for transcription jobs to complete");
                let results = futures.join().await;
                trace!("transcription job completed");

                for res in results {
                    match res {
                        Ok(resp) => successes.push(resp.into()),
                        Err(err) => {
                            trace!("transcription request failed, error {err}");
                            failures.push(WitFailedTranscription {
                                request_id: err.request_id().to_string(),
                                error: WitSttError::from(err),
                            })
                        }
                    }
                }
            }

            Ok(WitMultiTranscriptionResult {
                successes,
                failures,
            })
        })
    }
}

impl TryFrom<WitAudioFormat> for AudioFormat {
    type Error = WitSttError;

    fn try_from(wit_format: WitAudioFormat) -> Result<Self, Self::Error> {
        match wit_format {
            WitAudioFormat::Wav => Ok(AudioFormat::Wav),
            WitAudioFormat::Mp3 => Ok(AudioFormat::Mp3),
            WitAudioFormat::Flac => Ok(AudioFormat::Flac),
            WitAudioFormat::Ogg => Ok(AudioFormat::Ogg),
            format => Err(WitSttError::UnsupportedFormat(format!(
                "{format:?}is not supported"
            ))),
        }
    }
}

impl TryFrom<WitTranscribeOptions> for TranscriptionConfig {
    type Error = WitSttError;

    fn try_from(options: WitTranscribeOptions) -> Result<Self, Self::Error> {
        if let Some(language_code) = &options.language {
            if !transcription::api::is_supported_language(language_code) {
                return Err(WitSttError::UnsupportedLanguage(language_code.clone()));
            }
        }

        let vocabulary: Vec<_> = options
            .vocabulary
            .map(|vocab| {
                vocab
                    .phrases
                    .into_iter()
                    .map(|phrase| phrase.value)
                    .collect()
            })
            .unwrap_or_default();

        let diarization_config = options.diarization.map(|dc| DiarizationConfig {
            enabled: dc.enabled,
            max_speakers: dc.max_speaker_count.unwrap_or(30) as u8,
        });

        let enable_multi_channel = options.enable_multi_channel.unwrap_or(false);

        Ok(TranscriptionConfig {
            language: options.language,
            model: options.model,
            diarization: diarization_config,
            enable_multi_channel,
            vocabulary,
        })
    }
}

impl TryFrom<SttTranscriptionRequest> for TranscriptionRequest {
    type Error = WitSttError;

    fn try_from(request: SttTranscriptionRequest) -> Result<Self, Self::Error> {
        let transcription_config: Option<TranscriptionConfig> =
            if let Some(options) = request.options {
                Some(options.try_into()?)
            } else {
                None
            };

        Ok(TranscriptionRequest {
            request_id: request.request_id,
            audio: request.audio,
            audio_config: AudioConfig {
                format: request.config.format.try_into()?,
                channels: request.config.channels,
            },
            transcription_config,
        })
    }
}

impl From<TranscriptionResponse> for WitTranscriptionResult {
    fn from(response: TranscriptionResponse) -> Self {
        let aws_results = &response.aws_transcription.results;

        // AWS Transcription result does not contain duration information, so we calculate it from the last item's end time
        let duration_seconds = aws_results
            .items
            .last()
            .and_then(|item| item.end_time.as_ref())
            .and_then(|time_str| time_str.parse::<f32>().ok())
            .unwrap_or(0.0);

        let metadata = WitTranscriptionMetadata {
            duration_seconds,
            audio_size_bytes: response.audio_size_bytes as u32,
            request_id: response.request_id,
            model: response.model,
            language: response.language,
        };

        let channels = if let Some(ref channel_labels) = aws_results.channel_labels {
            channel_labels
                .channels
                .iter()
                .map(|channel| channel.channel_label.clone())
                .collect()
        } else {
            vec!["ch_0".to_string()]
        };

        let wit_channels: Vec<_> = channels
            .into_iter()
            .map(|channel_id| {
                let channel_segments: Vec<&_> = aws_results
                    .audio_segments
                    .iter()
                    .filter(|segment| {
                        segment
                            .channel_label
                            .as_ref()
                            .unwrap_or(&"ch_0".to_string())
                            == &channel_id
                    })
                    .collect();

                // Create channel transcript by unioning all segment transcripts
                let channel_transcript = channel_segments
                    .iter()
                    .map(|segment| segment.transcript.as_str())
                    .collect::<Vec<_>>()
                    .join(" ");

                let wit_segments: Vec<_> = channel_segments
                    .into_iter()
                    .map(|segment| {
                        // Look up words from items using segment's item IDs
                        let wit_words: Vec<WitWordSegment> = segment
                            .items
                            .iter()
                            .filter_map(|item_id| {
                                // Find the item with this ID
                                aws_results
                                    .items
                                    .iter()
                                    .find(|item| item.id == Some(*item_id))
                            })
                            .filter(|item| item.item_type == "pronunciation") // Only pronunciation items, not punctuation
                            .filter_map(|item| {
                                // Get the first (best) alternative
                                let alternative = item.alternatives.first()?;

                                // Parse confidence from string
                                let confidence = alternative.confidence.parse::<f32>().ok();

                                // Create timing info if available
                                let timing_info = match (&item.start_time, &item.end_time) {
                                    (Some(start_str), Some(end_str)) => {
                                        match (start_str.parse::<f32>(), end_str.parse::<f32>()) {
                                            (Ok(start), Ok(end)) => Some(WitTimingInfo {
                                                start_time_seconds: start,
                                                end_time_seconds: end,
                                            }),
                                            _ => None,
                                        }
                                    }
                                    _ => None,
                                };

                                Some(WitWordSegment {
                                    text: alternative.content.clone(),
                                    timing_info,
                                    confidence,
                                    speaker_id: item.speaker_label.clone(),
                                })
                            })
                            .collect();

                        WitTranscriptionSegment {
                            transcript: segment.transcript.clone(),
                            timing_info: Some(WitTimingInfo {
                                start_time_seconds: segment
                                    .start_time
                                    .parse::<f32>()
                                    .unwrap_or(0.0),
                                end_time_seconds: segment.end_time.parse::<f32>().unwrap_or(0.0),
                            }),
                            speaker_id: segment.speaker_label.clone(),
                            words: wit_words,
                        }
                    })
                    .collect();

                WitTranscriptionChannel {
                    id: channel_id,
                    transcript: channel_transcript,
                    segments: wit_segments,
                }
            })
            .collect();

        WitTranscriptionResult {
            transcript_metadata: metadata,
            channels: wit_channels,
        }
    }
}

impl ExtendedGuest for SttComponent {}

type DurableDeepgramComponent = DurableStt<SttComponent>;

golem_stt::export_stt!(DurableDeepgramComponent with_types_in golem_stt);
