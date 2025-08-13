use crate::config::AmazonConfig;
use golem_stt::golem::stt::transcription::{AudioConfig, TranscribeOptions, TranscriptionResult, TranscriptAlternative};
use golem_stt::golem::stt::types::WordSegment;
use golem_stt::golem::stt::types::SttError;
use serde::Deserialize;

#[derive(Deserialize)]
struct TranscriptRoot {
    results: Option<Results>,
}

#[derive(Deserialize)]
struct Results {
    transcripts: Option<Vec<TranscriptText>>,
    items: Option<Vec<Item>>, 
    speaker_labels: Option<SpeakerLabels>,
}

#[derive(Deserialize, Clone)]
struct TranscriptText { transcript: String }

#[derive(Deserialize)]
struct Item {
    #[serde(rename = "type")]
    item_type: String,
    start_time: Option<String>,
    end_time: Option<String>,
    alternatives: Vec<Alt>,
    speaker_label: Option<String>,
}

#[derive(Deserialize)]
struct Alt { content: String, confidence: Option<String> }

#[derive(Deserialize)]
struct SpeakerLabels { speakers: Option<u32> }

fn parse_f32(s: Option<&String>) -> Option<f32> {
    s.and_then(|v| v.parse::<f32>().ok())
}

pub fn transcribe_once(audio: Vec<u8>, cfg: &AmazonConfig, options: Option<TranscribeOptions>, config: AudioConfig) -> Result<TranscriptionResult, SttError> {
    let _ = (audio, cfg, options, config);
    Err(SttError::UnsupportedOperation("not implemented".into()))
}

pub fn map_transcript(json: &str, language: String, model: Option<String>, audio_size: usize, request_id: String, duration_seconds: f32) -> Result<TranscriptionResult, SttError> {
    let root: TranscriptRoot = serde_json::from_str(json).map_err(|e| SttError::TranscriptionFailed(format!("parse transcript {e}")))?;
    let mut words: Vec<WordSegment> = Vec::new();
    if let Some(results) = root.results {
        if let Some(items) = results.items {
            for it in items.into_iter() {
                if it.item_type == "pronunciation" {
                    let text = it.alternatives.get(0).map(|a| a.content.clone()).unwrap_or_default();
                    let confidence = it.alternatives.get(0).and_then(|a| a.confidence.as_ref()).and_then(|c| c.parse::<f32>().ok());
                    let start = parse_f32(it.start_time.as_ref()).unwrap_or(0.0);
                    let end = parse_f32(it.end_time.as_ref()).unwrap_or(start);
                    let speaker = it.speaker_label.clone();
                    words.push(WordSegment { text, start_time: start, end_time: end, confidence, speaker_id: speaker });
                }
            }
        }
        let full_text = results.transcripts.and_then(|t| t.get(0).cloned()).map(|t| t.transcript).unwrap_or_default();
        let alt = TranscriptAlternative { text: full_text, confidence: 1.0, words };
        let meta = golem_stt::golem::stt::types::TranscriptionMetadata {
            duration_seconds,
            audio_size_bytes: audio_size as u32,
            request_id,
            model,
            language,
        };
        return Ok(TranscriptionResult { alternatives: vec![alt], metadata: meta });
    }
    Err(SttError::TranscriptionFailed("empty transcript".into()))
}

