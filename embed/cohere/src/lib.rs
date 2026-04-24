use client::EmbeddingsApi;
use conversions::create_embed_request;
use golem_ai_embed::{
    config::with_config_key,
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
mod conversions;

pub struct Cohere;

impl Cohere {
    const ENV_VAR_NAME: &'static str = "COHERE_API_KEY";

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
    fn generate(inputs: Vec<ContentPart>, config: Config) -> Result<GolemEmbeddingResponse, Error> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());
        with_config_key(Self::ENV_VAR_NAME, Err, |cohere_api_key| {
            let client = EmbeddingsApi::new(cohere_api_key);
            Self::embeddings(client, inputs, config)
        })
    }

    fn rerank(
        query: String,
        documents: Vec<String>,
        config: Config,
    ) -> Result<RerankResponse, Error> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());
        with_config_key(Self::ENV_VAR_NAME, Err, |cohere_api_key| {
            let client = EmbeddingsApi::new(cohere_api_key);
            Self::rerank(client, query, documents, config)
        })
    }
}

impl ExtendedEmbeddingProvider for Cohere {}

pub type DurableCohere = DurableEmbed<Cohere>;
