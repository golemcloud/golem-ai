use crate::EmbeddingProvider;
use std::marker::PhantomData;

/// Wraps an embed implementation with custom durability
pub struct DurableEmbed<Impl> {
    phantom: PhantomData<Impl>,
}

/// Trait to be implemented in addition to the embed `EmbeddingProvider` trait when wrapping it with durability.
pub trait ExtendedEmbeddingProvider: EmbeddingProvider + 'static {}

/// When the durability feature flag is off, wrapping with `DurableEmbed` is just a passthrough
#[cfg(not(feature = "durability"))]
mod passthrough_impl {
    use crate::durability::{DurableEmbed, ExtendedEmbeddingProvider};
    use crate::golem::embed::embed::{
        Config, ContentPart, EmbeddingProvider, EmbeddingResponse, Error, RerankResponse,
    };

    impl<Impl: ExtendedEmbeddingProvider> EmbeddingProvider for DurableEmbed<Impl> {
        fn generate(inputs: Vec<ContentPart>, config: Config) -> Result<EmbeddingResponse, Error> {
            Impl::generate(inputs, config)
        }

        fn rerank(
            query: String,
            documents: Vec<String>,
            config: Config,
        ) -> Result<RerankResponse, Error> {
            Impl::rerank(query, documents, config)
        }
    }
}

/// When the durability feature flag is on, wrapping with `DurableEmbed` adds custom durability
/// on top of the provider-specific embed implementation using Golem's special host functions and
/// the `golem-rust` helper library.
///
/// There will be custom durability entries saved in the oplog, with the full embed request and configuration
/// stored as input, and the full response stored as output. To serialize these in a way it is
/// observable by oplog consumers, each relevant data type has to be converted to/from `ValueAndType`
/// which is implemented using the type classes and builder in the `golem-rust` library.
#[cfg(feature = "durability")]
mod durable_impl {
    use crate::durability::{DurableEmbed, ExtendedEmbeddingProvider};
    use crate::model::{Config, ContentPart, EmbeddingResponse, Error, RerankResponse};
    use crate::EmbeddingProvider;
    use golem_rust::bindings::golem::durability::durability::DurableFunctionType;
    use golem_rust::durability::Durability;
    use golem_rust::{with_persistence_level, FromValueAndType, IntoValue, PersistenceLevel};

    impl<Impl: ExtendedEmbeddingProvider> EmbeddingProvider for DurableEmbed<Impl> {
        fn generate(inputs: Vec<ContentPart>, config: Config) -> Result<EmbeddingResponse, Error> {
            let durability = Durability::<EmbeddingResponse, Error>::new(
                "golem_ai_embed",
                "generate",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::generate(inputs.clone(), config.clone())
                });
                durability.persist(GenerateInput { inputs, config }, result)
            } else {
                durability.replay()
            }
        }

        fn rerank(
            query: String,
            documents: Vec<String>,
            config: Config,
        ) -> Result<RerankResponse, Error> {
            let durability = Durability::<RerankResponse, Error>::new(
                "golem_ai_embed",
                "rerank",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::rerank(query.clone(), documents.clone(), config.clone())
                });
                durability.persist(
                    RerankInput {
                        query,
                        documents,
                        config,
                    },
                    result,
                )
            } else {
                durability.replay()
            }
        }
    }

    #[derive(Debug, Clone, PartialEq, IntoValue, FromValueAndType)]
    struct GenerateInput {
        inputs: Vec<ContentPart>,
        config: Config,
    }

    #[derive(Debug, Clone, PartialEq, IntoValue, FromValueAndType)]
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
