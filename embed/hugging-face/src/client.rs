use std::{collections::HashMap, fmt::Debug};

use golem_ai_embed::{
    config::SecretSource,
    error::{error_code_from_status, from_reqwest_error},
    model::Error,
};
use golem_wasi_http::{Client, Method, Response};
use log::trace;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

const BASE_URL: &str = "https://router.huggingface.co/hf-inference";

/// The Hugging Face API client for creating embeddings.
///
/// The API key is intentionally stored as a [`SecretSource`] (not as a
/// resolved `String`) so that the secret value is fetched fresh from
/// its source — which in golem mode is the agent host — right before
/// each outgoing HTTP request. This is what lets host-side secret
/// rotation take effect on the very next request.
///
/// Based on https://huggingface.co/docs/inference-providers/providers/hf-inference#feature-extraction
/// Request body schemma https://huggingface.co/docs/inference-providers/tasks/feature-extraction
pub struct EmbeddingsApi {
    huggingface_api_key: SecretSource,
    client: Client,
}

impl EmbeddingsApi {
    pub fn new(config: &crate::config::HuggingFaceConfig) -> Self {
        let client = Client::builder()
            .build()
            .expect("Failed to initialize HTTP client");
        Self {
            huggingface_api_key: config.api_key.clone(),
            client,
        }
    }

    pub fn generate_embedding(
        &self,
        request: EmbeddingRequest,
        model: &str,
    ) -> Result<EmbeddingResponse, Error> {
        trace!("Sending request to Hugging Face API: {request:?}");
        // Resolve the API key right before issuing the request so that
        // hot-rotated host secrets take effect on the next request.
        let api_key = self.huggingface_api_key.get();
        let response = self
            .client
            .request(
                Method::POST,
                format!("{BASE_URL}/models/{model}/pipeline/feature-extraction"),
            )
            .bearer_auth(&api_key)
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
