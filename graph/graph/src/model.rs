pub mod types {
    #[derive(
        Clone, Copy, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue,
    )]
    pub struct Date {
        pub year: u32,
        pub month: u8,
        pub day: u8,
    }

    #[derive(
        Clone, Copy, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue,
    )]
    pub struct Time {
        pub hour: u8,
        pub minute: u8,
        pub second: u8,
        pub nanosecond: u32,
    }

    #[derive(
        Clone, Copy, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue,
    )]
    pub struct Datetime {
        pub date: Date,
        pub time: Time,
        pub timezone_offset_minutes: Option<i16>,
    }

    #[derive(
        Clone, Copy, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue,
    )]
    pub struct Duration {
        pub seconds: i64,
        pub nanoseconds: u32,
    }

    #[derive(
        Clone, Copy, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue,
    )]
    pub struct Point {
        pub longitude: f64,
        pub latitude: f64,
        pub altitude: Option<f64>,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct Linestring {
        pub coordinates: Vec<Point>,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct Polygon {
        pub exterior: Vec<Point>,
        pub holes: Option<Vec<Vec<Point>>>,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub enum PropertyValue {
        NullValue,
        Boolean(bool),
        Int8(i8),
        Int16(i16),
        Int32(i32),
        Int64(i64),
        Uint8(u8),
        Uint16(u16),
        Uint32(u32),
        Uint64(u64),
        Float32Value(f32),
        Float64Value(f64),
        StringValue(String),
        Bytes(Vec<u8>),
        Date(Date),
        Time(Time),
        Datetime(Datetime),
        Duration(Duration),
        Point(Point),
        Linestring(Linestring),
        Polygon(Polygon),
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub enum ElementId {
        StringValue(String),
        Int64(i64),
        Uuid(String),
    }

    pub type PropertyMap = Vec<(String, PropertyValue)>;

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct Vertex {
        pub id: ElementId,
        pub vertex_type: String,
        pub additional_labels: Vec<String>,
        pub properties: PropertyMap,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct Edge {
        pub id: ElementId,
        pub edge_type: String,
        pub from_vertex: ElementId,
        pub to_vertex: ElementId,
        pub properties: PropertyMap,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct Path {
        pub vertices: Vec<Vertex>,
        pub edges: Vec<Edge>,
        pub length: u32,
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
    pub enum Direction {
        Outgoing,
        Incoming,
        Both,
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
    pub enum ComparisonOperator {
        Equal,
        NotEqual,
        LessThan,
        LessThanOrEqual,
        GreaterThan,
        GreaterThanOrEqual,
        Contains,
        StartsWith,
        EndsWith,
        RegexMatch,
        InList,
        NotInList,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct FilterCondition {
        pub property: String,
        pub operator: ComparisonOperator,
        pub value: PropertyValue,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct SortSpec {
        pub property: String,
        pub ascending: bool,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub enum QueryResult {
        Vertices(Vec<Vertex>),
        Edges(Vec<Edge>),
        Paths(Vec<Path>),
        Values(Vec<PropertyValue>),
        Maps(Vec<Vec<(String, PropertyValue)>>),
    }

    pub type QueryParameters = Vec<(String, PropertyValue)>;

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct ExecuteQueryOptions {
        pub query: String,
        pub parameters: Option<QueryParameters>,
        pub timeout_seconds: Option<u32>,
        pub max_results: Option<u32>,
        pub explain: Option<bool>,
        pub profile: Option<bool>,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct CreateVertexOptions {
        pub vertex_type: String,
        pub properties: Option<PropertyMap>,
        pub labels: Option<Vec<String>>,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct UpdateVertexOptions {
        pub id: ElementId,
        pub properties: PropertyMap,
        pub partial: Option<bool>,
        pub create_missing: Option<bool>,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct CreateEdgeOptions {
        pub edge_type: String,
        pub from_vertex: ElementId,
        pub to_vertex: ElementId,
        pub properties: Option<PropertyMap>,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct CreateMissingEdgeOptions {
        pub edge_type: String,
        pub from_vertex: ElementId,
        pub to_vertex: ElementId,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct UpdateEdgeOptions {
        pub id: ElementId,
        pub properties: PropertyMap,
        pub partial: Option<bool>,
        pub create_missing_with: Option<CreateMissingEdgeOptions>,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct FindVerticesOptions {
        pub vertex_type: Option<String>,
        pub filters: Option<Vec<FilterCondition>>,
        pub sort: Option<Vec<SortSpec>>,
        pub limit: Option<u32>,
        pub offset: Option<u32>,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct FindEdgesOptions {
        pub edge_types: Option<Vec<String>>,
        pub filters: Option<Vec<FilterCondition>>,
        pub sort: Option<Vec<SortSpec>>,
        pub limit: Option<u32>,
        pub offset: Option<u32>,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct GetAdjacentVerticesOptions {
        pub vertex_id: ElementId,
        pub direction: Direction,
        pub edge_types: Option<Vec<String>>,
        pub limit: Option<u32>,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct GetConnectedEdgesOptions {
        pub vertex_id: ElementId,
        pub direction: Direction,
        pub edge_types: Option<Vec<String>>,
        pub limit: Option<u32>,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct QueryExecutionResult {
        pub query_result_value: QueryResult,
        pub execution_time_ms: Option<u32>,
        pub rows_affected: Option<u32>,
        pub explanation: Option<String>,
        pub profile_data: Option<String>,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct PathOptions {
        pub max_depth: Option<u32>,
        pub edge_types: Option<Vec<String>>,
        pub vertex_types: Option<Vec<String>>,
        pub vertex_filters: Option<Vec<FilterCondition>>,
        pub edge_filters: Option<Vec<FilterCondition>>,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct GetNeighborhoodOptions {
        pub center: ElementId,
        pub depth: u32,
        pub direction: Direction,
        pub edge_types: Option<Vec<String>>,
        pub max_vertices: Option<u32>,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct Subgraph {
        pub vertices: Vec<Vertex>,
        pub edges: Vec<Edge>,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct FindShortestPathOptions {
        pub from_vertex: ElementId,
        pub to_vertex: ElementId,
        pub path: Option<PathOptions>,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct FindAllPathsOptions {
        pub from_vertex: ElementId,
        pub to_vertex: ElementId,
        pub path: Option<PathOptions>,
        pub limit: Option<u32>,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct PathExistsOptions {
        pub from_vertex: ElementId,
        pub to_vertex: ElementId,
        pub path: Option<PathOptions>,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct GetVerticesAtDistanceOptions {
        pub source: ElementId,
        pub distance: u32,
        pub direction: Direction,
        pub edge_types: Option<Vec<String>>,
    }
}

pub mod errors {
    pub type ElementId = super::types::ElementId;

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub enum GraphError {
        UnsupportedOperation(String),
        ConnectionFailed(String),
        AuthenticationFailed(String),
        AuthorizationFailed(String),
        ElementNotFound(ElementId),
        DuplicateElement(ElementId),
        SchemaViolation(String),
        ConstraintViolation(String),
        InvalidPropertyType(String),
        InvalidQuery(String),
        TransactionFailed(String),
        TransactionConflict,
        TransactionTimeout,
        DeadlockDetected,
        Timeout,
        ResourceExhausted(String),
        InternalError(String),
        ServiceUnavailable(String),
    }

    impl core::fmt::Display for GraphError {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            write!(f, "{:?}", self)
        }
    }

    impl std::error::Error for GraphError {}
}

pub mod transactions {
    use crate::TransactionInterface;

    pub type Vertex = super::types::Vertex;
    pub type Edge = super::types::Edge;
    pub type Path = super::types::Path;
    pub type ElementId = super::types::ElementId;
    pub type PropertyMap = super::types::PropertyMap;
    pub type PropertyValue = super::types::PropertyValue;
    pub type FilterCondition = super::types::FilterCondition;
    pub type SortSpec = super::types::SortSpec;
    pub type Direction = super::types::Direction;
    pub type Subgraph = super::types::Subgraph;
    pub type ExecuteQueryOptions = super::types::ExecuteQueryOptions;
    pub type QueryExecutionResult = super::types::QueryExecutionResult;
    pub type FindShortestPathOptions = super::types::FindShortestPathOptions;
    pub type FindAllPathsOptions = super::types::FindAllPathsOptions;
    pub type FindEdgesOptions = super::types::FindEdgesOptions;
    pub type GetAdjacentVerticesOptions = super::types::GetAdjacentVerticesOptions;
    pub type GetConnectedEdgesOptions = super::types::GetConnectedEdgesOptions;
    pub type GetVerticesAtDistanceOptions = super::types::GetVerticesAtDistanceOptions;
    pub type FindVerticesOptions = super::types::FindVerticesOptions;
    pub type GetNeighborhoodOptions = super::types::GetNeighborhoodOptions;
    pub type PathExistsOptions = super::types::PathExistsOptions;
    pub type CreateVertexOptions = super::types::CreateVertexOptions;
    pub type UpdateVertexOptions = super::types::UpdateVertexOptions;
    pub type CreateEdgeOptions = super::types::CreateEdgeOptions;
    pub type UpdateEdgeOptions = super::types::UpdateEdgeOptions;
    pub type GraphError = super::errors::GraphError;

    pub struct Transaction {
        inner: Box<dyn TransactionInterface>,
    }

    impl Transaction {
        pub fn new<T: TransactionInterface>(val: T) -> Self {
            Self {
                inner: Box::new(val),
            }
        }

        pub fn get<T: TransactionInterface>(&self) -> &T {
            self.inner
                .as_any()
                .downcast_ref::<T>()
                .expect("Transaction type mismatch")
        }

        pub fn get_mut<T: TransactionInterface>(&mut self) -> &mut T {
            self.inner
                .as_any_mut()
                .downcast_mut::<T>()
                .expect("Transaction type mismatch")
        }
    }

    impl std::ops::Deref for Transaction {
        type Target = dyn TransactionInterface;
        fn deref(&self) -> &Self::Target {
            &*self.inner
        }
    }

    impl std::ops::DerefMut for Transaction {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut *self.inner
        }
    }

    impl std::fmt::Debug for Transaction {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("Transaction").finish()
        }
    }
}

pub mod connection {
    use crate::GraphInterface;

    pub type GraphError = super::errors::GraphError;
    pub type Transaction = super::transactions::Transaction;

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct ConnectionConfig {
        pub hosts: Option<Vec<String>>,
        pub port: Option<u16>,
        pub database_name: Option<String>,
        pub username: Option<String>,
        pub password: Option<String>,
        pub timeout_seconds: Option<u32>,
        pub max_connections: Option<u32>,
        pub provider_config: Vec<(String, String)>,
    }

    #[derive(
        Clone, Copy, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue,
    )]
    pub struct GraphStatistics {
        pub vertex_count: Option<u64>,
        pub edge_count: Option<u64>,
        pub label_count: Option<u32>,
        pub property_count: Option<u64>,
    }

    pub struct Graph {
        inner: Box<dyn GraphInterface>,
    }

    impl Graph {
        pub fn new<T: GraphInterface>(val: T) -> Self {
            Self {
                inner: Box::new(val),
            }
        }

        pub fn get<T: GraphInterface>(&self) -> &T {
            self.inner
                .as_any()
                .downcast_ref::<T>()
                .expect("Graph type mismatch")
        }

        pub fn get_mut<T: GraphInterface>(&mut self) -> &mut T {
            self.inner
                .as_any_mut()
                .downcast_mut::<T>()
                .expect("Graph type mismatch")
        }
    }

    impl std::ops::Deref for Graph {
        type Target = dyn GraphInterface;
        fn deref(&self) -> &Self::Target {
            &*self.inner
        }
    }

    impl std::ops::DerefMut for Graph {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut *self.inner
        }
    }

    impl std::fmt::Debug for Graph {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("Graph").finish()
        }
    }
}

pub mod schema {
    use crate::SchemaManagerInterface;

    pub type PropertyValue = super::types::PropertyValue;
    pub type GraphError = super::errors::GraphError;
    pub type ConnectionConfig = super::connection::ConnectionConfig;

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
    pub enum PropertyType {
        Boolean,
        Int32,
        Int64,
        Float32Type,
        Float64Type,
        StringType,
        Bytes,
        Date,
        Datetime,
        Point,
        ListType,
        MapType,
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
    pub enum IndexType {
        Exact,
        Range,
        Text,
        Geospatial,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct PropertyDefinition {
        pub name: String,
        pub property_type: PropertyType,
        pub required: bool,
        pub unique: bool,
        pub default_value: Option<PropertyValue>,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct VertexLabelSchema {
        pub label: String,
        pub properties: Vec<PropertyDefinition>,
        pub container: Option<String>,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct EdgeLabelSchema {
        pub label: String,
        pub properties: Vec<PropertyDefinition>,
        pub from_labels: Option<Vec<String>>,
        pub to_labels: Option<Vec<String>>,
        pub container: Option<String>,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct IndexDefinition {
        pub name: String,
        pub label: String,
        pub properties: Vec<String>,
        pub index_type: IndexType,
        pub unique: bool,
        pub container: Option<String>,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct EdgeTypeDefinition {
        pub collection: String,
        pub from_collections: Vec<String>,
        pub to_collections: Vec<String>,
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
    pub enum ContainerType {
        VertexContainer,
        EdgeContainer,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct ContainerInfo {
        pub name: String,
        pub container_type: ContainerType,
        pub element_count: Option<u64>,
    }

    pub struct SchemaManager {
        inner: Box<dyn SchemaManagerInterface>,
    }

    impl SchemaManager {
        pub fn new<T: SchemaManagerInterface>(val: T) -> Self {
            Self {
                inner: Box::new(val),
            }
        }

        pub fn get<T: SchemaManagerInterface>(&self) -> &T {
            self.inner
                .as_any()
                .downcast_ref::<T>()
                .expect("SchemaManager type mismatch")
        }

        pub fn get_mut<T: SchemaManagerInterface>(&mut self) -> &mut T {
            self.inner
                .as_any_mut()
                .downcast_mut::<T>()
                .expect("SchemaManager type mismatch")
        }
    }

    impl std::ops::Deref for SchemaManager {
        type Target = dyn SchemaManagerInterface;
        fn deref(&self) -> &Self::Target {
            &*self.inner
        }
    }

    impl std::ops::DerefMut for SchemaManager {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut *self.inner
        }
    }

    impl std::fmt::Debug for SchemaManager {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("SchemaManager").finish()
        }
    }
}
