use crate::exports::golem::tts::advanced::{
    AudioSample as WitAudioSample, LongFormResult as WitLongFormResult,
    VoiceDesignParams as WitVoiceDesignParams,
};
use crate::exports::golem::tts::streaming::SynthesisOptions as WitSynthesisOptions;
use crate::exports::golem::tts::types::{
    AudioChunk as WitAudioChunk, SynthesisResult as WitSynthesisResult, TextInput as WitTextInput,
    TimingInfo as WitTimingInfo, TtsError as WitTtsError,
};
use crate::exports::golem::tts::synthesis::ValidationResult as WitValidationResult;
use crate::exports::golem::tts::voices::{VoiceFilter as WitVoiceFilter, VoiceInfo as WitVoiceInfo};

pub struct SynthesisRequest {
    pub input: WitTextInput,
    pub voice_id: String,
    pub options: Option<WitSynthesisOptions>,
}

pub struct StreamRequest {
    pub voice_id: String,
    pub options: Option<WitSynthesisOptions>,
}

pub struct StreamChunk {
    pub data: Vec<u8>,
    pub sequence_number: u32,
    pub is_final: bool,
    pub timing_info: Option<WitTimingInfo>,
}

pub trait TtsGuest {
    fn list_voices(filter: Option<WitVoiceFilter>) -> Result<Vec<WitVoiceInfo>, WitTtsError>;
    fn get_voice(voice_id: String) -> Result<WitVoiceInfo, WitTtsError>;
    fn search_voices(
        query: String,
        filter: Option<WitVoiceFilter>,
    ) -> Result<Vec<WitVoiceInfo>, WitTtsError>;
    fn list_languages() -> Result<Vec<String>, WitTtsError>;

    fn synthesize(request: SynthesisRequest) -> Result<WitSynthesisResult, WitTtsError>;
    fn synthesize_batch(
        requests: Vec<SynthesisRequest>,
    ) -> Result<Vec<WitSynthesisResult>, WitTtsError>;
    fn get_timing_marks(
        input: WitTextInput,
        voice_id: String,
    ) -> Result<Vec<WitTimingInfo>, WitTtsError>;
    fn validate_input(
        input: WitTextInput,
        voice_id: String,
    ) -> Result<WitValidationResult, WitTtsError>;

    type SynthesisStream: TtsStreamGuest;
    fn create_stream(request: StreamRequest) -> Result<Self::SynthesisStream, WitTtsError>;
    type VoiceConversionStream: VoiceConversionStreamGuest;
    fn create_voice_conversion_stream(
        request: StreamRequest,
    ) -> Result<Self::VoiceConversionStream, WitTtsError>;

    fn create_voice_clone(
        name: String,
        audio_samples: Vec<WitAudioSample>,
        description: Option<String>,
    ) -> Result<String, WitTtsError>;
    fn design_voice(
        name: String,
        characteristics: WitVoiceDesignParams,
    ) -> Result<String, WitTtsError>;
    fn convert_voice(
        input_audio: Vec<u8>,
        target_voice: String,
        preserve_timing: Option<bool>,
    ) -> Result<WitSynthesisResult, WitTtsError>;
    fn generate_sound_effect(
        description: String,
        duration_seconds: Option<f32>,
        style_influence: Option<f32>,
    ) -> Result<WitSynthesisResult, WitTtsError>;
    fn synthesize_long_form(
        content: String,
        voice_id: String,
        output_location: String,
        chapter_breaks: Option<Vec<u32>>,
    ) -> Result<WitLongFormResult, WitTtsError>;
}

pub trait TtsStreamGuest {
    fn send_text(&self, input: WitTextInput) -> Result<(), WitTtsError>;
    fn finish(&self) -> Result<(), WitTtsError>;
    fn receive_chunk(&self) -> Result<Option<WitAudioChunk>, WitTtsError>;
    fn has_pending_audio(&self) -> bool;
    fn close(&self);
}

pub trait VoiceConversionStreamGuest {
    fn send_audio(&self, audio_data: Vec<u8>) -> Result<(), WitTtsError>;
    fn receive_converted(&self) -> Result<Option<WitAudioChunk>, WitTtsError>;
    fn finish(&self) -> Result<(), WitTtsError>;
    fn close(&self);
}
