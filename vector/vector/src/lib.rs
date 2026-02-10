pub mod config;
pub mod durability;
pub mod error;
pub mod model;

use golem_rust::golem_wasm::{NodeBuilder, ResourceMode, Uri, WitValueExtractor};
use golem_rust::value_and_type::{FromValueAndType, IntoValue, TypeNodeBuilder};

use model::analytics::{CollectionStats, FieldStats};
use model::collections::{CollectionInfo, DistanceMetric, IndexConfig, Metadata, VectorError};
use model::connection::{ConnectionStatus, Credentials};
use model::namespaces::NamespaceInfo;
use model::search::{SearchQuery, SearchResult, VectorData};
use model::search_extended::{
    ContextPair, GroupedSearchResult, RecommendationExample, RecommendationStrategy,
};
use model::types::{FilterExpression, FilterFunc, MetadataFunc, MetadataValue};
use model::vectors::{BatchResult, Id, ListResponse, VectorRecord};
use std::cell::RefCell;
use std::str::FromStr;

const METADATA_FUNC_ID: u64 = 1;
const FILTER_FUNC_ID: u64 = 2;

pub trait FuncProvider {
    type MetadataFunc: MetadataFuncInterface;
    type FilterFunc: FilterFuncInterface;
}

pub trait MetadataFuncInterface: 'static {
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
    fn get(&self) -> MetadataValue;
}

pub trait FilterFuncInterface: 'static {
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
    fn get(&self) -> FilterExpression;
}

pub trait CollectionProvider {
    fn upsert_collection(
        name: String,
        description: Option<String>,
        dimension: u32,
        metric: DistanceMetric,
        index_config: Option<IndexConfig>,
        metadata: Option<Metadata>,
    ) -> Result<CollectionInfo, VectorError>;
    fn list_collections() -> Result<Vec<String>, VectorError>;
    fn get_collection(name: String) -> Result<CollectionInfo, VectorError>;
    fn update_collection(
        name: String,
        description: Option<String>,
        metadata: Option<Metadata>,
    ) -> Result<CollectionInfo, VectorError>;
    fn delete_collection(name: String) -> Result<(), VectorError>;
    fn collection_exists(name: String) -> Result<bool, VectorError>;
}

#[allow(clippy::too_many_arguments)]
pub trait SearchProvider {
    fn search_vectors(
        collection: String,
        query: SearchQuery,
        limit: u32,
        filter: Option<model::search::FilterExpression>,
        namespace: Option<String>,
        include_vectors: Option<bool>,
        include_metadata: Option<bool>,
        min_score: Option<f32>,
        max_distance: Option<f32>,
        search_params: Option<Vec<(String, String)>>,
    ) -> Result<Vec<SearchResult>, model::search::VectorError>;
    fn find_similar(
        collection: String,
        vector: VectorData,
        limit: u32,
        namespace: Option<String>,
    ) -> Result<Vec<SearchResult>, model::search::VectorError>;
    fn batch_search(
        collection: String,
        queries: Vec<SearchQuery>,
        limit: u32,
        filter: Option<model::search::FilterExpression>,
        namespace: Option<String>,
        include_vectors: Option<bool>,
        include_metadata: Option<bool>,
        search_params: Option<Vec<(String, String)>>,
    ) -> Result<Vec<Vec<SearchResult>>, model::search::VectorError>;
}

macro_rules! impl_resource_traits {
    ($ResourceType:ty, $InnerType:ty, $UriString:literal, $TypeIdConstant:ident) => {
        impl Clone for $ResourceType {
            fn clone(&self) -> Self {
                Self::new(self.get::<$InnerType>().clone())
            }
        }

        impl PartialEq for $ResourceType {
            fn eq(&self, other: &Self) -> bool {
                self.get::<$InnerType>() == other.get::<$InnerType>()
            }
        }

        impl IntoValue for $ResourceType {
            fn add_to_builder<B: NodeBuilder>(self, builder: B) -> B::Result {
                builder.handle(
                    Uri {
                        value: $UriString.to_string(),
                    },
                    0u64,
                )
            }

            fn add_to_type_builder<B: TypeNodeBuilder>(builder: B) -> B::Result {
                builder.handle(None, None, $TypeIdConstant, ResourceMode::Owned)
            }
        }

        impl FromValueAndType for $ResourceType {
            fn from_extractor<'a, 'b>(
                extractor: &'a impl WitValueExtractor<'a, 'b>,
            ) -> Result<Self, String> {
                <$InnerType>::from_extractor(extractor).map(Self::new)
            }
        }
    };
}

#[allow(clippy::too_many_arguments)]
pub trait SearchExtendedProvider {
    fn recommend_vectors(
        collection: String,
        positive: Vec<RecommendationExample>,
        negative: Option<Vec<RecommendationExample>>,
        limit: u32,
        filter: Option<model::search_extended::FilterExpression>,
        namespace: Option<String>,
        strategy: Option<RecommendationStrategy>,
        include_vectors: Option<bool>,
        include_metadata: Option<bool>,
    ) -> Result<Vec<model::search_extended::SearchResult>, model::search_extended::VectorError>;
    fn discover_vectors(
        collection: String,
        target: Option<RecommendationExample>,
        context_pairs: Vec<ContextPair>,
        limit: u32,
        filter: Option<model::search_extended::FilterExpression>,
        namespace: Option<String>,
        include_vectors: Option<bool>,
        include_metadata: Option<bool>,
    ) -> Result<Vec<model::search_extended::SearchResult>, model::search_extended::VectorError>;
    fn search_groups(
        collection: String,
        query: model::search_extended::SearchQuery,
        group_by: String,
        group_size: u32,
        max_groups: u32,
        filter: Option<model::search_extended::FilterExpression>,
        namespace: Option<String>,
        include_vectors: Option<bool>,
        include_metadata: Option<bool>,
    ) -> Result<Vec<GroupedSearchResult>, model::search_extended::VectorError>;
    fn search_range(
        collection: String,
        vector: model::search_extended::VectorData,
        min_distance: Option<f32>,
        max_distance: f32,
        filter: Option<model::search_extended::FilterExpression>,
        namespace: Option<String>,
        limit: Option<u32>,
        include_vectors: Option<bool>,
        include_metadata: Option<bool>,
    ) -> Result<Vec<model::search_extended::SearchResult>, model::search_extended::VectorError>;
    fn search_text(
        collection: String,
        query_text: String,
        limit: u32,
        filter: Option<model::search_extended::FilterExpression>,
        namespace: Option<String>,
    ) -> Result<Vec<model::search_extended::SearchResult>, model::search_extended::VectorError>;
}

pub trait NamespacesProvider {
    fn upsert_namespace(
        collection: String,
        namespace: String,
        metadata: Option<model::namespaces::Metadata>,
    ) -> Result<NamespaceInfo, model::namespaces::VectorError>;
    fn list_namespaces(
        collection: String,
    ) -> Result<Vec<NamespaceInfo>, model::namespaces::VectorError>;
    fn get_namespace(
        collection: String,
        namespace: String,
    ) -> Result<NamespaceInfo, model::namespaces::VectorError>;
    fn delete_namespace(
        collection: String,
        namespace: String,
    ) -> Result<(), model::namespaces::VectorError>;
    fn namespace_exists(
        collection: String,
        namespace: String,
    ) -> Result<bool, model::namespaces::VectorError>;
}

pub trait AnalyticsProvider {
    fn get_collection_stats(
        collection: String,
        namespace: Option<String>,
    ) -> Result<CollectionStats, model::analytics::VectorError>;
    fn get_field_stats(
        collection: String,
        field: String,
        namespace: Option<String>,
    ) -> Result<FieldStats, model::analytics::VectorError>;
    fn get_field_distribution(
        collection: String,
        field: String,
        limit: Option<u32>,
        namespace: Option<String>,
    ) -> Result<Vec<(model::analytics::MetadataValue, u64)>, model::analytics::VectorError>;
}

pub trait ConnectionProvider {
    fn connect(
        endpoint: String,
        credentials: Option<Credentials>,
        timeout_ms: Option<u32>,
        options: Option<model::connection::Metadata>,
    ) -> Result<(), model::connection::VectorError>;
    fn disconnect() -> Result<(), model::connection::VectorError>;
    fn get_connection_status() -> Result<ConnectionStatus, model::connection::VectorError>;
    fn test_connection(
        endpoint: String,
        credentials: Option<Credentials>,
        timeout_ms: Option<u32>,
        options: Option<model::connection::Metadata>,
    ) -> Result<bool, model::connection::VectorError>;
}

pub trait VectorsProvider {
    fn upsert_vectors(
        collection: String,
        vectors: Vec<VectorRecord>,
        namespace: Option<String>,
    ) -> Result<BatchResult, model::vectors::VectorError>;
    fn upsert_vector(
        collection: String,
        id: Id,
        vector: model::vectors::VectorData,
        metadata: Option<model::vectors::Metadata>,
        namespace: Option<String>,
    ) -> Result<(), model::vectors::VectorError>;
    fn get_vectors(
        collection: String,
        ids: Vec<Id>,
        namespace: Option<String>,
        include_vectors: Option<bool>,
        include_metadata: Option<bool>,
    ) -> Result<Vec<VectorRecord>, model::vectors::VectorError>;
    fn get_vector(
        collection: String,
        id: Id,
        namespace: Option<String>,
    ) -> Result<Option<VectorRecord>, model::vectors::VectorError>;
    fn update_vector(
        collection: String,
        id: Id,
        vector: Option<model::vectors::VectorData>,
        metadata: Option<model::vectors::Metadata>,
        namespace: Option<String>,
        merge_metadata: Option<bool>,
    ) -> Result<(), model::vectors::VectorError>;
    fn delete_vectors(
        collection: String,
        ids: Vec<Id>,
        namespace: Option<String>,
    ) -> Result<u32, model::vectors::VectorError>;
    fn delete_by_filter(
        collection: String,
        filter: model::vectors::FilterExpression,
        namespace: Option<String>,
    ) -> Result<u32, model::vectors::VectorError>;
    fn delete_namespace(
        collection: String,
        namespace: String,
    ) -> Result<u32, model::vectors::VectorError>;
    fn list_vectors(
        collection: String,
        namespace: Option<String>,
        filter: Option<model::vectors::FilterExpression>,
        limit: Option<u32>,
        cursor: Option<String>,
        include_vectors: Option<bool>,
        include_metadata: Option<bool>,
    ) -> Result<ListResponse, model::vectors::VectorError>;
    fn count_vectors(
        collection: String,
        filter: Option<model::vectors::FilterExpression>,
        namespace: Option<String>,
    ) -> Result<u64, model::vectors::VectorError>;
}

impl_resource_traits!(
    MetadataFunc,
    MetadataValue,
    "golem:vector/types/metadata-func",
    METADATA_FUNC_ID
);
impl_resource_traits!(
    FilterFunc,
    FilterExpression,
    "golem:vector/types/filter-func",
    FILTER_FUNC_ID
);

impl MetadataFuncInterface for MetadataValue {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn get(&self) -> MetadataValue {
        self.clone()
    }
}

impl FilterFuncInterface for FilterExpression {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn get(&self) -> FilterExpression {
        self.clone()
    }
}

struct LoggingState {
    logging_initialized: bool,
}

impl LoggingState {
    fn init(&mut self) {
        if !self.logging_initialized {
            let _ = wasi_logger::Logger::install();
            let max_level: log::LevelFilter =
                log::LevelFilter::from_str(&std::env::var("GOLEM_VECTOR_LOG").unwrap_or_default())
                    .unwrap_or(log::LevelFilter::Info);
            log::set_max_level(max_level);
            self.logging_initialized = true;
        }
    }
}

thread_local! {
    static LOGGING_STATE: RefCell<LoggingState> = const { RefCell::new(LoggingState {
        logging_initialized: false,
    }) };
}

pub fn init_logging() {
    LOGGING_STATE.with_borrow_mut(|state| state.init());
}
