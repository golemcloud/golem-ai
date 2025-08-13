use golem_stt::golem::stt::vocabularies::{Guest as VocabulariesGuest, Vocabulary};
use golem_stt::golem::stt::types::SttError;
use std::collections::HashMap;
use std::sync::Mutex;
use once_cell::sync::Lazy;

static VOCABS: Lazy<Mutex<HashMap<String, Vec<String>>>> = Lazy::new(|| Mutex::new(HashMap::new()));

pub struct WhisperVocabulariesComponent;

impl VocabulariesGuest for WhisperVocabulariesComponent {
    type Vocabulary = WhisperVocabulary;

    fn create_vocabulary(name: String, phrases: Vec<String>) -> Result<Vocabulary, SttError> {
        let mut map = VOCABS.lock().map_err(|_| SttError::InternalError("lock".into()))?;
        map.insert(name.clone(), phrases);
        Ok(Vocabulary::new(WhisperVocabulary { name }))
    }
}

pub struct WhisperVocabulary { name: String }

impl golem_stt::golem::stt::vocabularies::GuestVocabulary for WhisperVocabulary {
    fn get_name(&self) -> String { self.name.clone() }
    fn get_phrases(&self) -> Vec<String> {
        VOCABS.lock().ok().and_then(|m| m.get(&self.name).cloned()).unwrap_or_default()
    }
    fn delete(&self) -> Result<(), SttError> {
        if let Ok(mut m) = VOCABS.lock() { m.remove(&self.name); }
        Ok(())
    }
}
