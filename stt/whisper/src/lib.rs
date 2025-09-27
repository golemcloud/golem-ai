use std::time::Duration;

use futures_concurrency::future::Join;
use golem_stt::durability::{DurableStt, ExtendedGuest};
use golem_stt::guest::{SttTranscriptionGuest, SttTranscriptionRequest};
use golem_stt::http::WstdHttpClient;
use golem_stt::transcription::SttProviderClient;
use golem_stt::LOGGING_STATE;
use itertools::Itertools;

use once_cell::sync::OnceCell;

use golem_stt::error::Error as SttError;
use golem_stt::golem::stt::types::{
    AudioFormat as WitAudioFormat, SttError as WitSttError, TimingInfo as WitTimingInfo,
    TranscriptionChannel as WitTranscriptionChannel,
    TranscriptionMetadata as WitTranscriptionMetadata,
    TranscriptionResult as WitTranscriptionResult, TranscriptionSegment as WitTranscriptionSegment,
    WordSegment as WitWordSegment,
};

use golem_stt::golem::stt::transcription::{
    FailedTranscription as WitFailedTranscription,
    MultiTranscriptionResult as WitMultiTranscriptionResult,
    TranscribeOptions as WitTranscribeOptions,
};

use golem_stt::golem::stt::languages::{Guest as LanguageGuest, LanguageInfo};
use transcription::{
    AudioConfig, AudioFormat, TranscriptionConfig, TranscriptionRequest, TranscriptionResponse,
    TranscriptionsApi,
};
use wstd::runtime::block_on;

mod transcription;

static API_CLIENT: OnceCell<TranscriptionsApi<WstdHttpClient>> = OnceCell::new();

#[allow(unused)]
struct SttComponent;

impl SttComponent {
    fn create_or_get_client() -> Result<&'static TranscriptionsApi<WstdHttpClient>, SttError> {
        API_CLIENT.get_or_try_init(|| {
            let api_key = std::env::var("OPENAI_API_KEY").map_err(|err| {
                SttError::EnvVariablesNotSet(format!("Failed to load OPENAI_API_KEY: {err}"))
            })?;

            let api_client = TranscriptionsApi::new(
                api_key,
                WstdHttpClient::new_with_timeout(Duration::from_secs(60), Duration::from_secs(600)),
            );

            Ok(api_client)
        })
    }
}

impl LanguageGuest for SttComponent {
    fn list_languages() -> Result<Vec<LanguageInfo>, WitSttError> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());

        let supported_languages = transcription::get_supported_languages();
        Ok(supported_languages
            .iter()
            .map(|lang| LanguageInfo {
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

            for chunk in requests.into_iter().chunks(16).into_iter() {
                let req_vec: Vec<_> = chunk.collect();

                let futures = req_vec
                    .into_iter()
                    .map(|request| api_client.transcribe_audio(request))
                    .collect::<Vec<_>>();

                let results = futures.join().await;

                for res in results {
                    match res {
                        Ok(resp) => successes.push(resp.into()),
                        Err(err) => failures.push(WitFailedTranscription {
                            request_id: err.request_id().to_string(),
                            error: WitSttError::from(err),
                        }),
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
        let prompt = options
            .vocabulary
            .map(|c| c.phrases.into_iter().map(|p| p.value).join(", "));

        if let Some(ref language_code) = options.language {
            if transcription::is_supported_language(language_code) {
                return Err(WitSttError::UnsupportedLanguage(language_code.to_owned()));
            }
        }

        Ok(TranscriptionConfig {
            language: options.language,
            prompt,
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
            },
            transcription_config,
        })
    }
}

impl From<TranscriptionResponse> for WitTranscriptionResult {
    fn from(response: TranscriptionResponse) -> Self {
        let transcription = response.whisper_transcription;

        let metadata = WitTranscriptionMetadata {
            duration_seconds: transcription.usage.seconds as f32,
            audio_size_bytes: response.audio_size_bytes as u32,
            request_id: response.request_id,
            model: Some("whisper-1".to_string()),
            language: transcription.language,
        };

        let wit_word_segments: Vec<_> = transcription
            .words
            .into_iter()
            .map(|word| WitWordSegment {
                text: word.word,
                timing_info: Some(WitTimingInfo {
                    start_time_seconds: word.start as f32,
                    end_time_seconds: word.end as f32,
                }),
                confidence: None,
                speaker_id: None,
            })
            .collect();

        let segment = WitTranscriptionSegment {
            transcript: transcription.text.clone(),
            timing_info: None,
            speaker_id: None,
            words: wit_word_segments,
        };

        let channel = WitTranscriptionChannel {
            id: "0".to_string(),
            transcript: transcription.text.clone(),
            segments: vec![segment],
        };

        WitTranscriptionResult {
            transcript_metadata: metadata,
            channels: vec![channel],
        }
    }
}

impl ExtendedGuest for SttComponent {}

type DurableDeepgramComponent = DurableStt<SttComponent>;

golem_stt::export_stt!(DurableDeepgramComponent with_types_in golem_stt);
