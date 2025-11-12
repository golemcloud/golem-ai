use std::{collections::HashMap, fmt::Debug};

use golem_embed::{
    error::{error_code_from_status, from_reqwest_error},
    golem::embed::embed::Error,
};
use log::trace;
use golem_wasi_http::{Client, Method, Response};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

const BASE_URL: &str = "https://router.huggingface.co/hf-inference";

/// The Hugging Face API client for creating embeddings.
///
/// Based on https://huggingface.co/docs/inference-providers/providers/hf-inference#feature-extraction
/// Request body schemma https://huggingface.co/docs/inference-providers/tasks/feature-extraction
pub struct EmbeddingsApi {
    huggingface_api_key: String,
    client: Client,
}

impl EmbeddingsApi {
    pub fn new(huggingface_api_key: String) -> Self {
        let client = Client::builder()
            .build()
            .expect("Failed to initialize HTTP client");
        Self {
            huggingface_api_key,
            client,
        }
    }

    pub fn generate_embedding(
        &self,
        request: EmbeddingRequest,
        model: &str,
    ) -> Result<EmbeddingResponse, Error> {
        trace!("Sending request to Hugging Face API: {request:?}");
        let response = self
            .client
            .request(
                Method::POST,
                format!("{BASE_URL}/models/{model}/pipeline/feature-extraction"),
            )
            .bearer_auth(&self.huggingface_api_key)
            .json(&request)
            .send()
            .map_err(|err| from_reqwest_error("Request failed", err))?;
        parse_response::<EmbeddingResponse>(response)
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
pub struct EmbeddingRequest {
    pub inputs: Vec<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncate: Option<bool>,

    #[serde(flatten)]
    pub provider_params: HashMap<String, serde_json::Value>,
}

pub type EmbeddingResponse = Vec<Vec<f32>>;
