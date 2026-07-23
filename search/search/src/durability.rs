use crate::model::{IndexName, SearchHit, SearchQuery};
use crate::SearchProvider;
use std::marker::PhantomData;

pub struct DurableSearch<Impl> {
    phantom: PhantomData<Impl>,
}

#[allow(async_fn_in_trait)]
pub trait ExtendedSearchProvider: SearchProvider + 'static {
    async fn unwrapped_stream(
        provider_config: Self::ProviderConfig,
        index: IndexName,
        query: SearchQuery,
    ) -> Self::SearchStream;

    fn retry_query(original_query: &SearchQuery, partial_hits: &[SearchHit]) -> SearchQuery {
        let mut retry_query = original_query.clone();
        if !partial_hits.is_empty() {
            retry_query.offset = Some(
                original_query.offset.unwrap_or(0) + u32::try_from(partial_hits.len()).unwrap(),
            );
        }
        retry_query
    }
}

#[cfg(not(feature = "golem"))]
mod passthrough_impl {
    use super::{DurableSearch, ExtendedSearchProvider};
    use crate::init_logging;
    use crate::model::{
        CreateIndexOptions, Doc, DocumentId, IndexName, Schema, SearchError, SearchQuery,
        SearchResults, SearchStream,
    };
    use crate::SearchProvider;

    impl<Impl: ExtendedSearchProvider> SearchProvider for DurableSearch<Impl> {
        type SearchStream = Impl::SearchStream;
        type ProviderConfig = Impl::ProviderConfig;

        async fn create_index(
            c: Self::ProviderConfig,
            o: CreateIndexOptions,
        ) -> Result<(), SearchError> {
            init_logging();
            Impl::create_index(c, o).await
        }
        async fn delete_index(c: Self::ProviderConfig, name: IndexName) -> Result<(), SearchError> {
            init_logging();
            Impl::delete_index(c, name).await
        }
        async fn list_indexes(c: Self::ProviderConfig) -> Result<Vec<IndexName>, SearchError> {
            init_logging();
            Impl::list_indexes(c).await
        }
        async fn upsert(
            c: Self::ProviderConfig,
            index: IndexName,
            doc: Doc,
        ) -> Result<(), SearchError> {
            init_logging();
            Impl::upsert(c, index, doc).await
        }
        async fn upsert_many(
            c: Self::ProviderConfig,
            index: IndexName,
            docs: Vec<Doc>,
        ) -> Result<(), SearchError> {
            init_logging();
            Impl::upsert_many(c, index, docs).await
        }
        async fn delete(
            c: Self::ProviderConfig,
            index: IndexName,
            id: DocumentId,
        ) -> Result<(), SearchError> {
            init_logging();
            Impl::delete(c, index, id).await
        }
        async fn delete_many(
            c: Self::ProviderConfig,
            index: IndexName,
            ids: Vec<DocumentId>,
        ) -> Result<(), SearchError> {
            init_logging();
            Impl::delete_many(c, index, ids).await
        }
        async fn get(
            c: Self::ProviderConfig,
            index: IndexName,
            id: DocumentId,
        ) -> Result<Option<Doc>, SearchError> {
            init_logging();
            Impl::get(c, index, id).await
        }
        async fn search(
            c: Self::ProviderConfig,
            index: IndexName,
            query: SearchQuery,
        ) -> Result<SearchResults, SearchError> {
            init_logging();
            Impl::search(c, index, query).await
        }
        async fn stream_search(
            c: Self::ProviderConfig,
            index: IndexName,
            query: SearchQuery,
        ) -> Result<SearchStream, SearchError> {
            init_logging();
            Impl::stream_search(c, index, query).await
        }
        async fn get_schema(
            c: Self::ProviderConfig,
            index: IndexName,
        ) -> Result<Schema, SearchError> {
            init_logging();
            Impl::get_schema(c, index).await
        }
        async fn update_schema(
            c: Self::ProviderConfig,
            index: IndexName,
            schema: Schema,
        ) -> Result<(), SearchError> {
            init_logging();
            Impl::update_schema(c, index, schema).await
        }
    }
}

#[cfg(feature = "golem")]
mod durable_impl {
    use super::{DurableSearch, ExtendedSearchProvider};
    use crate::init_logging;
    use crate::model::{
        CreateIndexOptions, Doc, DocumentId, IndexName, Schema, SearchError, SearchHit,
        SearchQuery, SearchResults, SearchStream,
    };
    use crate::{SearchFuture, SearchProvider, SearchStreamInterface};
    use golem_rust::durability::{Durability, DurableFunctionType};
    use golem_rust::{
        with_persistence_level, with_persistence_level_async, FromSchema, IntoSchema,
        PersistenceLevel,
    };
    use std::cell::RefCell;
    use std::fmt::{Display, Formatter};
    use std::rc::Rc;

    #[derive(Debug, Clone, PartialEq, IntoSchema, FromSchema)]
    struct DeleteIndexInput {
        name: IndexName,
    }
    #[derive(Debug, Clone, PartialEq, IntoSchema, FromSchema)]
    struct UpsertInput {
        index: IndexName,
        doc: Doc,
    }
    #[derive(Debug, Clone, PartialEq, IntoSchema, FromSchema)]
    struct UpsertManyInput {
        index: IndexName,
        docs: Vec<Doc>,
    }
    #[derive(Debug, Clone, PartialEq, IntoSchema, FromSchema)]
    struct DeleteInput {
        index: IndexName,
        id: DocumentId,
    }
    #[derive(Debug, Clone, PartialEq, IntoSchema, FromSchema)]
    struct DeleteManyInput {
        index: IndexName,
        ids: Vec<DocumentId>,
    }
    #[derive(Debug, Clone, PartialEq, IntoSchema, FromSchema)]
    struct GetInput {
        index: IndexName,
        id: DocumentId,
    }
    #[derive(Debug, Clone, PartialEq, IntoSchema, FromSchema)]
    struct SearchInput {
        index: IndexName,
        query: SearchQuery,
    }
    #[derive(Debug, Clone, PartialEq, IntoSchema, FromSchema)]
    struct StreamSearchInput {
        index: IndexName,
        query: SearchQuery,
    }
    #[derive(Debug, Clone, PartialEq, IntoSchema, FromSchema)]
    struct GetSchemaInput {
        index: IndexName,
    }
    #[derive(Debug, Clone, PartialEq, IntoSchema, FromSchema)]
    struct UpdateSchemaInput {
        index: IndexName,
        schema: Schema,
    }
    #[derive(Debug, Clone, PartialEq, IntoSchema, FromSchema)]
    struct NoInput;
    #[derive(Debug, Clone, PartialEq, IntoSchema, FromSchema)]
    struct NoOutput;
    #[derive(Debug, Clone, PartialEq, IntoSchema, FromSchema)]
    struct UnusedError;
    #[derive(Debug, Clone, PartialEq, IntoSchema, FromSchema)]
    struct ListIndexesOutput {
        names: Vec<IndexName>,
    }
    #[derive(Debug, Clone, PartialEq, IntoSchema, FromSchema)]
    struct GetDocOutput {
        doc: Option<Doc>,
    }
    #[derive(Debug, Clone, PartialEq, IntoSchema, FromSchema)]
    struct SearchOutput {
        results: SearchResults,
    }
    #[derive(Debug, Clone, PartialEq, IntoSchema, FromSchema)]
    struct GetSchemaOutput {
        schema: Schema,
    }

    impl Display for UnusedError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "UnusedError")
        }
    }

    impl<Impl: ExtendedSearchProvider> SearchProvider for DurableSearch<Impl> {
        type SearchStream = DurableSearchStream<Impl>;
        type ProviderConfig = Impl::ProviderConfig;

        async fn create_index(
            c: Self::ProviderConfig,
            options: CreateIndexOptions,
        ) -> Result<(), SearchError> {
            init_logging();
            let d = Durability::<NoOutput, SearchError>::new(
                "golem_ai_search",
                "create_index",
                DurableFunctionType::WriteRemote,
            );
            if d.is_live() {
                let r = with_persistence_level_async(PersistenceLevel::PersistNothing, || {
                    Impl::create_index(c, options.clone())
                })
                .await
                .map(|()| NoOutput);
                d.persist(options, r).map(|_| ())
            } else {
                d.replay().map(|_: NoOutput| ())
            }
        }
        async fn delete_index(c: Self::ProviderConfig, name: IndexName) -> Result<(), SearchError> {
            init_logging();
            let d = Durability::<NoOutput, SearchError>::new(
                "golem_ai_search",
                "delete_index",
                DurableFunctionType::WriteRemote,
            );
            if d.is_live() {
                let r = with_persistence_level_async(PersistenceLevel::PersistNothing, || {
                    Impl::delete_index(c, name.clone())
                })
                .await
                .map(|()| NoOutput);
                d.persist(DeleteIndexInput { name }, r).map(|_| ())
            } else {
                d.replay().map(|_: NoOutput| ())
            }
        }
        async fn list_indexes(c: Self::ProviderConfig) -> Result<Vec<IndexName>, SearchError> {
            init_logging();
            let d = Durability::<ListIndexesOutput, SearchError>::new(
                "golem_ai_search",
                "list_indexes",
                DurableFunctionType::ReadRemote,
            );
            if d.is_live() {
                let r = with_persistence_level_async(PersistenceLevel::PersistNothing, || {
                    Impl::list_indexes(c)
                })
                .await
                .map(|names| ListIndexesOutput { names });
                d.persist(NoInput, r).map(|x| x.names)
            } else {
                d.replay().map(|x: ListIndexesOutput| x.names)
            }
        }
        async fn upsert(
            c: Self::ProviderConfig,
            index: IndexName,
            doc: Doc,
        ) -> Result<(), SearchError> {
            init_logging();
            let d = Durability::<NoOutput, SearchError>::new(
                "golem_ai_search",
                "upsert",
                DurableFunctionType::WriteRemote,
            );
            if d.is_live() {
                let r = with_persistence_level_async(PersistenceLevel::PersistNothing, || {
                    Impl::upsert(c, index.clone(), doc.clone())
                })
                .await
                .map(|()| NoOutput);
                d.persist(UpsertInput { index, doc }, r).map(|_| ())
            } else {
                d.replay().map(|_: NoOutput| ())
            }
        }
        async fn upsert_many(
            c: Self::ProviderConfig,
            index: IndexName,
            docs: Vec<Doc>,
        ) -> Result<(), SearchError> {
            init_logging();
            let d = Durability::<NoOutput, SearchError>::new(
                "golem_ai_search",
                "upsert_many",
                DurableFunctionType::WriteRemote,
            );
            if d.is_live() {
                let r = with_persistence_level_async(PersistenceLevel::PersistNothing, || {
                    Impl::upsert_many(c, index.clone(), docs.clone())
                })
                .await
                .map(|()| NoOutput);
                d.persist(UpsertManyInput { index, docs }, r).map(|_| ())
            } else {
                d.replay().map(|_: NoOutput| ())
            }
        }
        async fn delete(
            c: Self::ProviderConfig,
            index: IndexName,
            id: DocumentId,
        ) -> Result<(), SearchError> {
            init_logging();
            let d = Durability::<NoOutput, SearchError>::new(
                "golem_ai_search",
                "delete",
                DurableFunctionType::WriteRemote,
            );
            if d.is_live() {
                let r = with_persistence_level_async(PersistenceLevel::PersistNothing, || {
                    Impl::delete(c, index.clone(), id.clone())
                })
                .await
                .map(|()| NoOutput);
                d.persist(DeleteInput { index, id }, r).map(|_| ())
            } else {
                d.replay().map(|_: NoOutput| ())
            }
        }
        async fn delete_many(
            c: Self::ProviderConfig,
            index: IndexName,
            ids: Vec<DocumentId>,
        ) -> Result<(), SearchError> {
            init_logging();
            let d = Durability::<NoOutput, SearchError>::new(
                "golem_ai_search",
                "delete_many",
                DurableFunctionType::WriteRemote,
            );
            if d.is_live() {
                let r = with_persistence_level_async(PersistenceLevel::PersistNothing, || {
                    Impl::delete_many(c, index.clone(), ids.clone())
                })
                .await
                .map(|()| NoOutput);
                d.persist(DeleteManyInput { index, ids }, r).map(|_| ())
            } else {
                d.replay().map(|_: NoOutput| ())
            }
        }
        async fn get(
            c: Self::ProviderConfig,
            index: IndexName,
            id: DocumentId,
        ) -> Result<Option<Doc>, SearchError> {
            init_logging();
            let d = Durability::<GetDocOutput, SearchError>::new(
                "golem_ai_search",
                "get",
                DurableFunctionType::ReadRemote,
            );
            if d.is_live() {
                let r = with_persistence_level_async(PersistenceLevel::PersistNothing, || {
                    Impl::get(c, index.clone(), id.clone())
                })
                .await
                .map(|doc| GetDocOutput { doc });
                d.persist(GetInput { index, id }, r).map(|x| x.doc)
            } else {
                d.replay().map(|x: GetDocOutput| x.doc)
            }
        }
        async fn search(
            c: Self::ProviderConfig,
            index: IndexName,
            query: SearchQuery,
        ) -> Result<SearchResults, SearchError> {
            init_logging();
            let d = Durability::<SearchOutput, SearchError>::new(
                "golem_ai_search",
                "search",
                DurableFunctionType::ReadRemote,
            );
            if d.is_live() {
                let r = with_persistence_level_async(PersistenceLevel::PersistNothing, || {
                    Impl::search(c, index.clone(), query.clone())
                })
                .await
                .map(|results| SearchOutput { results });
                d.persist(SearchInput { index, query }, r)
                    .map(|x| x.results)
            } else {
                d.replay().map(|x: SearchOutput| x.results)
            }
        }
        async fn stream_search(
            c: Self::ProviderConfig,
            index: IndexName,
            query: SearchQuery,
        ) -> Result<SearchStream, SearchError> {
            init_logging();
            let d = Durability::<NoOutput, UnusedError>::new(
                "golem_ai_search",
                "stream_search",
                DurableFunctionType::ReadRemote,
            );
            if d.is_live() {
                let stream = with_persistence_level_async(PersistenceLevel::PersistNothing, || {
                    Impl::unwrapped_stream(c.clone(), index.clone(), query.clone())
                })
                .await;
                let _ = d.persist_infallible(StreamSearchInput { index, query }, NoOutput);
                Ok(SearchStream::new(DurableSearchStream::<Impl>::live(
                    c, stream,
                )))
            } else {
                let _: NoOutput = d.replay_infallible();
                Ok(SearchStream::new(DurableSearchStream::<Impl>::replay(
                    c, index, query,
                )))
            }
        }
        async fn get_schema(
            c: Self::ProviderConfig,
            index: IndexName,
        ) -> Result<Schema, SearchError> {
            init_logging();
            let d = Durability::<GetSchemaOutput, SearchError>::new(
                "golem_ai_search",
                "get_schema",
                DurableFunctionType::ReadRemote,
            );
            if d.is_live() {
                let r = with_persistence_level_async(PersistenceLevel::PersistNothing, || {
                    Impl::get_schema(c, index.clone())
                })
                .await
                .map(|schema| GetSchemaOutput { schema });
                d.persist(GetSchemaInput { index }, r).map(|x| x.schema)
            } else {
                d.replay().map(|x: GetSchemaOutput| x.schema)
            }
        }
        async fn update_schema(
            c: Self::ProviderConfig,
            index: IndexName,
            schema: Schema,
        ) -> Result<(), SearchError> {
            init_logging();
            let d = Durability::<NoOutput, SearchError>::new(
                "golem_ai_search",
                "update_schema",
                DurableFunctionType::WriteRemote,
            );
            if d.is_live() {
                let r = with_persistence_level_async(PersistenceLevel::PersistNothing, || {
                    Impl::update_schema(c, index.clone(), schema.clone())
                })
                .await
                .map(|()| NoOutput);
                d.persist(UpdateSchemaInput { index, schema }, r)
                    .map(|_| ())
            } else {
                d.replay().map(|_: NoOutput| ())
            }
        }
    }

    enum DurableSearchStreamState<Impl: ExtendedSearchProvider> {
        Live {
            stream: Rc<Impl::SearchStream>,
        },
        Replay {
            index: IndexName,
            query: Box<SearchQuery>,
            partial_result: Vec<SearchHit>,
            finished: bool,
        },
    }

    pub struct DurableSearchStream<Impl: ExtendedSearchProvider> {
        provider_config: Impl::ProviderConfig,
        state: RefCell<Option<DurableSearchStreamState<Impl>>>,
    }

    impl<Impl: ExtendedSearchProvider> DurableSearchStream<Impl> {
        fn live(provider_config: Impl::ProviderConfig, stream: Impl::SearchStream) -> Self {
            Self {
                provider_config,
                state: RefCell::new(Some(DurableSearchStreamState::Live {
                    stream: Rc::new(stream),
                })),
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
                    partial_result: Vec::new(),
                    finished: false,
                })),
            }
        }
    }

    impl<Impl: ExtendedSearchProvider> Drop for DurableSearchStream<Impl> {
        fn drop(&mut self) {
            if let Some(DurableSearchStreamState::Live { stream }) = self.state.take() {
                with_persistence_level(PersistenceLevel::PersistNothing, || drop(stream));
            }
        }
    }

    enum LiveAction<S> {
        Stream(Rc<S>),
        Continue(IndexName, Box<SearchQuery>),
        Finished,
    }

    impl<Impl: ExtendedSearchProvider> SearchStreamInterface for DurableSearchStream<Impl> {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }

        fn get_next(&self) -> SearchFuture<'_, Option<Vec<SearchHit>>> {
            Box::pin(async move {
                let durability = Durability::<Option<Vec<SearchHit>>, UnusedError>::new(
                    "golem_ai_search",
                    "get_next",
                    DurableFunctionType::ReadRemote,
                );
                if !durability.is_live() {
                    let result: Option<Vec<SearchHit>> = durability.replay_infallible();
                    let mut state = self.state.borrow_mut();
                    match state.as_mut().unwrap() {
                        DurableSearchStreamState::Replay {
                            partial_result,
                            finished,
                            ..
                        } => {
                            if let Some(hits) = &result {
                                partial_result.extend_from_slice(hits);
                            } else {
                                *finished = true;
                            }
                        }
                        DurableSearchStreamState::Live { .. } => {
                            unreachable!("Durable search stream cannot be live during replay")
                        }
                    }
                    return result;
                }

                let action = {
                    let state = self.state.borrow();
                    match state.as_ref().unwrap() {
                        DurableSearchStreamState::Live { stream } => {
                            LiveAction::Stream(Rc::clone(stream))
                        }
                        DurableSearchStreamState::Replay {
                            index,
                            query,
                            partial_result,
                            finished,
                        } => {
                            if *finished {
                                LiveAction::Finished
                            } else {
                                LiveAction::Continue(
                                    index.clone(),
                                    Box::new(Impl::retry_query(query, partial_result)),
                                )
                            }
                        }
                    }
                };

                match action {
                    LiveAction::Finished => None,
                    LiveAction::Stream(stream) => {
                        let result = with_persistence_level_async(
                            PersistenceLevel::PersistNothing,
                            || async move { stream.get_next().await },
                        )
                        .await;
                        durability.persist_infallible(NoInput, result.clone())
                    }
                    LiveAction::Continue(index, query) => {
                        let config = self.provider_config.clone();
                        let (stream, result) = with_persistence_level_async(
                            PersistenceLevel::PersistNothing,
                            || async move {
                                let stream =
                                    Rc::new(Impl::unwrapped_stream(config, index, *query).await);
                                let result = stream.get_next().await;
                                (stream, result)
                            },
                        )
                        .await;
                        let result = durability.persist_infallible(NoInput, result.clone());
                        let mut state = self.state.borrow_mut();
                        if matches!(
                            state.as_ref(),
                            Some(DurableSearchStreamState::Replay { .. })
                        ) {
                            *state = Some(DurableSearchStreamState::Live { stream });
                        } else {
                            with_persistence_level(PersistenceLevel::PersistNothing, || {
                                drop(stream)
                            });
                        }
                        result
                    }
                }
            })
        }
    }
}
