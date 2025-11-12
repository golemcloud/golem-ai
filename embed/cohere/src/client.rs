use std::{collections::HashMap, fmt::Debug};

use golem_embed::{
    error::{error_code_from_status, from_reqwest_error},
    golem::embed::embed::Error,
};
use log::trace;
use golem_wasi_http::{Client, Method, Response};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

const BASE_URL: &str = "https://api.cohere.ai";

/// The Cohere API client for creating embeddings.
///
/// Based on https://docs.cohere.com/reference/embed
pub struct EmbeddingsApi {
    cohere_api_key: String,
    client: Client,
}

impl EmbeddingsApi {
    pub fn new(cohere_api_key: String) -> Self {
        let client = Client::builder()
            .build()
            .expect("Failed to initialize HTTP client");
        Self {
            cohere_api_key,
            client,
        }
    }

    pub fn generate_embeding(&self, request: EmbeddingRequest) -> Result<EmbeddingResponse, Error> {
        trace!("Sending request to Cohere API: {request:?}");
        let response = self
            .client
            .request(Method::POST, format!("{BASE_URL}/v2/embed"))
            .bearer_auth(&self.cohere_api_key)
            .json(&request)
            .send()
            .map_err(|err| from_reqwest_error("Request failed", err))?;
        trace!("Recived response: {response:#?}");
        parse_response::<EmbeddingResponse>(response)
    }

    pub fn rerank(&self, request: RerankRequest) -> Result<RerankResponse, Error> {
        trace!("Sending request to Cohere API: {request:?}");
        let response = self
            .client
            .request(Method::POST, format!("{BASE_URL}/v2/rerank"))
            .bearer_auth(&self.cohere_api_key)
            .json(&request)
            .send()
            .map_err(|err| from_reqwest_error("Request failed", err))?;
        trace!("Recived response: {response:#?}");
        parse_response::<RerankResponse>(response)
    }
}

fn parse_response<T: DeserializeOwned + Debug>(response: Response) -> Result<T, Error> {
    let status = response.status();
    let response_text = response
        .text()
        .map_err(|err| from_reqwest_error("Failed to read response body", err))?;
    match serde_json::from_str::<T>(&response_text) {
        Ok(response_data) => {
            trace!("Response from Hugging Face API: {response_data:?}");
            Ok(response_data)
        }
        Err(error) => {
            trace!("Error parsing response: {error:?}");
            Err(Error {
                code: error_code_from_status(status),
                message: format!("Failed to decode response body: {response_text}"),
                provider_error_json: Some(error.to_string()),
            })
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InputType {
    #[serde(rename = "search_document")]
    SearchDocument,
    #[serde(rename = "search_query")]
    SearchQuery,
    #[serde(rename = "classification")]
    Classification,
    #[serde(rename = "clustering")]
    Clustering,
    #[serde(rename = "image")]
    Image,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmbeddingType {
    #[serde(rename = "float")]
    Float,
    #[serde(rename = "int8")]
    Int8,
    #[serde(rename = "uint8")]
    Uint8,
    #[serde(rename = "binary")]
    Binary,
    #[serde(rename = "ubinary")]
    Ubinary,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EmbeddingRequest {
    pub model: String,
    pub input_type: InputType,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub texts: Option<Vec<String>>,

    /// DataUri format:jpeg,png    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_dimension: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_types: Option<Vec<EmbeddingType>>,

    #[serde(flatten)]
    pub provider_params: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EmbeddingResponse {
    pub id: String,

    pub embeddings: EmbeddingData,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<ImageResponse>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub texts: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<Meta>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImageResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bit_depth: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EmbeddingData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub float: Option<Vec<Vec<f32>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub int8: Option<Vec<Vec<i8>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uint8: Option<Vec<Vec<u8>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binary: Option<Vec<Vec<i8>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ubinary: Option<Vec<Vec<u8>>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RerankRequest {
    pub model: String,
    pub query: String,
    pub documents: Vec<String>,

    #[serde(flatten)]
    pub provider_params: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RerankResponse {
    pub results: Vec<RerankData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<Meta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warnings: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RerankData {
    pub index: u32,
    pub relevance_score: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Meta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_version: Option<ApiVersion>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billed_units: Option<BilledUnits>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens: Option<MetaTokens>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warnings: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MetaTokens {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ApiVersion {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_deprecated: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_experimental: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BilledUnits {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_units: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub classifications: Option<u32>,
}
