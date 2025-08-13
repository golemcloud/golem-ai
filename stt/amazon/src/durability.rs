use golem_rust::durability::Durability;
use golem_stt::golem::stt::transcription::TranscriptionResult;

pub fn persist_result(_key: &str, _result: &TranscriptionResult) {
    let _ = Durability::persist_infallible("stt_amazon_result", _result);
}

