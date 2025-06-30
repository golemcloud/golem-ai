use golem_llm::error::{error_code_from_status, from_event_source_error, from_reqwest_error};
use golem_llm::event_source::EventSource;
use golem_llm::golem::llm::llm::{Error, ErrorCode};
use hmac::{Hmac, Mac};
use log::trace;
use reqwest::{Client, Method, Response};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fmt::Debug;
use std::time::{SystemTime, UNIX_EPOCH};
use time::OffsetDateTime;

/// AWS Bedrock client for creating model responses
pub struct BedrockClient {
    access_key_id: String,
    secret_access_key: String,
    region: String,
    client: Client,
}

impl BedrockClient {
    pub fn new(access_key_id: String, secret_access_key: String, region: String) -> Self {
        let client = Client::new();
        Self {
            access_key_id,
            secret_access_key,
            region,
            client,
        }
    }

    pub fn converse(
        &self,
        model_id: &str,
        request: ConverseRequest,
    ) -> Result<ConverseResponse, Error> {
        trace!("Sending request to Bedrock API: {request:?}");
        let url = format!(
            "https://bedrock-runtime.{}.amazonaws.com/model/{}/converse",
            self.region, model_id
        );

        let body = serde_json::to_string(&request).map_err(|err| Error {
            code: ErrorCode::InternalError,
            message: "Failed to serialize request".to_string(),
            provider_error_json: Some(err.to_string()),
        })?;

        let host = format!("bedrock-runtime.{}.amazonaws.com", self.region);
        let headers = generate_sigv4_headers(
            &self.access_key_id,
            &self.secret_access_key,
            &self.region,
            "bedrock",
            "POST",
            &format!("/model/{model_id}/converse"),
            &host,
            &body,
        )
        .map_err(|err| Error {
            code: ErrorCode::InternalError,
            message: "Failed to sign headers".to_string(),
            provider_error_json: Some(err.to_string()),
        })?;

        let mut request_builder = self.client.request(Method::POST, &url);
        for (key, value) in headers {
            request_builder = request_builder.header(key, value);
        }

        let response: Response = request_builder.body(body).send().map_err(|err| {
            trace!("HTTP request failed with error: {err:?}");
            from_reqwest_error("Request failed", err)
        })?;

        trace!("Received response from Bedrock API: {response:?}");

        parse_response(response)
    }

    pub fn converse_stream(
        &self,
        model_id: &str,
        request: ConverseRequest,
    ) -> Result<EventSource, Error> {
        trace!("Sending streaming request to Bedrock API: {request:?}");
        let url = format!(
            "https://bedrock-runtime.{}.amazonaws.com/model/{}/converse-stream",
            self.region, model_id
        );

        let body = serde_json::to_string(&request).map_err(|err| Error {
            code: ErrorCode::InternalError,
            message: "Failed to serialize request".to_string(),
            provider_error_json: Some(err.to_string()),
        })?;

        let host = format!("bedrock-runtime.{}.amazonaws.com", self.region);
        let headers = generate_sigv4_headers(
            &self.access_key_id,
            &self.secret_access_key,
            &self.region,
            "bedrock",
            "POST",
            &format!("/model/{model_id}/converse-stream"),
            &host,
            &body,
        )
        .map_err(|err| Error {
            code: ErrorCode::InternalError,
            message: "Failed to sign headers".to_string(),
            provider_error_json: Some(err.to_string()),
        })?;

        let mut request_builder = self.client.request(Method::POST, &url);
        for (key, value) in headers {
            request_builder = request_builder.header(key, value);
        }

        trace!("Sending streaming HTTP request to Bedrock...");
        let response: Response = request_builder.body(body).send().map_err(|err| {
            trace!("HTTP request failed with error: {err:?}");
            from_reqwest_error("Request failed", err)
        })?;

        trace!("Initializing SSE stream");
        trace!("Response: {:?}", response.headers().clone());
        EventSource::new(response)
            .map_err(|err| from_event_source_error("Failed to create SSE stream", err))
    }
}

#[allow(clippy::too_many_arguments)]
pub fn generate_sigv4_headers(
    access_key: &str,
    secret_key: &str,
    region: &str,
    service: &str,
    method: &str,
    uri: &str,
    host: &str,
    body: &str,
) -> Result<Vec<(String, String)>, Box<dyn std::error::Error>> {
    use std::collections::BTreeMap;

    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let timestamp = OffsetDateTime::from_unix_timestamp(now.as_secs() as i64).unwrap();

    let date_str = format!(
        "{:04}{:02}{:02}",
        timestamp.year(),
        timestamp.month() as u8,
        timestamp.day()
    );
    let datetime_str = format!(
        "{:04}{:02}{:02}T{:02}{:02}{:02}Z",
        timestamp.year(),
        timestamp.month() as u8,
        timestamp.day(),
        timestamp.hour(),
        timestamp.minute(),
        timestamp.second()
    );

    let (canonical_uri, canonical_query_string) = if let Some(query_pos) = uri.find('?') {
        let path = &uri[..query_pos];
        let query = &uri[query_pos + 1..];

        let encoded_path = if path.contains(':') {
            path.replace(':', "%3A")
        } else {
            path.to_string()
        };

        let mut query_params: Vec<&str> = query.split('&').collect();
        query_params.sort();
        (encoded_path, query_params.join("&"))
    } else {
        let encoded_path = if uri.contains(':') {
            uri.replace(':', "%3A")
        } else {
            uri.to_string()
        };
        (encoded_path, String::new())
    };

    let mut headers = BTreeMap::new();
    headers.insert("content-type", "application/x-amz-json-1.0");
    headers.insert("host", host);
    headers.insert("x-amz-date", &datetime_str);

    let canonical_headers = headers
        .iter()
        .map(|(k, v)| format!("{}:{}", k.to_lowercase().trim(), v.trim()))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";

    let signed_headers = headers
        .keys()
        .map(|k| k.to_lowercase())
        .collect::<Vec<_>>()
        .join(";");

    let payload_hash = format!("{:x}", Sha256::digest(body.as_bytes()));

    let canonical_request = format!(
        "{method}\n{canonical_uri}\n{canonical_query_string}\n{canonical_headers}\n{signed_headers}\n{payload_hash}"
    );

    let credential_scope = format!("{date_str}/{region}/{service}/aws4_request");
    let canonical_request_hash = format!("{:x}", Sha256::digest(canonical_request.as_bytes()));
    let string_to_sign =
        format!("AWS4-HMAC-SHA256\n{datetime_str}\n{credential_scope}\n{canonical_request_hash}");

    type HmacSha256 = Hmac<Sha256>;

    let mut mac = HmacSha256::new_from_slice(format!("AWS4{secret_key}").as_bytes())?;
    mac.update(date_str.as_bytes());
    let date_key = mac.finalize().into_bytes();

    let mut mac = HmacSha256::new_from_slice(&date_key)?;
    mac.update(region.as_bytes());
    let region_key = mac.finalize().into_bytes();

    let mut mac = HmacSha256::new_from_slice(&region_key)?;
    mac.update(service.as_bytes());
    let service_key = mac.finalize().into_bytes();

    let mut mac = HmacSha256::new_from_slice(&service_key)?;
    mac.update(b"aws4_request");
    let signing_key = mac.finalize().into_bytes();

    let mut mac = HmacSha256::new_from_slice(&signing_key)?;
    mac.update(string_to_sign.as_bytes());
    let signature = format!("{:x}", mac.finalize().into_bytes());

    let auth_header = format!(
        "AWS4-HMAC-SHA256 Credential={access_key}/{credential_scope}, SignedHeaders={signed_headers}, Signature={signature}"
    );

    let result_headers = vec![
        ("authorization".to_string(), auth_header),
        ("x-amz-date".to_string(), datetime_str),
        (
            "content-type".to_string(),
            "application/x-amz-json-1.0".to_string(),
        ),
    ];

    Ok(result_headers)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConverseRequest {
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<Vec<SystemContentBlock>>,
    #[serde(rename = "inferenceConfig", skip_serializing_if = "Option::is_none")]
    pub inference_config: Option<InferenceConfig>,
    #[serde(rename = "toolConfig", skip_serializing_if = "Option::is_none")]
    pub tool_config: Option<ToolConfig>,
    #[serde(rename = "guardrailConfig", skip_serializing_if = "Option::is_none")]
    pub guardrail_config: Option<GuardrailConfig>,
    #[serde(
        rename = "additionalModelRequestFields",
        skip_serializing_if = "Option::is_none"
    )]
    pub additional_model_request_fields: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: Vec<ContentBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Role {
    #[serde(rename = "user")]
    User,
    #[serde(rename = "assistant")]
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ContentBlock {
    Text {
        text: String,
    },
    Image {
        image: ImageBlock,
    },
    ToolUse {
        #[serde(rename = "toolUse")]
        tool_use: ToolUseBlock,
    },
    ToolResult {
        #[serde(rename = "toolResult")]
        tool_result: ToolResultBlock,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageBlock {
    pub format: ImageFormat,
    pub source: ImageSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUseBlock {
    #[serde(rename = "toolUseId")]
    pub tool_use_id: String,
    pub name: String,
    pub input: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultBlock {
    #[serde(rename = "toolUseId")]
    pub tool_use_id: String,
    pub content: Vec<ToolResultContentBlock>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<ToolResultStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImageFormat {
    #[serde(rename = "png")]
    Png,
    #[serde(rename = "jpeg")]
    Jpeg,
    #[serde(rename = "gif")]
    Gif,
    #[serde(rename = "webp")]
    Webp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageSource {
    pub bytes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ToolResultContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image {
        #[serde(rename = "format")]
        format: ImageFormat,
        #[serde(rename = "source")]
        source: ImageSource,
    },
    #[serde(rename = "json")]
    Json { json: Value },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolResultStatus {
    #[serde(rename = "success")]
    Success,
    #[serde(rename = "error")]
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SystemContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceConfig {
    #[serde(rename = "maxTokens", skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(rename = "topP", skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(rename = "stopSequences", skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfig {
    pub tools: Vec<Tool>,
    #[serde(rename = "toolChoice", skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    #[serde(rename = "toolSpec")]
    pub tool_spec: ToolSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: ToolInputSchema,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInputSchema {
    pub json: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolChoice {
    Auto { auto: serde_json::Value },
    Any { any: serde_json::Value },
    Tool { tool: ToolChoiceTool },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolChoiceTool {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardrailConfig {
    #[serde(rename = "guardrailIdentifier")]
    pub guardrail_identifier: String,
    #[serde(rename = "guardrailVersion")]
    pub guardrail_version: String,
    pub trace: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConverseResponse {
    pub output: Output,
    #[serde(rename = "stopReason")]
    pub stop_reason: StopReason,
    pub usage: Usage,
    pub metrics: Metrics,
    #[serde(
        rename = "additionalModelResponseFields",
        skip_serializing_if = "Option::is_none"
    )]
    pub additional_model_response_fields: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Output {
    pub message: Message,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StopReason {
    #[serde(rename = "end_turn")]
    EndTurn,
    #[serde(rename = "tool_use")]
    ToolUse,
    #[serde(rename = "max_tokens")]
    MaxTokens,
    #[serde(rename = "stop_sequence")]
    StopSequence,
    #[serde(rename = "guardrail_intervened")]
    GuardrailIntervened,
    #[serde(rename = "content_filtered")]
    ContentFiltered,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    #[serde(rename = "inputTokens")]
    pub input_tokens: u32,
    #[serde(rename = "outputTokens")]
    pub output_tokens: u32,
    #[serde(rename = "totalTokens")]
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metrics {
    #[serde(rename = "latencyMs")]
    pub latency_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub message: String,
    #[serde(rename = "type")]
    pub error_type: String,
}

fn parse_response<T: DeserializeOwned + Debug>(response: Response) -> Result<T, Error> {
    let status = response.status();
    if status.is_success() {
        let body = response
            .json::<T>()
            .map_err(|err| from_reqwest_error("Failed to decode response body", err))?;

        trace!("Received response from Bedrock API: {body:?}");

        Ok(body)
    } else {
        let body = response
            .text()
            .map_err(|err| from_reqwest_error("Failed to receive error response body", err))?;
        trace!("Received {status} response from Bedrock API: {body:?}");

        Err(Error {
            code: error_code_from_status(status),
            message: format!("Request failed with {status}: {body}"),
            provider_error_json: Some(body),
        })
    }
}
