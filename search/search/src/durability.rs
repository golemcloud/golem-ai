use crate::model::{IndexName, SearchHit, SearchQuery};
use crate::wasi_compat::Pollable;
use crate::SearchProvider;
use std::marker::PhantomData;

pub struct DurableSearch<Impl> {
    phantom: PhantomData<Impl>,
}

pub trait ExtendedSearchProvider: SearchProvider + 'static {
    fn unwrapped_stream(
        provider_config: Self::ProviderConfig,
        index: IndexName,
        query: SearchQuery,
    ) -> Self::SearchStream;

    /// Creates the retry query with the original query and any partial results received.
    /// There is a default implementation here, but it can be overridden with provider-specific
    /// queries if needed.
    fn retry_query(original_query: &SearchQuery, partial_hits: &[SearchHit]) -> SearchQuery {
        let mut retry_query = original_query.clone();

        // If we have partial results, we might want to exclude already seen document IDs
        // or adjust pagination to continue from where we left off
        if !partial_hits.is_empty() {
            let current_offset = original_query.offset.unwrap_or(0);
            let received_count = partial_hits.len() as u32;
            retry_query.offset = Some(current_offset + received_count);
        }

        retry_query
    }

    fn subscribe(stream: &Self::SearchStream) -> Pollable;
}

/// When the durability feature flag is off, `DurableSearch<Impl>` is a transparent wrapper that
/// forwards every call to the inner provider without any oplog persistence.
#[cfg(not(feature = "golem"))]
mod passthrough_impl {
    use crate::durability::{DurableSearch, ExtendedSearchProvider};
    use crate::init_logging;
    use crate::model::{
        CreateIndexOptions, Doc, DocumentId, IndexName, Schema, SearchError, SearchQuery,
        SearchResults, SearchStream,
    };
    use crate::SearchProvider;

    impl<Impl: ExtendedSearchProvider> SearchProvider for DurableSearch<Impl> {
        type SearchStream = Impl::SearchStream;
        type ProviderConfig = Impl::ProviderConfig;

        fn create_index(
            provider_config: Self::ProviderConfig,
            options: CreateIndexOptions,
        ) -> Result<(), SearchError> {
            init_logging();
            Impl::create_index(provider_config, options)
        }

        fn delete_index(
            provider_config: Self::ProviderConfig,
            name: IndexName,
        ) -> Result<(), SearchError> {
            init_logging();
            Impl::delete_index(provider_config, name)
        }

        fn list_indexes(
            provider_config: Self::ProviderConfig,
        ) -> Result<Vec<IndexName>, SearchError> {
            init_logging();
            Impl::list_indexes(provider_config)
        }

        fn upsert(
            provider_config: Self::ProviderConfig,
            index: IndexName,
            doc: Doc,
        ) -> Result<(), SearchError> {
            init_logging();
            Impl::upsert(provider_config, index, doc)
        }

        fn upsert_many(
            provider_config: Self::ProviderConfig,
            index: IndexName,
            docs: Vec<Doc>,
        ) -> Result<(), SearchError> {
            init_logging();
            Impl::upsert_many(provider_config, index, docs)
        }

        fn delete(
            provider_config: Self::ProviderConfig,
            index: IndexName,
            id: DocumentId,
        ) -> Result<(), SearchError> {
            init_logging();
            Impl::delete(provider_config, index, id)
        }

        fn delete_many(
            provider_config: Self::ProviderConfig,
            index: IndexName,
            ids: Vec<DocumentId>,
        ) -> Result<(), SearchError> {
            init_logging();
            Impl::delete_many(provider_config, index, ids)
        }

        fn get(
            provider_config: Self::ProviderConfig,
            index: IndexName,
            id: DocumentId,
        ) -> Result<Option<Doc>, SearchError> {
            init_logging();
            Impl::get(provider_config, index, id)
        }

        fn search(
            provider_config: Self::ProviderConfig,
            index: IndexName,
            query: SearchQuery,
        ) -> Result<SearchResults, SearchError> {
            init_logging();
            Impl::search(provider_config, index, query)
        }

        fn stream_search(
            provider_config: Self::ProviderConfig,
            index: IndexName,
            query: SearchQuery,
        ) -> Result<SearchStream, SearchError> {
            init_logging();
            Impl::stream_search(provider_config, index, query)
        }

        fn get_schema(
            provider_config: Self::ProviderConfig,
            index: IndexName,
        ) -> Result<Schema, SearchError> {
            init_logging();
            Impl::get_schema(provider_config, index)
        }

        fn update_schema(
            provider_config: Self::ProviderConfig,
            index: IndexName,
            schema: Schema,
        ) -> Result<(), SearchError> {
            init_logging();
            Impl::update_schema(provider_config, index, schema)
        }
    }
}

/// When the durability feature flag is on, wrapping with `DurableSearch` adds custom durability
/// on top of the provider-specific search implementation using Golem's special host functions and
/// the `golem-rust` helper library.
///
/// The `provider_config` is intentionally **not** persisted in the input payloads because it
/// can carry secrets (API keys etc.). Instead, every replay path expects the caller to supply
/// a fresh `provider_config`. For the streaming case, the `provider_config` is captured inside
/// the `DurableSearchStream` so that subsequent stream-continuation HTTP calls can re-resolve
/// any contained secrets right before each request.
#[cfg(feature = "golem")]
mod durable_impl {
    use crate::durability::{DurableSearch, ExtendedSearchProvider};
    use crate::model::{CreateIndexOptions, SearchStream};
    use crate::model::{
        Doc, DocumentId, IndexName, Schema, SearchError, SearchHit, SearchQuery, SearchResults,
    };
    use crate::wasi_compat::Pollable;
    use crate::{init_logging, SearchProvider, SearchStreamInterface};
    use golem_rust::bindings::golem::durability::durability::{
        DurableFunctionType, LazyInitializedPollable,
    };
    use golem_rust::durability::Durability;
    use golem_rust::{with_persistence_level, FromValueAndType, IntoValue, PersistenceLevel};
    use std::cell::RefCell;
    use std::fmt::{Display, Formatter};

    #[derive(Debug, Clone, IntoValue)]
    struct DeleteIndexInput {
        name: IndexName,
    }

    #[derive(Debug, Clone, IntoValue)]
    struct UpsertInput {
        index: IndexName,
        doc: Doc,
    }

    #[derive(Debug, Clone, IntoValue)]
    struct UpsertManyInput {
        index: IndexName,
        docs: Vec<Doc>,
    }

    #[derive(Debug, Clone, IntoValue)]
    struct DeleteInput {
        index: IndexName,
        id: DocumentId,
    }

    #[derive(Debug, Clone, IntoValue)]
    struct DeleteManyInput {
        index: IndexName,
        ids: Vec<DocumentId>,
    }

    #[derive(Debug, Clone, IntoValue)]
    struct GetInput {
        index: IndexName,
        id: DocumentId,
    }

    #[derive(Debug, Clone, IntoValue)]
    struct SearchInput {
        index: IndexName,
        query: SearchQuery,
    }

    #[derive(Debug, Clone, IntoValue)]
    struct StreamSearchInput {
        index: IndexName,
        query: SearchQuery,
    }

    #[derive(Debug, Clone, IntoValue)]
    struct GetSchemaInput {
        index: IndexName,
    }

    #[derive(Debug, Clone, IntoValue)]
    struct UpdateSchemaInput {
        index: IndexName,
        schema: Schema,
    }

    #[derive(Debug, IntoValue)]
    struct NoInput;

    #[derive(Debug, Clone, FromValueAndType, IntoValue)]
    struct NoOutput;

    #[derive(Debug, FromValueAndType, IntoValue)]
    struct UnusedError;

    impl Display for UnusedError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "UnusedError")
        }
    }

    #[derive(Debug, Clone, FromValueAndType, IntoValue)]
    struct ListIndexesOutput {
        names: Vec<IndexName>,
    }

    #[derive(Debug, Clone, FromValueAndType, IntoValue)]
    struct GetDocOutput {
        doc: Option<Doc>,
    }

    #[derive(Debug, Clone, FromValueAndType, IntoValue)]
    struct SearchOutput {
        results: SearchResults,
    }

    #[derive(Debug, Clone, FromValueAndType, IntoValue)]
    struct GetSchemaOutput {
        schema: Schema,
    }

    impl<Impl: ExtendedSearchProvider> SearchProvider for DurableSearch<Impl> {
        type SearchStream = DurableSearchStream<Impl>;
        type ProviderConfig = Impl::ProviderConfig;

        fn create_index(
            provider_config: Self::ProviderConfig,
            options: CreateIndexOptions,
        ) -> Result<(), SearchError> {
            init_logging();

            let durability = Durability::<NoOutput, SearchError>::new(
                "golem_ai_search",
                "create_index",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::create_index(provider_config, options.clone()).map(|()| NoOutput)
                });
                // NOTE: `provider_config` deliberately not included in the persisted input,
                // because it can carry secrets (API keys etc.).
                durability.persist(options, result).map(|_: NoOutput| ())
            } else {
                durability.replay().map(|_: NoOutput| ())
            }
        }

        fn delete_index(
            provider_config: Self::ProviderConfig,
            name: IndexName,
        ) -> Result<(), SearchError> {
            init_logging();

            let durability = Durability::<NoOutput, SearchError>::new(
                "golem_ai_search",
                "delete_index",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::delete_index(provider_config, name.clone()).map(|()| NoOutput)
                });
                // NOTE: `provider_config` deliberately not included in the persisted input.
                durability
                    .persist(DeleteIndexInput { name }, result)
                    .map(|_: NoOutput| ())
            } else {
                durability.replay().map(|_: NoOutput| ())
            }
        }

        fn list_indexes(
            provider_config: Self::ProviderConfig,
        ) -> Result<Vec<IndexName>, SearchError> {
            init_logging();

            let durability = Durability::<ListIndexesOutput, SearchError>::new(
                "golem_ai_search",
                "list_indexes",
                DurableFunctionType::ReadRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::list_indexes(provider_config).map(|names| ListIndexesOutput { names })
                });
                durability
                    .persist(NoInput, result)
                    .map(|result| result.names)
            } else {
                durability
                    .replay()
                    .map(|result: ListIndexesOutput| result.names)
            }
        }

        fn upsert(
            provider_config: Self::ProviderConfig,
            index: IndexName,
            doc: Doc,
        ) -> Result<(), SearchError> {
            init_logging();

            let durability = Durability::<NoOutput, SearchError>::new(
                "golem_ai_search",
                "upsert",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::upsert(provider_config, index.clone(), doc.clone()).map(|()| NoOutput)
                });
                // NOTE: `provider_config` deliberately not included in the persisted input.
                durability
                    .persist(UpsertInput { index, doc }, result)
                    .map(|_: NoOutput| ())
            } else {
                durability.replay().map(|_: NoOutput| ())
            }
        }

        fn upsert_many(
            provider_config: Self::ProviderConfig,
            index: IndexName,
            docs: Vec<Doc>,
        ) -> Result<(), SearchError> {
            init_logging();

            let durability = Durability::<NoOutput, SearchError>::new(
                "golem_ai_search",
                "upsert_many",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::upsert_many(provider_config, index.clone(), docs.clone())
                        .map(|_| NoOutput)
                });
                // NOTE: `provider_config` deliberately not included in the persisted input.
                durability
                    .persist(UpsertManyInput { index, docs }, result)
                    .map(|_: NoOutput| ())
            } else {
                durability.replay().map(|_: NoOutput| {})
            }
        }

        fn delete(
            provider_config: Self::ProviderConfig,
            index: IndexName,
            id: DocumentId,
        ) -> Result<(), SearchError> {
            init_logging();

            let durability = Durability::<NoOutput, SearchError>::new(
                "golem_ai_search",
                "delete",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::delete(provider_config, index.clone(), id.clone()).map(|()| NoOutput)
                });
                // NOTE: `provider_config` deliberately not included in the persisted input.
                durability
                    .persist(DeleteInput { index, id }, result)
                    .map(|_: NoOutput| ())
            } else {
                durability.replay().map(|_: NoOutput| ())
            }
        }

        fn delete_many(
            provider_config: Self::ProviderConfig,
            index: IndexName,
            ids: Vec<DocumentId>,
        ) -> Result<(), SearchError> {
            init_logging();

            let durability = Durability::<NoOutput, SearchError>::new(
                "golem_ai_search",
                "delete_many",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::delete_many(provider_config, index.clone(), ids.clone()).map(|_| NoOutput)
                });
                // NOTE: `provider_config` deliberately not included in the persisted input.
                durability
                    .persist(DeleteManyInput { index, ids }, result)
                    .map(|_: NoOutput| ())
            } else {
                durability.replay().map(|_: NoOutput| ())
            }
        }

        fn get(
            provider_config: Self::ProviderConfig,
            index: IndexName,
            id: DocumentId,
        ) -> Result<Option<Doc>, SearchError> {
            init_logging();

            let durability = Durability::<GetDocOutput, SearchError>::new(
                "golem_ai_search",
                "get",
                DurableFunctionType::ReadRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::get(provider_config, index.clone(), id.clone())
                        .map(|doc| GetDocOutput { doc })
                });
                // NOTE: `provider_config` deliberately not included in the persisted input.
                durability
                    .persist(GetInput { index, id }, result)
                    .map(|result| result.doc)
            } else {
                durability.replay().map(|result: GetDocOutput| result.doc)
            }
        }

        fn search(
            provider_config: Self::ProviderConfig,
            index: IndexName,
            query: SearchQuery,
        ) -> Result<SearchResults, SearchError> {
            init_logging();

            let durability = Durability::<SearchOutput, SearchError>::new(
                "golem_ai_search",
                "search",
                DurableFunctionType::ReadRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::search(provider_config, index.clone(), query.clone())
                        .map(|results| SearchOutput { results })
                });
                // NOTE: `provider_config` deliberately not included in the persisted input.
                durability
                    .persist(SearchInput { index, query }, result)
                    .map(|result| result.results)
            } else {
                durability
                    .replay()
                    .map(|results: SearchOutput| results.results)
            }
        }

        fn stream_search(
            provider_config: Self::ProviderConfig,
            index: IndexName,
            query: SearchQuery,
        ) -> Result<SearchStream, SearchError> {
            init_logging();

            let durability = Durability::<NoOutput, UnusedError>::new(
                "golem_ai_search",
                "stream_search",
                DurableFunctionType::ReadRemote,
            );
            if durability.is_live() {
                let provider_config_for_stream = provider_config.clone();
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    SearchStream::new(DurableSearchStream::<Impl>::live(
                        provider_config_for_stream.clone(),
                        Impl::unwrapped_stream(
                            provider_config_for_stream,
                            index.clone(),
                            query.clone(),
                        ),
                    ))
                });
                // NOTE: `provider_config` deliberately not included in the persisted input.
                let _ = durability.persist_infallible(StreamSearchInput { index, query }, NoOutput);
                Ok(result)
            } else {
                let _: NoOutput = durability.replay_infallible();
                Ok(SearchStream::new(DurableSearchStream::<Impl>::replay(
                    provider_config,
                    index,
                    query,
                )))
            }
        }

        fn get_schema(
            provider_config: Self::ProviderConfig,
            index: IndexName,
        ) -> Result<Schema, SearchError> {
            init_logging();

            let durability = Durability::<GetSchemaOutput, SearchError>::new(
                "golem_ai_search",
                "get_schema",
                DurableFunctionType::ReadRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::get_schema(provider_config, index.clone())
                        .map(|schema| GetSchemaOutput { schema })
                });
                // NOTE: `provider_config` deliberately not included in the persisted input.
                durability
                    .persist(GetSchemaInput { index }, result)
                    .map(|schema| schema.schema)
            } else {
                durability
                    .replay()
                    .map(|schema: GetSchemaOutput| schema.schema)
            }
        }

        fn update_schema(
            provider_config: Self::ProviderConfig,
            index: IndexName,
            schema: Schema,
        ) -> Result<(), SearchError> {
            init_logging();

            let durability = Durability::<NoOutput, SearchError>::new(
                "golem_ai_search",
                "update_schema",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::update_schema(provider_config, index.clone(), schema.clone())
                        .map(|()| NoOutput)
                });
                // NOTE: `provider_config` deliberately not included in the persisted input.
                durability
                    .persist(UpdateSchemaInput { index, schema }, result)
                    .map(|_: NoOutput| ())
            } else {
                durability.replay().map(|_: NoOutput| ())
            }
        }
    }

    /// Represents the durable search stream's state
    ///
    /// In live mode it directly calls the underlying Search stream which is implemented on
    /// top of a streaming search response.
    ///
    /// In replay mode it buffers the replayed search hits, and also tracks the created pollables
    /// to be able to reattach them to the new live stream when the switch to live mode
    /// happens.
    ///
    /// When reaching the end of the replay mode, if the replayed stream was not finished yet,
    /// the retry query implemented in `ExtendedSearchProvider` is used to create a new Search
    /// response stream and continue the search seamlessly. The `provider_config` (which carries
    /// any secrets) is kept inside this struct so that subsequent live requests can re-resolve
    /// those secrets immediately before each HTTP call.
    enum DurableSearchStreamState<Impl: ExtendedSearchProvider> {
        Live {
            stream: Impl::SearchStream,
            pollables: Vec<LazyInitializedPollable>,
        },
        Replay {
            index: IndexName,
            query: Box<SearchQuery>,
            pollables: Vec<LazyInitializedPollable>,
            partial_result: Vec<SearchHit>,
            finished: bool,
        },
    }

    pub struct DurableSearchStream<Impl: ExtendedSearchProvider> {
        provider_config: Impl::ProviderConfig,
        state: RefCell<Option<DurableSearchStreamState<Impl>>>,
        subscription: RefCell<Option<Pollable>>,
    }

    impl<Impl: ExtendedSearchProvider> DurableSearchStream<Impl> {
        fn live(provider_config: Impl::ProviderConfig, stream: Impl::SearchStream) -> Self {
            Self {
                provider_config,
                state: RefCell::new(Some(DurableSearchStreamState::Live {
                    stream,
                    pollables: Vec::new(),
                })),
                subscription: RefCell::new(None),
            }
        }

        fn replay(
            provider_config: Impl::ProviderConfig,
            index: IndexName,
            query: SearchQuery,
        ) -> Self {
            Self {
                provider_config,
                state: RefCell::new(Some(DurableSearchStreamState::Replay {
                    index,
                    query: Box::new(query),
                    pollables: Vec::new(),
                    partial_result: Vec::new(),
                    finished: false,
                })),
                subscription: RefCell::new(None),
            }
        }

        fn subscribe(&self) -> Pollable {
            let mut state = self.state.borrow_mut();
            match &mut *state {
                Some(DurableSearchStreamState::Live { stream, .. }) => Impl::subscribe(stream),
                Some(DurableSearchStreamState::Replay { pollables, .. }) => {
                    let lazy_pollable = LazyInitializedPollable::new();
                    let pollable = lazy_pollable.subscribe();
                    pollables.push(lazy_pollable);
                    pollable
                }
                None => {
                    unreachable!()
                }
            }
        }
    }

    impl<Impl: ExtendedSearchProvider> Drop for DurableSearchStream<Impl> {
        fn drop(&mut self) {
            let _ = self.subscription.take();
            match self.state.take() {
                Some(DurableSearchStreamState::Live {
                    mut pollables,
                    stream,
                }) => {
                    with_persistence_level(PersistenceLevel::PersistNothing, move || {
                        pollables.clear();
                        drop(stream);
                    });
                }
                Some(DurableSearchStreamState::Replay { mut pollables, .. }) => {
                    pollables.clear();
                }
                None => {}
            }
        }
    }

    impl<Impl: ExtendedSearchProvider> SearchStreamInterface for DurableSearchStream<Impl> {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }

        fn get_next(&self) -> Option<Vec<SearchHit>> {
            let durability = Durability::<Option<Vec<SearchHit>>, UnusedError>::new(
                "golem_ai_search",
                "get_next",
                DurableFunctionType::ReadRemote,
            );
            if durability.is_live() {
                let mut state = self.state.borrow_mut();
                let (result, new_live_stream) = match &*state {
                    Some(DurableSearchStreamState::Live { stream, .. }) => {
                        let result =
                            with_persistence_level(PersistenceLevel::PersistNothing, || {
                                stream.get_next()
                            });
                        (durability.persist_infallible(NoInput, result.clone()), None)
                    }
                    Some(DurableSearchStreamState::Replay {
                        index,
                        query,
                        pollables,
                        partial_result,
                        finished,
                    }) => {
                        if *finished {
                            (None, None)
                        } else {
                            let extended_query = Impl::retry_query(query, partial_result);

                            let (stream, first_live_result) =
                                with_persistence_level(PersistenceLevel::PersistNothing, || {
                                    let stream = <Impl as ExtendedSearchProvider>::unwrapped_stream(
                                        self.provider_config.clone(),
                                        index.clone(),
                                        extended_query,
                                    );

                                    for lazy_initialized_pollable in pollables {
                                        lazy_initialized_pollable.set(Impl::subscribe(&stream));
                                    }

                                    let next = stream.get_next();
                                    (stream, next)
                                });
                            durability.persist_infallible(NoInput, first_live_result.clone());

                            (first_live_result, Some(stream))
                        }
                    }
                    None => {
                        unreachable!()
                    }
                };

                if let Some(stream) = new_live_stream {
                    let pollables = match state.take() {
                        Some(DurableSearchStreamState::Live { pollables, .. }) => pollables,
                        Some(DurableSearchStreamState::Replay { pollables, .. }) => pollables,
                        None => {
                            unreachable!()
                        }
                    };
                    *state = Some(DurableSearchStreamState::Live { stream, pollables });
                }

                result
            } else {
                let result: Option<Vec<SearchHit>> = durability.replay_infallible();
                let mut state = self.state.borrow_mut();
                match &mut *state {
                    Some(DurableSearchStreamState::Live { .. }) => {
                        unreachable!("Durable search stream cannot be in live mode during replay")
                    }
                    Some(DurableSearchStreamState::Replay {
                        partial_result,
                        finished,
                        ..
                    }) => {
                        if let Some(ref result) = result {
                            partial_result.extend_from_slice(result);
                        } else {
                            *finished = true;
                        }
                    }
                    None => {
                        unreachable!()
                    }
                }
                result
            }
        }

        fn blocking_get_next(&self) -> Vec<SearchHit> {
            let mut subscription = self.subscription.borrow_mut();
            if subscription.is_none() {
                *subscription = Some(self.subscribe());
            }
            let subscription = subscription.as_mut().unwrap();
            let mut result = Vec::new();
            loop {
                subscription.block();
                match self.get_next() {
                    Some(hits) => {
                        result.extend(hits);
                        break result;
                    }
                    None => continue,
                }
            }
        }
    }
}
