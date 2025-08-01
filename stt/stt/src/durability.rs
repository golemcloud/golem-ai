use crate::golem::stt::transcription::{Guest as TranscriptionGuest, GuestTranscriptionStream, TranscribeOptions};
use crate::golem::stt::vocabularies::{Guest as VocabulariesGuest, GuestVocabulary};
use crate::golem::stt::languages::{Guest as LanguagesGuest, LanguageInfo};
use crate::golem::stt::types::{
    AudioConfig, TranscriptionResult, SttError, TranscriptAlternative
};
use golem_rust::*;

pub trait ExtendedTranscriptionGuest: TranscriptionGuest {
    // Optional additional methods can be added here
}

pub trait ExtendedVocabulariesGuest: VocabulariesGuest {
    // Optional additional methods can be added here  
}

pub trait ExtendedLanguagesGuest: LanguagesGuest {
    // Optional additional methods can be added here
}

pub trait ExtendedGuest: ExtendedTranscriptionGuest + ExtendedVocabulariesGuest + ExtendedLanguagesGuest {}

pub struct DurableSTT<T: ExtendedGuest>(T);

impl<T: ExtendedGuest> TranscriptionGuest for DurableSTT<T> {
    type TranscriptionStream = T::TranscriptionStream;

    fn transcribe(
        audio: Vec<u8>,
        config: AudioConfig,
        options: Option<TranscribeOptions>,
    ) -> Result<TranscriptionResult, SttError> {
        T::transcribe(audio, config, options)
    }

    fn transcribe_stream(
        config: AudioConfig,
        options: Option<TranscribeOptions>,
    ) -> Result<crate::golem::stt::transcription::TranscriptionStream, SttError> {
        T::transcribe_stream(config, options)
    }
}

impl<T: ExtendedGuest> VocabulariesGuest for DurableSTT<T> {
    type Vocabulary = T::Vocabulary;

    fn create_vocabulary(
        name: String,
        phrases: Vec<String>,
    ) -> Result<crate::golem::stt::vocabularies::Vocabulary, SttError> {
        T::create_vocabulary(name, phrases)
    }
}

impl<T: ExtendedGuest> LanguagesGuest for DurableSTT<T> {
    fn list_languages() -> Result<Vec<LanguageInfo>, SttError> {
        T::list_languages()
    }
}

pub struct DurableTranscriptionStream<T>(T);

impl<T: GuestTranscriptionStream> GuestTranscriptionStream for DurableTranscriptionStream<T> {
    fn send_audio(&self, chunk: Vec<u8>) -> Result<(), SttError> {
        self.0.send_audio(chunk)
    }

    fn finish(&self) -> Result<(), SttError> {
        self.0.finish()
    }

    fn receive_alternative(&self) -> Result<Option<TranscriptAlternative>, SttError> {
        self.0.receive_alternative()
    }

    fn close(&self) {
        self.0.close()
    }
}

pub struct DurableVocabulary<T>(T);

impl<T: GuestVocabulary> GuestVocabulary for DurableVocabulary<T> {
    fn get_name(&self) -> String {
        self.0.get_name()
    }

    fn get_phrases(&self) -> Vec<String> {
        self.0.get_phrases()
    }

    fn delete(&self) -> Result<(), SttError> {
        self.0.delete()
    }
}