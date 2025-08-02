use golem_stt::mapping::{
    TranscriptAlternativeOut, TranscriptionMetadataOut, TranscriptionResultOut, WordSegmentOut,
};
use serde::Deserialize;

/// Minimal AWS Transcribe response mapping (StartTranscriptionJob/GetTranscriptionJob JSON)
/// Reference: https://docs.aws.amazon.com/transcribe/latest/dg/API_TranscriptionJob.html
///
/// Note: AWS often returns a TranscriptFileUri to a JSON in S3 for batch jobs. This mapping
/// models the JSON content of that transcript file. For streaming, partial results have a similar
/// structure under "Transcript.Results".
#[derive(Debug, Deserialize)]
pub struct AwsTranscriptFile {
    #[serde(default)]
    pub job_name: Option<String>,
    #[serde(default)]
    pub account_id: Option<String>,
    #[serde(default)]
    pub results: Option<AwsResults>,
    #[serde(default)]
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AwsResults {
    #[serde(default)]
    pub transcripts: Vec<AwsTranscriptText>,
    #[serde(default)]
    pub items: Vec<AwsItem>,
    // speaker_labels block may exist when speaker diarization was enabled
    #[serde(default)]
    pub speaker_labels: Option<AwsSpeakerLabels>,
    // language_code sometimes available
    #[serde(default)]
    pub language_code: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AwsTranscriptText {
    #[serde(default)]
    pub transcript: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum AwsItem {
    #[serde(rename = "pronunciation")]
    Pronunciation {
        #[serde(default)]
        start_time: Option<String>,
        #[serde(default)]
        end_time: Option<String>,
        #[serde(default)]
        alternatives: Vec<AwsAlternative>,
        #[serde(default)]
        speaker_label: Option<String>,
    },
    #[serde(rename = "punctuation")]
    Punctuation {
        #[serde(default)]
        alternatives: Vec<AwsAlternative>,
    },
}

#[derive(Debug, Deserialize)]
pub struct AwsAlternative {
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub confidence: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AwsSpeakerLabels {
    #[serde(default)]
    pub segments: Vec<AwsSpeakerSegment>,
}

#[derive(Debug, Deserialize)]
pub struct AwsSpeakerSegment {
    #[serde(default)]
    pub speaker_label: Option<String>,
    // start_time/end_time as string seconds
    #[serde(default)]
    pub start_time: Option<String>,
    #[serde(default)]
    pub end_time: Option<String>,
    #[serde(default)]
    pub items: Vec<AwsSpeakerItemRef>,
}

#[derive(Debug, Deserialize)]
pub struct AwsSpeakerItemRef {
    #[serde(default)]
    pub speaker_label: Option<String>,
    #[serde(default)]
    pub start_time: Option<String>,
    #[serde(default)]
    pub end_time: Option<String>,
    #[serde(default)]
    pub item: Option<String>, // reference to word content in items
}

fn parse_secs(s: &Option<String>) -> f32 {
    s.as_ref()
        .and_then(|x| x.parse::<f32>().ok())
        .unwrap_or(0.0)
}

fn parse_conf(s: &Option<String>) -> Option<f32> {
    s.as_ref().and_then(|x| x.parse::<f32>().ok())
}

pub fn map_aws_to_out(
    resp: AwsTranscriptFile,
    audio_size: u32,
    request_id: Option<String>,
    model: Option<String>,
    language_fallback: &str,
) -> Option<TranscriptionResultOut> {
    let results = resp.results?;

    // Build words from pronunciation items; punctuation items are merged into text
    let mut words: Vec<WordSegmentOut> = Vec::new();
    for it in &results.items {
        if let AwsItem::Pronunciation {
            start_time,
            end_time,
            alternatives,
            speaker_label,
        } = it
        {
            let best = alternatives.first();
            let text = best.map(|a| a.content.clone()).unwrap_or_default();
            if text.is_empty() {
                continue;
            }
            let confidence = parse_conf(&best.and_then(|a| a.confidence.clone()));
            words.push(WordSegmentOut {
                text,
                start_time: parse_secs(start_time),
                end_time: parse_secs(end_time),
                confidence,
                speaker_id: speaker_label.clone(),
            });
        }
    }

    let full_text = results
        .transcripts
        .first()
        .map(|t| t.transcript.clone())
        .unwrap_or_else(|| {
            // Join words to form a best-effort transcript if transcripts missing
            words
                .iter()
                .map(|w| w.text.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        });

    let alternative = TranscriptAlternativeOut {
        text: full_text,
        confidence: 1.0, // AWS file may not provide overall alt confidence
        words,
    };

    let language = results
        .language_code
        .clone()
        .unwrap_or_else(|| language_fallback.to_string());

    let metadata = TranscriptionMetadataOut {
        duration_seconds: 0.0,
        audio_size_bytes: audio_size,
        request_id: request_id.unwrap_or_else(|| "unknown".to_string()),
        model,
        language,
    };

    Some(TranscriptionResultOut {
        alternatives: vec![alternative],
        metadata,
    })
}
