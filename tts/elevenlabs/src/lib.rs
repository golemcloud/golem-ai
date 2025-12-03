use crate::client::{ElevenLabsTtsApi, Voice as ElevenLabsVoice};
use crate::conversions::{
    audio_data_to_synthesis_result, create_voice_request_from_samples,
    elevenlabs_voice_to_voice_info, estimate_audio_duration, get_max_chars_for_model,
    models_to_language_info, split_text_intelligently, synthesis_options_to_tts_request,
    validate_synthesis_input, voice_design_params_to_create_request, voice_filter_to_list_params,
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
    AudioChunk, AudioFormat, LanguageCode, SynthesisResult, TextInput, TimingInfo, TtsError,
    VoiceGender, VoiceQuality, VoiceSettings,
};
use golem_tts::golem::tts::voices::{
    Guest as VoicesGuest, GuestVoice, GuestVoiceResults, LanguageInfo, Voice, VoiceFilter,
    VoiceInfo, VoiceResults,
};
use log::{info, trace, warn};
use std::cell::{Cell, RefCell};

mod client;
mod conversions;

struct ElevenLabsVoiceImpl {
    voice_data: ElevenLabsVoice,
    client: ElevenLabsTtsApi,
}

impl ElevenLabsVoiceImpl {
    fn new(voice_data: ElevenLabsVoice, client: ElevenLabsTtsApi) -> Self {
        Self { voice_data, client }
    }
}

impl GuestVoice for ElevenLabsVoiceImpl {
    fn get_id(&self) -> String {
        self.voice_data.voice_id.clone()
    }

    fn get_name(&self) -> String {
        self.voice_data.name.clone()
    }

    fn get_provider_id(&self) -> Option<String> {
        Some("elevenlabs".to_string())
    }

    fn get_language(&self) -> LanguageCode {
        "en-US".to_string()
    }

    fn get_additional_languages(&self) -> Vec<LanguageCode> {
        vec![]
    }

    fn get_gender(&self) -> VoiceGender {
        conversions::infer_gender_from_name(&self.voice_data.name).unwrap_or(VoiceGender::Neutral)
    }

    fn get_quality(&self) -> VoiceQuality {
        conversions::infer_quality_from_category(&self.voice_data.category)
            .unwrap_or(VoiceQuality::Standard)
    }

    fn get_description(&self) -> Option<String> {
        self.voice_data.description.clone()
    }

    fn supports_ssml(&self) -> bool {
        true
    }

    fn get_sample_rates(&self) -> Vec<u32> {
        vec![22050, 44100]
    }

    fn get_supported_formats(&self) -> Vec<AudioFormat> {
        vec![AudioFormat::Mp3, AudioFormat::Wav, AudioFormat::Pcm]
    }

    fn update_settings(&self, _settings: VoiceSettings) -> Result<(), TtsError> {
        Err(TtsError::UnsupportedOperation(
            "Voice settings update not supported by ElevenLabs".to_string(),
        ))
    }

    fn delete(&self) -> Result<(), TtsError> {
        match self.client.delete_voice(&self.voice_data.voice_id) {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }

    fn clone(&self) -> Result<Voice, TtsError> {
        Err(TtsError::UnsupportedOperation(
            "Voice cloning not directly supported by ElevenLabs API".to_string(),
        ))
    }

    fn preview(&self, text: String) -> Result<Vec<u8>, TtsError> {
        let params = crate::client::TextToSpeechParams {
            enable_logging: Some(false),
            optimize_streaming_latency: Some(0),
            output_format: Some("mp3_22050_32".to_string()),
        };

        let model_version = self.client.get_model_version();
        let supports_language_code = !model_version.contains("multilingual");

        let request = crate::client::TextToSpeechRequest {
            text,
            model_id: Some(model_version.to_string()),
            language_code: if supports_language_code {
                Some("en".to_string())
            } else {
                None
            },
            voice_settings: self.voice_data.settings.clone(),
            pronunciation_dictionary_locators: None,
            seed: None,
            previous_text: None,
            next_text: None,
            previous_request_ids: None,
            next_request_ids: None,
            apply_text_normalization: Some("auto".to_string()),
            apply_language_text_normalization: Some(false),
            use_pvc_as_ivc: Some(false),
        };

        self.client
            .text_to_speech(&self.voice_data.voice_id, &request, Some(params))
    }
}

struct ElevenLabsVoiceResults {
    voices: RefCell<Vec<VoiceInfo>>,
    current_index: Cell<usize>,
    has_more: Cell<bool>,
    total_count: Option<u32>,
}

impl ElevenLabsVoiceResults {
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

impl GuestVoiceResults for ElevenLabsVoiceResults {
    fn has_more(&self) -> bool {
        self.has_more.get()
    }

    fn get_next(&self) -> Result<Vec<VoiceInfo>, TtsError> {
        let voices = self.voices.borrow();
        let current_idx = self.current_index.get();

        if current_idx >= voices.len() {
            self.has_more.set(false);
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

struct ElevenLabsSynthesisStream {
    voice_id: String,
    client: ElevenLabsTtsApi,
    current_request: RefCell<Option<crate::client::TextToSpeechRequest>>,
    params: RefCell<Option<crate::client::TextToSpeechParams>>,
    response_stream: RefCell<Option<reqwest::Response>>,
    chunk_buffer: RefCell<Vec<u8>>,
    bytes_streamed: Cell<usize>,
    total_chunks_received: Cell<u32>,
    finished: Cell<bool>,
    sequence_number: Cell<u32>,
}

impl ElevenLabsSynthesisStream {
    fn new(voice_id: String, client: ElevenLabsTtsApi, options: Option<SynthesisOptions>) -> Self {
        let (request, params) =
            conversions::synthesis_options_to_tts_request(options, client.get_model_version());

        Self {
            voice_id,
            client,
            current_request: RefCell::new(Some(request)),
            params: RefCell::new(params),
            response_stream: RefCell::new(None),
            chunk_buffer: RefCell::new(Vec::new()),
            bytes_streamed: Cell::new(0),
            total_chunks_received: Cell::new(0),
            finished: Cell::new(false),
            sequence_number: Cell::new(0),
        }
    }

    #[allow(dead_code)]
    fn get_progress(&self) -> (usize, u32) {
        (self.bytes_streamed.get(), self.total_chunks_received.get())
    }
}

impl GuestSynthesisStream for ElevenLabsSynthesisStream {
    fn send_text(&self, input: TextInput) -> Result<(), TtsError> {
        if self.finished.get() {
            return Err(TtsError::InternalError("Stream is finished".to_string()));
        }

        {
            let mut request_ref = self.current_request.borrow_mut();
            if let Some(mut request) = request_ref.take() {
                info!(
                    "[DEBUG] ElevenLabs send_text - Updating request with new text: '{}'",
                    input.content
                );
                request.text = input.content;
                *request_ref = Some(request);
                info!("[DEBUG] ElevenLabs send_text - Successfully updated request");
            } else {
                warn!("[DEBUG] ElevenLabs send_text - Warning: No current request found to update");
            }
        }

        Ok(())
    }

    fn finish(&self) -> Result<(), TtsError> {
        if let Some(request) = self.current_request.borrow().as_ref() {
            if !request.text.is_empty() {
                match self.client.text_to_speech_stream(
                    &self.voice_id,
                    request,
                    self.params.borrow().clone(),
                ) {
                    Ok(response) => {
                        self.response_stream.borrow_mut().replace(response);
                    }
                    Err(e) => {
                        self.finished.set(true);
                        return Err(e);
                    }
                }
            }
        }

        Ok(())
    }

    fn receive_chunk(&self) -> Result<Option<AudioChunk>, TtsError> {
        if self.finished.get() {
            return Ok(None);
        }

        if let Some(response) = self.response_stream.borrow_mut().take() {
            const CHUNK_SIZE: usize = 4096;

            match response.bytes() {
                Ok(bytes) => {
                    if bytes.is_empty() {
                        self.finished.set(true);
                        return Ok(None);
                    }

                    let mut current_buffer = self.chunk_buffer.borrow_mut();
                    current_buffer.extend_from_slice(&bytes);

                    if current_buffer.len() >= CHUNK_SIZE || bytes.len() < CHUNK_SIZE {
                        let chunk_data: Vec<u8> = if current_buffer.len() <= CHUNK_SIZE {
                            current_buffer.drain(..).collect()
                        } else {
                            current_buffer.drain(..CHUNK_SIZE).collect()
                        };

                        let sequence = self.sequence_number.get();
                        self.sequence_number.set(sequence + 1);
                        self.bytes_streamed
                            .set(self.bytes_streamed.get() + chunk_data.len());
                        self.total_chunks_received
                            .set(self.total_chunks_received.get() + 1);

                        let is_final = bytes.len() < CHUNK_SIZE && current_buffer.is_empty();
                        if is_final {
                            self.finished.set(true);
                        }

                        let chunk = AudioChunk {
                            data: chunk_data.clone(),
                            sequence_number: sequence,
                            is_final,
                            timing_info: Some(TimingInfo {
                                start_time_seconds: (self.bytes_streamed.get() as f32) / 22050.0,
                                end_time_seconds: Some(estimate_audio_duration(&chunk_data, 22050)),
                                text_offset: None,
                                mark_type: None,
                            }),
                        };

                        return Ok(Some(chunk));
                    }

                    Ok(None)
                }
                Err(e) => {
                    self.finished.set(true);
                    Err(TtsError::NetworkError(format!("Stream read error: {}", e)))
                }
            }
        } else {
            self.finished.set(true);
            Ok(None)
        }
    }

    fn has_pending_audio(&self) -> bool {
        !self.finished.get()
            && (self.response_stream.borrow().is_some()
                || !self.chunk_buffer.borrow().is_empty()
                || (!self
                    .current_request
                    .borrow()
                    .as_ref()
                    .is_none_or(|r| r.text.is_empty())))
    }

    fn get_status(&self) -> StreamStatus {
        if self.finished.get() {
            StreamStatus::Finished
        } else if self.response_stream.borrow().is_some() {
            StreamStatus::Processing
        } else {
            StreamStatus::Ready
        }
    }

    fn close(&self) {
        self.finished.set(true);
        self.response_stream.borrow_mut().take();
    }
}

struct ElevenLabsVoiceConversionStream {
    voice_id: String,
    client: ElevenLabsTtsApi,
    audio_buffer: RefCell<Vec<u8>>,
    finished: Cell<bool>,
    sequence_number: Cell<u32>,
}

impl ElevenLabsVoiceConversionStream {
    fn new(voice_id: String, client: ElevenLabsTtsApi) -> Self {
        Self {
            voice_id,
            client,
            audio_buffer: RefCell::new(Vec::new()),
            finished: Cell::new(false),
            sequence_number: Cell::new(0),
        }
    }
}

impl GuestVoiceConversionStream for ElevenLabsVoiceConversionStream {
    fn send_audio(&self, audio_data: Vec<u8>) -> Result<(), TtsError> {
        if self.finished.get() {
            return Err(TtsError::InternalError("Stream is finished".to_string()));
        }

        self.audio_buffer
            .borrow_mut()
            .extend_from_slice(&audio_data);
        Ok(())
    }

    fn receive_converted(&self) -> Result<Option<AudioChunk>, TtsError> {
        if self.finished.get() {
            return Ok(None);
        }

        let audio_data = self.audio_buffer.borrow().clone();
        if !audio_data.is_empty() {
            let request = crate::client::SpeechToSpeechRequest {
                audio_data,
                model_id: Some("eleven_english_sts_v2".to_string()),
                voice_settings: None,
                seed: None,
            };

            match self.client.speech_to_speech(&self.voice_id, &request, None) {
                Ok(converted_audio) => {
                    let seq = self.sequence_number.get();
                    self.sequence_number.set(seq + 1);

                    let chunk = AudioChunk {
                        data: converted_audio,
                        sequence_number: seq,
                        is_final: true,
                        timing_info: None,
                    };

                    self.audio_buffer.borrow_mut().clear();
                    self.finished.set(true);

                    return Ok(Some(chunk));
                }
                Err(e) => {
                    self.finished.set(true);
                    return Err(e);
                }
            }
        }

        Ok(None)
    }

    fn finish(&self) -> Result<(), TtsError> {
        self.finished.set(true);
        Ok(())
    }

    fn close(&self) {
        self.finished.set(true);
        self.audio_buffer.borrow_mut().clear();
    }
}

struct ElevenLabsPronunciationLexicon {
    name: String,
    language: LanguageCode,
    entries: RefCell<Vec<PronunciationEntry>>,
}

impl ElevenLabsPronunciationLexicon {
    fn new(name: String, language: LanguageCode, entries: Option<Vec<PronunciationEntry>>) -> Self {
        Self {
            name,
            language,
            entries: RefCell::new(entries.unwrap_or_default()),
        }
    }
}

impl GuestPronunciationLexicon for ElevenLabsPronunciationLexicon {
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
            part_of_speech: None,
        });
        Ok(())
    }

    fn remove_entry(&self, word: String) -> Result<(), TtsError> {
        self.entries.borrow_mut().retain(|entry| entry.word != word);
        Ok(())
    }

    fn export_content(&self) -> Result<String, TtsError> {
        let entries = self.entries.borrow();
        let mut content = format!(
            "# Pronunciation Lexicon: {}\n# Language: {}\n\n",
            self.name, self.language
        );

        for entry in entries.iter() {
            content.push_str(&format!("{}: {}\n", entry.word, entry.pronunciation));
        }

        Ok(content)
    }
}

struct ElevenLabsLongFormOperation {
    content: String,
    output_location: String,
    voice_id: String,
    client: ElevenLabsTtsApi,
    status: Cell<OperationStatus>,
    progress: Cell<f32>,
    audio_chunks: RefCell<Option<Vec<Vec<u8>>>>,
}

impl ElevenLabsLongFormOperation {
    fn new(
        content: String,
        output_location: String,
        voice_id: String,
        client: ElevenLabsTtsApi,
        _chapter_breaks: Option<Vec<u32>>,
    ) -> Self {
        Self {
            content,
            output_location,
            voice_id,
            client,
            status: Cell::new(OperationStatus::Processing),
            progress: Cell::new(0.0),
            audio_chunks: RefCell::new(None),
        }
    }

    fn process_long_form(&self) -> Result<(), TtsError> {
        let max_chunk_size = 4500;
        let chunks = self.client.synthesize_long_form_batch(
            &self.voice_id,
            &self.content,
            None,
            max_chunk_size,
        )?;

        self.audio_chunks.borrow_mut().replace(chunks);
        self.status.set(OperationStatus::Completed);
        self.progress.set(1.0);
        Ok(())
    }
}

impl GuestLongFormOperation for ElevenLabsLongFormOperation {
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
            if self.status.get() == OperationStatus::Processing {
                self.process_long_form()?;
            } else {
                return Err(TtsError::InternalError(
                    "Operation not completed".to_string(),
                ));
            }
        }

        let audio_chunks = self.audio_chunks.borrow();
        let chunks = audio_chunks
            .as_ref()
            .ok_or_else(|| TtsError::InternalError("Audio chunks not available".to_string()))?;

        let total_audio_size: usize = chunks.iter().map(|chunk| chunk.len()).sum();

        let estimated_duration = (total_audio_size as f64) / 16000.0;

        Ok(LongFormResult {
            output_location: self.output_location.clone(),
            total_duration: estimated_duration as f32,
            chapter_durations: None,
            metadata: golem_tts::golem::tts::types::SynthesisMetadata {
                duration_seconds: estimated_duration as f32,
                character_count: self.content.len() as u32,
                word_count: self.content.split_whitespace().count() as u32,
                audio_size_bytes: total_audio_size as u32,
                request_id: format!("elevenlabs-long-form-{}", self.voice_id),
                provider_info: Some("elevenlabs".to_string()),
            },
        })
    }
}

struct ElevenLabsComponent;

impl ElevenLabsComponent {
    const ENV_VAR_NAME: &'static str = "ELEVENLABS_API_KEY";
    const MODEL_VERSION_ENV_VAR: &'static str = "ELEVENLABS_MODEL_VERSION";

    fn create_client() -> Result<ElevenLabsTtsApi, TtsError> {
        with_config_key(Self::ENV_VAR_NAME, Err, |api_key| {
            let model_version = std::env::var(Self::MODEL_VERSION_ENV_VAR)
                .unwrap_or_else(|_| "eleven_multilingual_v2".to_string());
            Ok(ElevenLabsTtsApi::new(api_key.to_string(), model_version))
        })
    }
}

impl VoicesGuest for ElevenLabsComponent {
    type Voice = ElevenLabsVoiceImpl;
    type VoiceResults = ElevenLabsVoiceResults;

    fn list_voices(filter: Option<VoiceFilter>) -> Result<VoiceResults, TtsError> {
        let client = Self::create_client()?;
        let params = voice_filter_to_list_params(filter);

        match client.list_voices(params) {
            Ok(response) => {
                let voices: Vec<VoiceInfo> = response
                    .voices
                    .into_iter()
                    .map(elevenlabs_voice_to_voice_info)
                    .collect();

                Ok(VoiceResults::new(ElevenLabsVoiceResults::new(
                    voices,
                    response.total_count,
                )))
            }
            Err(e) => Err(e),
        }
    }

    fn get_voice(voice_id: String) -> Result<Voice, TtsError> {
        let client = Self::create_client()?;

        match client.get_voice(&voice_id) {
            Ok(voice_data) => {
                let voice_impl = ElevenLabsVoiceImpl::new(voice_data, client);
                Ok(Voice::new(voice_impl))
            }
            Err(e) => Err(e),
        }
    }

    fn search_voices(filter: Option<VoiceFilter>) -> Result<Vec<VoiceInfo>, TtsError> {
        let search_filter = filter.unwrap_or(VoiceFilter {
            language: None,
            gender: None,
            quality: None,
            supports_ssml: None,
            provider: None,
            search_query: None,
        });

        let client = Self::create_client()?;
        // ElevenLabs provides native API filtering based on gender, quality, and search query
        let params = voice_filter_to_list_params(Some(search_filter));

        match client.list_voices(params) {
            Ok(response) => {
                let voices = response
                    .voices
                    .into_iter()
                    .map(elevenlabs_voice_to_voice_info)
                    .collect();
                Ok(voices)
            }
            Err(e) => Err(e),
        }
    }

    fn list_languages() -> Result<Vec<LanguageInfo>, TtsError> {
        let client = Self::create_client()?;

        match client.get_models() {
            Ok(models) => Ok(models_to_language_info(models)),
            Err(e) => Err(e),
        }
    }
}

impl SynthesisGuest for ElevenLabsComponent {
    fn synthesize(
        input: TextInput,
        voice: golem_tts::golem::tts::voices::VoiceBorrow<'_>,
        options: Option<SynthesisOptions>,
    ) -> Result<SynthesisResult, TtsError> {
        validate_synthesis_input(&input, options.as_ref())?;

        let client = Self::create_client()?;
        let voice_id = voice.get::<ElevenLabsVoiceImpl>().get_id();

        let max_chunk_size = get_max_chars_for_model(client.get_model_version().into());

        if input.content.len() > max_chunk_size {
            trace!(
                "Using intelligent text chunking for content with {} characters (limit: {})",
                input.content.len(),
                max_chunk_size
            );

            let chunks = split_text_intelligently(&input.content, max_chunk_size);
            let mut all_audio_data = Vec::new();

            for chunk in chunks {
                let (mut request, params) =
                    synthesis_options_to_tts_request(options.clone(), client.get_model_version());
                request.text = chunk.clone();

                if let Some(language) = input.language.clone() {
                    let model_version = client.get_model_version();
                    let supports_language_code = !model_version.contains("multilingual");

                    if supports_language_code {
                        request.language_code = Some(language);
                    }
                }

                match client.text_to_speech(&voice_id, &request, params) {
                    Ok(audio_data) => {
                        all_audio_data.extend(audio_data);
                    }
                    Err(e) => return Err(e),
                }
            }

            Ok(audio_data_to_synthesis_result(
                all_audio_data,
                &input.content,
            ))
        } else {
            let (mut request, params) =
                synthesis_options_to_tts_request(options, client.get_model_version());
            request.text = input.content.clone();

            if let Some(language) = input.language {
                let model_version = client.get_model_version();
                let supports_language_code = !model_version.contains("multilingual");

                if supports_language_code {
                    request.language_code = Some(language);
                }
            }

            match client.text_to_speech(&voice_id, &request, params) {
                Ok(audio_data) => Ok(audio_data_to_synthesis_result(audio_data, &input.content)),
                Err(e) => Err(e),
            }
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

        let mut results = Vec::new();
        let client = Self::create_client()?;
        let voice_id = voice.get::<ElevenLabsVoiceImpl>().get_id();

        for input in inputs {
            let max_chunk_size = get_max_chars_for_model(client.get_model_version().into());

            if input.content.len() > max_chunk_size {
                trace!(
                    "Using intelligent text chunking for content with {} characters (limit: {})",
                    input.content.len(),
                    max_chunk_size
                );

                let chunks = split_text_intelligently(&input.content, max_chunk_size);
                let mut all_audio_data = Vec::new();

                for chunk in chunks {
                    let (mut request, params) = synthesis_options_to_tts_request(
                        options.clone(),
                        client.get_model_version(),
                    );
                    request.text = chunk.clone();

                    if let Some(language) = input.language.clone() {
                        let model_version = client.get_model_version();
                        let supports_language_code = !model_version.contains("multilingual");

                        if supports_language_code {
                            request.language_code = Some(language);
                        }
                    }

                    match client.text_to_speech(&voice_id, &request, params) {
                        Ok(audio_data) => {
                            all_audio_data.extend(audio_data);
                        }
                        Err(e) => return Err(e),
                    }
                }

                let result = audio_data_to_synthesis_result(all_audio_data, &input.content);
                results.push(result);
            } else {
                let (mut request, params) =
                    synthesis_options_to_tts_request(options.clone(), client.get_model_version());
                request.text = input.content.clone();

                if let Some(language) = input.language.clone() {
                    let model_version = client.get_model_version();
                    let supports_language_code = !model_version.contains("multilingual");

                    if supports_language_code {
                        request.language_code = Some(language);
                    }
                }

                match client.text_to_speech(&voice_id, &request, params) {
                    Ok(audio_data) => {
                        let result = audio_data_to_synthesis_result(audio_data, &input.content);
                        results.push(result);
                    }
                    Err(e) => return Err(e),
                }
            }
        }

        Ok(results)
    }

    fn get_timing_marks(
        _input: TextInput,
        _voice: golem_tts::golem::tts::voices::VoiceBorrow<'_>,
    ) -> Result<Vec<TimingInfo>, TtsError> {
        Err(TtsError::UnsupportedOperation(
            "Timing marks not supported by ElevenLabs".to_string(),
        ))
    }

    fn validate_input(
        input: TextInput,
        _voice: golem_tts::golem::tts::voices::VoiceBorrow<'_>,
    ) -> Result<ValidationResult, TtsError> {
        let client = Self::create_client()?;
        let model_version = client.get_model_version();

        Ok(crate::conversions::validate_text_input(
            &input.content,
            Some(model_version),
        ))
    }
}

impl StreamingGuest for ElevenLabsComponent {
    type SynthesisStream = ElevenLabsSynthesisStream;
    type VoiceConversionStream = ElevenLabsVoiceConversionStream;

    fn create_stream(
        voice: golem_tts::golem::tts::voices::VoiceBorrow<'_>,
        options: Option<SynthesisOptions>,
    ) -> Result<SynthesisStream, TtsError> {
        let client = Self::create_client()?;
        let voice_id = voice.get::<ElevenLabsVoiceImpl>().get_id();

        let stream = ElevenLabsSynthesisStream::new(voice_id, client, options);
        Ok(SynthesisStream::new(stream))
    }

    fn create_voice_conversion_stream(
        target_voice: golem_tts::golem::tts::voices::VoiceBorrow<'_>,
        _options: Option<SynthesisOptions>,
    ) -> Result<VoiceConversionStream, TtsError> {
        let client = Self::create_client()?;
        let voice_id = target_voice.get::<ElevenLabsVoiceImpl>().get_id();

        let stream = ElevenLabsVoiceConversionStream::new(voice_id, client);
        Ok(VoiceConversionStream::new(stream))
    }
}

impl AdvancedGuest for ElevenLabsComponent {
    type PronunciationLexicon = ElevenLabsPronunciationLexicon;
    type LongFormOperation = ElevenLabsLongFormOperation;

    fn create_voice_clone(
        name: String,
        audio_samples: Vec<AudioSample>,
        description: Option<String>,
    ) -> Result<Voice, TtsError> {
        let client = Self::create_client()?;
        let request = create_voice_request_from_samples(name, description, audio_samples);

        match client.create_voice(&request) {
            Ok(voice_data) => {
                let voice_impl = ElevenLabsVoiceImpl::new(voice_data, client);
                Ok(Voice::new(voice_impl))
            }
            Err(e) => Err(e),
        }
    }

    fn design_voice(_name: String, characteristics: VoiceDesignParams) -> Result<Voice, TtsError> {
        let client = Self::create_client()?;
        let request = voice_design_params_to_create_request(characteristics);

        match client.create_voice(&request) {
            Ok(voice_data) => {
                let voice_impl = ElevenLabsVoiceImpl::new(voice_data, client);
                Ok(Voice::new(voice_impl))
            }
            Err(e) => Err(e),
        }
    }

    fn convert_voice(
        input_audio: Vec<u8>,
        target_voice: golem_tts::golem::tts::voices::VoiceBorrow<'_>,
        _preserve_timing: Option<bool>,
    ) -> Result<Vec<u8>, TtsError> {
        let client = Self::create_client()?;
        let voice_id = target_voice.get::<ElevenLabsVoiceImpl>().get_id();

        let request = crate::client::SpeechToSpeechRequest {
            audio_data: input_audio,
            model_id: Some("eleven_english_sts_v2".to_string()),
            voice_settings: None,
            seed: None,
        };

        match client.speech_to_speech(&voice_id, &request, None) {
            Ok(converted_audio) => Ok(converted_audio),
            Err(e) => Err(e),
        }
    }

    fn generate_sound_effect(
        description: String,
        duration_seconds: Option<f32>,
        style_influence: Option<f32>,
    ) -> Result<Vec<u8>, TtsError> {
        let client = Self::create_client()?;

        let request = crate::client::SoundEffectRequest {
            text: description,
            duration_seconds: duration_seconds.map(|d| d as f64),
            prompt_influence: style_influence.map(|s| s as f64),
        };

        let params = crate::client::SoundEffectParams {
            output_format: Some("mp3_22050_32".to_string()),
        };

        match client.create_sound_effect(&request, Some(params)) {
            Ok(audio_data) => Ok(audio_data),
            Err(e) => Err(e),
        }
    }

    fn create_lexicon(
        name: String,
        language: LanguageCode,
        entries: Option<Vec<PronunciationEntry>>,
    ) -> Result<PronunciationLexicon, TtsError> {
        let lexicon = ElevenLabsPronunciationLexicon::new(name, language, entries);
        Ok(PronunciationLexicon::new(lexicon))
    }

    fn synthesize_long_form(
        content: String,
        voice: golem_tts::golem::tts::voices::VoiceBorrow<'_>,
        output_location: String,
        chapter_breaks: Option<Vec<u32>>,
    ) -> Result<LongFormOperation, TtsError> {
        let client = Self::create_client()?;
        let voice_id = voice.get::<ElevenLabsVoiceImpl>().get_id();

        let operation = ElevenLabsLongFormOperation::new(
            content,
            output_location,
            voice_id,
            client,
            chapter_breaks,
        );
        Ok(LongFormOperation::new(operation))
    }
}

impl ExtendedGuest for ElevenLabsComponent {
    fn unwrapped_synthesis_stream(
        voice: golem_tts::golem::tts::voices::VoiceBorrow<'_>,
        options: Option<SynthesisOptions>,
    ) -> Self::SynthesisStream {
        let client = Self::create_client().unwrap_or_else(|_| {
            ElevenLabsTtsApi::new("dummy".to_string(), "eleven_multilingual_v2".to_string())
        });
        let voice_id = voice.get::<ElevenLabsVoiceImpl>().get_id();

        ElevenLabsSynthesisStream::new(voice_id, client, options)
    }

    fn unwrapped_voice_conversion_stream(
        target_voice: golem_tts::golem::tts::voices::VoiceBorrow<'_>,
        _options: Option<SynthesisOptions>,
    ) -> Self::VoiceConversionStream {
        let client = Self::create_client().unwrap_or_else(|_| {
            ElevenLabsTtsApi::new("dummy".to_string(), "eleven_multilingual_v2".to_string())
        });
        let voice_id = target_voice.get::<ElevenLabsVoiceImpl>().get_id();

        ElevenLabsVoiceConversionStream::new(voice_id, client)
    }

    fn subscribe_synthesis_stream(_stream: &Self::SynthesisStream) -> Pollable {
        golem_rust::bindings::wasi::clocks::monotonic_clock::subscribe_duration(0)
    }

    fn subscribe_voice_conversion_stream(_stream: &Self::VoiceConversionStream) -> Pollable {
        golem_rust::bindings::wasi::clocks::monotonic_clock::subscribe_duration(0)
    }
}

type DurableElevenLabsComponent = DurableTts<ElevenLabsComponent>;

golem_tts::export_tts!(DurableElevenLabsComponent with_types_in golem_tts);
