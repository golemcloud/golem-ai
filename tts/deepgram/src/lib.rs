use crate::client::{get_available_models, DeepgramTtsApi, Model, RateLimitConfig};
use crate::conversions::{
    audio_data_to_synthesis_result, deepgram_model_to_voice_info, estimate_audio_duration,
    get_max_chars_for_model, models_to_language_info, split_text_intelligently,
    synthesis_options_to_tts_request, validate_synthesis_request, validate_text_input,
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
    AudioChunk, AudioFormat, LanguageCode, SynthesisMetadata, SynthesisResult, TextInput,
    TimingInfo, TtsError, VoiceGender, VoiceQuality, VoiceSettings,
};
use golem_tts::golem::tts::voices::{
    Guest as VoicesGuest, GuestVoice, GuestVoiceResults, LanguageInfo, Voice, VoiceFilter,
    VoiceInfo, VoiceResults,
};
use log::{info, warn};
use std::cell::{Cell, RefCell};

mod client;
mod conversions;

struct DeepgramVoiceImpl {
    model_data: Model,
    client: DeepgramTtsApi,
}

impl DeepgramVoiceImpl {
    fn new(model_data: Model, client: DeepgramTtsApi) -> Self {
        Self { model_data, client }
    }
}

impl GuestVoice for DeepgramVoiceImpl {
    fn get_id(&self) -> String {
        self.model_data.voice_id.clone()
    }

    fn get_name(&self) -> String {
        self.model_data.name.clone()
    }

    fn get_provider_id(&self) -> Option<String> {
        Some("Deepgram".to_string())
    }

    fn get_language(&self) -> LanguageCode {
        conversions::normalize_language_code(&self.model_data.language)
    }

    fn get_additional_languages(&self) -> Vec<LanguageCode> {
        vec![]
    }

    fn get_gender(&self) -> VoiceGender {
        conversions::parse_gender(&self.model_data.gender)
    }

    fn get_quality(&self) -> VoiceQuality {
        conversions::infer_quality_from_model(&self.model_data.voice_id)
    }

    fn get_description(&self) -> Option<String> {
        Some(format!(
            "{} voice with {} accent, {}. Characteristics: {}. Suitable for: {}",
            self.model_data.gender,
            self.model_data.accent,
            self.model_data.age,
            self.model_data.characteristics.join(", "),
            self.model_data.use_cases.join(", ")
        ))
    }

    fn supports_ssml(&self) -> bool {
        false
    }

    fn get_sample_rates(&self) -> Vec<u32> {
        vec![8000, 16000, 22050, 24000, 32000, 48000]
    }

    fn get_supported_formats(&self) -> Vec<AudioFormat> {
        vec![
            AudioFormat::Mp3,
            AudioFormat::Wav,
            AudioFormat::Pcm,
            AudioFormat::OggOpus,
            AudioFormat::Aac,
            AudioFormat::Flac,
            AudioFormat::Mulaw,
            AudioFormat::Alaw,
        ]
    }

    fn update_settings(&self, _settings: VoiceSettings) -> Result<(), TtsError> {
        Err(TtsError::UnsupportedOperation(
            "Deepgram does not support voice settings updates".to_string(),
        ))
    }

    fn delete(&self) -> Result<(), TtsError> {
        Err(TtsError::UnsupportedOperation(
            "Deepgram voices cannot be deleted".to_string(),
        ))
    }

    fn clone(&self) -> Result<Voice, TtsError> {
        Err(TtsError::UnsupportedOperation(
            "Deepgram does not support voice cloning".to_string(),
        ))
    }

    fn preview(&self, text: String) -> Result<Vec<u8>, TtsError> {
        let (request, params) = synthesis_options_to_tts_request(text, None)?;
        let mut params = params.unwrap();
        params.model = Some(self.model_data.voice_id.clone());

        self.client.text_to_speech(&request, Some(&params))
    }
}

struct DeepgramVoiceResults {
    voices: RefCell<Vec<VoiceInfo>>,
    current_index: Cell<usize>,
}

impl DeepgramVoiceResults {
    fn new(voices: Vec<VoiceInfo>) -> Self {
        Self {
            voices: RefCell::new(voices),
            current_index: Cell::new(0),
        }
    }
}

impl GuestVoiceResults for DeepgramVoiceResults {
    fn has_more(&self) -> bool {
        self.current_index.get() < self.voices.borrow().len()
    }

    fn get_next(&self) -> Result<Vec<VoiceInfo>, TtsError> {
        let voices = self.voices.borrow();
        let current = self.current_index.get();

        if current >= voices.len() {
            return Ok(vec![]);
        }

        let remaining: Vec<VoiceInfo> = voices[current..].to_vec();
        self.current_index.set(voices.len());

        Ok(remaining)
    }

    fn get_total_count(&self) -> Option<u32> {
        Some(self.voices.borrow().len() as u32)
    }
}

#[warn(dead_code)]
struct DeepgramSynthesisStream {
    client: DeepgramTtsApi,
    current_request: RefCell<Option<crate::client::TextToSpeechRequest>>,
    params: RefCell<Option<crate::client::TextToSpeechParams>>,
    response_stream: RefCell<Option<reqwest::Response>>,
    chunk_buffer: RefCell<Vec<u8>>,
    bytes_streamed: Cell<usize>,
    total_chunks_received: Cell<u32>,
    finished: Cell<bool>,
    sequence_number: Cell<u32>,
    stream_started: Cell<bool>,
}

impl DeepgramSynthesisStream {
    fn new(voice_id: String, client: DeepgramTtsApi, options: Option<SynthesisOptions>) -> Self {
        let (request, params) = synthesis_options_to_tts_request(String::new(), options)
            .unwrap_or_else(|_| {
                let request = crate::client::TextToSpeechRequest {
                    text: String::new(),
                };
                let params = Some(crate::client::TextToSpeechParams {
                    model: Some(voice_id.clone()),
                    encoding: Some("linear16".to_string()),
                    container: Some("wav".to_string()),
                    sample_rate: Some(24000),
                    bit_rate: None,
                });
                (request, params)
            });
        let mut params = params.unwrap();
        params.model = Some(voice_id.clone());

        Self {
            client,
            current_request: RefCell::new(Some(request)),
            params: RefCell::new(Some(params)),
            response_stream: RefCell::new(None),
            chunk_buffer: RefCell::new(Vec::new()),
            bytes_streamed: Cell::new(0),
            total_chunks_received: Cell::new(0),
            finished: Cell::new(false),
            sequence_number: Cell::new(0),
            stream_started: Cell::new(false),
        }
    }

    #[allow(dead_code)]
    fn get_progress(&self) -> (usize, u32) {
        (self.bytes_streamed.get(), self.total_chunks_received.get())
    }
}

impl GuestSynthesisStream for DeepgramSynthesisStream {
    fn send_text(&self, input: TextInput) -> Result<(), TtsError> {
        info!("[DEEPGRAM] send_text called with: '{}'", input.content);

        if self.finished.get() {
            warn!("[DEEPGRAM] Stream already finished, returning error");
            return Err(TtsError::InvalidConfiguration(
                "Stream already finished".to_string(),
            ));
        }

        {
            let mut request_ref = self.current_request.borrow_mut();
            if let Some(mut request) = request_ref.take() {
                info!(
                    "[DEEPGRAM] Updating request text from '{}' to '{}'",
                    request.text, input.content
                );
                request.text = input.content;
                *request_ref = Some(request);
            } else {
                warn!("[DEEPGRAM] Warning: No current request to update");
            }
        }

        Ok(())
    }

    fn finish(&self) -> Result<(), TtsError> {
        info!("[DEEPGRAM] finish() called");

        if self.stream_started.get() {
            info!("[DEEPGRAM] Stream already started, returning OK");
            return Ok(());
        }

        warn!("[DEEPGRAM] Checking current request state...");
        let request_debug = self.current_request.borrow();
        match request_debug.as_ref() {
            Some(req) => {
                info!(
                    "[DEEPGRAM] Request exists with text: '{}' (length: {})",
                    req.text,
                    req.text.len()
                );
            }
            None => {
                warn!("[DEEPGRAM] No request found!");
                return Err(TtsError::InternalError("No request available".to_string()));
            }
        }
        drop(request_debug);

        if let Some(request) = self.current_request.borrow().as_ref() {
            warn!("[DEEPGRAM] Current request text: '{}'", request.text);
            if !request.text.is_empty() {
                warn!("[DEEPGRAM] Making API call to Deepgram...");

                if let Some(params) = self.params.borrow().as_ref() {
                    warn!(
                        "[DEEPGRAM] Params - model: {:?}, encoding: {:?}, sample_rate: {:?}",
                        params.model, params.encoding, params.sample_rate
                    );
                } else {
                    warn!("[DEEPGRAM] Warning: No params available");
                }

                match self
                    .client
                    .text_to_speech_stream(request, self.params.borrow().as_ref())
                {
                    Ok(response) => {
                        *self.response_stream.borrow_mut() = Some(response);
                        self.stream_started.set(true);
                    }
                    Err(e) => {
                        self.finished.set(true);
                        return Err(e);
                    }
                }
            } else {
                return Err(TtsError::InvalidText(
                    "No text provided for synthesis".to_string(),
                ));
            }
        } else {
            return Err(TtsError::InternalError("No request available".to_string()));
        }

        Ok(())
    }

    fn receive_chunk(&self) -> Result<Option<AudioChunk>, TtsError> {
        if self.finished.get() {
            return Ok(None);
        }

        if !self.stream_started.get() && self.has_pending_audio() {
            self.finish()?;
        }

        if let Some(response) = self.response_stream.borrow_mut().take() {
            const CHUNK_SIZE: usize = 8192;

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
                            data: chunk_data,
                            sequence_number: sequence,
                            is_final,
                            timing_info: None,
                        };

                        return Ok(Some(chunk));
                    }

                    Ok(None)
                }
                Err(e) => {
                    self.finished.set(true);
                    Err(TtsError::NetworkError(format!(
                        "Failed to read response: {}",
                        e
                    )))
                }
            }
        } else {
            if self.stream_started.get() && self.chunk_buffer.borrow().is_empty() {
                self.finished.set(true);
            }
            Ok(None)
        }
    }

    fn has_pending_audio(&self) -> bool {
        !self.finished.get()
            && (self.response_stream.borrow().is_some()
                || !self.chunk_buffer.borrow().is_empty()
                || (!self.stream_started.get()
                    && self
                        .current_request
                        .borrow()
                        .as_ref()
                        .is_some_and(|r| !r.text.is_empty())))
    }

    fn get_status(&self) -> StreamStatus {
        if self.finished.get() {
            StreamStatus::Finished
        } else if self.stream_started.get() || self.response_stream.borrow().is_some() {
            StreamStatus::Processing
        } else {
            StreamStatus::Ready
        }
    }

    fn close(&self) {
        self.finished.set(true);
        self.stream_started.set(false);
        *self.response_stream.borrow_mut() = None;
        self.chunk_buffer.borrow_mut().clear();
    }
}

struct DeepgramVoiceConversionStream {
    _voice_id: String,
}

impl DeepgramVoiceConversionStream {
    fn new(voice_id: String, _client: DeepgramTtsApi) -> Self {
        Self {
            _voice_id: voice_id,
        }
    }
}

impl GuestVoiceConversionStream for DeepgramVoiceConversionStream {
    fn send_audio(&self, _audio_data: Vec<u8>) -> Result<(), TtsError> {
        Err(TtsError::UnsupportedOperation(
            "Deepgram does not support voice conversion".to_string(),
        ))
    }

    fn receive_converted(&self) -> Result<Option<AudioChunk>, TtsError> {
        Err(TtsError::UnsupportedOperation(
            "Deepgram does not support voice conversion".to_string(),
        ))
    }

    fn finish(&self) -> Result<(), TtsError> {
        Err(TtsError::UnsupportedOperation(
            "Deepgram does not support voice conversion".to_string(),
        ))
    }

    fn close(&self) {}
}

struct DeepgramPronunciationLexicon {
    _name: String,
}

impl DeepgramPronunciationLexicon {
    fn new(
        name: String,
        _language: LanguageCode,
        _entries: Option<Vec<PronunciationEntry>>,
    ) -> Self {
        Self { _name: name }
    }
}

impl GuestPronunciationLexicon for DeepgramPronunciationLexicon {
    fn get_name(&self) -> String {
        self._name.clone()
    }

    fn get_language(&self) -> LanguageCode {
        "en".to_string()
    }

    fn get_entry_count(&self) -> u32 {
        0
    }

    fn add_entry(&self, _word: String, _pronunciation: String) -> Result<(), TtsError> {
        Err(TtsError::UnsupportedOperation(
            "Deepgram does not support pronunciation lexicons".to_string(),
        ))
    }

    fn remove_entry(&self, _word: String) -> Result<(), TtsError> {
        Err(TtsError::UnsupportedOperation(
            "Deepgram does not support pronunciation lexicons".to_string(),
        ))
    }

    fn export_content(&self) -> Result<String, TtsError> {
        Err(TtsError::UnsupportedOperation(
            "Deepgram does not support pronunciation lexicons".to_string(),
        ))
    }
}

struct DeepgramLongFormOperation {
    content: String,
    voice_id: String,
    client: DeepgramTtsApi,
    status: Cell<OperationStatus>,
    progress: Cell<f32>,
    audio_chunks: RefCell<Option<Vec<Vec<u8>>>>,
}

impl DeepgramLongFormOperation {
    fn new(
        content: String,
        _output_location: String,
        voice_id: String,
        client: DeepgramTtsApi,
        _chapter_breaks: Option<Vec<u32>>,
    ) -> Self {
        Self {
            content,
            voice_id,
            client,
            status: Cell::new(OperationStatus::Pending),
            progress: Cell::new(0.0),
            audio_chunks: RefCell::new(None),
        }
    }

    fn process_long_form(&self) -> Result<(), TtsError> {
        self.status.set(OperationStatus::Processing);

        let chunks = split_text_intelligently(&self.content, 1000);
        let mut audio_chunks = Vec::new();

        for (i, chunk) in chunks.iter().enumerate() {
            let (request, mut params) = synthesis_options_to_tts_request(chunk.clone(), None)?;
            if let Some(ref mut p) = params {
                p.model = Some(self.voice_id.clone());
            }

            match self
                .client
                .text_to_speech_with_metadata(&request, params.as_ref())
            {
                Ok(tts_response) => {
                    audio_chunks.push(tts_response.audio_data);
                    self.progress.set((i + 1) as f32 / chunks.len() as f32);
                }
                Err(e) => {
                    self.status.set(OperationStatus::Failed);
                    return Err(e);
                }
            }
        }

        *self.audio_chunks.borrow_mut() = Some(audio_chunks);
        self.status.set(OperationStatus::Completed);
        self.progress.set(1.0);

        Ok(())
    }
}

impl GuestLongFormOperation for DeepgramLongFormOperation {
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
            return Err(TtsError::InvalidConfiguration(
                "Operation not completed".to_string(),
            ));
        }

        if let Some(chunks) = self.audio_chunks.borrow().as_ref() {
            let mut combined_audio = Vec::new();
            for chunk in chunks {
                combined_audio.extend_from_slice(chunk);
            }

            let result = LongFormResult {
                output_location: "deepgram-synthesis".to_string(),
                total_duration: estimate_audio_duration(&combined_audio, 24000),
                chapter_durations: None,
                metadata: audio_data_to_synthesis_result(
                    combined_audio.clone(),
                    &self.content,
                    "linear16",
                    24000,
                )
                .metadata,
            };

            Ok(result)
        } else {
            Err(TtsError::InternalError(
                "No audio data available".to_string(),
            ))
        }
    }
}

struct DeepgramComponent;

impl DeepgramComponent {
    const ENV_VAR_NAME: &'static str = "DEEPGRAM_API_KEY";
    const API_VERSION_ENV_VAR: &'static str = "DEEPGRAM_API_VERSION";

    fn create_client() -> Result<DeepgramTtsApi, TtsError> {
        with_config_key(Self::ENV_VAR_NAME, Err, |api_key| {
            let api_version =
                std::env::var(Self::API_VERSION_ENV_VAR).unwrap_or_else(|_| "v1".to_string());
            Ok(DeepgramTtsApi::new(api_key.to_string(), api_version))
        })
    }

    fn create_client_with_rate_limit(
        rate_limit_config: RateLimitConfig,
    ) -> Result<DeepgramTtsApi, TtsError> {
        with_config_key(Self::ENV_VAR_NAME, Err, |api_key| {
            let api_version =
                std::env::var(Self::API_VERSION_ENV_VAR).unwrap_or_else(|_| "v1".to_string());
            Ok(DeepgramTtsApi::new(api_key.to_string(), api_version)
                .with_rate_limit_config(rate_limit_config))
        })
    }

    fn create_batch_client() -> Result<DeepgramTtsApi, TtsError> {
        let batch_config = RateLimitConfig {
            max_retries: 5,
            initial_delay: std::time::Duration::from_millis(500),
            max_delay: std::time::Duration::from_secs(60),
            backoff_multiplier: 1.5,
        };
        Self::create_client_with_rate_limit(batch_config)
    }

    fn create_streaming_client() -> Result<DeepgramTtsApi, TtsError> {
        let streaming_config = RateLimitConfig {
            max_retries: 3,
            initial_delay: std::time::Duration::from_millis(200),
            max_delay: std::time::Duration::from_secs(5),
            backoff_multiplier: 2.0,
        };
        Self::create_client_with_rate_limit(streaming_config)
    }
}

impl VoicesGuest for DeepgramComponent {
    type Voice = DeepgramVoiceImpl;
    type VoiceResults = DeepgramVoiceResults;

    fn list_voices(filter: Option<VoiceFilter>) -> Result<VoiceResults, TtsError> {
        let client = Self::create_client()?;
        let models = get_available_models();

        if let Some(f) = filter.as_ref() {
            let mut voice_filters = crate::client::VoiceFilters::new();

            if let Some(lang) = &f.language {
                voice_filters = voice_filters.with_language(lang.clone());
            }

            if let Some(gender) = f.gender {
                let gender_str = match gender {
                    VoiceGender::Male => "masculine",
                    VoiceGender::Female => "feminine",
                    VoiceGender::Neutral => "neutral",
                };
                voice_filters = voice_filters.with_gender(gender_str.to_string());
            }

            if let Some(quality) = f.quality {
                let _quality_filter = match quality {
                    VoiceQuality::Standard => crate::client::VoiceQuality::Standard,
                    VoiceQuality::Premium => crate::client::VoiceQuality::Premium,
                    VoiceQuality::Neural => crate::client::VoiceQuality::Professional,
                    VoiceQuality::Studio => crate::client::VoiceQuality::Professional,
                };
                voice_filters = voice_filters.with_version(crate::client::ModelVersion::Aura2);
            }

            if let Some(query) = &f.search_query {
                voice_filters = voice_filters.with_search(query.clone());
            }

            let filtered_response = client.get_models_filtered(&voice_filters)?;
            let voice_infos: Vec<VoiceInfo> = filtered_response
                .models
                .into_iter()
                .map(deepgram_model_to_voice_info)
                .collect();

            return Ok(VoiceResults::new(DeepgramVoiceResults::new(voice_infos)));
        }

        let mut voice_infos: Vec<VoiceInfo> = models
            .into_iter()
            .map(deepgram_model_to_voice_info)
            .collect();

        if let Some(f) = filter {
            voice_infos.retain(|voice| {
                let mut matches = true;

                if let Some(gender) = f.gender {
                    matches = matches && voice.gender == gender;
                }

                if let Some(quality) = f.quality {
                    matches = matches && voice.quality == quality;
                }

                if let Some(lang) = &f.language {
                    matches = matches && voice.language == *lang;
                }

                if let Some(provider) = &f.provider {
                    matches = matches && voice.provider.contains(provider);
                }

                if let Some(query) = &f.search_query {
                    let query_lower = query.to_lowercase();
                    matches = matches
                        && (voice.name.to_lowercase().contains(&query_lower)
                            || voice
                                .description
                                .as_ref()
                                .is_some_and(|d| d.to_lowercase().contains(&query_lower))
                            || voice
                                .use_cases
                                .iter()
                                .any(|uc| uc.to_lowercase().contains(&query_lower)));
                }

                matches
            });
        }

        Ok(VoiceResults::new(DeepgramVoiceResults::new(voice_infos)))
    }

    fn get_voice(voice_id: String) -> Result<Voice, TtsError> {
        let client = Self::create_client()?;
        let models = get_available_models();

        if let Some(model) = models.into_iter().find(|m| m.voice_id == voice_id) {
            Ok(Voice::new(DeepgramVoiceImpl::new(model, client)))
        } else {
            Err(TtsError::VoiceNotFound(format!(
                "Voice '{}' not found",
                voice_id
            )))
        }
    }

    fn search_voices(
        query: String,
        filter: Option<VoiceFilter>,
    ) -> Result<Vec<VoiceInfo>, TtsError> {
        let client = Self::create_client()?;

        let search_results = client.search_models(&query)?;
        let mut voice_infos: Vec<VoiceInfo> = search_results
            .into_iter()
            .map(deepgram_model_to_voice_info)
            .collect();

        if let Some(f) = filter {
            voice_infos.retain(|voice| {
                let mut matches = true;

                if let Some(gender) = f.gender {
                    matches = matches && voice.gender == gender;
                }

                if let Some(quality) = f.quality {
                    matches = matches && voice.quality == quality;
                }

                if let Some(lang) = &f.language {
                    matches = matches && voice.language == *lang;
                }

                if let Some(provider) = &f.provider {
                    matches = matches && voice.provider.contains(provider);
                }

                matches
            });
        }

        Ok(voice_infos)
    }

    fn list_languages() -> Result<Vec<LanguageInfo>, TtsError> {
        let models = get_available_models();
        Ok(models_to_language_info(models))
    }
}

impl SynthesisGuest for DeepgramComponent {
    fn synthesize(
        input: TextInput,
        voice: golem_tts::golem::tts::voices::VoiceBorrow<'_>,
        options: Option<SynthesisOptions>,
    ) -> Result<SynthesisResult, TtsError> {
        validate_synthesis_request(
            &input.content,
            input.text_type,
            input.language.as_deref(),
            options.as_ref(),
        )?;

        let client = Self::create_client()?;
        let voice_id = voice.get::<DeepgramVoiceImpl>().get_id();

        let max_chars = get_max_chars_for_model(Some(&voice_id));

        if input.content.len() <= max_chars {
            let (request, mut params) =
                synthesis_options_to_tts_request(input.content.clone(), options)?;
            if let Some(ref mut p) = params {
                p.model = Some(voice_id);
            }

            let tts_response = client.text_to_speech_with_metadata(&request, params.as_ref())?;
            let encoding = params
                .as_ref()
                .and_then(|p| p.encoding.as_ref())
                .unwrap_or(&"linear16".to_string())
                .clone();
            let sample_rate = params.as_ref().and_then(|p| p.sample_rate).unwrap_or(24000);

            let mut synthesis_result = audio_data_to_synthesis_result(
                tts_response.audio_data,
                &input.content,
                &encoding,
                sample_rate,
            );

            synthesis_result.metadata.provider_info = Some(format!(
                "Deepgram TTS - Model: {}, Characters: {}",
                tts_response.metadata.dg_model_name, tts_response.metadata.dg_char_count
            ));

            Ok(synthesis_result)
        } else {
            let chunks = split_text_intelligently(&input.content, max_chars);
            let mut combined_audio = Vec::new();
            let mut total_characters = 0u32;
            let mut total_words = 0u32;
            let mut total_duration = 0.0f32;

            for (chunk_index, chunk) in chunks.iter().enumerate() {
                let (request, mut params) =
                    synthesis_options_to_tts_request(chunk.clone(), options.clone())?;
                if let Some(ref mut p) = params {
                    p.model = Some(voice_id.clone());
                }

                let tts_response =
                    client.text_to_speech_with_metadata(&request, params.as_ref())?;

                combined_audio.extend_from_slice(&tts_response.audio_data);

                total_characters += chunk.chars().count() as u32;
                total_words += chunk.split_whitespace().count() as u32;

                let encoding = params
                    .as_ref()
                    .and_then(|p| p.encoding.as_ref())
                    .cloned()
                    .unwrap_or_else(|| "linear16".to_string());
                let sample_rate = params.as_ref().and_then(|p| p.sample_rate).unwrap_or(24000);
                total_duration += estimate_audio_duration(&tts_response.audio_data, sample_rate);

                if chunk_index < chunks.len() - 1 {
                    let silence_samples = (sample_rate as f32 * 0.1) as usize;
                    let silence_bytes = match encoding.as_str() {
                        "linear16" => silence_samples * 2,
                        "mulaw" | "alaw" => silence_samples,
                        _ => silence_samples * 2,
                    };
                    combined_audio.extend(vec![0u8; silence_bytes]);
                    total_duration += 0.1;
                }
            }

            let audio_size_bytes = combined_audio.len() as u32;
            let synthesis_result = SynthesisResult {
                audio_data: combined_audio,
                metadata: SynthesisMetadata {
                    duration_seconds: total_duration,
                    character_count: total_characters,
                    word_count: total_words,
                    audio_size_bytes,
                    request_id: format!("deepgram-chunked-{}", chrono::Utc::now().timestamp()),
                    provider_info: Some(format!(
                        "Deepgram TTS - Model: {}, Chunks: {}, Total Characters: {}",
                        voice_id,
                        chunks.len(),
                        total_characters
                    )),
                },
            };

            Ok(synthesis_result)
        }
    }

    fn synthesize_batch(
        inputs: Vec<TextInput>,
        voice: golem_tts::golem::tts::voices::VoiceBorrow<'_>,
        options: Option<SynthesisOptions>,
    ) -> Result<Vec<SynthesisResult>, TtsError> {
        let mut results = Vec::new();
        let client = Self::create_batch_client()?;
        let voice_id = voice.get::<DeepgramVoiceImpl>().get_id();

        for input in inputs {
            validate_synthesis_request(
                &input.content,
                input.text_type,
                input.language.as_deref(),
                options.as_ref(),
            )?;

            let (request, mut params) =
                synthesis_options_to_tts_request(input.content.clone(), options.clone())?;
            if let Some(ref mut p) = params {
                p.model = Some(voice_id.clone());
            }

            match client.text_to_speech_with_metadata(&request, params.as_ref()) {
                Ok(tts_response) => {
                    let encoding = params
                        .as_ref()
                        .and_then(|p| p.encoding.as_ref())
                        .unwrap_or(&"linear16".to_string())
                        .clone();
                    let sample_rate = params.as_ref().and_then(|p| p.sample_rate).unwrap_or(24000);

                    let mut synthesis_result = audio_data_to_synthesis_result(
                        tts_response.audio_data,
                        &input.content,
                        &encoding,
                        sample_rate,
                    );

                    synthesis_result.metadata.provider_info = Some(format!(
                        "Deepgram TTS - Model: {}, Characters: {}",
                        tts_response.metadata.dg_model_name, tts_response.metadata.dg_char_count
                    ));

                    results.push(synthesis_result);
                }
                Err(e) => {
                    return Err(e);
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
            "Timing marks not supported by Deepgram".to_string(),
        ))
    }

    fn validate_input(
        input: TextInput,
        voice: golem_tts::golem::tts::voices::VoiceBorrow<'_>,
    ) -> Result<ValidationResult, TtsError> {
        let voice_id = voice.get::<DeepgramVoiceImpl>().get_id();

        let mut _is_valid = true;
        let mut messages = Vec::new();

        if input.content.is_empty() {
            _is_valid = false;
            messages.push("Text input cannot be empty".to_string());
        }

        let char_limit = if voice_id.starts_with("aura-2") {
            2000
        } else {
            1000
        };
        if input.content.len() > char_limit {
            _is_valid = false;
            messages.push(format!(
                "Text exceeds {} character limit for {}",
                char_limit, voice_id
            ));
        }

        if input
            .content
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\r' && c != '\t')
        {
            messages.push(
                "Warning: Text contains control characters that may not be processed correctly"
                    .to_string(),
            );
        }

        let _message = if messages.is_empty() {
            None
        } else {
            Some(messages.join("; "))
        };

        Ok(validate_text_input(&input.content, Some(&voice_id)))
    }
}

impl StreamingGuest for DeepgramComponent {
    type SynthesisStream = DeepgramSynthesisStream;
    type VoiceConversionStream = DeepgramVoiceConversionStream;

    fn create_stream(
        voice: golem_tts::golem::tts::voices::VoiceBorrow<'_>,
        options: Option<SynthesisOptions>,
    ) -> Result<SynthesisStream, TtsError> {
        let client = Self::create_streaming_client()?;
        let voice_id = voice.get::<DeepgramVoiceImpl>().get_id();

        let stream = DeepgramSynthesisStream::new(voice_id, client, options);
        Ok(SynthesisStream::new(stream))
    }

    fn create_voice_conversion_stream(
        target_voice: golem_tts::golem::tts::voices::VoiceBorrow<'_>,
        _options: Option<SynthesisOptions>,
    ) -> Result<VoiceConversionStream, TtsError> {
        let client = Self::create_client()?;
        let voice_id = target_voice.get::<DeepgramVoiceImpl>().get_id();

        let stream = DeepgramVoiceConversionStream::new(voice_id, client);
        Ok(VoiceConversionStream::new(stream))
    }
}

impl AdvancedGuest for DeepgramComponent {
    type PronunciationLexicon = DeepgramPronunciationLexicon;
    type LongFormOperation = DeepgramLongFormOperation;

    fn create_voice_clone(
        _name: String,
        _audio_samples: Vec<AudioSample>,
        _description: Option<String>,
    ) -> Result<Voice, TtsError> {
        Err(TtsError::UnsupportedOperation(
            "Deepgram does not support voice cloning".to_string(),
        ))
    }

    fn design_voice(_name: String, _characteristics: VoiceDesignParams) -> Result<Voice, TtsError> {
        Err(TtsError::UnsupportedOperation(
            "Deepgram does not support voice design".to_string(),
        ))
    }

    fn convert_voice(
        _input_audio: Vec<u8>,
        _target_voice: golem_tts::golem::tts::voices::VoiceBorrow<'_>,
        _preserve_timing: Option<bool>,
    ) -> Result<Vec<u8>, TtsError> {
        Err(TtsError::UnsupportedOperation(
            "Deepgram does not support voice conversion".to_string(),
        ))
    }

    fn generate_sound_effect(
        _description: String,
        _duration_seconds: Option<f32>,
        _style_influence: Option<f32>,
    ) -> Result<Vec<u8>, TtsError> {
        Err(TtsError::UnsupportedOperation(
            "Deepgram does not support sound effect generation".to_string(),
        ))
    }

    fn create_lexicon(
        name: String,
        language: LanguageCode,
        entries: Option<Vec<PronunciationEntry>>,
    ) -> Result<PronunciationLexicon, TtsError> {
        let lexicon = DeepgramPronunciationLexicon::new(name, language, entries);
        Ok(PronunciationLexicon::new(lexicon))
    }

    fn synthesize_long_form(
        content: String,
        voice: golem_tts::golem::tts::voices::VoiceBorrow<'_>,
        output_location: String,
        chapter_breaks: Option<Vec<u32>>,
    ) -> Result<LongFormOperation, TtsError> {
        let client = Self::create_batch_client()?;
        let voice_id = voice.get::<DeepgramVoiceImpl>().get_id();

        let operation = DeepgramLongFormOperation::new(
            content,
            output_location,
            voice_id,
            client,
            chapter_breaks,
        );

        operation.process_long_form()?;

        Ok(LongFormOperation::new(operation))
    }
}

impl ExtendedGuest for DeepgramComponent {
    fn unwrapped_synthesis_stream(
        voice: golem_tts::golem::tts::voices::VoiceBorrow<'_>,
        options: Option<SynthesisOptions>,
    ) -> Self::SynthesisStream {
        let client = Self::create_streaming_client()
            .unwrap_or_else(|_| DeepgramTtsApi::new("dummy".to_string(), "v1".to_string()));
        let voice_id = voice.get::<DeepgramVoiceImpl>().get_id();

        DeepgramSynthesisStream::new(voice_id, client, options)
    }

    fn unwrapped_voice_conversion_stream(
        target_voice: golem_tts::golem::tts::voices::VoiceBorrow<'_>,
        _options: Option<SynthesisOptions>,
    ) -> Self::VoiceConversionStream {
        let client = Self::create_client()
            .unwrap_or_else(|_| DeepgramTtsApi::new("dummy".to_string(), "v1".to_string()));
        let voice_id = target_voice.get::<DeepgramVoiceImpl>().get_id();

        DeepgramVoiceConversionStream::new(voice_id, client)
    }

    fn subscribe_synthesis_stream(_stream: &Self::SynthesisStream) -> Pollable {
        golem_rust::bindings::wasi::clocks::monotonic_clock::subscribe_duration(0)
    }

    fn subscribe_voice_conversion_stream(_stream: &Self::VoiceConversionStream) -> Pollable {
        golem_rust::bindings::wasi::clocks::monotonic_clock::subscribe_duration(0)
    }
}

type DurableDeepgramComponent = DurableTts<DeepgramComponent>;

golem_tts::export_tts!(DurableDeepgramComponent with_types_in golem_tts);
