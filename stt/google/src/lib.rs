use golem_stt::durability::{DurableStt, ExtendedGuest};
use golem_stt::guest::{SttTranscriptionGuest, SttTranscriptionRequest};
use once_cell::sync::OnceCell;

use golem_stt::error::Error as SttError;
use golem_stt::http::WstdHttpClient;
use golem_stt::runtime::WasiAsyncRuntime;
use golem_stt::transcription::SttProviderClient;
use golem_stt::LOGGING_STATE;

use log::trace;
use transcription::api::{SpeechToTextApi, TranscriptionResponse};
use transcription::request::{
    AudioConfig, AudioFormat, DiarizationConfig, Phrase, TranscriptionConfig, TranscriptionRequest,
};

use golem_stt::golem::stt::languages::{
    Guest as WitLanguageGuest, LanguageInfo as WitLanguageInfo,
};

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

use futures_concurrency::future::Join;
use itertools::Itertools;
use wstd::runtime::block_on;

use crate::transcription::{CloudStorageClient, ServiceAccountKey, SpeechToTextClient};

mod transcription;

static API_CLIENT: OnceCell<
    SpeechToTextApi<
        CloudStorageClient<WstdHttpClient>,
        SpeechToTextClient<WstdHttpClient, WasiAsyncRuntime>,
    >,
> = OnceCell::new();

#[allow(unused)]
struct SttComponent;

impl SttComponent {
    fn create_or_get_client() -> Result<
        &'static SpeechToTextApi<
            CloudStorageClient<WstdHttpClient>,
            SpeechToTextClient<WstdHttpClient, WasiAsyncRuntime>,
        >,
        SttError,
    > {
        API_CLIENT.get_or_try_init(|| {
            let location = std::env::var("GOOGLE_LOCATION").map_err(|err| {
                SttError::EnvVariablesNotSet(format!("Failed to load GOOGLE_LOCATION: {err}"))
            })?;

            let bucket_name = std::env::var("GOOGLE_BUCKET_NAME").map_err(|err| {
                SttError::EnvVariablesNotSet(format!("Failed to load GOOGLE_BUCKET_NAME: {err}"))
            })?;

            let service_acc_key = if let Ok(creds_json_file) =
                std::env::var("GOOGLE_APPLICATION_CREDENTIALS")
            {
                let bytes = read_file_to_bytes(&creds_json_file).map_err(|err| {
                    SttError::AuthError(format!("Failed to read Google credentials file: {err}"))
                })?;
                let service_acc_key: ServiceAccountKey =
                    serde_json::from_slice(&bytes).map_err(|err| {
                        SttError::AuthError(format!("Failed to parse Google credentials: {err}"))
                    })?;
                service_acc_key
            } else {
                let project_id = std::env::var("GOOGLE_PROJECT_ID").map_err(|err| {
                    SttError::EnvVariablesNotSet(format!("Failed to load GOOGLE_PROJECT_ID: {err}"))
                })?;

                let client_email = std::env::var("GOOGLE_CLIENT_EMAIL").map_err(|err| {
                    SttError::EnvVariablesNotSet(format!(
                        "Failed to load GOOGLE_CLIENT_EMAIL: {err}"
                    ))
                })?;

                let private_key = std::env::var("GOOGLE_PRIVATE_KEY").map_err(|err| {
                    SttError::EnvVariablesNotSet(format!(
                        "Failed to load GOOGLE_PRIVATE_KEY: {err}"
                    ))
                })?;

                ServiceAccountKey::new(project_id, client_email, private_key)
            };

            let api_client = SpeechToTextApi::live(bucket_name, service_acc_key, location)?;

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

            api_response.try_into()
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
                    Err(error) => {
                        failures.push(WitFailedTranscription {
                            request_id: id,
                            error,
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
            //             trace!("transcription request failed, error {}", err);
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
                        Ok(resp) => {
                            let request_id = resp.request_id.clone();
                            match resp.try_into() {
                                Ok(transcription) => successes.push(transcription),
                                Err(error) => {
                                    trace!("transcription request parsing failed, error {error}");
                                    failures.push(WitFailedTranscription { request_id, error })
                                }
                            }
                        }
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

fn read_file_to_bytes(path: &str) -> std::io::Result<Vec<u8>> {
    use std::fs;
    use std::io::Read;

    let mut file = fs::File::open(path)?;
    let metadata = file.metadata()?;
    let file_size = metadata.len() as usize;

    let mut buffer = Vec::with_capacity(file_size);
    file.read_to_end(&mut buffer)?;

    Ok(buffer)
}

impl TryFrom<WitAudioFormat> for AudioFormat {
    type Error = WitSttError;

    fn try_from(wit_format: WitAudioFormat) -> Result<Self, Self::Error> {
        match wit_format {
            WitAudioFormat::Wav => Ok(AudioFormat::Wav),
            WitAudioFormat::Mp3 => Ok(AudioFormat::Mp3),
            WitAudioFormat::Flac => Ok(AudioFormat::Flac),
            WitAudioFormat::Ogg => Ok(AudioFormat::OggOpus),
            format => Err(WitSttError::UnsupportedFormat(format!(
                "{format:?} is not supported by Google Speech-to-Text"
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

        let language_codes = options.language.map(|lang| vec![lang]);

        let phrases: Vec<_> = options
            .vocabulary
            .map(|vocab| {
                vocab
                    .phrases
                    .into_iter()
                    .map(|phrase| Phrase {
                        value: phrase.value,
                        boost: phrase.boost,
                    })
                    .collect()
            })
            .unwrap_or_default();

        let diarization_config = options.diarization.map(|dc| DiarizationConfig {
            enabled: dc.enabled,
            min_speaker_count: dc.min_speaker_count.map(|count| count as i32).or(Some(1)), // set default value to 1
            max_speaker_count: dc.max_speaker_count.map(|count| count as i32),
        });

        let enable_multi_channel = options.enable_multi_channel.unwrap_or(false);
        let enable_profanity_filter = options.profanity_filter.unwrap_or(false);

        Ok(TranscriptionConfig {
            language_codes,
            model: options.model,
            enable_profanity_filter,
            diarization: diarization_config,
            enable_multi_channel,
            phrases,
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
                sample_rate_hertz: request.config.sample_rate,
                channels: request.config.channels,
            },
            transcription_config,
        })
    }
}

impl TryFrom<TranscriptionResponse> for WitTranscriptionResult {
    type Error = WitSttError;

    fn try_from(response: TranscriptionResponse) -> Result<Self, Self::Error> {
        let gcp_results = &response.gcp_transcription.results;

        fn parse_google_duration(duration_str: &str) -> Result<f32, WitSttError> {
            duration_str
                .trim_end_matches('s')
                .parse::<f32>()
                .map_err(|_| {
                    WitSttError::InternalError(format!(
                        "Failed to parse duration for {duration_str}",
                    ))
                })
        }

        // Calculate duration from the metadata or fallback to last word's end time
        let duration_seconds = if let Some(metadata) = response.gcp_transcription.metadata.as_ref()
        {
            if let Some(duration_str) = metadata.total_billed_duration.as_ref() {
                parse_google_duration(duration_str)?
            } else {
                gcp_results
                    .iter()
                    .filter_map(|result| result.result_end_offset.as_ref())
                    .next_back()
                    .map(|offset| parse_google_duration(offset))
                    .transpose()?
                    .unwrap_or(0.0)
            }
        } else {
            gcp_results
                .iter()
                .filter_map(|result| result.result_end_offset.as_ref())
                .next_back()
                .map(|offset| parse_google_duration(offset))
                .transpose()?
                .unwrap_or(0.0)
        };

        let metadata = WitTranscriptionMetadata {
            duration_seconds,
            audio_size_bytes: response.audio_size_bytes as u32,
            request_id: response.request_id,
            model: response.model,
            language: response.language,
        };

        let unique_channels: Vec<i32> = gcp_results
            .iter()
            .map(|result| result.channel_tag.unwrap_or(0))
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        let wit_channels: Result<Vec<_>, WitSttError> = unique_channels
            .into_iter()
            .map(
                |channel_tag| -> Result<WitTranscriptionChannel, WitSttError> {
                    let channel_results: Vec<_> = gcp_results
                        .iter()
                        .filter(|result| result.channel_tag.unwrap_or(0) == channel_tag)
                        .collect();

                    let channel_transcript = channel_results
                        .iter()
                        .filter_map(|result| {
                            result
                                .alternatives
                                .first()
                                .map(|alt| alt.transcript.as_str())
                        })
                        .collect::<Vec<_>>()
                        .join(" ");

                    // Convert each result to a transcription segment using the first alternative
                    let wit_segments: Result<Vec<_>, WitSttError> = channel_results
                        .into_iter()
                        .filter_map(|result| {
                            result.alternatives.first().map(|alternative| {
                                let timing_info = None;

                                let wit_words: Result<Vec<_>, WitSttError> = alternative
                                    .words
                                    .iter()
                                    .map(|word| -> Result<WitWordSegment, WitSttError> {
                                        let word_timing =
                                            match (&word.start_offset, &word.end_offset) {
                                                (Some(start), Some(end)) => Some(WitTimingInfo {
                                                    start_time_seconds: parse_google_duration(
                                                        start,
                                                    )?,
                                                    end_time_seconds: parse_google_duration(end)?,
                                                }),
                                                _ => None,
                                            };

                                        Ok(WitWordSegment {
                                            text: word.word.clone(),
                                            timing_info: word_timing,
                                            confidence: word.confidence,
                                            speaker_id: word.speaker_label.clone(),
                                        })
                                    })
                                    .collect();

                                Ok(WitTranscriptionSegment {
                                    transcript: alternative.transcript.clone(),
                                    timing_info,
                                    speaker_id: alternative
                                        .words
                                        .first()
                                        .and_then(|w| w.speaker_label.clone()),
                                    words: wit_words?,
                                })
                            })
                        })
                        .collect();

                    Ok(WitTranscriptionChannel {
                        id: channel_tag.to_string(),
                        transcript: channel_transcript,
                        segments: wit_segments?,
                    })
                },
            )
            .collect();

        Ok(WitTranscriptionResult {
            transcript_metadata: metadata,
            channels: wit_channels?,
        })
    }
}

impl ExtendedGuest for SttComponent {}

type DurableDeepgramComponent = DurableStt<SttComponent>;

golem_stt::export_stt!(DurableDeepgramComponent with_types_in golem_stt);
