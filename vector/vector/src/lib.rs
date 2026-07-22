pub mod config;
pub mod durability;
pub mod error;
pub mod model;

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

#[allow(async_fn_in_trait)]
pub trait CollectionProvider {
    /// Provider-specific configuration that the caller resolves and passes in.
    type ProviderConfig: Clone + 'static;

    async fn upsert_collection(
        provider_config: Self::ProviderConfig,
        name: String,
        description: Option<String>,
        dimension: u32,
        metric: DistanceMetric,
        index_config: Option<IndexConfig>,
        metadata: Option<Metadata>,
    ) -> Result<CollectionInfo, VectorError>;
    async fn list_collections(
        provider_config: Self::ProviderConfig,
    ) -> Result<Vec<String>, VectorError>;
    async fn get_collection(
        provider_config: Self::ProviderConfig,
        name: String,
    ) -> Result<CollectionInfo, VectorError>;
    async fn update_collection(
        provider_config: Self::ProviderConfig,
        name: String,
        description: Option<String>,
        metadata: Option<Metadata>,
    ) -> Result<CollectionInfo, VectorError>;
    async fn delete_collection(
        provider_config: Self::ProviderConfig,
        name: String,
    ) -> Result<(), VectorError>;
    async fn collection_exists(
        provider_config: Self::ProviderConfig,
        name: String,
    ) -> Result<bool, VectorError>;
}

#[allow(clippy::too_many_arguments)]
#[allow(async_fn_in_trait)]
pub trait SearchProvider {
    /// Provider-specific configuration that the caller resolves and passes in.
    type ProviderConfig: Clone + 'static;

    async fn search_vectors(
        provider_config: Self::ProviderConfig,
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
    async fn find_similar(
        provider_config: Self::ProviderConfig,
        collection: String,
        vector: VectorData,
        limit: u32,
        namespace: Option<String>,
    ) -> Result<Vec<SearchResult>, model::search::VectorError>;
    async fn batch_search(
        provider_config: Self::ProviderConfig,
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

impl Clone for MetadataFunc {
    fn clone(&self) -> Self {
        Self::new(self.get::<MetadataValue>().clone())
    }
}

impl PartialEq for MetadataFunc {
    fn eq(&self, other: &Self) -> bool {
        self.get::<MetadataValue>() == other.get::<MetadataValue>()
    }
}

impl Clone for FilterFunc {
    fn clone(&self) -> Self {
        Self::new(self.get::<FilterExpression>().clone())
    }
}

impl PartialEq for FilterFunc {
    fn eq(&self, other: &Self) -> bool {
        self.get::<FilterExpression>() == other.get::<FilterExpression>()
    }
}

#[cfg(feature = "golem")]
macro_rules! impl_transparent_schema {
    ($wrapper:ty, $inner:ty) => {
        impl golem_rust::IntoSchema for $wrapper {
            fn type_id() -> golem_rust::schema::TypeId {
                <$inner as golem_rust::IntoSchema>::type_id()
            }

            fn register_in(
                builder: &mut golem_rust::schema::SchemaBuilder,
            ) -> golem_rust::SchemaType {
                <$inner as golem_rust::IntoSchema>::register_in(builder)
            }

            fn to_value(&self) -> golem_rust::SchemaValue {
                self.get::<$inner>().to_value()
            }
        }

        impl golem_rust::FromSchema for $wrapper {
            fn from_value(
                value: &golem_rust::SchemaValue,
            ) -> Result<Self, golem_rust::schema::FromSchemaError> {
                <$inner as golem_rust::FromSchema>::from_value(value).map(Self::new)
            }
        }
    };
}

#[cfg(feature = "golem")]
impl_transparent_schema!(MetadataFunc, MetadataValue);
#[cfg(feature = "golem")]
impl_transparent_schema!(FilterFunc, FilterExpression);

#[allow(clippy::too_many_arguments)]
#[allow(async_fn_in_trait)]
pub trait SearchExtendedProvider {
    /// Provider-specific configuration that the caller resolves and passes in.
    type ProviderConfig: Clone + 'static;

    async fn recommend_vectors(
        provider_config: Self::ProviderConfig,
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
    async fn discover_vectors(
        provider_config: Self::ProviderConfig,
        collection: String,
        target: Option<RecommendationExample>,
        context_pairs: Vec<ContextPair>,
        limit: u32,
        filter: Option<model::search_extended::FilterExpression>,
        namespace: Option<String>,
        include_vectors: Option<bool>,
        include_metadata: Option<bool>,
    ) -> Result<Vec<model::search_extended::SearchResult>, model::search_extended::VectorError>;
    async fn search_groups(
        provider_config: Self::ProviderConfig,
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
    async fn search_range(
        provider_config: Self::ProviderConfig,
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
    async fn search_text(
        provider_config: Self::ProviderConfig,
        collection: String,
        query_text: String,
        limit: u32,
        filter: Option<model::search_extended::FilterExpression>,
        namespace: Option<String>,
    ) -> Result<Vec<model::search_extended::SearchResult>, model::search_extended::VectorError>;
}

#[allow(async_fn_in_trait)]
pub trait NamespacesProvider {
    /// Provider-specific configuration that the caller resolves and passes in.
    type ProviderConfig: Clone + 'static;

    async fn upsert_namespace(
        provider_config: Self::ProviderConfig,
        collection: String,
        namespace: String,
        metadata: Option<model::namespaces::Metadata>,
    ) -> Result<NamespaceInfo, model::namespaces::VectorError>;
    async fn list_namespaces(
        provider_config: Self::ProviderConfig,
        collection: String,
    ) -> Result<Vec<NamespaceInfo>, model::namespaces::VectorError>;
    async fn get_namespace(
        provider_config: Self::ProviderConfig,
        collection: String,
        namespace: String,
    ) -> Result<NamespaceInfo, model::namespaces::VectorError>;
    async fn delete_namespace(
        provider_config: Self::ProviderConfig,
        collection: String,
        namespace: String,
    ) -> Result<(), model::namespaces::VectorError>;
    async fn namespace_exists(
        provider_config: Self::ProviderConfig,
        collection: String,
        namespace: String,
    ) -> Result<bool, model::namespaces::VectorError>;
}

#[allow(async_fn_in_trait)]
pub trait AnalyticsProvider {
    /// Provider-specific configuration that the caller resolves and passes in.
    type ProviderConfig: Clone + 'static;

    async fn get_collection_stats(
        provider_config: Self::ProviderConfig,
        collection: String,
        namespace: Option<String>,
    ) -> Result<CollectionStats, model::analytics::VectorError>;
    async fn get_field_stats(
        provider_config: Self::ProviderConfig,
        collection: String,
        field: String,
        namespace: Option<String>,
    ) -> Result<FieldStats, model::analytics::VectorError>;
    async fn get_field_distribution(
        provider_config: Self::ProviderConfig,
        collection: String,
        field: String,
        limit: Option<u32>,
        namespace: Option<String>,
    ) -> Result<Vec<(model::analytics::MetadataValue, u64)>, model::analytics::VectorError>;
}

#[allow(async_fn_in_trait)]
pub trait ConnectionProvider {
    /// Provider-specific configuration that the caller resolves and passes in.
    type ProviderConfig: Clone + 'static;

    async fn connect(
        provider_config: Self::ProviderConfig,
        endpoint: String,
        credentials: Option<Credentials>,
        timeout_ms: Option<u32>,
        options: Option<model::connection::Metadata>,
    ) -> Result<(), model::connection::VectorError>;
    async fn disconnect(
        provider_config: Self::ProviderConfig,
    ) -> Result<(), model::connection::VectorError>;
    async fn get_connection_status(
        provider_config: Self::ProviderConfig,
    ) -> Result<ConnectionStatus, model::connection::VectorError>;
    async fn test_connection(
        provider_config: Self::ProviderConfig,
        endpoint: String,
        credentials: Option<Credentials>,
        timeout_ms: Option<u32>,
        options: Option<model::connection::Metadata>,
    ) -> Result<bool, model::connection::VectorError>;
}

#[allow(async_fn_in_trait)]
pub trait VectorsProvider {
    /// Provider-specific configuration that the caller resolves and passes in.
    type ProviderConfig: Clone + 'static;

    async fn upsert_vectors(
        provider_config: Self::ProviderConfig,
        collection: String,
        vectors: Vec<VectorRecord>,
        namespace: Option<String>,
    ) -> Result<BatchResult, model::vectors::VectorError>;
    async fn upsert_vector(
        provider_config: Self::ProviderConfig,
        collection: String,
        id: Id,
        vector: model::vectors::VectorData,
        metadata: Option<model::vectors::Metadata>,
        namespace: Option<String>,
    ) -> Result<(), model::vectors::VectorError>;
    async fn get_vectors(
        provider_config: Self::ProviderConfig,
        collection: String,
        ids: Vec<Id>,
        namespace: Option<String>,
        include_vectors: Option<bool>,
        include_metadata: Option<bool>,
    ) -> Result<Vec<VectorRecord>, model::vectors::VectorError>;
    async fn get_vector(
        provider_config: Self::ProviderConfig,
        collection: String,
        id: Id,
        namespace: Option<String>,
    ) -> Result<Option<VectorRecord>, model::vectors::VectorError>;
    async fn update_vector(
        provider_config: Self::ProviderConfig,
        collection: String,
        id: Id,
        vector: Option<model::vectors::VectorData>,
        metadata: Option<model::vectors::Metadata>,
        namespace: Option<String>,
        merge_metadata: Option<bool>,
    ) -> Result<(), model::vectors::VectorError>;
    async fn delete_vectors(
        provider_config: Self::ProviderConfig,
        collection: String,
        ids: Vec<Id>,
        namespace: Option<String>,
    ) -> Result<u32, model::vectors::VectorError>;
    async fn delete_by_filter(
        provider_config: Self::ProviderConfig,
        collection: String,
        filter: model::vectors::FilterExpression,
        namespace: Option<String>,
    ) -> Result<u32, model::vectors::VectorError>;
    async fn delete_namespace(
        provider_config: Self::ProviderConfig,
        collection: String,
        namespace: String,
    ) -> Result<u32, model::vectors::VectorError>;
    #[allow(clippy::too_many_arguments)]
    async fn list_vectors(
        provider_config: Self::ProviderConfig,
        collection: String,
        namespace: Option<String>,
        filter: Option<model::vectors::FilterExpression>,
        limit: Option<u32>,
        cursor: Option<String>,
        include_vectors: Option<bool>,
        include_metadata: Option<bool>,
    ) -> Result<ListResponse, model::vectors::VectorError>;
    async fn count_vectors(
        provider_config: Self::ProviderConfig,
        collection: String,
        filter: Option<model::vectors::FilterExpression>,
        namespace: Option<String>,
    ) -> Result<u64, model::vectors::VectorError>;
}

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
