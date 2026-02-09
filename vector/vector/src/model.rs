pub mod types {
    use crate::{FilterFuncInterface, MetadataFuncInterface};

    pub type Id = String;
    pub type DenseVector = Vec<f32>;
    pub type Metadata = Vec<(String, MetadataValue)>;

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct SparseVector {
        pub indices: Vec<u32>,
        pub values: Vec<f32>,
        pub total_dimensions: u32,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct BinaryVector {
        pub data: Vec<u8>,
        pub dimensions: u32,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct HalfVector {
        pub data: Vec<f32>,
        pub dimensions: u32,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub enum VectorData {
        Dense(DenseVector),
        Sparse(SparseVector),
        Binary(BinaryVector),
        Half(HalfVector),
        Named(Vec<(String, DenseVector)>),
        Hybrid((DenseVector, SparseVector)),
    }

    #[repr(u8)]
    #[derive(
        Clone,
        Copy,
        Debug,
        Eq,
        Ord,
        PartialEq,
        PartialOrd,
        golem_rust::FromValueAndType,
        golem_rust::IntoValue,
    )]
    pub enum DistanceMetric {
        Cosine,
        Euclidean,
        DotProduct,
        Manhattan,
        Hamming,
        Jaccard,
    }

    pub struct MetadataFunc {
        inner: Box<dyn MetadataFuncInterface>,
    }

    impl std::fmt::Debug for MetadataFunc {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("MetadataFunc").finish()
        }
    }

    impl MetadataFunc {
        pub fn new<T: MetadataFuncInterface>(val: T) -> Self {
            Self {
                inner: Box::new(val),
            }
        }

        pub fn get<T: MetadataFuncInterface>(&self) -> &T {
            self.inner
                .as_any()
                .downcast_ref::<T>()
                .expect("MetadataFunc type mismatch")
        }

        pub fn get_mut<T: MetadataFuncInterface>(&mut self) -> &mut T {
            self.inner
                .as_any_mut()
                .downcast_mut::<T>()
                .expect("MetadataFunc type mismatch")
        }
    }

    impl std::ops::Deref for MetadataFunc {
        type Target = dyn MetadataFuncInterface;
        fn deref(&self) -> &Self::Target {
            &*self.inner
        }
    }

    impl std::ops::DerefMut for MetadataFunc {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut *self.inner
        }
    }

    #[derive(
        Clone, Copy, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue,
    )]
    pub struct GeoCoordinates {
        pub latitude: f64,
        pub longitude: f64,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub enum MetadataValue {
        StringVal(String),
        NumberVal(f64),
        IntegerVal(i64),
        BooleanVal(bool),
        ArrayVal(Vec<MetadataFunc>),
        ObjectVal(Vec<(String, MetadataFunc)>),
        NullVal,
        GeoVal(GeoCoordinates),
        DatetimeVal(String),
        BlobVal(Vec<u8>),
    }

    #[repr(u8)]
    #[derive(
        Clone,
        Copy,
        Debug,
        Eq,
        Ord,
        PartialEq,
        PartialOrd,
        golem_rust::FromValueAndType,
        golem_rust::IntoValue,
    )]
    pub enum FilterOperator {
        Eq,
        Ne,
        Gt,
        Gte,
        Lt,
        Lte,
        In,
        Nin,
        Contains,
        NotContains,
        Regex,
        GeoWithin,
        GeoBbox,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct FilterCondition {
        pub field: String,
        pub operator: FilterOperator,
        pub value: MetadataValue,
    }

    pub struct FilterFunc {
        inner: Box<dyn FilterFuncInterface>,
    }

    impl std::fmt::Debug for FilterFunc {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("FilterFunc").finish()
        }
    }

    impl FilterFunc {
        pub fn new<T: FilterFuncInterface>(val: T) -> Self {
            Self {
                inner: Box::new(val),
            }
        }

        pub fn get<T: FilterFuncInterface>(&self) -> &T {
            self.inner
                .as_any()
                .downcast_ref::<T>()
                .expect("FilterFunc type mismatch")
        }

        pub fn get_mut<T: FilterFuncInterface>(&mut self) -> &mut T {
            self.inner
                .as_any_mut()
                .downcast_mut::<T>()
                .expect("FilterFunc type mismatch")
        }
    }

    impl std::ops::Deref for FilterFunc {
        type Target = dyn FilterFuncInterface;
        fn deref(&self) -> &Self::Target {
            &*self.inner
        }
    }

    impl std::ops::DerefMut for FilterFunc {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut *self.inner
        }
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub enum FilterExpression {
        Condition(FilterCondition),
        And(Vec<FilterFunc>),
        Or(Vec<FilterFunc>),
        Not(FilterFunc),
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct VectorRecord {
        pub id: Id,
        pub vector: VectorData,
        pub metadata: Option<Metadata>,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct SearchResult {
        pub id: Id,
        pub score: f32,
        pub distance: f32,
        pub vector: Option<VectorData>,
        pub metadata: Option<Metadata>,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub enum VectorError {
        NotFound(String),
        AlreadyExists(String),
        InvalidParams(String),
        UnsupportedFeature(String),
        DimensionMismatch(String),
        InvalidVector(String),
        Unauthorized(String),
        RateLimited(String),
        ProviderError(String),
        ConnectionError(String),
    }

    impl core::fmt::Display for VectorError {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            write!(f, "{:?}", self)
        }
    }

    impl std::error::Error for VectorError {}
}

pub mod collections {
    pub type Id = super::types::Id;
    pub type DistanceMetric = super::types::DistanceMetric;
    pub type VectorError = super::types::VectorError;
    pub type Metadata = super::types::Metadata;

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct IndexConfig {
        pub index_type: Option<String>,
        pub parameters: Vec<(String, String)>,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct CollectionInfo {
        pub name: String,
        pub description: Option<String>,
        pub dimension: u32,
        pub metric: DistanceMetric,
        pub vector_count: u64,
        pub size_bytes: Option<u64>,
        pub index_ready: bool,
        pub created_at: Option<u64>,
        pub updated_at: Option<u64>,
        pub provider_stats: Option<Metadata>,
    }
}

pub mod search {
    pub type Id = super::types::Id;
    pub type VectorData = super::types::VectorData;
    pub type SearchResult = super::types::SearchResult;
    pub type FilterExpression = super::types::FilterExpression;
    pub type VectorError = super::types::VectorError;

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub enum SearchQuery {
        Vector(VectorData),
        ById(Id),
        MultiVector(Vec<(String, VectorData)>),
    }
}

pub mod search_extended {
    pub type Id = super::types::Id;
    pub type VectorData = super::types::VectorData;
    pub type SearchResult = super::types::SearchResult;
    pub type FilterExpression = super::types::FilterExpression;
    pub type VectorError = super::types::VectorError;
    pub type MetadataValue = super::types::MetadataValue;
    pub type SearchQuery = super::search::SearchQuery;

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub enum RecommendationExample {
        VectorId(Id),
        VectorData(VectorData),
    }

    #[repr(u8)]
    #[derive(
        Clone,
        Copy,
        Debug,
        Eq,
        Ord,
        PartialEq,
        PartialOrd,
        golem_rust::FromValueAndType,
        golem_rust::IntoValue,
    )]
    pub enum RecommendationStrategy {
        AverageVector,
        BestScore,
        Centroid,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct ContextPair {
        pub positive: RecommendationExample,
        pub negative: RecommendationExample,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct GroupedSearchResult {
        pub group_value: MetadataValue,
        pub results: Vec<SearchResult>,
        pub group_count: u32,
    }
}

pub mod namespaces {
    pub type VectorError = super::types::VectorError;
    pub type Metadata = super::types::Metadata;

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct NamespaceInfo {
        pub name: String,
        pub collection: String,
        pub vector_count: u64,
        pub size_bytes: u64,
        pub created_at: Option<u64>,
        pub metadata: Option<Metadata>,
    }
}

pub mod analytics {
    pub type VectorError = super::types::VectorError;
    pub type MetadataValue = super::types::MetadataValue;
    pub type FilterExpression = super::types::FilterExpression;

    #[derive(
        Clone, Copy, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue,
    )]
    pub struct NamespaceStats {
        pub vector_count: u64,
        pub size_bytes: u64,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct DistanceStats {
        pub min_distance: f32,
        pub max_distance: f32,
        pub avg_distance: f32,
        pub percentiles: Vec<(f32, f32)>,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct CollectionStats {
        pub vector_count: u64,
        pub dimension: u32,
        pub size_bytes: u64,
        pub index_size_bytes: Option<u64>,
        pub namespace_stats: Vec<(String, NamespaceStats)>,
        pub distance_distribution: Option<DistanceStats>,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct FieldStats {
        pub field_name: String,
        pub value_count: u64,
        pub unique_values: u64,
        pub null_count: u64,
        pub data_type: String,
        pub sample_values: Vec<MetadataValue>,
    }
}

pub mod connection {
    pub type VectorError = super::types::VectorError;
    pub type Metadata = super::types::Metadata;

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct OauthConfig {
        pub client_id: String,
        pub client_secret: Option<String>,
        pub token_url: String,
        pub scope: Option<String>,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub enum Credentials {
        ApiKey(String),
        UsernamePassword((String, String)),
        Token(String),
        Certificate(Vec<u8>),
        Oauth(OauthConfig),
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct ConnectionStatus {
        pub connected: bool,
        pub provider: Option<String>,
        pub endpoint: Option<String>,
        pub last_activity: Option<u64>,
        pub connection_id: Option<String>,
    }
}

pub mod vectors {
    pub type Id = super::types::Id;
    pub type VectorRecord = super::types::VectorRecord;
    pub type VectorData = super::types::VectorData;
    pub type Metadata = super::types::Metadata;
    pub type FilterExpression = super::types::FilterExpression;
    pub type VectorError = super::types::VectorError;

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct BatchResult {
        pub success_count: u32,
        pub failure_count: u32,
        pub errors: Vec<(u32, VectorError)>,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct ListResponse {
        pub vectors: Vec<VectorRecord>,
        pub next_cursor: Option<String>,
        pub total_count: Option<u64>,
    }
}
