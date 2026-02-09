pub mod types {
    pub type LanguageCode = String;

    #[repr(u8)]
    #[derive(
        Clone,
        Copy,
        Debug,
        Eq,
        Ord,
        PartialEq,
        PartialOrd,
        golem_rust::FromValueAndType,
        golem_rust::IntoValue,
    )]
    pub enum QuotaUnit {
        Characters,
        Requests,
        Seconds,
        Credits,
    }

    #[derive(
        Clone, Copy, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue,
    )]
    pub struct QuotaInfo {
        pub used: u32,
        pub limit: u32,
        pub reset_time: u64,
        pub unit: QuotaUnit,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub enum TtsError {
        InvalidText(String),
        TextTooLong(u32),
        InvalidSsml(String),
        UnsupportedLanguage(String),
        VoiceNotFound(String),
        ModelNotFound(String),
        VoiceUnavailable(String),
        Unauthorized(String),
        AccessDenied(String),
        QuotaExceeded(QuotaInfo),
        RateLimited(u32),
        InsufficientCredits,
        SynthesisFailed(String),
        UnsupportedOperation(String),
        InvalidConfiguration(String),
        ServiceUnavailable(String),
        NetworkError(String),
        InternalError(String),
        InvalidStorageLocation(String),
        StorageAccessDenied(String),
    }

    impl core::fmt::Display for TtsError {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            write!(f, "{:?}", self)
        }
    }

    impl std::error::Error for TtsError {}

    #[repr(u8)]
    #[derive(
        Clone,
        Copy,
        Debug,
        Eq,
        Ord,
        PartialEq,
        PartialOrd,
        golem_rust::FromValueAndType,
        golem_rust::IntoValue,
    )]
    pub enum VoiceGender {
        Male,
        Female,
        Neutral,
    }

    #[repr(u8)]
    #[derive(
        Clone,
        Copy,
        Debug,
        Eq,
        Ord,
        PartialEq,
        PartialOrd,
        golem_rust::FromValueAndType,
        golem_rust::IntoValue,
    )]
    pub enum VoiceQuality {
        Standard,
        Premium,
        Neural,
        Studio,
    }

    #[repr(u8)]
    #[derive(
        Clone,
        Copy,
        Debug,
        Eq,
        Ord,
        PartialEq,
        PartialOrd,
        golem_rust::FromValueAndType,
        golem_rust::IntoValue,
    )]
    pub enum TextType {
        Plain,
        Ssml,
    }

    #[repr(u8)]
    #[derive(
        Clone,
        Copy,
        Debug,
        Eq,
        Ord,
        PartialEq,
        PartialOrd,
        golem_rust::FromValueAndType,
        golem_rust::IntoValue,
    )]
    pub enum AudioFormat {
        Mp3,
        Wav,
        Pcm,
        OggOpus,
        Aac,
        Flac,
        Mulaw,
        Alaw,
    }

    #[derive(
        Clone, Copy, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue,
    )]
    pub struct AudioConfig {
        pub format: AudioFormat,
        pub sample_rate: Option<u32>,
        pub bit_rate: Option<u32>,
        pub channels: Option<u8>,
    }

    #[derive(
        Clone, Copy, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue,
    )]
    pub struct VoiceSettings {
        pub speed: Option<f32>,
        pub pitch: Option<f32>,
        pub volume: Option<f32>,
        pub stability: Option<f32>,
        pub similarity: Option<f32>,
        pub style: Option<f32>,
    }

    #[repr(u8)]
    #[derive(
        Clone,
        Copy,
        Debug,
        Eq,
        Ord,
        PartialEq,
        PartialOrd,
        golem_rust::FromValueAndType,
        golem_rust::IntoValue,
    )]
    pub enum AudioEffects {
        TelephoneQuality,
        HeadphoneOptimized,
        SpeakerOptimized,
        CarAudioOptimized,
        NoiseReduction,
        BassBoost,
        TrebleBoost,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct TextInput {
        pub content: String,
        pub text_type: TextType,
        pub language: Option<LanguageCode>,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct SynthesisMetadata {
        pub duration_seconds: f32,
        pub character_count: u32,
        pub word_count: u32,
        pub audio_size_bytes: u32,
        pub request_id: String,
        pub provider_info: Option<String>,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct SynthesisResult {
        pub audio_data: Vec<u8>,
        pub metadata: SynthesisMetadata,
    }

    #[repr(u8)]
    #[derive(
        Clone,
        Copy,
        Debug,
        Eq,
        Ord,
        PartialEq,
        PartialOrd,
        golem_rust::FromValueAndType,
        golem_rust::IntoValue,
    )]
    pub enum TimingMarkType {
        Word,
        Sentence,
        Paragraph,
        SsmlMark,
        Viseme,
    }

    #[derive(
        Clone, Copy, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue,
    )]
    pub struct TimingInfo {
        pub start_time_seconds: f32,
        pub end_time_seconds: Option<f32>,
        pub text_offset: Option<u32>,
        pub mark_type: Option<TimingMarkType>,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct AudioChunk {
        pub data: Vec<u8>,
        pub sequence_number: u32,
        pub is_final: bool,
        pub timing_info: Option<TimingInfo>,
    }
}

pub mod voices {
    use crate::{VoiceInterface, VoiceResultsInterface};

    pub type TtsError = super::types::TtsError;
    pub type LanguageCode = super::types::LanguageCode;
    pub type VoiceGender = super::types::VoiceGender;
    pub type VoiceQuality = super::types::VoiceQuality;
    pub type AudioFormat = super::types::AudioFormat;
    pub type VoiceSettings = super::types::VoiceSettings;

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct VoiceFilter {
        pub language: Option<LanguageCode>,
        pub gender: Option<VoiceGender>,
        pub quality: Option<VoiceQuality>,
        pub supports_ssml: Option<bool>,
        pub provider: Option<String>,
        pub search_query: Option<String>,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct VoiceInfo {
        pub id: String,
        pub name: String,
        pub language: LanguageCode,
        pub additional_languages: Vec<LanguageCode>,
        pub gender: VoiceGender,
        pub quality: VoiceQuality,
        pub description: Option<String>,
        pub provider: String,
        pub sample_rate: u32,
        pub is_custom: bool,
        pub is_cloned: bool,
        pub preview_url: Option<String>,
        pub use_cases: Vec<String>,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct LanguageInfo {
        pub code: LanguageCode,
        pub name: String,
        pub native_name: String,
        pub voice_count: u32,
    }

    pub struct Voice {
        inner: Box<dyn VoiceInterface>,
    }

    impl Voice {
        pub fn new<T: VoiceInterface>(val: T) -> Self {
            Self {
                inner: Box::new(val),
            }
        }

        pub fn get<T: VoiceInterface>(&self) -> &T {
            self.inner
                .as_any()
                .downcast_ref::<T>()
                .expect("Voice type mismatch")
        }

        pub fn get_mut<T: VoiceInterface>(&mut self) -> &mut T {
            self.inner
                .as_any_mut()
                .downcast_mut::<T>()
                .expect("Voice type mismatch")
        }
    }

    impl std::ops::Deref for Voice {
        type Target = dyn VoiceInterface;

        fn deref(&self) -> &Self::Target {
            &*self.inner
        }
    }

    impl std::ops::DerefMut for Voice {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut *self.inner
        }
    }

    pub struct VoiceBorrow<'a> {
        inner: &'a dyn VoiceInterface,
    }

    impl<'a> VoiceBorrow<'a> {
        pub fn new(inner: &'a dyn VoiceInterface) -> Self {
            Self { inner }
        }

        pub fn get<T: VoiceInterface>(&self) -> &T {
            self.inner
                .as_any()
                .downcast_ref::<T>()
                .expect("VoiceBorrow type mismatch")
        }
    }

    impl<'a> std::ops::Deref for VoiceBorrow<'a> {
        type Target = dyn VoiceInterface;

        fn deref(&self) -> &Self::Target {
            self.inner
        }
    }

    pub struct VoiceResults {
        inner: Box<dyn VoiceResultsInterface>,
    }

    impl VoiceResults {
        pub fn new<T: VoiceResultsInterface>(val: T) -> Self {
            Self {
                inner: Box::new(val),
            }
        }

        pub fn get<T: VoiceResultsInterface>(&self) -> &T {
            self.inner
                .as_any()
                .downcast_ref::<T>()
                .expect("VoiceResults type mismatch")
        }

        pub fn get_mut<T: VoiceResultsInterface>(&mut self) -> &mut T {
            self.inner
                .as_any_mut()
                .downcast_mut::<T>()
                .expect("VoiceResults type mismatch")
        }
    }

    impl std::ops::Deref for VoiceResults {
        type Target = dyn VoiceResultsInterface;

        fn deref(&self) -> &Self::Target {
            &*self.inner
        }
    }

    impl std::ops::DerefMut for VoiceResults {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut *self.inner
        }
    }
}

pub mod synthesis {
    pub type TextInput = super::types::TextInput;
    pub type AudioConfig = super::types::AudioConfig;
    pub type VoiceSettings = super::types::VoiceSettings;
    pub type AudioEffects = super::types::AudioEffects;
    pub type SynthesisResult = super::types::SynthesisResult;
    pub type TtsError = super::types::TtsError;
    pub type TimingInfo = super::types::TimingInfo;
    pub type Voice = super::voices::Voice;
    pub type VoiceBorrow<'a> = super::voices::VoiceBorrow<'a>;

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct SynthesisContext {
        pub previous_text: Option<String>,
        pub next_text: Option<String>,
        pub topic: Option<String>,
        pub emotion: Option<String>,
        pub speaking_style: Option<String>,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct SynthesisOptions {
        pub audio_config: Option<AudioConfig>,
        pub voice_settings: Option<VoiceSettings>,
        pub audio_effects: Option<Vec<AudioEffects>>,
        pub enable_timing: Option<bool>,
        pub enable_word_timing: Option<bool>,
        pub seed: Option<u32>,
        pub model_version: Option<String>,
        pub context: Option<SynthesisContext>,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct ValidationResult {
        pub is_valid: bool,
        pub character_count: u32,
        pub estimated_duration: Option<f32>,
        pub warnings: Vec<String>,
        pub errors: Vec<String>,
    }
}

pub mod streaming {
    use crate::{SynthesisStreamInterface, VoiceConversionStreamInterface};

    pub type TextInput = super::types::TextInput;
    pub type AudioConfig = super::types::AudioConfig;
    pub type VoiceSettings = super::types::VoiceSettings;
    pub type AudioChunk = super::types::AudioChunk;
    pub type TtsError = super::types::TtsError;
    pub type TimingInfo = super::types::TimingInfo;
    pub type Voice = super::voices::Voice;
    pub type VoiceBorrow<'a> = super::voices::VoiceBorrow<'a>;
    pub type SynthesisOptions = super::synthesis::SynthesisOptions;

    #[repr(u8)]
    #[derive(
        Clone,
        Copy,
        Debug,
        Eq,
        Ord,
        PartialEq,
        PartialOrd,
        golem_rust::FromValueAndType,
        golem_rust::IntoValue,
    )]
    pub enum StreamStatus {
        Ready,
        Processing,
        Finished,
        Error,
        Closed,
    }

    pub struct SynthesisStream {
        inner: Box<dyn SynthesisStreamInterface>,
    }

    impl SynthesisStream {
        pub fn new<T: SynthesisStreamInterface>(val: T) -> Self {
            Self {
                inner: Box::new(val),
            }
        }

        pub fn get<T: SynthesisStreamInterface>(&self) -> &T {
            self.inner
                .as_any()
                .downcast_ref::<T>()
                .expect("SynthesisStream type mismatch")
        }

        pub fn get_mut<T: SynthesisStreamInterface>(&mut self) -> &mut T {
            self.inner
                .as_any_mut()
                .downcast_mut::<T>()
                .expect("SynthesisStream type mismatch")
        }
    }

    impl std::ops::Deref for SynthesisStream {
        type Target = dyn SynthesisStreamInterface;

        fn deref(&self) -> &Self::Target {
            &*self.inner
        }
    }

    impl std::ops::DerefMut for SynthesisStream {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut *self.inner
        }
    }

    pub struct VoiceConversionStream {
        inner: Box<dyn VoiceConversionStreamInterface>,
    }

    impl VoiceConversionStream {
        pub fn new<T: VoiceConversionStreamInterface>(val: T) -> Self {
            Self {
                inner: Box::new(val),
            }
        }

        pub fn get<T: VoiceConversionStreamInterface>(&self) -> &T {
            self.inner
                .as_any()
                .downcast_ref::<T>()
                .expect("VoiceConversionStream type mismatch")
        }

        pub fn get_mut<T: VoiceConversionStreamInterface>(&mut self) -> &mut T {
            self.inner
                .as_any_mut()
                .downcast_mut::<T>()
                .expect("VoiceConversionStream type mismatch")
        }
    }

    impl std::ops::Deref for VoiceConversionStream {
        type Target = dyn VoiceConversionStreamInterface;

        fn deref(&self) -> &Self::Target {
            &*self.inner
        }
    }

    impl std::ops::DerefMut for VoiceConversionStream {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut *self.inner
        }
    }
}

pub mod advanced {
    use crate::{LongFormOperationInterface, PronunciationLexiconInterface};

    pub type TtsError = super::types::TtsError;
    pub type AudioConfig = super::types::AudioConfig;
    pub type LanguageCode = super::types::LanguageCode;
    pub type VoiceGender = super::types::VoiceGender;
    pub type SynthesisMetadata = super::types::SynthesisMetadata;
    pub type Voice = super::voices::Voice;
    pub type VoiceBorrow<'a> = super::voices::VoiceBorrow<'a>;

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct AudioSample {
        pub data: Vec<u8>,
        pub transcript: Option<String>,
        pub quality_rating: Option<u8>,
    }

    #[repr(u8)]
    #[derive(
        Clone,
        Copy,
        Debug,
        Eq,
        Ord,
        PartialEq,
        PartialOrd,
        golem_rust::FromValueAndType,
        golem_rust::IntoValue,
    )]
    pub enum AgeCategory {
        Child,
        YoungAdult,
        MiddleAged,
        Elderly,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct VoiceDesignParams {
        pub gender: VoiceGender,
        pub age_category: AgeCategory,
        pub accent: String,
        pub personality_traits: Vec<String>,
        pub reference_voice: Option<String>,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct PronunciationEntry {
        pub word: String,
        pub pronunciation: String,
        pub part_of_speech: Option<String>,
    }

    #[repr(u8)]
    #[derive(
        Clone,
        Copy,
        Debug,
        Eq,
        Ord,
        PartialEq,
        PartialOrd,
        golem_rust::FromValueAndType,
        golem_rust::IntoValue,
    )]
    pub enum OperationStatus {
        Pending,
        Processing,
        Completed,
        Failed,
        Cancelled,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct LongFormResult {
        pub output_location: String,
        pub total_duration: f32,
        pub chapter_durations: Option<Vec<f32>>,
        pub metadata: SynthesisMetadata,
    }

    pub struct PronunciationLexicon {
        inner: Box<dyn PronunciationLexiconInterface>,
    }

    impl PronunciationLexicon {
        pub fn new<T: PronunciationLexiconInterface>(val: T) -> Self {
            Self {
                inner: Box::new(val),
            }
        }

        pub fn get<T: PronunciationLexiconInterface>(&self) -> &T {
            self.inner
                .as_any()
                .downcast_ref::<T>()
                .expect("PronunciationLexicon type mismatch")
        }

        pub fn get_mut<T: PronunciationLexiconInterface>(&mut self) -> &mut T {
            self.inner
                .as_any_mut()
                .downcast_mut::<T>()
                .expect("PronunciationLexicon type mismatch")
        }
    }

    impl std::ops::Deref for PronunciationLexicon {
        type Target = dyn PronunciationLexiconInterface;

        fn deref(&self) -> &Self::Target {
            &*self.inner
        }
    }

    impl std::ops::DerefMut for PronunciationLexicon {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut *self.inner
        }
    }

    pub struct LongFormOperation {
        inner: Box<dyn LongFormOperationInterface>,
    }

    impl LongFormOperation {
        pub fn new<T: LongFormOperationInterface>(val: T) -> Self {
            Self {
                inner: Box::new(val),
            }
        }

        pub fn get<T: LongFormOperationInterface>(&self) -> &T {
            self.inner
                .as_any()
                .downcast_ref::<T>()
                .expect("LongFormOperation type mismatch")
        }

        pub fn get_mut<T: LongFormOperationInterface>(&mut self) -> &mut T {
            self.inner
                .as_any_mut()
                .downcast_mut::<T>()
                .expect("LongFormOperation type mismatch")
        }
    }

    impl std::ops::Deref for LongFormOperation {
        type Target = dyn LongFormOperationInterface;

        fn deref(&self) -> &Self::Target {
            &*self.inner
        }
    }

    impl std::ops::DerefMut for LongFormOperation {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut *self.inner
        }
    }
}
