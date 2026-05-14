use client::EmbeddingsApi;
use conversions::create_embed_request;
use golem_ai_embed::{
    durability::{DurableEmbed, ExtendedEmbeddingProvider},
    model::{
        Config, ContentPart, EmbeddingResponse as GolemEmbeddingResponse, Error, RerankResponse,
    },
    EmbeddingProvider, LOGGING_STATE,
};

use crate::conversions::{
    create_rerank_request, process_embedding_response, process_rerank_response,
};

mod client;
pub mod config;
mod conversions;

pub use config::CohereConfig;
#[cfg(feature = "golem")]
pub use config::CohereHostConfig;

pub struct Cohere;

impl Cohere {
    fn embeddings(
        client: EmbeddingsApi,
        inputs: Vec<ContentPart>,
        config: Config,
    ) -> Result<GolemEmbeddingResponse, Error> {
        let request = create_embed_request(inputs, config.clone());
        match request {
            Ok(request) => match client.generate_embeding(request) {
                Ok(response) => process_embedding_response(response, config),
                Err(err) => Err(err),
            },
            Err(err) => Err(err),
        }
    }

    fn rerank(
        client: EmbeddingsApi,
        query: String,
        documents: Vec<String>,
        config: Config,
    ) -> Result<RerankResponse, Error> {
        let request = create_rerank_request(query, documents, config.clone());
        match request {
            Ok(request) => match client.rerank(request) {
                Ok(response) => process_rerank_response(response, config),
                Err(err) => Err(err),
            },
            Err(err) => Err(err),
        }
    }
}

impl EmbeddingProvider for Cohere {
    type ProviderConfig = CohereConfig;

    fn generate(
        provider_config: Self::ProviderConfig,
        inputs: Vec<ContentPart>,
        config: Config,
    ) -> Result<GolemEmbeddingResponse, Error> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());
        let client = EmbeddingsApi::new(&provider_config);
        Self::embeddings(client, inputs, config)
    }

    fn rerank(
        provider_config: Self::ProviderConfig,
        query: String,
        documents: Vec<String>,
        config: Config,
    ) -> Result<RerankResponse, Error> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());
        let client = EmbeddingsApi::new(&provider_config);
        Self::rerank(client, query, documents, config)
    }
}

impl ExtendedEmbeddingProvider for Cohere {}

pub type DurableCohere = DurableEmbed<Cohere>;
