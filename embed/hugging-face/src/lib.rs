mod client;
pub mod config;
mod conversions;

use client::EmbeddingsApi;
use conversions::{create_embedding_request, process_embedding_response};
use golem_ai_embed::{
    durability::{DurableEmbed, ExtendedEmbeddingProvider},
    model::{Config, ContentPart, EmbeddingResponse, Error, ErrorCode, RerankResponse},
    EmbeddingProvider, LOGGING_STATE,
};

pub use config::HuggingFaceConfig;
#[cfg(feature = "golem")]
pub use config::HuggingFaceHostConfig;

pub struct HuggingFace;

impl HuggingFace {
    fn embeddings(
        client: EmbeddingsApi,
        inputs: Vec<ContentPart>,
        config: Config,
    ) -> Result<EmbeddingResponse, Error> {
        let (request, model) = create_embedding_request(inputs, config)?;
        match client.generate_embedding(request, &model) {
            Ok(response) => process_embedding_response(response, model),
            Err(err) => Err(err),
        }
    }
}

impl EmbeddingProvider for HuggingFace {
    type ProviderConfig = HuggingFaceConfig;

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
            message: "Hugging Face inference does not support rerank".to_string(),
            provider_error_json: None,
        })
    }
}

impl ExtendedEmbeddingProvider for HuggingFace {}

pub type DurableHuggingFace = DurableEmbed<HuggingFace>;
