use serde::{Deserialize, Serialize};

/// A vertex result entry returned from graph operations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VertexResult {
    /// Kind of message (should be `"vertex"`)
    pub kind: String,
    /// Unique identifier of the vertex
    pub id: String,
    /// Primary vertex type/label
    #[serde(rename = "vertex-type")]
    pub vertex_type: String,
    /// Additional labels (for multi-label systems like neo4j)
    #[serde(rename = "additional-labels")]
    pub additional_labels: Option<Vec<String>>,
    /// Properties associated with the vertex
    pub properties: std::collections::HashMap<String, serde_json::Value>,
    /// Creation timestamp (if available)
    #[serde(rename = "created-at")]
    pub created_at: Option<String>,
    /// Last modified timestamp (if available)
    #[serde(rename = "updated-at")]
    pub updated_at: Option<String>,
}

/// An edge result entry returned from graph operations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EdgeResult {
    /// Kind of message (should be `"edge"`)
    pub kind: String,
    /// Unique identifier of the edge
    pub id: String,
    /// Edge type/relationship type
    #[serde(rename = "edge-type")]
    pub edge_type: String,
    /// Source vertex identifier
    #[serde(rename = "from-vertex")]
    pub from_vertex: String,
    /// Target vertex identifier
    #[serde(rename = "to-vertex")]
    pub to_vertex: String,
    /// Properties associated with the edge
    pub properties: std::collections::HashMap<String, serde_json::Value>,
    /// Creation timestamp (if available)
    #[serde(rename = "created-at")]
    pub created_at: Option<String>,
    /// Last modified timestamp (if available)
    #[serde(rename = "updated-at")]
    pub updated_at: Option<String>,
}

/// A path result containing a sequence of vertices and edges.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PathResult {
    /// Kind of message (should be `"path"`)
    pub kind: String,
    /// Vertices in the path
    pub vertices: Vec<VertexResult>,
    /// Edges connecting the vertices
    pub edges: Vec<EdgeResult>,
    /// Length of the path
    pub length: u32,
    /// Total weight/cost of the path (if applicable)
    pub weight: Option<f64>,
}

/// Graph operation metadata, typically emitted at the end of operations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GraphMetadata {
    /// Kind of message (should be `"meta"`)
    pub kind: String,
    /// Original operation or query string
    pub operation: String,
    /// Total number of results returned
    #[serde(rename = "total-results")]
    pub total_results: Option<u64>,
    /// Time taken to perform the operation (in milliseconds)
    #[serde(rename = "execution-time-ms")]
    pub execution_time_ms: Option<f32>,
    /// Transaction ID (if applicable)
    #[serde(rename = "transaction-id")]
    pub transaction_id: Option<String>,
    /// Database provider information
    pub provider: Option<String>,
    /// Database version
    #[serde(rename = "db-version")]
    pub db_version: Option<String>,
    /// Connection information
    #[serde(rename = "connection-info")]
    pub connection_info: Option<ConnectionInfo>,
    /// Current page number (for paginated results)
    #[serde(rename = "current-page")]
    pub current_page: Option<u32>,
    /// Token for fetching the next page
    #[serde(rename = "next-page-token")]
    pub next_page_token: Option<String>,
}

/// Connection status and performance information.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConnectionInfo {
    /// Database host
    pub host: String,
    /// Connection pool size
    #[serde(rename = "pool-size")]
    pub pool_size: Option<u32>,
    /// Active connections
    #[serde(rename = "active-connections")]
    pub active_connections: Option<u32>,
    /// Connection latency in milliseconds
    #[serde(rename = "latency-ms")]
    pub latency_ms: Option<f32>,
}

/// Transaction isolation level.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum IsolationLevel {
    ReadUncommitted,
    ReadCommitted,
    RepeatableRead,
    Serializable,
}

/// Marker indicating the end of a stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEnd {
    /// Kind of message (should be `"done"`)
    pub kind: String,
}

/// A parsed item from the graph operation stream.
#[derive(Debug, Clone, PartialEq)]
pub enum GraphStreamEntry {
    /// A vertex result
    Vertex(VertexResult),
    /// An edge result
    Edge(EdgeResult),
    /// A path result
    Path(PathResult),
    /// Operation metadata
    Metadata(GraphMetadata),
    /// Stream termination signal
    Done,
    /// An unrecognized or malformed entry
    Unknown(String),
}
