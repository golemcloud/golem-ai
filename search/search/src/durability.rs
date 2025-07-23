use crate::golem::search::core::Guest;
use crate::golem::search::types::{IndexName, SearchHit, SearchQuery};
use golem_rust::wasm_rpc::Pollable;
use std::marker::PhantomData;

pub struct DurableSearch<Impl> {
    phantom: PhantomData<Impl>,
}

pub trait ExtendedGuest: Guest + 'static {
    fn unwrapped_stream(index: IndexName, query: SearchQuery) -> Self::SearchStream;

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

/// When the durability feature flag is off, wrapping with `DurableSearch` is just a passthrough
#[cfg(not(feature = "durability"))]
mod passthrough_impl {
    use crate::durability::{DurableSearch, ExtendedGuest};
    use crate::golem::search::core::{Guest, SearchStream};
    use crate::golem::search::types::{
        Doc, DocumentId, IndexName, Schema, SearchError, SearchQuery, SearchResults,
    };

    impl<Impl: ExtendedGuest> Guest for DurableSearch<Impl> {
        type SearchStream = Impl::SearchStream;

        fn create_index(name: IndexName, schema: Option<Schema>) -> Result<(), SearchError> {
            Impl::create_index(name, schema)
        }

        fn delete_index(name: IndexName) -> Result<(), SearchError> {
            Impl::delete_index(name)
        }

        fn list_indexes() -> Result<Vec<IndexName>, SearchError> {
            Impl::list_indexes()
        }

        fn upsert(index: IndexName, doc: Doc) -> Result<(), SearchError> {
            Impl::upsert(index, doc)
        }

        fn upsert_many(index: IndexName, docs: Vec<Doc>) -> Result<(), SearchError> {
            Impl::upsert_many(index, docs)
        }

        fn delete(index: IndexName, id: DocumentId) -> Result<(), SearchError> {
            Impl::delete(index, id)
        }

        fn delete_many(index: IndexName, ids: Vec<DocumentId>) -> Result<(), SearchError> {
            Impl::delete_many(index, ids)
        }

        fn get(index: IndexName, id: DocumentId) -> Result<Option<Doc>, SearchError> {
            Impl::get(index, id)
        }

        fn search(index: IndexName, query: SearchQuery) -> Result<SearchResults, SearchError> {
            Impl::search(index, query)
        }

        fn stream_search(
            index: IndexName,
            query: SearchQuery,
        ) -> Result<SearchStream, SearchError> {
            Impl::stream_search(index, query)
        }

        fn get_schema(index: IndexName) -> Result<Schema, SearchError> {
            Impl::get_schema(index)
        }

        fn update_schema(index: IndexName, schema: Schema) -> Result<(), SearchError> {
            Impl::update_schema(index, schema)
        }
    }
}

#[cfg(feature = "durability")]
mod durable_impl {
    use crate::durability::{DurableSearch, ExtendedGuest};
    use crate::golem::search::core::{Guest, GuestSearchStream, SearchStream};
    use crate::golem::search::types::{
        Doc, DocumentId, IndexName, Schema, SearchError, SearchHit, SearchQuery, SearchResults,
    };
    use golem_rust::bindings::golem::durability::durability::{
        DurableFunctionType, LazyInitializedPollable,
    };
    use golem_rust::durability::Durability;
    use golem_rust::wasm_rpc::Pollable;
    use golem_rust::{with_persistence_level, FromValueAndType, IntoValue, PersistenceLevel};
    use std::cell::RefCell;
    use std::fmt::{Display, Formatter};

    #[derive(Debug, Clone, IntoValue)]
    struct CreateIndexInput {
        name: IndexName,
        schema: Option<Schema>,
    }

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
    struct VoidResult;

    #[derive(Debug, Clone, FromValueAndType, IntoValue)]
    struct IndexNamesResult {
        names: Vec<IndexName>,
    }

    #[derive(Debug, Clone, FromValueAndType, IntoValue)]
    struct OptionalDocResult {
        doc: Option<Doc>,
    }

    #[derive(Debug, Clone, FromValueAndType, IntoValue)]
    struct SearchResultsWrapper {
        results: SearchResults,
    }

    #[derive(Debug, Clone, FromValueAndType, IntoValue)]
    struct SchemaWrapper {
        schema: Schema,
    }

    impl<Impl: ExtendedGuest> Guest for DurableSearch<Impl> {
        type SearchStream = DurableSearchStream<Impl>;

        fn create_index(name: IndexName, schema: Option<Schema>) -> Result<(), SearchError> {
            let durability = Durability::<VoidResult, SearchError>::new(
                "golem_search",
                "create_index",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::create_index(name.clone(), schema.clone())
                });
                match result {
                    Ok(()) => {
                        let _ = durability
                            .persist_infallible(CreateIndexInput { name, schema }, VoidResult);
                        Ok(())
                    }
                    Err(e) => Err(e),
                }
            } else {
                let _: VoidResult = durability.replay_infallible();
                Ok(())
            }
        }

        fn delete_index(name: IndexName) -> Result<(), SearchError> {
            let durability = Durability::<VoidResult, SearchError>::new(
                "golem_search",
                "delete_index",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::delete_index(name.clone())
                });
                match result {
                    Ok(()) => {
                        let _ =
                            durability.persist_infallible(DeleteIndexInput { name }, VoidResult);
                        Ok(())
                    }
                    Err(e) => Err(e),
                }
            } else {
                let _: VoidResult = durability.replay_infallible();
                Ok(())
            }
        }

        fn list_indexes() -> Result<Vec<IndexName>, SearchError> {
            let durability = Durability::<IndexNamesResult, SearchError>::new(
                "golem_search",
                "list_indexes",
                DurableFunctionType::ReadRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::list_indexes()
                });
                match result {
                    Ok(names) => {
                        let _ = durability.persist_infallible(
                            NoInput,
                            IndexNamesResult {
                                names: names.clone(),
                            },
                        );
                        Ok(names)
                    }
                    Err(e) => Err(e),
                }
            } else {
                let wrapper: IndexNamesResult = durability.replay_infallible();
                Ok(wrapper.names)
            }
        }

        fn upsert(index: IndexName, doc: Doc) -> Result<(), SearchError> {
            let durability = Durability::<VoidResult, SearchError>::new(
                "golem_search",
                "upsert",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::upsert(index.clone(), doc.clone())
                });
                match result {
                    Ok(()) => {
                        let _ =
                            durability.persist_infallible(UpsertInput { index, doc }, VoidResult);
                        Ok(())
                    }
                    Err(e) => Err(e),
                }
            } else {
                let _: VoidResult = durability.replay_infallible();
                Ok(())
            }
        }

        fn upsert_many(index: IndexName, docs: Vec<Doc>) -> Result<(), SearchError> {
            let durability = Durability::<VoidResult, SearchError>::new(
                "golem_search",
                "upsert_many",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::upsert_many(index.clone(), docs.clone())
                });
                match result {
                    Ok(()) => {
                        let _ = durability
                            .persist_infallible(UpsertManyInput { index, docs }, VoidResult);
                        Ok(())
                    }
                    Err(e) => Err(e),
                }
            } else {
                let _: VoidResult = durability.replay_infallible();
                Ok(())
            }
        }

        fn delete(index: IndexName, id: DocumentId) -> Result<(), SearchError> {
            let durability = Durability::<VoidResult, SearchError>::new(
                "golem_search",
                "delete",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::delete(index.clone(), id.clone())
                });
                match result {
                    Ok(()) => {
                        let _ =
                            durability.persist_infallible(DeleteInput { index, id }, VoidResult);
                        Ok(())
                    }
                    Err(e) => Err(e),
                }
            } else {
                let _: VoidResult = durability.replay_infallible();
                Ok(())
            }
        }

        fn delete_many(index: IndexName, ids: Vec<DocumentId>) -> Result<(), SearchError> {
            let durability = Durability::<VoidResult, SearchError>::new(
                "golem_search",
                "delete_many",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::delete_many(index.clone(), ids.clone())
                });
                match result {
                    Ok(()) => {
                        let _ = durability
                            .persist_infallible(DeleteManyInput { index, ids }, VoidResult);
                        Ok(())
                    }
                    Err(e) => Err(e),
                }
            } else {
                let _: VoidResult = durability.replay_infallible();
                Ok(())
            }
        }

        fn get(index: IndexName, id: DocumentId) -> Result<Option<Doc>, SearchError> {
            let durability = Durability::<OptionalDocResult, SearchError>::new(
                "golem_search",
                "get",
                DurableFunctionType::ReadRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::get(index.clone(), id.clone())
                });
                match result {
                    Ok(doc) => {
                        let _ = durability.persist_infallible(
                            GetInput { index, id },
                            OptionalDocResult { doc: doc.clone() },
                        );
                        Ok(doc)
                    }
                    Err(e) => Err(e),
                }
            } else {
                let wrapper: OptionalDocResult = durability.replay_infallible();
                Ok(wrapper.doc)
            }
        }

        fn search(index: IndexName, query: SearchQuery) -> Result<SearchResults, SearchError> {
            let durability = Durability::<SearchResultsWrapper, SearchError>::new(
                "golem_search",
                "search",
                DurableFunctionType::ReadRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::search(index.clone(), query.clone())
                });
                match result {
                    Ok(results) => {
                        let _ = durability.persist_infallible(
                            SearchInput { index, query },
                            SearchResultsWrapper {
                                results: results.clone(),
                            },
                        );
                        Ok(results)
                    }
                    Err(e) => Err(e),
                }
            } else {
                let wrapper: SearchResultsWrapper = durability.replay_infallible();
                Ok(wrapper.results)
            }
        }

        fn stream_search(
            index: IndexName,
            query: SearchQuery,
        ) -> Result<SearchStream, SearchError> {
            let durability = Durability::<NoOutput, UnusedError>::new(
                "golem_search",
                "stream_search",
                DurableFunctionType::ReadRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    SearchStream::new(DurableSearchStream::<Impl>::live(Impl::unwrapped_stream(
                        index.clone(),
                        query.clone(),
                    )))
                });
                let _ = durability.persist_infallible(StreamSearchInput { index, query }, NoOutput);
                Ok(result)
            } else {
                let _: NoOutput = durability.replay_infallible();
                Ok(SearchStream::new(DurableSearchStream::<Impl>::replay(
                    index, query,
                )))
            }
        }

        fn get_schema(index: IndexName) -> Result<Schema, SearchError> {
            let durability = Durability::<SchemaWrapper, SearchError>::new(
                "golem_search",
                "get_schema",
                DurableFunctionType::ReadRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::get_schema(index.clone())
                });
                match result {
                    Ok(schema) => {
                        let _ = durability.persist_infallible(
                            GetSchemaInput { index },
                            SchemaWrapper {
                                schema: schema.clone(),
                            },
                        );
                        Ok(schema)
                    }
                    Err(e) => Err(e),
                }
            } else {
                let wrapper: SchemaWrapper = durability.replay_infallible();
                Ok(wrapper.schema)
            }
        }

        fn update_schema(index: IndexName, schema: Schema) -> Result<(), SearchError> {
            let durability = Durability::<VoidResult, SearchError>::new(
                "golem_search",
                "update_schema",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::update_schema(index.clone(), schema.clone())
                });
                match result {
                    Ok(()) => {
                        let _ = durability
                            .persist_infallible(UpdateSchemaInput { index, schema }, VoidResult);
                        Ok(())
                    }
                    Err(e) => Err(e),
                }
            } else {
                let _: VoidResult = durability.replay_infallible();
                Ok(())
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
    /// the retry query implemented in `ExtendedGuest` is used to create a new Search response
    /// stream and continue the search seamlessly.
    enum DurableSearchStreamState<Impl: ExtendedGuest> {
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

    pub struct DurableSearchStream<Impl: ExtendedGuest> {
        state: RefCell<Option<DurableSearchStreamState<Impl>>>,
        subscription: RefCell<Option<Pollable>>,
    }

    impl<Impl: ExtendedGuest> DurableSearchStream<Impl> {
        fn live(stream: Impl::SearchStream) -> Self {
            Self {
                state: RefCell::new(Some(DurableSearchStreamState::Live {
                    stream,
                    pollables: Vec::new(),
                })),
                subscription: RefCell::new(None),
            }
        }

        fn replay(index: IndexName, query: SearchQuery) -> Self {
            Self {
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

    impl<Impl: ExtendedGuest> Drop for DurableSearchStream<Impl> {
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

    impl<Impl: ExtendedGuest> GuestSearchStream for DurableSearchStream<Impl> {
        fn get_next(&self) -> Option<Vec<SearchHit>> {
            let durability = Durability::<Option<Vec<SearchHit>>, UnusedError>::new(
                "golem_search",
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
                                    let stream = <Impl as ExtendedGuest>::unwrapped_stream(
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

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::golem::search::types::*;
        use golem_rust::value_and_type::{FromValueAndType, IntoValueAndType};

        fn roundtrip_test<T>(value: T) -> T
        where
            T: IntoValueAndType + FromValueAndType + Clone + std::fmt::Debug + PartialEq,
        {
            let vnt = value.clone().into_value_and_type();
            let deserialized = T::from_value_and_type(vnt).unwrap();
            assert_eq!(value, deserialized);
            deserialized
        }

        #[test]
        fn search_error_roundtrip() {
            roundtrip_test(SearchError::IndexNotFound);
            roundtrip_test(SearchError::InvalidQuery("invalid syntax".to_string()));
            roundtrip_test(SearchError::Unsupported);
            roundtrip_test(SearchError::Internal(
                "database connection failed".to_string(),
            ));
            roundtrip_test(SearchError::Timeout);
            roundtrip_test(SearchError::RateLimited);
        }

        #[test]
        fn doc_roundtrip() {
            let doc = Doc {
                id: "test-id-123".to_string(),
                content:
                    r#"{"title": "Test Document", "author": "John Doe", "tags": ["rust", "wasm"]}"#
                        .to_string(),
            };
            roundtrip_test(doc);
        }

        #[test]
        fn highlight_config_roundtrip() {
            let config = HighlightConfig {
                fields: vec!["title".to_string(), "content".to_string()],
                pre_tag: Some("<mark>".to_string()),
                post_tag: Some("</mark>".to_string()),
                max_length: Some(150),
            };
            roundtrip_test(config);

            // Test with minimal fields
            let minimal_config = HighlightConfig {
                fields: vec!["title".to_string()],
                pre_tag: None,
                post_tag: None,
                max_length: None,
            };
            roundtrip_test(minimal_config);
        }

        #[test]
        fn search_config_roundtrip() {
            let config = SearchConfig {
                timeout_ms: Some(5000),
                boost_fields: vec![("title".to_string(), 2.0), ("content".to_string(), 1.0)],
                attributes_to_retrieve: vec!["id".to_string(), "title".to_string()],
                language: Some("en".to_string()),
                typo_tolerance: Some(true),
                exact_match_boost: Some(1.5),
                provider_params: Some(r#"{"custom_param": "value"}"#.to_string()),
            };
            roundtrip_test(config);

            // Test with minimal fields
            let minimal_config = SearchConfig {
                timeout_ms: None,
                boost_fields: vec![],
                attributes_to_retrieve: vec![],
                language: None,
                typo_tolerance: None,
                exact_match_boost: None,
                provider_params: None,
            };
            roundtrip_test(minimal_config);
        }

        #[test]
        fn search_query_roundtrip() {
            let query = SearchQuery {
                q: Some("rust programming language".to_string()),
                filters: vec!["category:programming".to_string(), "lang:en".to_string()],
                sort: vec!["score:desc".to_string(), "date:asc".to_string()],
                facets: vec!["category".to_string(), "author".to_string()],
                page: Some(2),
                per_page: Some(20),
                offset: Some(40),
                highlight: Some(HighlightConfig {
                    fields: vec!["title".to_string(), "content".to_string()],
                    pre_tag: Some("<em>".to_string()),
                    post_tag: Some("</em>".to_string()),
                    max_length: Some(200),
                }),
                config: Some(SearchConfig {
                    timeout_ms: Some(3000),
                    boost_fields: vec![("title".to_string(), 3.0)],
                    attributes_to_retrieve: vec!["id".to_string(), "title".to_string()],
                    language: Some("en".to_string()),
                    typo_tolerance: Some(false),
                    exact_match_boost: Some(2.0),
                    provider_params: None,
                }),
            };
            roundtrip_test(query);

            // Test minimal query
            let minimal_query = SearchQuery {
                q: None,
                filters: vec![],
                sort: vec![],
                facets: vec![],
                page: None,
                per_page: None,
                offset: None,
                highlight: None,
                config: None,
            };
            roundtrip_test(minimal_query);
        }

        #[test]
        fn search_hit_roundtrip() {
            let hit = SearchHit {
                id: "doc-123".to_string(),
                score: Some(0.95),
                content: Some(
                    r#"{"title": "Rust Programming", "content": "A guide to Rust"}"#.to_string(),
                ),
                highlights: Some(r#"{"title": ["<em>Rust</em> Programming"]}"#.to_string()),
            };
            roundtrip_test(hit);

            // Test minimal hit
            let minimal_hit = SearchHit {
                id: "doc-456".to_string(),
                score: None,
                content: None,
                highlights: None,
            };
            roundtrip_test(minimal_hit);
        }

        #[test]
        fn search_results_roundtrip() {
            let results = SearchResults {
                total: Some(150),
                page: Some(1),
                per_page: Some(10),
                hits: vec![
                    SearchHit {
                        id: "doc-1".to_string(),
                        score: Some(0.98),
                        content: Some(r#"{"title": "First Document"}"#.to_string()),
                        highlights: None,
                    },
                    SearchHit {
                        id: "doc-2".to_string(),
                        score: Some(0.85),
                        content: Some(r#"{"title": "Second Document"}"#.to_string()),
                        highlights: Some(r#"{"title": ["<em>Second</em> Document"]}"#.to_string()),
                    },
                ],
                facets: Some(r#"{"category": {"programming": 50, "tutorial": 25}}"#.to_string()),
                took_ms: Some(15),
            };
            roundtrip_test(results);

            // Test empty results
            let empty_results = SearchResults {
                total: Some(0),
                page: None,
                per_page: None,
                hits: vec![],
                facets: None,
                took_ms: Some(5),
            };
            roundtrip_test(empty_results);
        }

        #[test]
        fn field_type_roundtrip() {
            roundtrip_test(FieldType::Text);
            roundtrip_test(FieldType::Keyword);
            roundtrip_test(FieldType::Integer);
            roundtrip_test(FieldType::Float);
            roundtrip_test(FieldType::Boolean);
            roundtrip_test(FieldType::Date);
            roundtrip_test(FieldType::GeoPoint);
        }

        #[test]
        fn schema_field_roundtrip() {
            let field = SchemaField {
                name: "title".to_string(),
                field_type: FieldType::Text,
                required: true,
                facet: false,
                sort: true,
                index: true,
            };
            roundtrip_test(field);

            // Test keyword field with different settings
            let keyword_field = SchemaField {
                name: "category".to_string(),
                field_type: FieldType::Keyword,
                required: false,
                facet: true,
                sort: false,
                index: true,
            };
            roundtrip_test(keyword_field);

            // Test numeric field
            let numeric_field = SchemaField {
                name: "price".to_string(),
                field_type: FieldType::Float,
                required: true,
                facet: true,
                sort: true,
                index: false,
            };
            roundtrip_test(numeric_field);
        }

        #[test]
        fn schema_roundtrip() {
            let schema = Schema {
                fields: vec![
                    SchemaField {
                        name: "id".to_string(),
                        field_type: FieldType::Keyword,
                        required: true,
                        facet: false,
                        sort: false,
                        index: true,
                    },
                    SchemaField {
                        name: "title".to_string(),
                        field_type: FieldType::Text,
                        required: true,
                        facet: false,
                        sort: true,
                        index: true,
                    },
                    SchemaField {
                        name: "category".to_string(),
                        field_type: FieldType::Keyword,
                        required: false,
                        facet: true,
                        sort: false,
                        index: true,
                    },
                    SchemaField {
                        name: "price".to_string(),
                        field_type: FieldType::Float,
                        required: false,
                        facet: true,
                        sort: true,
                        index: false,
                    },
                    SchemaField {
                        name: "published_date".to_string(),
                        field_type: FieldType::Date,
                        required: false,
                        facet: false,
                        sort: true,
                        index: true,
                    },
                ],
                primary_key: Some("id".to_string()),
            };
            roundtrip_test(schema);

            // Test minimal schema
            let minimal_schema = Schema {
                fields: vec![],
                primary_key: None,
            };
            roundtrip_test(minimal_schema);
        }

        #[test]
        fn retry_query_logic_test() {
            // Test the retry query logic directly without implementing the full trait
            fn test_retry_query(
                original_query: &SearchQuery,
                partial_hits: &[SearchHit],
            ) -> SearchQuery {
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

            let original_query = SearchQuery {
                q: Some("test".to_string()),
                filters: vec![],
                sort: vec![],
                facets: vec![],
                page: Some(1),
                per_page: Some(10),
                offset: Some(0),
                highlight: None,
                config: None,
            };

            // Test retry with no partial hits
            let retry_query_empty = test_retry_query(&original_query, &[]);
            assert_eq!(retry_query_empty.offset, Some(0));

            // Test retry with partial hits
            let partial_hits = vec![
                SearchHit {
                    id: "doc1".to_string(),
                    score: Some(0.9),
                    content: None,
                    highlights: None,
                },
                SearchHit {
                    id: "doc2".to_string(),
                    score: Some(0.8),
                    content: None,
                    highlights: None,
                },
            ];

            let retry_query_with_hits = test_retry_query(&original_query, &partial_hits);
            assert_eq!(retry_query_with_hits.offset, Some(2)); // 0 + 2 hits

            // Test retry with existing offset
            let mut query_with_offset = original_query.clone();
            query_with_offset.offset = Some(20);

            let retry_query_offset = test_retry_query(&query_with_offset, &partial_hits);
            assert_eq!(retry_query_offset.offset, Some(22)); // 20 + 2 hits
        }

        #[test]
        fn index_name_and_document_id_types() {
            // These are type aliases for strings, but test them anyway
            let index_name: IndexName = "test-index".to_string();
            let document_id: DocumentId = "doc-123".to_string();

            assert_eq!(index_name, "test-index");
            assert_eq!(document_id, "doc-123");
        }

        #[test]
        fn complex_nested_structures() {
            // Test complex nested structures with all optional fields filled
            let complex_query = SearchQuery {
                q: Some("advanced search query with special chars: !@#$%".to_string()),
                filters: vec![
                    "category:electronics".to_string(),
                    "price:[100 TO 500]".to_string(),
                    "availability:true".to_string(),
                ],
                sort: vec![
                    "price:asc".to_string(),
                    "_score:desc".to_string(),
                    "date:desc".to_string(),
                ],
                facets: vec![
                    "category".to_string(),
                    "brand".to_string(),
                    "color".to_string(),
                ],
                page: Some(5),
                per_page: Some(50),
                offset: Some(200),
                highlight: Some(HighlightConfig {
                    fields: vec![
                        "title".to_string(),
                        "description".to_string(),
                        "content".to_string(),
                    ],
                    pre_tag: Some("<strong class='highlight'>".to_string()),
                    post_tag: Some("</strong>".to_string()),
                    max_length: Some(300),
                }),
                config: Some(SearchConfig {
                    timeout_ms: Some(10000),
                    boost_fields: vec![
                        ("title".to_string(), 5.0),
                        ("description".to_string(), 2.5),
                        ("content".to_string(), 1.0),
                        ("tags".to_string(), 3.0),
                    ],
                    attributes_to_retrieve: vec![
                        "id".to_string(),
                        "title".to_string(),
                        "description".to_string(),
                        "price".to_string(),
                        "image_url".to_string(),
                    ],
                    language: Some("en-US".to_string()),
                    typo_tolerance: Some(true),
                    exact_match_boost: Some(10.0),
                    provider_params: Some(
                        r#"{"index_settings": {"similarity": "BM25", "k1": 1.5, "b": 0.75}}"#
                            .to_string(),
                    ),
                }),
            };
            roundtrip_test(complex_query);
        }
    }
}
