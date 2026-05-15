use golem_ai_embed::{
    durability::{DurableEmbed, ExtendedEmbeddingProvider},
    model::{Config, ContentPart, EmbeddingResponse, Error, RerankResponse},
    EmbeddingProvider, LOGGING_STATE,
};

use crate::{
    client::VoyageAIApi,
    conversations::{
        create_embedding_request, create_rerank_request, process_embedding_response,
        process_rerank_response,
    },
};

mod client;
pub mod config;
mod conversations;

pub use config::VoyageAiConfig;
#[cfg(feature = "golem")]
pub use config::VoyageAiHostConfig;

pub struct VoyageAI;

impl VoyageAI {
    fn embeddings(
        client: VoyageAIApi,
        inputs: Vec<ContentPart>,
        config: Config,
    ) -> Result<EmbeddingResponse, Error> {
        let request = create_embedding_request(inputs, config.clone());
        match request {
            Ok(request) => match client.generate_embedding(request) {
                Ok(response) => process_embedding_response(config.output_dtype, response),
                Err(err) => Err(err),
            },
            Err(err) => Err(err),
        }
    }

    fn rerank(
        client: VoyageAIApi,
        query: String,
        documents: Vec<String>,
        config: Config,
    ) -> Result<RerankResponse, Error> {
        let request = create_rerank_request(query, documents, config);
        match request {
            Ok(request) => match client.rerank(request) {
                Ok(response) => process_rerank_response(response),
                Err(err) => Err(err),
            },
            Err(err) => Err(err),
        }
    }
}

impl EmbeddingProvider for VoyageAI {
    type ProviderConfig = VoyageAiConfig;

    fn generate(
        provider_config: Self::ProviderConfig,
        inputs: Vec<ContentPart>,
        config: Config,
    ) -> Result<EmbeddingResponse, Error> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());
        let client = VoyageAIApi::new(&provider_config);
        Self::embeddings(client, inputs, config)
    }

    fn rerank(
        provider_config: Self::ProviderConfig,
        query: String,
        documents: Vec<String>,
        config: Config,
    ) -> Result<RerankResponse, Error> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());
        let client = VoyageAIApi::new(&provider_config);
        Self::rerank(client, query, documents, config)
    }
}

impl ExtendedEmbeddingProvider for VoyageAI {}

pub type DurableVoyageAI = DurableEmbed<VoyageAI>;
