use crate::client::{AudioEncoding, GoogleTtsApi, Voice as GoogleVoice};
use crate::conversions::{
    audio_data_to_synthesis_result, combine_audio_chunks, create_validation_result,
    estimate_audio_duration, google_voice_to_voice_info, google_voices_to_language_info,
    split_text_intelligently, synthesis_options_to_tts_request, validate_synthesis_input,
    voice_filter_to_language_code,
};
use golem_rust::wasm_rpc::Pollable;
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
    AudioChunk, AudioFormat, LanguageCode, SynthesisMetadata, SynthesisResult, TextInput,
    TimingInfo, TtsError, VoiceGender, VoiceQuality, VoiceSettings,
};
use golem_tts::golem::tts::voices::{
    Guest as VoicesGuest, GuestVoice, GuestVoiceResults, LanguageInfo, Voice, VoiceFilter,
    VoiceInfo, VoiceResults,
};
use std::cell::{Cell, RefCell};

mod client;
mod conversions;

struct GoogleVoiceImpl {
    voice_data: GoogleVoice,
    client: GoogleTtsApi,
}

impl GoogleVoiceImpl {
    fn new(voice_data: GoogleVoice, client: GoogleTtsApi) -> Self {
        Self { voice_data, client }
    }
}

impl GuestVoice for GoogleVoiceImpl {
    fn get_id(&self) -> String {
        self.voice_data.name.clone()
    }

    fn get_name(&self) -> String {
        conversions::extract_display_name(&self.voice_data.name)
    }

    fn get_provider_id(&self) -> Option<String> {
        Some("google".to_string())
    }

    fn get_language(&self) -> LanguageCode {
        self.voice_data
            .language_codes
            .first()
            .cloned()
            .unwrap_or_else(|| "en-US".to_string())
    }

    fn get_additional_languages(&self) -> Vec<LanguageCode> {
        self.voice_data
            .language_codes
            .iter()
            .skip(1)
            .cloned()
            .collect()
    }

    fn get_gender(&self) -> VoiceGender {
        conversions::ssml_gender_to_voice_gender(&self.voice_data.ssml_gender)
    }

    fn get_quality(&self) -> VoiceQuality {
        conversions::infer_quality_from_voice(&self.voice_data)
    }

    fn get_description(&self) -> Option<String> {
        Some(conversions::generate_voice_description(&self.voice_data))
    }

    fn supports_ssml(&self) -> bool {
        true
    }

    fn get_sample_rates(&self) -> Vec<u32> {
        vec![
            self.voice_data.natural_sample_rate_hertz as u32,
            22050,
            24000,
            44100,
            48000,
        ]
    }

    fn get_supported_formats(&self) -> Vec<AudioFormat> {
        vec![
            AudioFormat::Mp3,
            AudioFormat::Wav,
            AudioFormat::OggOpus,
            AudioFormat::Pcm,
            AudioFormat::Mulaw,
            AudioFormat::Alaw,
        ]
    }

    fn update_settings(&self, _settings: VoiceSettings) -> Result<(), TtsError> {
        Err(TtsError::UnsupportedOperation(
            "Voice settings update not supported by Google Cloud TTS".to_string(),
        ))
    }

    fn delete(&self) -> Result<(), TtsError> {
        Err(TtsError::UnsupportedOperation(
            "Built-in voices cannot be deleted in Google Cloud TTS".to_string(),
        ))
    }

    fn clone(&self) -> Result<Voice, TtsError> {
        Err(TtsError::UnsupportedOperation(
            "Voice cloning not supported by Google Cloud TTS".to_string(),
        ))
    }

    fn preview(&self, text: String) -> Result<Vec<u8>, TtsError> {
        let input = TextInput {
            content: text,
            text_type: golem_tts::golem::tts::types::TextType::Plain,
            language: Some(self.get_language()),
        };
        let voice_name = &self.voice_data.name;
        let language_code = &self.get_language();
        let (request, _) =
            conversions::synthesis_options_to_tts_request(&input, voice_name, language_code, None);

        self.client.text_to_speech(&request)
    }
}

struct GoogleVoiceResults {
    voices: RefCell<Vec<VoiceInfo>>,
    current_index: Cell<usize>,
    has_more: Cell<bool>,
    total_count: Option<u32>,
}

impl GoogleVoiceResults {
    fn new(voices: Vec<VoiceInfo>, total_count: Option<u32>) -> Self {
        let has_more = !voices.is_empty();
        Self {
            voices: RefCell::new(voices),
            current_index: Cell::new(0),
            has_more: Cell::new(has_more),
            total_count,
        }
    }
}

impl GuestVoiceResults for GoogleVoiceResults {
    fn has_more(&self) -> bool {
        self.has_more.get()
    }

    fn get_next(&self) -> Result<Vec<VoiceInfo>, TtsError> {
        let voices = self.voices.borrow();
        let current_idx = self.current_index.get();

        if current_idx >= voices.len() {
            return Ok(vec![]);
        }

        const BATCH_SIZE: usize = 10;
        let end_idx = std::cmp::min(current_idx + BATCH_SIZE, voices.len());
        let batch = voices[current_idx..end_idx].to_vec();

        self.current_index.set(end_idx);
        self.has_more.set(end_idx < voices.len());

        Ok(batch)
    }

    fn get_total_count(&self) -> Option<u32> {
        self.total_count
    }
}

struct GoogleSynthesisStream {
    client: GoogleTtsApi,
    current_request: RefCell<Option<crate::client::SynthesizeSpeechRequest>>,
    chunk_buffer: RefCell<Vec<u8>>,
    bytes_streamed: Cell<usize>,
    total_chunks_received: Cell<u32>,
    finished: Cell<bool>,
    sequence_number: Cell<u32>,
}

impl GoogleSynthesisStream {
    fn new(client: GoogleTtsApi, options: Option<SynthesisOptions>) -> Self {
        let default_input = TextInput {
            content: "".to_string(),
            text_type: golem_tts::golem::tts::types::TextType::Plain,
            language: Some("en-US".to_string()),
        };
        let (request, _) = conversions::synthesis_options_to_tts_request(
            &default_input,
            "en-US-Standard-A",
            "en-US",
            options,
        );

        Self {
            client,
            current_request: RefCell::new(Some(request)),
            chunk_buffer: RefCell::new(Vec::new()),
            bytes_streamed: Cell::new(0),
            total_chunks_received: Cell::new(0),
            finished: Cell::new(false),
            sequence_number: Cell::new(0),
        }
    }
}

impl GuestSynthesisStream for GoogleSynthesisStream {
    fn send_text(&self, input: TextInput) -> Result<(), TtsError> {
        if self.finished.get() {
            return Err(TtsError::UnsupportedOperation(
                "Stream is finished".to_string(),
            ));
        }

        let mut request_opt = self.current_request.borrow_mut();
        if let Some(request) = request_opt.as_mut() {
            match input.text_type {
                golem_tts::golem::tts::types::TextType::Plain => {
                    request.input.text = Some(input.content.clone());
                    request.input.ssml = None;
                }
                golem_tts::golem::tts::types::TextType::Ssml => {
                    request.input.ssml = Some(input.content.clone());
                    request.input.text = None;
                }
            }
        }

        Ok(())
    }

    fn finish(&self) -> Result<(), TtsError> {
        if self.finished.get() {
            return Ok(());
        }

        if let Some(request) = self.current_request.borrow().as_ref() {
            let audio_data = self.client.text_to_speech(request)?;

            let mut buffer = self.chunk_buffer.borrow_mut();
            buffer.extend_from_slice(&audio_data);
        }

        self.finished.set(true);
        Ok(())
    }

    fn receive_chunk(&self) -> Result<Option<AudioChunk>, TtsError> {
        let mut buffer = self.chunk_buffer.borrow_mut();

        if buffer.is_empty() {
            return Ok(None);
        }

        const CHUNK_SIZE: usize = 4096;
        let chunk_size = std::cmp::min(CHUNK_SIZE, buffer.len());
        let chunk_data = buffer.drain(..chunk_size).collect::<Vec<u8>>();

        let sequence = self.sequence_number.get();
        self.sequence_number.set(sequence + 1);
        self.bytes_streamed
            .set(self.bytes_streamed.get() + chunk_data.len());
        self.total_chunks_received
            .set(self.total_chunks_received.get() + 1);

        Ok(Some(AudioChunk {
            data: chunk_data,
            sequence_number: sequence,
            is_final: buffer.is_empty() && self.finished.get(),
            timing_info: None,
        }))
    }

    fn has_pending_audio(&self) -> bool {
        !self.chunk_buffer.borrow().is_empty()
    }

    fn get_status(&self) -> StreamStatus {
        if self.finished.get() && self.chunk_buffer.borrow().is_empty() {
            StreamStatus::Finished
        } else if self.finished.get() {
            StreamStatus::Processing
        } else {
            StreamStatus::Ready
        }
    }

    fn close(&self) {
        self.finished.set(true);
        self.chunk_buffer.borrow_mut().clear();
    }
}

struct GoogleVoiceConversionStream {
    finished: Cell<bool>,
}

impl GoogleVoiceConversionStream {
    fn new() -> Self {
        Self {
            finished: Cell::new(false),
        }
    }
}

impl GuestVoiceConversionStream for GoogleVoiceConversionStream {
    fn send_audio(&self, _audio_data: Vec<u8>) -> Result<(), TtsError> {
        Err(TtsError::UnsupportedOperation(
            "Voice conversion not supported by Google Cloud TTS".to_string(),
        ))
    }

    fn receive_converted(&self) -> Result<Option<AudioChunk>, TtsError> {
        Err(TtsError::UnsupportedOperation(
            "Voice conversion not supported by Google Cloud TTS".to_string(),
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

struct GooglePronunciationLexicon {
    name: String,
    language: LanguageCode,
    entries: RefCell<Vec<PronunciationEntry>>,
}

impl GooglePronunciationLexicon {
    fn new(name: String, language: LanguageCode, entries: Option<Vec<PronunciationEntry>>) -> Self {
        Self {
            name,
            language,
            entries: RefCell::new(entries.unwrap_or_default()),
        }
    }
}

impl GuestPronunciationLexicon for GooglePronunciationLexicon {
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
        self.entries.borrow_mut().push(PronunciationEntry {
            word,
            pronunciation,
            part_of_speech: Some("unknown".to_string()),
        });
        Ok(())
    }

    fn remove_entry(&self, word: String) -> Result<(), TtsError> {
        self.entries.borrow_mut().retain(|entry| entry.word != word);
        Ok(())
    }

    fn export_content(&self) -> Result<String, TtsError> {
        let entries = self.entries.borrow();
        let content = entries
            .iter()
            .map(|entry| format!("{}: {}", entry.word, entry.pronunciation))
            .collect::<Vec<_>>()
            .join("\n");
        Ok(content)
    }
}

struct GoogleLongFormOperation {
    content: String,
    output_location: String,
    client: GoogleTtsApi,
    status: Cell<OperationStatus>,
    progress: Cell<f32>,
    audio_chunks: RefCell<Option<Vec<Vec<u8>>>>,
    request_template: RefCell<crate::client::SynthesizeSpeechRequest>,
}

impl GoogleLongFormOperation {
    fn new(
        content: String,
        output_location: String,
        client: GoogleTtsApi,
        options: Option<SynthesisOptions>,
    ) -> Self {
        let default_input = TextInput {
            content: content.clone(),
            text_type: golem_tts::golem::tts::types::TextType::Plain,
            language: Some("en-US".to_string()),
        };
        let (request, _) = conversions::synthesis_options_to_tts_request(
            &default_input,
            "en-US-Standard-A",
            "en-US",
            options,
        );

        Self {
            content,
            output_location,
            client,
            status: Cell::new(OperationStatus::Pending),
            progress: Cell::new(0.0),
            audio_chunks: RefCell::new(None),
            request_template: RefCell::new(request),
        }
    }

    fn process_long_form(&self) -> Result<(), TtsError> {
        self.status.set(OperationStatus::Processing);

        let chunks = self.split_content_intelligently(&self.content, 4000);
        let total_chunks = chunks.len();
        let mut audio_results = Vec::new();

        for (i, chunk) in chunks.iter().enumerate() {
            let mut request = self.request_template.borrow().clone();
            request.input.text = Some(chunk.clone());

            match self.client.text_to_speech(&request) {
                Ok(audio_data) => {
                    audio_results.push(audio_data);
                    let progress = (i + 1) as f32 / total_chunks as f32;
                    self.progress.set(progress);
                }
                Err(e) => {
                    self.status.set(OperationStatus::Failed);
                    return Err(e);
                }
            }
        }

        self.audio_chunks.replace(Some(audio_results));
        self.status.set(OperationStatus::Completed);
        self.progress.set(1.0);

        Ok(())
    }

    fn split_content_intelligently(&self, content: &str, max_chunk_size: usize) -> Vec<String> {
        let mut chunks = Vec::new();
        let mut current_chunk = String::new();

        let sentences: Vec<&str> = content.split('.').collect();

        for sentence in sentences {
            let sentence_with_period = format!("{}.", sentence.trim());

            if current_chunk.len() + sentence_with_period.len() <= max_chunk_size {
                current_chunk.push_str(&sentence_with_period);
                current_chunk.push(' ');
            } else {
                if !current_chunk.is_empty() {
                    chunks.push(current_chunk.trim().to_string());
                    current_chunk.clear();
                }

                if sentence_with_period.len() > max_chunk_size {
                    chunks.extend(
                        self.split_at_word_boundaries(&sentence_with_period, max_chunk_size),
                    );
                } else {
                    current_chunk = sentence_with_period;
                }
            }
        }

        if !current_chunk.trim().is_empty() {
            chunks.push(current_chunk.trim().to_string());
        }

        chunks
    }

    fn split_at_word_boundaries(&self, text: &str, max_size: usize) -> Vec<String> {
        let mut chunks = Vec::new();
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut current_chunk = String::new();

        for word in words {
            if current_chunk.len() + word.len() < max_size {
                if !current_chunk.is_empty() {
                    current_chunk.push(' ');
                }
                current_chunk.push_str(word);
            } else if !current_chunk.is_empty() {
                chunks.push(current_chunk);
                current_chunk = word.to_string();
            } else {
                chunks.push(word.to_string());
            }
        }

        if !current_chunk.is_empty() {
            chunks.push(current_chunk);
        }

        chunks
    }
}

impl GuestLongFormOperation for GoogleLongFormOperation {
    fn get_status(&self) -> OperationStatus {
        self.status.get()
    }

    fn get_progress(&self) -> f32 {
        self.progress.get()
    }

    fn cancel(&self) -> Result<(), TtsError> {
        self.status.set(OperationStatus::Cancelled);
        Ok(())
    }

    fn get_result(&self) -> Result<LongFormResult, TtsError> {
        if self.status.get() != OperationStatus::Completed {
            return Err(TtsError::UnsupportedOperation(
                "Operation not completed".to_string(),
            ));
        }

        let audio_chunks = self.audio_chunks.borrow();
        let chunks = audio_chunks
            .as_ref()
            .ok_or_else(|| TtsError::UnsupportedOperation("No audio data available".to_string()))?;

        let combined_audio: Vec<u8> = chunks.iter().flatten().cloned().collect();

        let sample_rate = self
            .request_template
            .borrow()
            .audio_config
            .sample_rate_hertz
            .unwrap_or(22050) as u32;
        let encoding = &self.request_template.borrow().audio_config.audio_encoding;
        let total_duration = estimate_audio_duration(&combined_audio, sample_rate, encoding);

        Ok(LongFormResult {
            output_location: self.output_location.clone(),
            total_duration,
            chapter_durations: None,
            metadata: SynthesisMetadata {
                duration_seconds: total_duration,
                character_count: self.content.len() as u32,
                word_count: self.content.split_whitespace().count() as u32,
                audio_size_bytes: combined_audio.len() as u32,
                request_id: format!("google-{}", uuid::Uuid::new_v4()),
                provider_info: Some("Google Cloud TTS".to_string()),
            },
        })
    }
}

struct GoogleComponent;

impl GoogleComponent {
    const CREDENTIALS_ENV_VAR: &'static str = "GOOGLE_APPLICATION_CREDENTIALS";
    const PROJECT_ENV_VAR: &'static str = "GOOGLE_CLOUD_PROJECT";

    fn create_client() -> Result<GoogleTtsApi, TtsError> {
        let credentials_path = std::env::var(Self::CREDENTIALS_ENV_VAR).ok();
        let project_id = std::env::var(Self::PROJECT_ENV_VAR).ok();

        GoogleTtsApi::new(credentials_path, project_id)
    }
}

impl VoicesGuest for GoogleComponent {
    type Voice = GoogleVoiceImpl;
    type VoiceResults = GoogleVoiceResults;

    fn list_voices(filter: Option<VoiceFilter>) -> Result<VoiceResults, TtsError> {
        let client = Self::create_client()?;
        let language_code = voice_filter_to_language_code(filter);

        let response = client.list_voices(language_code.as_deref())?;

        let voice_infos: Vec<VoiceInfo> = response
            .voices
            .into_iter()
            .map(google_voice_to_voice_info)
            .collect();

        let total_count = Some(voice_infos.len() as u32);
        let results = GoogleVoiceResults::new(voice_infos, total_count);

        Ok(VoiceResults::new(results))
    }

    fn get_voice(voice_id: String) -> Result<Voice, TtsError> {
        let client = Self::create_client()?;

        let response = client.list_voices(None)?;

        let voice_data = response
            .voices
            .into_iter()
            .find(|v| v.name == voice_id)
            .ok_or(TtsError::VoiceNotFound(voice_id))?;

        let voice_impl = GoogleVoiceImpl::new(voice_data, client);
        Ok(Voice::new(voice_impl))
    }

    fn search_voices(
        query: String,
        filter: Option<VoiceFilter>,
    ) -> Result<Vec<VoiceInfo>, TtsError> {
        let client = Self::create_client()?;
        let language_code = voice_filter_to_language_code(filter);

        let response = client.list_voices(language_code.as_deref())?;

        let query_lower = query.to_lowercase();
        let matching_voices: Vec<VoiceInfo> = response
            .voices
            .into_iter()
            .filter(|voice| {
                voice.name.to_lowercase().contains(&query_lower)
                    || voice
                        .language_codes
                        .iter()
                        .any(|lang| lang.to_lowercase().contains(&query_lower))
            })
            .map(google_voice_to_voice_info)
            .collect();

        Ok(matching_voices)
    }

    fn list_languages() -> Result<Vec<LanguageInfo>, TtsError> {
        let client = Self::create_client()?;
        let response = client.list_voices(None)?;

        Ok(google_voices_to_language_info(response.voices))
    }
}

impl SynthesisGuest for GoogleComponent {
    fn synthesize(
        input: TextInput,
        voice: golem_tts::golem::tts::voices::VoiceBorrow<'_>,
        options: Option<SynthesisOptions>,
    ) -> Result<SynthesisResult, TtsError> {
        validate_synthesis_input(&input, options.as_ref())?;

        let client = Self::create_client()?;
        let voice_impl = voice.get::<GoogleVoiceImpl>();
        let voice_name = voice_impl.get_id();
        let language_code = voice_impl.get_language();

        if input.content.len() > 5000 {
            let chunks = split_text_intelligently(&input.content, 5000);

            let mut audio_chunks = Vec::new();
            for chunk in chunks {
                let chunk_input = TextInput {
                    content: chunk,
                    language: input.language.clone(),
                    text_type: input.text_type,
                };
                let (chunk_request, _) = synthesis_options_to_tts_request(
                    &chunk_input,
                    &voice_name,
                    &language_code,
                    options.clone(),
                );
                let chunk_audio = client.text_to_speech(&chunk_request)?;
                audio_chunks.push(chunk_audio);
            }

            let format = options
                .as_ref()
                .and_then(|o| o.audio_config)
                .map(|ac| ac.format)
                .unwrap_or(AudioFormat::Mp3);

            let combined_audio = combine_audio_chunks(audio_chunks, &format);

            let encoding = &AudioEncoding::Mp3;
            let sample_rate = options
                .as_ref()
                .and_then(|o| o.audio_config)
                .and_then(|ac| ac.sample_rate)
                .unwrap_or(22050);

            Ok(audio_data_to_synthesis_result(
                combined_audio,
                &input.content,
                encoding,
                sample_rate,
            ))
        } else {
            let (request, _) =
                synthesis_options_to_tts_request(&input, &voice_name, &language_code, options);

            let audio_data = client.text_to_speech(&request)?;
            let text = request
                .input
                .text
                .as_deref()
                .or(request.input.ssml.as_deref())
                .unwrap_or("");
            let encoding = &request.audio_config.audio_encoding;
            let sample_rate = request.audio_config.sample_rate_hertz.unwrap_or(22050) as u32;

            Ok(audio_data_to_synthesis_result(
                audio_data,
                text,
                encoding,
                sample_rate,
            ))
        }
    }

    fn synthesize_batch(
        inputs: Vec<TextInput>,
        voice: golem_tts::golem::tts::voices::VoiceBorrow<'_>,
        options: Option<SynthesisOptions>,
    ) -> Result<Vec<SynthesisResult>, TtsError> {
        for input in &inputs {
            validate_synthesis_input(input, options.as_ref())?;
        }

        let client = Self::create_client()?;
        let mut results = Vec::new();

        for input in inputs {
            let voice_impl = voice.get::<GoogleVoiceImpl>();
            let voice_name = voice_impl.get_id();
            let language_code = voice_impl.get_language();

            if input.content.len() > 5000 {
                let chunks = split_text_intelligently(&input.content, 5000);

                let mut audio_chunks = Vec::new();
                for chunk in chunks {
                    let chunk_input = TextInput {
                        content: chunk,
                        language: input.language.clone(),
                        text_type: input.text_type,
                    };
                    let (chunk_request, _) = synthesis_options_to_tts_request(
                        &chunk_input,
                        &voice_name,
                        &language_code,
                        options.clone(),
                    );
                    let chunk_audio = client.text_to_speech(&chunk_request)?;
                    audio_chunks.push(chunk_audio);
                }

                let format = options
                    .as_ref()
                    .and_then(|o| o.audio_config)
                    .map(|ac| ac.format)
                    .unwrap_or(AudioFormat::Mp3);

                let combined_audio = combine_audio_chunks(audio_chunks, &format);

                let encoding = &AudioEncoding::Mp3;
                let sample_rate = options
                    .as_ref()
                    .and_then(|o| o.audio_config)
                    .and_then(|ac| ac.sample_rate)
                    .unwrap_or(22050);

                results.push(audio_data_to_synthesis_result(
                    combined_audio,
                    &input.content,
                    encoding,
                    sample_rate,
                ));
            } else {
                let (request, _) = synthesis_options_to_tts_request(
                    &input,
                    &voice_name,
                    &language_code,
                    options.clone(),
                );
                let audio_data = client.text_to_speech(&request)?;
                let text = request
                    .input
                    .text
                    .as_deref()
                    .or(request.input.ssml.as_deref())
                    .unwrap_or("");
                let encoding = &request.audio_config.audio_encoding;
                let sample_rate = request.audio_config.sample_rate_hertz.unwrap_or(22050) as u32;

                results.push(audio_data_to_synthesis_result(
                    audio_data,
                    text,
                    encoding,
                    sample_rate,
                ));
            }
        }

        Ok(results)
    }

    fn get_timing_marks(
        _input: TextInput,
        _voice: golem_tts::golem::tts::voices::VoiceBorrow<'_>,
    ) -> Result<Vec<TimingInfo>, TtsError> {
        Err(TtsError::UnsupportedOperation(
            "Timing marks not supported by Google Cloud TTS".to_string(),
        ))
    }

    fn validate_input(
        input: TextInput,
        _voice: golem_tts::golem::tts::voices::VoiceBorrow<'_>,
    ) -> Result<ValidationResult, TtsError> {
        match validate_synthesis_input(&input, None) {
            Ok(_) => Ok(create_validation_result(true, None)),
            Err(e) => {
                let message = match e {
                    TtsError::InvalidText(msg) => msg,
                    TtsError::InvalidSsml(msg) => msg,
                    TtsError::UnsupportedLanguage(msg) => msg,
                    TtsError::InvalidConfiguration(msg) => msg,
                    _ => "Validation failed".to_string(),
                };
                Ok(create_validation_result(false, Some(message)))
            }
        }
    }
}

impl StreamingGuest for GoogleComponent {
    type SynthesisStream = GoogleSynthesisStream;
    type VoiceConversionStream = GoogleVoiceConversionStream;

    fn create_stream(
        _voice: golem_tts::golem::tts::voices::VoiceBorrow<'_>,
        options: Option<SynthesisOptions>,
    ) -> Result<SynthesisStream, TtsError> {
        let client = Self::create_client()?;
        let stream = GoogleSynthesisStream::new(client, options);
        Ok(SynthesisStream::new(stream))
    }

    fn create_voice_conversion_stream(
        _target_voice: golem_tts::golem::tts::voices::VoiceBorrow<'_>,
        _options: Option<SynthesisOptions>,
    ) -> Result<VoiceConversionStream, TtsError> {
        let stream = GoogleVoiceConversionStream::new();
        Ok(VoiceConversionStream::new(stream))
    }
}

impl AdvancedGuest for GoogleComponent {
    type PronunciationLexicon = GooglePronunciationLexicon;
    type LongFormOperation = GoogleLongFormOperation;

    fn create_voice_clone(
        _name: String,
        _audio_samples: Vec<AudioSample>,
        _description: Option<String>,
    ) -> Result<Voice, TtsError> {
        Err(TtsError::UnsupportedOperation(
            "Voice cloning not supported by Google Cloud TTS".to_string(),
        ))
    }

    fn design_voice(_name: String, _characteristics: VoiceDesignParams) -> Result<Voice, TtsError> {
        Err(TtsError::UnsupportedOperation(
            "Voice design not supported by Google Cloud TTS".to_string(),
        ))
    }

    fn convert_voice(
        _input_audio: Vec<u8>,
        _target_voice: golem_tts::golem::tts::voices::VoiceBorrow<'_>,
        _preserve_timing: Option<bool>,
    ) -> Result<Vec<u8>, TtsError> {
        Err(TtsError::UnsupportedOperation(
            "Voice conversion not supported by Google Cloud TTS".to_string(),
        ))
    }

    fn generate_sound_effect(
        _description: String,
        _duration_seconds: Option<f32>,
        _style_influence: Option<f32>,
    ) -> Result<Vec<u8>, TtsError> {
        Err(TtsError::UnsupportedOperation(
            "Sound effect generation not supported by Google Cloud TTS".to_string(),
        ))
    }

    fn create_lexicon(
        name: String,
        language: LanguageCode,
        entries: Option<Vec<PronunciationEntry>>,
    ) -> Result<PronunciationLexicon, TtsError> {
        let lexicon = GooglePronunciationLexicon::new(name, language, entries);
        Ok(PronunciationLexicon::new(lexicon))
    }

    fn synthesize_long_form(
        content: String,
        _voice: golem_tts::golem::tts::voices::VoiceBorrow<'_>,
        output_location: String,
        _chapter_breaks: Option<Vec<u32>>,
    ) -> Result<LongFormOperation, TtsError> {
        let client = Self::create_client()?;
        let operation = GoogleLongFormOperation::new(content, output_location, client, None);

        operation.process_long_form()?;

        Ok(LongFormOperation::new(operation))
    }
}

impl ExtendedGuest for GoogleComponent {
    fn unwrapped_synthesis_stream(
        _voice: golem_tts::golem::tts::voices::VoiceBorrow<'_>,
        options: Option<SynthesisOptions>,
    ) -> Self::SynthesisStream {
        let client = Self::create_client().expect("Failed to create Google client");
        GoogleSynthesisStream::new(client, options)
    }

    fn unwrapped_voice_conversion_stream(
        _target_voice: golem_tts::golem::tts::voices::VoiceBorrow<'_>,
        _options: Option<SynthesisOptions>,
    ) -> Self::VoiceConversionStream {
        GoogleVoiceConversionStream::new()
    }

    fn subscribe_synthesis_stream(_stream: &Self::SynthesisStream) -> Pollable {
        golem_rust::bindings::wasi::clocks::monotonic_clock::subscribe_duration(0)
    }

    fn subscribe_voice_conversion_stream(_stream: &Self::VoiceConversionStream) -> Pollable {
        golem_rust::bindings::wasi::clocks::monotonic_clock::subscribe_duration(0)
    }
}

type DurableGoogleComponent = DurableTts<GoogleComponent>;

golem_tts::export_tts!(DurableGoogleComponent with_types_in golem_tts);
