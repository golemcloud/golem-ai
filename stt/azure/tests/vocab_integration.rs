use golem_stt_azure::vocabularies::AzureVocabulariesComponent;
use golem_stt::golem::stt::vocabularies::{Guest as VocabulariesGuest, Vocabulary, GuestVocabulary};
use golem_stt_azure::vocabularies::AzureVocabulary;
use std::env;

#[test]
fn vocab_create_get_delete_env_guarded() {
    if env::var("AZURE_SPEECH_KEY").is_err() || env::var("AZURE_SPEECH_REGION").is_err() {
        return;
    }

    let name = format!("golem-test-{}", uuid::Uuid::new_v4());
    let phrases = vec!["golem".to_string(), "stt".to_string(), "azure".to_string()];

    let vocab: Vocabulary = AzureVocabulariesComponent::create_vocabulary(name.clone(), phrases.clone()).expect("create vocabulary");
    let inner: AzureVocabulary = vocab.into_inner();
    assert_eq!(inner.get_name(), name);

    let fetched = inner.get_phrases();
    assert!(!fetched.is_empty());

    inner.delete().expect("delete vocabulary");
}