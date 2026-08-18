use crate::EmbeddingProvider;
use std::marker::PhantomData;

/// Wraps an embed implementation with custom durability
pub struct DurableEmbed<Impl> {
    phantom: PhantomData<Impl>,
}

/// Trait to be implemented in addition to the embed `EmbeddingProvider` trait when wrapping it with durability.
pub trait ExtendedEmbeddingProvider: EmbeddingProvider + 'static {}

/// When the durability feature flag is off, `DurableEmbed<Impl>` is a transparent wrapper that
/// forwards every call to the inner provider without any oplog persistence.
#[cfg(not(feature = "golem"))]
mod passthrough_impl {
    use crate::durability::{DurableEmbed, ExtendedEmbeddingProvider};
    use crate::model::{Config, ContentPart, EmbeddingResponse, Error, RerankResponse};
    use crate::EmbeddingProvider;

    impl<Impl: ExtendedEmbeddingProvider> EmbeddingProvider for DurableEmbed<Impl> {
        type ProviderConfig = Impl::ProviderConfig;

        async fn generate(
            provider_config: Self::ProviderConfig,
            inputs: Vec<ContentPart>,
            config: Config,
        ) -> Result<EmbeddingResponse, Error> {
            Impl::generate(provider_config, inputs, config).await
        }

        async fn rerank(
            provider_config: Self::ProviderConfig,
            query: String,
            documents: Vec<String>,
            config: Config,
        ) -> Result<RerankResponse, Error> {
            Impl::rerank(provider_config, query, documents, config).await
        }
    }
}

/// When the durability feature flag is on, wrapping with `DurableEmbed` adds custom durability
/// on top of the provider-specific embed implementation using Golem's special host functions and
/// the `golem-rust` helper library.
///
/// There will be custom durability entries saved in the oplog, with the full embed request and configuration
/// stored as input, and the full response stored as output. To serialize these in a way it is
/// observable by oplog consumers, each relevant data type has to be converted to/from the shared
/// schema representation implemented by the `golem-rust` library.
#[cfg(feature = "golem")]
mod durable_impl {
    use crate::durability::{DurableEmbed, ExtendedEmbeddingProvider};
    use crate::model::{Config, ContentPart, EmbeddingResponse, Error, RerankResponse};
    use crate::EmbeddingProvider;
    use golem_rust::durability::{Durability, DurableFunctionType};
    use golem_rust::{FromSchema, IntoSchema};

    impl<Impl: ExtendedEmbeddingProvider> EmbeddingProvider for DurableEmbed<Impl> {
        type ProviderConfig = Impl::ProviderConfig;

        async fn generate(
            provider_config: Self::ProviderConfig,
            inputs: Vec<ContentPart>,
            config: Config,
        ) -> Result<EmbeddingResponse, Error> {
            let input = GenerateInput {
                inputs: inputs.clone(),
                config: config.clone(),
            };
            Durability::<EmbeddingResponse, Error>::new(
                "golem_ai_embed",
                "generate",
                DurableFunctionType::WriteRemote,
                &input,
            )
            .run_async(|| Impl::generate(provider_config, inputs, config))
            .await
            // NOTE: `provider_config` deliberately not included in the persisted input,
            // because it can carry secrets (API keys etc.).
        }

        async fn rerank(
            provider_config: Self::ProviderConfig,
            query: String,
            documents: Vec<String>,
            config: Config,
        ) -> Result<RerankResponse, Error> {
            let input = RerankInput {
                query: query.clone(),
                documents: documents.clone(),
                config: config.clone(),
            };
            Durability::<RerankResponse, Error>::new(
                "golem_ai_embed",
                "rerank",
                DurableFunctionType::WriteRemote,
                &input,
            )
            .run_async(|| Impl::rerank(provider_config, query, documents, config))
            .await
            // NOTE: `provider_config` deliberately not included in the persisted input,
            // because it can carry secrets (API keys etc.).
        }
    }

    #[derive(Debug, Clone, PartialEq, IntoSchema, FromSchema)]
    struct GenerateInput {
        inputs: Vec<ContentPart>,
        config: Config,
    }

    #[derive(Debug, Clone, PartialEq, IntoSchema, FromSchema)]
    struct RerankInput {
        query: String,
        documents: Vec<String>,
        config: Config,
    }

    impl From<&Error> for Error {
        fn from(err: &Error) -> Self {
            err.clone()
        }
    }
}
