mod client;
pub mod config;
mod conversions;

use client::EmbeddingsApi;
use conversions::{create_request, process_embedding_response};
use golem_ai_embed::{
    durability::{DurableEmbed, ExtendedEmbeddingProvider},
    model::{Config, ContentPart, EmbeddingResponse, Error, ErrorCode, RerankResponse},
    EmbeddingProvider, LOGGING_STATE,
};

pub use config::OpenAiEmbedConfig;
#[cfg(feature = "golem")]
pub use config::OpenAiEmbedHostConfig;

pub struct OpenAI;

impl OpenAI {
    fn embeddings(
        client: EmbeddingsApi,
        inputs: Vec<ContentPart>,
        config: Config,
    ) -> Result<EmbeddingResponse, Error> {
        let request = create_request(inputs, config);
        match request {
            Ok(request) => match client.generate_embeding(request) {
                Ok(response) => process_embedding_response(response),
                Err(err) => Err(err),
            },
            Err(err) => Err(err),
        }
    }
}

impl EmbeddingProvider for OpenAI {
    type ProviderConfig = OpenAiEmbedConfig;

    fn generate(
        provider_config: Self::ProviderConfig,
        inputs: Vec<ContentPart>,
        config: Config,
    ) -> Result<EmbeddingResponse, Error> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());
        let client = EmbeddingsApi::new(&provider_config);
        Self::embeddings(client, inputs, config)
    }

    fn rerank(
        _provider_config: Self::ProviderConfig,
        _query: String,
        _documents: Vec<String>,
        _config: Config,
    ) -> Result<RerankResponse, Error> {
        Err(Error {
            code: ErrorCode::Unsupported,
            message: "OpenAI does not support rerank".to_string(),
            provider_error_json: None,
        })
    }
}

impl ExtendedEmbeddingProvider for OpenAI {}

pub type DurableOpenAI = DurableEmbed<OpenAI>;
