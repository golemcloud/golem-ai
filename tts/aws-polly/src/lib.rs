//! TTS-AWS-Polly WASM Component
//!
//! Implements the golem:tts@1.0.0 WIT interface for AWS Polly.

#[allow(warnings)]
mod bindings;

// Re-export bindings for sub-module access
pub use bindings::exports::golem::tts::advanced as wit_advanced;
pub use bindings::exports::golem::tts::streaming as wit_streaming;
pub use bindings::exports::golem::tts::synthesis as wit_synthesis;
pub use bindings::exports::golem::tts::types as wit_types;
pub use bindings::exports::golem::tts::voices as wit_voices;

// Internal modules
mod advanced;
mod client;
mod sigv4;
mod streaming;
mod synthesis;
mod types;
mod voices;

use wit_advanced::Guest as AdvancedGuest;
use wit_streaming::Guest as StreamingGuest;
use wit_synthesis::Guest as SynthesisGuest;
use wit_voices::Guest as VoicesGuest;

struct TtsAwsPollyComponent;

impl VoicesGuest for TtsAwsPollyComponent {
    type Voice = voices::VoiceImpl;
    type VoiceResults = voices::VoiceResultsImpl;

    fn list_voices(
        f: Option<wit_voices::VoiceFilter>,
    ) -> Result<wit_voices::VoiceResults, wit_types::TtsError> {
        return voices::list_voices(f);
    }
    fn get_voice(id: String) -> Result<wit_voices::Voice, wit_types::TtsError> {
        return voices::get_voice(id);
    }
    fn search_voices(
        q: String,
        f: Option<wit_voices::VoiceFilter>,
    ) -> Result<Vec<wit_voices::VoiceInfo>, wit_types::TtsError> {
        return voices::search_voices(q, f);
    }
    fn list_languages() -> Result<Vec<wit_voices::LanguageInfo>, wit_types::TtsError> {
        return voices::list_languages();
    }
}

impl SynthesisGuest for TtsAwsPollyComponent {
    fn synthesize(
        i: wit_types::TextInput,
        v: wit_voices::VoiceBorrow<'_>,
        o: Option<wit_synthesis::SynthesisOptions>,
    ) -> Result<wit_types::SynthesisResult, wit_types::TtsError> {
        return synthesis::synthesize(i, v.get(), o);
    }
    fn synthesize_batch(
        i: Vec<wit_types::TextInput>,
        v: wit_voices::VoiceBorrow<'_>,
        o: Option<wit_synthesis::SynthesisOptions>,
    ) -> Result<Vec<wit_types::SynthesisResult>, wit_types::TtsError> {
        return synthesis::synthesize_batch(i, v.get(), o);
    }
    fn get_timing_marks(
        i: wit_types::TextInput,
        v: wit_voices::VoiceBorrow<'_>,
    ) -> Result<Vec<wit_types::TimingInfo>, wit_types::TtsError> {
        return synthesis::get_timing_marks(i, v.get());
    }
    fn validate_input(
        i: wit_types::TextInput,
        v: wit_voices::VoiceBorrow<'_>,
    ) -> Result<wit_synthesis::ValidationResult, wit_types::TtsError> {
        return synthesis::validate_input(i, v.get());
    }
}

impl StreamingGuest for TtsAwsPollyComponent {
    type SynthesisStream = streaming::SynthesisStreamImpl;
    type VoiceConversionStream = streaming::VoiceConversionStreamImpl;

    fn create_stream(
        v: wit_voices::VoiceBorrow<'_>,
        o: Option<wit_synthesis::SynthesisOptions>,
    ) -> Result<wit_streaming::SynthesisStream, wit_types::TtsError> {
        return streaming::create_stream(v.get(), o);
    }
    fn create_voice_conversion_stream(
        v: wit_voices::VoiceBorrow<'_>,
        o: Option<wit_synthesis::SynthesisOptions>,
    ) -> Result<wit_streaming::VoiceConversionStream, wit_types::TtsError> {
        return streaming::create_voice_conversion_stream(v.get(), o);
    }
}

impl AdvancedGuest for TtsAwsPollyComponent {
    type PronunciationLexicon = advanced::PronunciationLexiconImpl;
    type LongFormOperation = advanced::LongFormOperationImpl;

    fn create_voice_clone(
        n: String,
        s: Vec<wit_advanced::AudioSample>,
        d: Option<String>,
    ) -> Result<wit_voices::Voice, wit_types::TtsError> {
        return advanced::create_voice_clone(n, s, d);
    }
    fn design_voice(
        n: String,
        c: wit_advanced::VoiceDesignParams,
    ) -> Result<wit_voices::Voice, wit_types::TtsError> {
        return advanced::design_voice(n, c);
    }
    fn convert_voice(
        a: Vec<u8>,
        v: wit_voices::VoiceBorrow<'_>,
        p: Option<bool>,
    ) -> Result<Vec<u8>, wit_types::TtsError> {
        return advanced::convert_voice(a, v.get(), p);
    }
    fn generate_sound_effect(
        d: String,
        dur: Option<f32>,
        i: Option<f32>,
    ) -> Result<Vec<u8>, wit_types::TtsError> {
        return advanced::generate_sound_effect(d, dur, i);
    }
    fn create_lexicon(
        n: String,
        l: String,
        e: Option<Vec<wit_advanced::PronunciationEntry>>,
    ) -> Result<wit_advanced::PronunciationLexicon, wit_types::TtsError> {
        return advanced::create_lexicon(n, l, e);
    }
    fn synthesize_long_form(
        c: String,
        v: wit_voices::VoiceBorrow<'_>,
        l: String,
        b: Option<Vec<u32>>,
    ) -> Result<wit_advanced::LongFormOperation, wit_types::TtsError> {
        return advanced::synthesize_long_form(c, v.get(), l, b);
    }
}

bindings::export!(TtsAwsPollyComponent with_types_in bindings);
