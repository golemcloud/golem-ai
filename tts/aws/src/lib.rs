use crate::client::{AwsPollyTtsApi, Voice as AwsVoice};
use crate::conversions::{
    audio_data_to_synthesis_result, aws_voice_to_voice_info, combine_audio_chunks,
    get_polly_language_info, polly_format_to_audio_format, split_text_intelligently,
    synthesis_options_to_polly_params, validate_polly_input, validate_synthesis_input,
    voice_filter_to_describe_params,
};
use golem_rust::wasm_rpc::Pollable;
use golem_tts::config::with_config_key;
use golem_tts::durability::{DurableTts, ExtendedGuest};
use golem_tts::golem::tts::advanced::{
    AudioSample, Guest as AdvancedGuest, GuestLongFormOperation, GuestPronunciationLexicon,
    LongFormOperation, LongFormResult, OperationStatus, PronunciationEntry, PronunciationLexicon,
    VoiceDesignParams,
};
use golem_tts::golem::tts::streaming::{
    Guest as StreamingGuest, GuestSynthesisStream, GuestVoiceConversionStream, StreamStatus,
    SynthesisStream, VoiceConversionStream,
};
use golem_tts::golem::tts::synthesis::{
    Guest as SynthesisGuest, SynthesisOptions, ValidationResult,
};
use golem_tts::golem::tts::types::{
    AudioChunk, AudioFormat, LanguageCode, SynthesisResult, TextInput, TimingInfo, TimingMarkType,
    TtsError, VoiceGender, VoiceQuality, VoiceSettings,
};
use golem_tts::golem::tts::voices::{
    Guest as VoicesGuest, GuestVoice, GuestVoiceResults, LanguageInfo, Voice, VoiceFilter,
    VoiceInfo, VoiceResults,
};
use log::trace;
use std::cell::{Cell, RefCell};

mod client;
mod conversions;

struct PollyVoiceImpl {
    voice_data: AwsVoice,
    client: AwsPollyTtsApi,
}

impl PollyVoiceImpl {
    fn new(voice_data: AwsVoice, client: AwsPollyTtsApi) -> Self {
        Self { voice_data, client }
    }
}

impl GuestVoice for PollyVoiceImpl {
    fn get_id(&self) -> String {
        self.voice_data.id.clone()
    }

    fn get_name(&self) -> String {
        self.voice_data.name.clone()
    }

    fn get_provider_id(&self) -> Option<String> {
        Some("aws-polly".to_string())
    }

    fn get_language(&self) -> LanguageCode {
        self.voice_data.language_code.clone()
    }

    fn get_additional_languages(&self) -> Vec<LanguageCode> {
        self.voice_data
            .additional_language_codes
            .clone()
            .unwrap_or_default()
    }

    fn get_gender(&self) -> VoiceGender {
        match self.voice_data.gender.to_lowercase().as_str() {
            "male" => VoiceGender::Male,
            "female" => VoiceGender::Female,
            _ => VoiceGender::Neutral,
        }
    }

    fn get_quality(&self) -> VoiceQuality {
        if self
            .voice_data
            .supported_engines
            .contains(&"neural".to_string())
        {
            VoiceQuality::Neural
        } else if self
            .voice_data
            .supported_engines
            .contains(&"generative".to_string())
        {
            VoiceQuality::Studio
        } else {
            VoiceQuality::Standard
        }
    }

    fn get_description(&self) -> Option<String> {
        Some(format!(
            "{} voice from AWS Polly",
            self.voice_data.language_name
        ))
    }

    fn supports_ssml(&self) -> bool {
        true
    }

    fn get_sample_rates(&self) -> Vec<u32> {
        vec![8000, 16000, 22050]
    }

    fn get_supported_formats(&self) -> Vec<AudioFormat> {
        vec![AudioFormat::Mp3, AudioFormat::Pcm, AudioFormat::OggOpus]
    }

    fn update_settings(&self, _settings: VoiceSettings) -> Result<(), TtsError> {
        Err(TtsError::UnsupportedOperation(
            "Voice settings cannot be permanently updated in AWS Polly".to_string(),
        ))
    }

    fn delete(&self) -> Result<(), TtsError> {
        Err(TtsError::UnsupportedOperation(
            "AWS Polly voices cannot be deleted".to_string(),
        ))
    }

    fn clone(&self) -> Result<Voice, TtsError> {
        Err(TtsError::UnsupportedOperation(
            "Voice cloning not supported for AWS Polly voices".to_string(),
        ))
    }

    fn preview(&self, text: String) -> Result<Vec<u8>, TtsError> {
        let preview_text = if text.len() > 100 {
            format!("{}...", &text[..97])
        } else {
            text
        };

        let params =
            synthesis_options_to_polly_params(None, self.voice_data.id.clone(), preview_text);

        self.client.synthesize_speech(params)
    }
}

struct PollyVoiceResults {
    voices: RefCell<Vec<VoiceInfo>>,
    current_index: Cell<usize>,
    has_more: Cell<bool>,
    total_count: Option<u32>,
    next_token: RefCell<Option<String>>,
    client: AwsPollyTtsApi,
    filter: RefCell<Option<VoiceFilter>>,
}

impl PollyVoiceResults {
    fn new(
        voices: Vec<VoiceInfo>,
        total_count: Option<u32>,
        next_token: Option<String>,
        client: AwsPollyTtsApi,
        filter: Option<VoiceFilter>,
    ) -> Self {
        let has_voices = !voices.is_empty();
        Self {
            voices: RefCell::new(voices),
            current_index: Cell::new(0),
            has_more: Cell::new(has_voices || next_token.is_some()),
            total_count,
            next_token: RefCell::new(next_token),
            client,
            filter: RefCell::new(filter),
        }
    }
}

impl GuestVoiceResults for PollyVoiceResults {
    fn has_more(&self) -> bool {
        self.has_more.get()
    }

    fn get_next(&self) -> Result<Vec<VoiceInfo>, TtsError> {
        let voices = self.voices.borrow();
        let start_index = self.current_index.get();

        if start_index < voices.len() {
            let batch_size = 10;
            let end_index = std::cmp::min(start_index + batch_size, voices.len());

            trace!(
                "PollyVoiceResults::get_next - start: {}, end: {}, total: {}",
                start_index,
                end_index,
                voices.len()
            );

            let batch = voices[start_index..end_index].to_vec();
            trace!("Returning batch of {} voices", batch.len());

            self.current_index.set(end_index);

            let has_more_local = end_index < voices.len();
            let has_more_remote = self.next_token.borrow().is_some();
            self.has_more.set(has_more_local || has_more_remote);

            return Ok(batch);
        }

        if let Some(token) = self.next_token.borrow().clone() {
            drop(voices);

            let mut params = voice_filter_to_describe_params(self.filter.borrow().clone());
            if let Some(ref mut p) = params {
                p.next_token = Some(token);
            } else {
                params = Some(crate::client::DescribeVoicesParams {
                    engine: None,
                    language_code: None,
                    include_additional_language_codes: Some(true),
                    next_token: Some(token),
                });
            }

            let response = self.client.describe_voices(params)?;
            let new_voice_infos: Vec<VoiceInfo> = response
                .voices
                .into_iter()
                .map(aws_voice_to_voice_info)
                .collect();

            *self.next_token.borrow_mut() = response.next_token;

            self.voices.borrow_mut().extend(new_voice_infos);

            let batch_size = 10;
            let new_voices = self.voices.borrow();
            let end_index = std::cmp::min(start_index + batch_size, new_voices.len());

            if start_index < new_voices.len() {
                let batch = new_voices[start_index..end_index].to_vec();
                self.current_index.set(end_index);

                let has_more_local = end_index < new_voices.len();
                let has_more_remote = self.next_token.borrow().is_some();
                self.has_more.set(has_more_local || has_more_remote);

                return Ok(batch);
            }
        }

        trace!("No more voices to return");
        self.has_more.set(false);
        Ok(Vec::new())
    }

    fn get_total_count(&self) -> Option<u32> {
        self.total_count
    }
}
struct PollySynthesisStream {
    voice_id: String,
    client: AwsPollyTtsApi,
    text_buffer: RefCell<String>,
    finished: Cell<bool>,
    audio_queue: RefCell<Vec<AudioChunk>>,
    sequence_number: Cell<u32>,
    synthesis_options: RefCell<Option<SynthesisOptions>>,
}

impl PollySynthesisStream {
    fn new(voice_id: String, client: AwsPollyTtsApi, options: Option<SynthesisOptions>) -> Self {
        Self {
            voice_id,
            client,
            text_buffer: RefCell::new(String::new()),
            finished: Cell::new(false),
            audio_queue: RefCell::new(Vec::new()),
            sequence_number: Cell::new(0),
            synthesis_options: RefCell::new(options),
        }
    }
}

impl GuestSynthesisStream for PollySynthesisStream {
    fn send_text(&self, input: TextInput) -> Result<(), TtsError> {
        if self.finished.get() {
            return Err(TtsError::InvalidConfiguration(
                "Stream already finished".to_string(),
            ));
        }

        let mut buffer = self.text_buffer.borrow_mut();
        buffer.push_str(&input.content);

        if buffer.len() > 1000
            || buffer.ends_with('.')
            || buffer.ends_with('!')
            || buffer.ends_with('?')
        {
            let text_to_process = buffer.clone();
            buffer.clear();

            let params = synthesis_options_to_polly_params(
                self.synthesis_options.borrow().clone(),
                self.voice_id.clone(),
                text_to_process,
            );

            let audio_data = self.client.synthesize_speech(params)?;
            let sequence = self.sequence_number.get();
            self.sequence_number.set(sequence + 1);

            let chunk = AudioChunk {
                data: audio_data,
                sequence_number: sequence,
                is_final: false,
                timing_info: None,
            };

            self.audio_queue.borrow_mut().push(chunk);
        }

        Ok(())
    }

    fn finish(&self) -> Result<(), TtsError> {
        if self.finished.get() {
            return Ok(());
        }

        let remaining_text = {
            let mut buffer = self.text_buffer.borrow_mut();
            let text = buffer.clone();
            buffer.clear();
            text
        };

        if !remaining_text.is_empty() {
            let params = synthesis_options_to_polly_params(
                self.synthesis_options.borrow().clone(),
                self.voice_id.clone(),
                remaining_text,
            );

            let audio_data = self.client.synthesize_speech(params)?;
            let sequence = self.sequence_number.get();
            self.sequence_number.set(sequence + 1);

            let chunk = AudioChunk {
                data: audio_data,
                sequence_number: sequence,
                is_final: true,
                timing_info: None,
            };

            self.audio_queue.borrow_mut().push(chunk);
        }

        self.finished.set(true);
        Ok(())
    }

    fn receive_chunk(&self) -> Result<Option<AudioChunk>, TtsError> {
        let mut queue = self.audio_queue.borrow_mut();
        Ok(queue.pop())
    }

    fn has_pending_audio(&self) -> bool {
        !self.audio_queue.borrow().is_empty()
    }

    fn get_status(&self) -> StreamStatus {
        if self.finished.get() && self.audio_queue.borrow().is_empty() {
            StreamStatus::Finished
        } else if self.finished.get() {
            StreamStatus::Processing
        } else {
            StreamStatus::Ready
        }
    }

    fn close(&self) {
        self.finished.set(true);
        self.audio_queue.borrow_mut().clear();
        self.text_buffer.borrow_mut().clear();
    }
}

struct PollyVoiceConversionStream {
    #[allow(dead_code)]
    voice_id: String,
    finished: Cell<bool>,
}

impl PollyVoiceConversionStream {
    fn new(voice_id: String, _client: AwsPollyTtsApi) -> Self {
        Self {
            voice_id,
            finished: Cell::new(false),
        }
    }
}

impl GuestVoiceConversionStream for PollyVoiceConversionStream {
    fn send_audio(&self, _audio_data: Vec<u8>) -> Result<(), TtsError> {
        Err(TtsError::UnsupportedOperation(
            "Voice conversion not supported by AWS Polly".to_string(),
        ))
    }

    fn receive_converted(&self) -> Result<Option<AudioChunk>, TtsError> {
        Err(TtsError::UnsupportedOperation(
            "Voice conversion not supported by AWS Polly".to_string(),
        ))
    }

    fn finish(&self) -> Result<(), TtsError> {
        self.finished.set(true);
        Ok(())
    }

    fn close(&self) {
        self.finished.set(true);
    }
}

struct PollyPronunciationLexicon {
    name: String,
    language: LanguageCode,
    entries: RefCell<Vec<PronunciationEntry>>,
    client: AwsPollyTtsApi,
}

impl PollyPronunciationLexicon {
    fn new(
        name: String,
        language: LanguageCode,
        entries: Option<Vec<PronunciationEntry>>,
        client: AwsPollyTtsApi,
    ) -> Self {
        Self {
            name,
            language,
            entries: RefCell::new(entries.unwrap_or_default()),
            client,
        }
    }
}

impl GuestPronunciationLexicon for PollyPronunciationLexicon {
    fn get_name(&self) -> String {
        self.name.clone()
    }

    fn get_language(&self) -> LanguageCode {
        self.language.clone()
    }

    fn get_entry_count(&self) -> u32 {
        self.entries.borrow().len() as u32
    }

    fn add_entry(&self, word: String, pronunciation: String) -> Result<(), TtsError> {
        let entry = PronunciationEntry {
            word,
            pronunciation,
            part_of_speech: None,
        };
        self.entries.borrow_mut().push(entry);

        self.sync_lexicon()?;
        Ok(())
    }

    fn remove_entry(&self, word: String) -> Result<(), TtsError> {
        self.entries.borrow_mut().retain(|entry| entry.word != word);

        self.sync_lexicon()?;
        Ok(())
    }

    fn export_content(&self) -> Result<String, TtsError> {
        let entries = self.entries.borrow();
        let mut lexicon_content = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<lexicon version="1.0" xmlns="http://www.w3.org/2005/01/pronunciation-lexicon" alphabet="ipa" xml:lang="{}">
"#,
            self.language
        );

        for entry in entries.iter() {
            lexicon_content.push_str(&format!(
                r#"  <lexeme>
    <grapheme>{}</grapheme>
    <phoneme>{}</phoneme>
  </lexeme>
"#,
                entry.word, entry.pronunciation
            ));
        }

        lexicon_content.push_str("</lexicon>");
        Ok(lexicon_content)
    }
}

impl PollyPronunciationLexicon {
    fn sync_lexicon(&self) -> Result<(), TtsError> {
        let content = self.export_content()?;
        self.client.put_lexicon(&self.name, &content)?;
        Ok(())
    }
}

struct PollyLongFormOperation {
    task_id: String,
    client: AwsPollyTtsApi,
    status: Cell<OperationStatus>,
    progress: Cell<f32>,
}

impl PollyLongFormOperation {
    fn new(task_id: String, client: AwsPollyTtsApi) -> Self {
        Self {
            task_id,
            client,
            status: Cell::new(OperationStatus::Pending),
            progress: Cell::new(0.0),
        }
    }

    fn update_status(&self) -> Result<(), TtsError> {
        let task = self.client.get_speech_synthesis_task(&self.task_id)?;

        let status = match task.task_status.as_deref() {
            Some("scheduled") => OperationStatus::Pending,
            Some("inProgress") => OperationStatus::Processing,
            Some("completed") => OperationStatus::Completed,
            Some("failed") => OperationStatus::Failed,
            _ => OperationStatus::Pending,
        };

        self.status.set(status);

        let progress = match status {
            OperationStatus::Pending => 0.0,
            OperationStatus::Processing => 0.5,
            OperationStatus::Completed => 1.0,
            OperationStatus::Failed => 0.0,
            OperationStatus::Cancelled => 0.0,
        };
        self.progress.set(progress);

        Ok(())
    }
}

impl GuestLongFormOperation for PollyLongFormOperation {
    fn get_status(&self) -> OperationStatus {
        let _ = self.update_status();
        self.status.get()
    }

    fn get_progress(&self) -> f32 {
        self.progress.get()
    }

    fn cancel(&self) -> Result<(), TtsError> {
        Err(TtsError::UnsupportedOperation(
            "Cannot cancel AWS Polly synthesis tasks".to_string(),
        ))
    }

    fn get_result(&self) -> Result<LongFormResult, TtsError> {
        let task = self.client.get_speech_synthesis_task(&self.task_id)?;

        if let Some(output_uri) = task.output_uri {
            let character_count = task.request_characters.unwrap_or(0) as u32;

            let estimated_word_count = if character_count > 0 {
                (character_count as f32 / 5.0).ceil() as u32
            } else {
                0
            };

            let estimated_duration = if estimated_word_count > 0 {
                (estimated_word_count as f32 / 150.0) * 60.0
            } else {
                0.0
            };

            let actual_audio_size = match self.client.get_s3_object_metadata(&output_uri) {
                Ok(metadata) => {
                    trace!(
                        "Retrieved S3 object metadata: {} bytes",
                        metadata.size_bytes
                    );
                    metadata.size_bytes as u32
                }
                Err(e) => {
                    trace!("Failed to get S3 object metadata, using estimation: {}", e);
                    if estimated_duration > 0.0 {
                        let format = task.output_format.as_deref().unwrap_or("mp3");
                        match format {
                            "mp3" => ((estimated_duration * 128000.0) / 8.0) as u32,
                            "pcm" => (estimated_duration * 22050.0 * 2.0) as u32,
                            "ogg_vorbis" => ((estimated_duration * 64000.0) / 8.0) as u32,
                            _ => 0,
                        }
                    } else {
                        0
                    }
                }
            };

            Ok(LongFormResult {
                output_location: output_uri,
                total_duration: estimated_duration,
                chapter_durations: None,
                metadata: golem_tts::golem::tts::types::SynthesisMetadata {
                    duration_seconds: estimated_duration,
                    character_count,
                    word_count: estimated_word_count,
                    audio_size_bytes: actual_audio_size,
                    request_id: self.task_id.clone(),
                    provider_info: Some("AWS Polly".to_string()),
                },
            })
        } else {
            Err(TtsError::InternalError(
                "Task result not available".to_string(),
            ))
        }
    }
}

struct AwsPollyComponent;

impl AwsPollyComponent {
    const ACCESS_KEY_ENV_VAR: &'static str = "AWS_ACCESS_KEY_ID";
    const SECRET_KEY_ENV_VAR: &'static str = "AWS_SECRET_ACCESS_KEY";
    const REGION_ENV_VAR: &'static str = "AWS_REGION";
    const SESSION_TOKEN_ENV_VAR: &'static str = "AWS_SESSION_TOKEN";

    fn create_client() -> Result<AwsPollyTtsApi, TtsError> {
        with_config_key(Self::ACCESS_KEY_ENV_VAR, Err, |access_key_id| {
            with_config_key(Self::SECRET_KEY_ENV_VAR, Err, |secret_access_key| {
                let region =
                    std::env::var(Self::REGION_ENV_VAR).unwrap_or_else(|_| "us-east-1".to_string());
                let session_token = std::env::var(Self::SESSION_TOKEN_ENV_VAR).ok();

                AwsPollyTtsApi::new(
                    access_key_id.to_string(),
                    secret_access_key.to_string(),
                    region,
                    session_token,
                )
            })
        })
    }
}

impl VoicesGuest for AwsPollyComponent {
    type Voice = PollyVoiceImpl;
    type VoiceResults = PollyVoiceResults;

    fn list_voices(filter: Option<VoiceFilter>) -> Result<VoiceResults, TtsError> {
        let client = Self::create_client()?;
        let params = voice_filter_to_describe_params(filter.clone());

        let response = client.describe_voices(params)?;
        trace!(
            "AWS describe_voices returned {} voices",
            response.voices.len()
        );

        let voice_infos: Vec<VoiceInfo> = response
            .voices
            .into_iter()
            .enumerate()
            .map(|(i, voice)| {
                trace!("Converting voice {}: {} ({})", i, voice.name, voice.id);
                aws_voice_to_voice_info(voice)
            })
            .collect();

        trace!("Converted to {} VoiceInfo objects", voice_infos.len());
        let total_count = Some(voice_infos.len() as u32);
        Ok(VoiceResults::new(PollyVoiceResults::new(
            voice_infos,
            total_count,
            response.next_token,
            client,
            filter,
        )))
    }

    fn get_voice(voice_id: String) -> Result<Voice, TtsError> {
        let client = Self::create_client()?;
        let response = client.describe_voices(None)?;

        let voice_data = response
            .voices
            .into_iter()
            .find(|v| v.id == voice_id)
            .ok_or_else(|| TtsError::VoiceNotFound(voice_id.clone()))?;

        Ok(Voice::new(PollyVoiceImpl::new(voice_data, client)))
    }

    fn search_voices(
        query: String,
        filter: Option<VoiceFilter>,
    ) -> Result<Vec<VoiceInfo>, TtsError> {
        let client = Self::create_client()?;
        let params = voice_filter_to_describe_params(filter);

        let response = client.describe_voices(params)?;
        let voice_infos: Vec<VoiceInfo> = response
            .voices
            .into_iter()
            .filter(|voice| {
                voice.name.to_lowercase().contains(&query.to_lowercase())
                    || voice
                        .language_name
                        .to_lowercase()
                        .contains(&query.to_lowercase())
                    || voice
                        .language_code
                        .to_lowercase()
                        .contains(&query.to_lowercase())
            })
            .map(aws_voice_to_voice_info)
            .collect();

        Ok(voice_infos)
    }

    fn list_languages() -> Result<Vec<LanguageInfo>, TtsError> {
        Ok(get_polly_language_info())
    }
}

impl SynthesisGuest for AwsPollyComponent {
    fn synthesize(
        input: TextInput,
        voice: golem_tts::golem::tts::voices::VoiceBorrow<'_>,
        options: Option<SynthesisOptions>,
    ) -> Result<SynthesisResult, TtsError> {
        validate_synthesis_input(&input, options.as_ref())?;

        let client = Self::create_client()?;
        let voice_id = voice.get::<PollyVoiceImpl>().get_id();

        if input.content.len() > 3000 {
            let chunks = split_text_intelligently(&input.content, 3000);

            let mut audio_chunks = Vec::new();
            for chunk in chunks {
                let chunk_input = TextInput {
                    content: chunk,
                    language: input.language.clone(),
                    text_type: input.text_type,
                };
                let params = synthesis_options_to_polly_params(
                    options.clone(),
                    voice_id.clone(),
                    chunk_input.content.clone(),
                );
                let chunk_audio = client.synthesize_speech(params)?;
                audio_chunks.push(chunk_audio);
            }

            let format = options
                .as_ref()
                .and_then(|o| o.audio_config)
                .map(|ac| ac.format)
                .unwrap_or(AudioFormat::Mp3);

            let combined_audio = combine_audio_chunks(audio_chunks, &format);

            let sample_rate = options
                .as_ref()
                .and_then(|o| o.audio_config)
                .and_then(|ac| ac.sample_rate)
                .unwrap_or(22050);

            Ok(audio_data_to_synthesis_result(
                combined_audio,
                &input.content,
                &format,
                sample_rate,
            ))
        } else {
            let params =
                synthesis_options_to_polly_params(options, voice_id, input.content.clone());
            let audio_data = client.synthesize_speech(params.clone())?;

            let format = polly_format_to_audio_format(&params.output_format);
            let sample_rate = params
                .sample_rate
                .and_then(|s| s.parse().ok())
                .unwrap_or(22050);

            Ok(audio_data_to_synthesis_result(
                audio_data,
                &input.content,
                &format,
                sample_rate,
            ))
        }
    }

    fn synthesize_batch(
        inputs: Vec<TextInput>,
        voice: golem_tts::golem::tts::voices::VoiceBorrow<'_>,
        options: Option<SynthesisOptions>,
    ) -> Result<Vec<SynthesisResult>, TtsError> {
        let client = Self::create_client()?;
        let voice_id = voice.get::<PollyVoiceImpl>().get_id();
        let mut results = Vec::new();

        for input in inputs {
            validate_synthesis_input(&input, options.as_ref())?;

            if input.content.len() > 3000 {
                let chunks = split_text_intelligently(&input.content, 3000);

                let mut audio_chunks = Vec::new();
                for chunk in chunks {
                    let chunk_input = TextInput {
                        content: chunk,
                        language: input.language.clone(),
                        text_type: input.text_type,
                    };
                    let params = synthesis_options_to_polly_params(
                        options.clone(),
                        voice_id.clone(),
                        chunk_input.content.clone(),
                    );
                    let chunk_audio = client.synthesize_speech(params)?;
                    audio_chunks.push(chunk_audio);
                }

                let format = options
                    .as_ref()
                    .and_then(|o| o.audio_config)
                    .map(|ac| ac.format)
                    .unwrap_or(AudioFormat::Mp3);

                let combined_audio = combine_audio_chunks(audio_chunks, &format);

                let sample_rate = options
                    .as_ref()
                    .and_then(|o| o.audio_config)
                    .and_then(|ac| ac.sample_rate)
                    .unwrap_or(22050);

                results.push(audio_data_to_synthesis_result(
                    combined_audio,
                    &input.content,
                    &format,
                    sample_rate,
                ));
            } else {
                let params = synthesis_options_to_polly_params(
                    options.clone(),
                    voice_id.clone(),
                    input.content.clone(),
                );
                let audio_data = client.synthesize_speech(params.clone())?;

                let format = polly_format_to_audio_format(&params.output_format);
                let sample_rate = params
                    .sample_rate
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(22050);

                results.push(audio_data_to_synthesis_result(
                    audio_data,
                    &input.content,
                    &format,
                    sample_rate,
                ));
            }
        }

        Ok(results)
    }

    fn get_timing_marks(
        input: TextInput,
        voice: golem_tts::golem::tts::voices::VoiceBorrow<'_>,
    ) -> Result<Vec<TimingInfo>, TtsError> {
        let client = Self::create_client()?;
        let voice_id = voice.get::<PollyVoiceImpl>().get_id();

        trace!(
            "Getting timing marks for voice: {}, text: {}",
            voice_id,
            input.content
        );

        let mut params = synthesis_options_to_polly_params(None, voice_id, input.content.clone());
        params.output_format = crate::client::OutputFormat::Json;
        params.speech_mark_types = Some(vec![
            crate::client::SpeechMarkType::Word,
            crate::client::SpeechMarkType::Sentence,
        ]);

        let response_data = client.synthesize_speech(params)?;

        let response_text = String::from_utf8(response_data).map_err(|_| {
            TtsError::InternalError("Invalid UTF-8 in speech marks response".to_string())
        })?;

        let mut timing_marks = Vec::new();

        for line in response_text.lines() {
            if line.trim().is_empty() {
                continue;
            }

            match serde_json::from_str::<serde_json::Value>(line) {
                Ok(mark) => {
                    if let (Some(mark_type), Some(start_time), Some(end_time), Some(_value)) = (
                        mark.get("type").and_then(|v| v.as_str()),
                        mark.get("time").and_then(|v| v.as_f64()),
                        mark.get("time").and_then(|v| v.as_f64()),
                        mark.get("value").and_then(|v| v.as_str()),
                    ) {
                        let timing_type = match mark_type {
                            "word" => Some(TimingMarkType::Word),
                            "sentence" => Some(TimingMarkType::Sentence),
                            _ => continue,
                        };

                        timing_marks.push(TimingInfo {
                            start_time_seconds: (start_time / 1000.0) as f32,
                            end_time_seconds: Some((end_time / 1000.0) as f32),
                            text_offset: None,
                            mark_type: timing_type,
                        });
                    }
                }
                Err(e) => {
                    trace!("Failed to parse speech mark line '{}': {}", line, e);
                }
            }
        }

        timing_marks.sort_by(|a, b| {
            a.start_time_seconds
                .partial_cmp(&b.start_time_seconds)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        trace!("Generated {} timing marks", timing_marks.len());
        Ok(timing_marks)
    }

    fn validate_input(
        input: TextInput,
        voice: golem_tts::golem::tts::voices::VoiceBorrow<'_>,
    ) -> Result<ValidationResult, TtsError> {
        let voice_id = voice.get::<PollyVoiceImpl>().get_id();
        Ok(validate_polly_input(&input.content, &voice_id))
    }
}

impl StreamingGuest for AwsPollyComponent {
    type SynthesisStream = PollySynthesisStream;
    type VoiceConversionStream = PollyVoiceConversionStream;

    fn create_stream(
        voice: golem_tts::golem::tts::voices::VoiceBorrow<'_>,
        options: Option<SynthesisOptions>,
    ) -> Result<SynthesisStream, TtsError> {
        let client = Self::create_client()?;
        let voice_id = voice.get::<PollyVoiceImpl>().get_id();

        Ok(SynthesisStream::new(PollySynthesisStream::new(
            voice_id, client, options,
        )))
    }

    fn create_voice_conversion_stream(
        target_voice: golem_tts::golem::tts::voices::VoiceBorrow<'_>,
        _options: Option<SynthesisOptions>,
    ) -> Result<VoiceConversionStream, TtsError> {
        let client = Self::create_client()?;
        let voice_id = target_voice.get::<PollyVoiceImpl>().get_id();

        Ok(VoiceConversionStream::new(PollyVoiceConversionStream::new(
            voice_id, client,
        )))
    }
}

impl AdvancedGuest for AwsPollyComponent {
    type PronunciationLexicon = PollyPronunciationLexicon;
    type LongFormOperation = PollyLongFormOperation;

    fn create_voice_clone(
        _name: String,
        _audio_samples: Vec<AudioSample>,
        _description: Option<String>,
    ) -> Result<golem_tts::golem::tts::voices::Voice, TtsError> {
        Err(TtsError::UnsupportedOperation(
            "Voice cloning not supported by AWS Polly".to_string(),
        ))
    }

    fn design_voice(
        _name: String,
        _characteristics: VoiceDesignParams,
    ) -> Result<golem_tts::golem::tts::voices::Voice, TtsError> {
        Err(TtsError::UnsupportedOperation(
            "Voice design not supported by AWS Polly".to_string(),
        ))
    }

    fn convert_voice(
        _input_audio: Vec<u8>,
        _target_voice: golem_tts::golem::tts::voices::VoiceBorrow<'_>,
        _preserve_timing: Option<bool>,
    ) -> Result<Vec<u8>, TtsError> {
        Err(TtsError::UnsupportedOperation(
            "Voice conversion not supported by AWS Polly".to_string(),
        ))
    }

    fn generate_sound_effect(
        _description: String,
        _duration_seconds: Option<f32>,
        _style_influence: Option<f32>,
    ) -> Result<Vec<u8>, TtsError> {
        Err(TtsError::UnsupportedOperation(
            "Sound effect generation not supported by AWS Polly".to_string(),
        ))
    }

    fn create_lexicon(
        name: String,
        language: LanguageCode,
        entries: Option<Vec<PronunciationEntry>>,
    ) -> Result<PronunciationLexicon, TtsError> {
        let client = Self::create_client()?;
        let lexicon = PollyPronunciationLexicon::new(name, language, entries, client);

        lexicon.sync_lexicon()?;

        Ok(PronunciationLexicon::new(lexicon))
    }

    fn synthesize_long_form(
        content: String,
        voice: golem_tts::golem::tts::voices::VoiceBorrow<'_>,
        output_location: String,
        _chapter_breaks: Option<Vec<u32>>,
    ) -> Result<LongFormOperation, TtsError> {
        let client = Self::create_client()?;
        let voice_id = voice.get::<PollyVoiceImpl>().get_id();
        let voice_language = voice.get::<PollyVoiceImpl>().get_language();

        let bucket_name = if output_location.starts_with("s3://") {
            output_location
                .strip_prefix("s3://")
                .and_then(|path| path.split('/').next())
                .unwrap_or("default-bucket")
                .to_string()
        } else {
            "default-bucket".to_string()
        };

        let params = crate::client::StartSpeechSynthesisTaskParams {
            engine: Some(crate::client::Engine::Neural),
            language_code: Some(voice_language),
            lexicon_names: None,
            output_format: crate::client::OutputFormat::Mp3,
            output_s3_bucket_name: bucket_name,
            output_s3_key_prefix: Some("polly-output".to_string()),
            sample_rate: None,
            sns_topic_arn: None,
            speech_mark_types: None,
            text: content,
            text_type: Some(crate::client::TextType::Text),
            voice_id,
        };

        let task = client.start_speech_synthesis_task(params)?;
        let task_id = task
            .task_id
            .ok_or_else(|| TtsError::InternalError("No task ID returned".to_string()))?;

        Ok(LongFormOperation::new(PollyLongFormOperation::new(
            task_id, client,
        )))
    }
}

impl ExtendedGuest for AwsPollyComponent {
    fn unwrapped_synthesis_stream(
        voice: golem_tts::golem::tts::voices::VoiceBorrow<'_>,
        options: Option<SynthesisOptions>,
    ) -> Self::SynthesisStream {
        let client = Self::create_client().unwrap_or_else(|_| {
            AwsPollyTtsApi::new(
                "dummy".to_string(),
                "dummy".to_string(),
                "us-east-1".to_string(),
                None,
            )
            .unwrap_or_else(|_| panic!("Failed to create fallback client"))
        });
        let voice_id = voice.get::<PollyVoiceImpl>().get_id();

        PollySynthesisStream::new(voice_id, client, options)
    }

    fn unwrapped_voice_conversion_stream(
        target_voice: golem_tts::golem::tts::voices::VoiceBorrow<'_>,
        _options: Option<SynthesisOptions>,
    ) -> Self::VoiceConversionStream {
        let client = Self::create_client().unwrap_or_else(|_| {
            AwsPollyTtsApi::new(
                "dummy".to_string(),
                "dummy".to_string(),
                "us-east-1".to_string(),
                None,
            )
            .unwrap_or_else(|_| panic!("Failed to create fallback client"))
        });
        let voice_id = target_voice.get::<PollyVoiceImpl>().get_id();

        PollyVoiceConversionStream::new(voice_id, client)
    }

    fn subscribe_synthesis_stream(_stream: &Self::SynthesisStream) -> Pollable {
        golem_rust::bindings::wasi::clocks::monotonic_clock::subscribe_duration(0)
    }

    fn subscribe_voice_conversion_stream(_stream: &Self::VoiceConversionStream) -> Pollable {
        golem_rust::bindings::wasi::clocks::monotonic_clock::subscribe_duration(0)
    }
}

type DurableAwsPollyComponent = DurableTts<AwsPollyComponent>;

golem_tts::export_tts!(DurableAwsPollyComponent with_types_in golem_tts);
