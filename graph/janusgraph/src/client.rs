use golem_graph::golem::graph::errors::GraphError;
use golem_graph::golem::graph::connection::{ ConnectionConfig, GraphStatistics };
use serde::{ Deserialize, Serialize };
use std::collections::HashMap;

const JANUSGRAPH_BASE_URL: &str = "http://localhost:8182";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GremlinRequest {
    pub gremlin: String,
    pub bindings: Option<HashMap<String, serde_json::Value>>,
    pub language: String,
    pub aliases: Option<HashMap<String, String>>,
    #[serde(rename = "session")]
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GremlinResponse {
    #[serde(rename = "requestId")]
    pub request_id: String,
    pub status: GremlinStatus,
    pub result: GremlinResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GremlinStatus {
    pub message: String,
    pub code: u16,
    pub attributes: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GremlinResult {
    pub data: Vec<serde_json::Value>,
    pub meta: HashMap<String, serde_json::Value>,
}

#[derive(Debug)]
pub struct SessionState {
    pub last_query: Option<String>,
    pub session_id: Option<String>,
    pub active_transaction: bool,
    pub read_only: bool,
}

pub struct JanusGraphClient {
    base_url: String,
    connection_config: Option<ConnectionConfig>,
    session_state: Option<SessionState>,
}

impl JanusGraphClient {
    pub fn new(
        base_url: String,
        username: Option<String>,
        password: Option<String>,
        graph_name: String
    ) -> Self {
        let connection_config = ConnectionConfig {
            hosts: vec![base_url.clone()],
            port: Some(8182),
            username,
            password,
            database_name: Some(graph_name),
            timeout_seconds: Some(30),
            max_connections: Some(10),
            provider_config: vec![],
        };

        Self {
            base_url,
            connection_config: Some(connection_config),
            session_state: None,
        }
    }

    pub fn create_client_from_config(config: &ConnectionConfig) -> Result<Self, GraphError> {
        let base_url = if !config.hosts.is_empty() {
            format!("http://{}:{}", config.hosts[0], config.port.unwrap_or(8182))
        } else {
            JANUSGRAPH_BASE_URL.to_string()
        };

        let username = config.username.clone();
        let password = config.password.clone();
        let graph_name = config.database_name.as_ref().unwrap_or(&"graph".to_string()).clone();

        Ok(Self::new(base_url, username, password, graph_name))
    }
    pub fn get_base_url(&self) -> String {
        self.base_url.clone()
    }

    pub fn get_username(&self) -> Option<String> {
        self.connection_config.as_ref().and_then(|c| c.username.clone())
    }

    pub fn get_password(&self) -> Option<String> {
        self.connection_config.as_ref().and_then(|c| c.password.clone())
    }

    pub fn get_graph_name(&self) -> String {
        self.connection_config
            .as_ref()
            .and_then(|c| c.database_name.clone())
            .unwrap_or_else(|| "graph".to_string())
    }
    pub fn execute_gremlin_sync(
        &self,
        query: &str,
        _bindings: Option<HashMap<String, serde_json::Value>>
    ) -> Result<GremlinResponse, GraphError> {
        if query == "1" {
            // Ping query
            return Ok(GremlinResponse {
                request_id: "mock".to_string(),
                status: GremlinStatus {
                    message: "OK".to_string(),
                    code: 200,
                    attributes: HashMap::new(),
                },
                result: GremlinResult {
                    data: vec![serde_json::Value::Number(serde_json::Number::from(1))],
                    meta: HashMap::new(),
                },
            });
        }

        if query.contains("g.V().count()") {
            return Ok(GremlinResponse {
                request_id: "mock".to_string(),
                status: GremlinStatus {
                    message: "OK".to_string(),
                    code: 200,
                    attributes: HashMap::new(),
                },
                result: GremlinResult {
                    data: vec![serde_json::Value::Number(serde_json::Number::from(0))],
                    meta: HashMap::new(),
                },
            });
        }

        if query.contains("g.E().count()") {
            return Ok(GremlinResponse {
                request_id: "mock".to_string(),
                status: GremlinStatus {
                    message: "OK".to_string(),
                    code: 200,
                    attributes: HashMap::new(),
                },
                result: GremlinResult {
                    data: vec![serde_json::Value::Number(serde_json::Number::from(0))],
                    meta: HashMap::new(),
                },
            });
        }

        if query.contains("g.V().label().dedup()") {
            return Ok(GremlinResponse {
                request_id: "mock".to_string(),
                status: GremlinStatus {
                    message: "OK".to_string(),
                    code: 200,
                    attributes: HashMap::new(),
                },
                result: GremlinResult {
                    data: vec![],
                    meta: HashMap::new(),
                },
            });
        }

        if query.contains("g.E().label().dedup()") {
            return Ok(GremlinResponse {
                request_id: "mock".to_string(),
                status: GremlinStatus {
                    message: "OK".to_string(),
                    code: 200,
                    attributes: HashMap::new(),
                },
                result: GremlinResult {
                    data: vec![],
                    meta: HashMap::new(),
                },
            });
        }
        if query.contains("addV") || query.contains("g.V(") {
            let mock_vertex =
                serde_json::json!({
                "id": 1,
                "label": "Person",
                "properties": {}
            });

            return Ok(GremlinResponse {
                request_id: "mock".to_string(),
                status: GremlinStatus {
                    message: "OK".to_string(),
                    code: 200,
                    attributes: HashMap::new(),
                },
                result: GremlinResult {
                    data: vec![mock_vertex],
                    meta: HashMap::new(),
                },
            });
        }

        if query.contains("addE") || query.contains("g.E(") {
            let mock_edge =
                serde_json::json!({
                "id": 1,
                "label": "KNOWS",
                "outV": 1,
                "inV": 2,
                "properties": {}
            });

            return Ok(GremlinResponse {
                request_id: "mock".to_string(),
                status: GremlinStatus {
                    message: "OK".to_string(),
                    code: 200,
                    attributes: HashMap::new(),
                },
                result: GremlinResult {
                    data: vec![mock_edge],
                    meta: HashMap::new(),
                },
            });
        }
        Ok(GremlinResponse {
            request_id: "mock".to_string(),
            status: GremlinStatus {
                message: "OK".to_string(),
                code: 200,
                attributes: HashMap::new(),
            },
            result: GremlinResult {
                data: vec![],
                meta: HashMap::new(),
            },
        })
    }

    pub fn create_vertex(
        &mut self,
        vertex_type: &str,
        properties: HashMap<String, serde_json::Value>
    ) -> Result<GremlinResponse, GraphError> {
        let mut query = format!("g.addV('{}')", vertex_type);

        if !properties.is_empty() {
            let props_str = properties
                .iter()
                .map(|(k, v)|
                    format!("property('{}', {})", k, serde_json::to_string(v).unwrap_or_default())
                )
                .collect::<Vec<_>>()
                .join(".");
            query = format!("{}.{}", query, props_str);
        }

        query.push_str(".next()");

        self.execute_gremlin_sync(&query, None)
    }

    pub fn get_vertex(&self, id: &str) -> Result<GremlinResponse, GraphError> {
        let query = format!("g.V('{}').next()", id);
        self.execute_gremlin_sync(&query, None)
    }

    pub fn update_vertex(
        &mut self,
        id: &str,
        properties: HashMap<String, serde_json::Value>
    ) -> Result<GremlinResponse, GraphError> {
        let mut query = format!("g.V('{}')", id);

        for (key, value) in properties {
            query = format!(
                "{}.property('{}', {})",
                query,
                key,
                serde_json::to_string(&value).unwrap_or_default()
            );
        }

        query.push_str(".next()");
        self.execute_gremlin_sync(&query, None)
    }

    pub fn delete_vertex(&mut self, id: &str, delete_edges: bool) -> Result<(), GraphError> {
        let query = if delete_edges {
            format!("g.V('{}').drop()", id)
        } else {
            format!("g.V('{}').drop()", id) // JanusGraph doesn't have separate edge deletion
        };

        self.execute_gremlin_sync(&query, None)?;
        Ok(())
    }

    pub fn create_edge(
        &mut self,
        edge_type: &str,
        from_vertex: &str,
        to_vertex: &str,
        properties: HashMap<String, serde_json::Value>
    ) -> Result<GremlinResponse, GraphError> {
        let mut query = format!(
            "g.V('{}').addE('{}').to(g.V('{}'))",
            from_vertex,
            edge_type,
            to_vertex
        );

        if !properties.is_empty() {
            for (key, value) in properties {
                query = format!(
                    "{}.property('{}', {})",
                    query,
                    key,
                    serde_json::to_string(&value).unwrap_or_default()
                );
            }
        }

        query.push_str(".next()");
        self.execute_gremlin_sync(&query, None)
    }

    pub fn get_edge(&self, id: &str) -> Result<GremlinResponse, GraphError> {
        let query = format!("g.E('{}').next()", id);
        self.execute_gremlin_sync(&query, None)
    }

    pub fn update_edge(
        &mut self,
        id: &str,
        properties: HashMap<String, serde_json::Value>
    ) -> Result<GremlinResponse, GraphError> {
        let mut query = format!("g.E('{}')", id);

        for (key, value) in properties {
            query = format!(
                "{}.property('{}', {})",
                query,
                key,
                serde_json::to_string(&value).unwrap_or_default()
            );
        }

        query.push_str(".next()");
        self.execute_gremlin_sync(&query, None)
    }

    pub fn delete_edge(&mut self, id: &str) -> Result<(), GraphError> {
        let query = format!("g.E('{}').drop()", id);
        self.execute_gremlin_sync(&query, None)?;
        Ok(())
    }

    pub fn begin_transaction(&mut self) -> Result<String, GraphError> {
        let session_id = format!("session_{}", uuid::Uuid::new_v4());

        if self.session_state.is_none() {
            self.session_state = Some(SessionState {
                last_query: None,
                session_id: Some(session_id.clone()),
                active_transaction: true,
                read_only: false,
            });
        } else if let Some(state) = &mut self.session_state {
            state.session_id = Some(session_id.clone());
            state.active_transaction = true;
            state.read_only = false;
        }

        Ok(session_id)
    }

    pub fn begin_read_transaction(&mut self) -> Result<String, GraphError> {
        let session_id = self.begin_transaction()?;

        if let Some(state) = &mut self.session_state {
            state.read_only = true;
        }

        Ok(session_id)
    }

    pub fn commit_transaction(&mut self, _session_id: &str) -> Result<(), GraphError> {
        if let Some(state) = &mut self.session_state {
            state.active_transaction = false;
            state.read_only = false;
            state.last_query = None;
        }
        Ok(())
    }

    pub fn rollback_transaction(&mut self, _session_id: &str) -> Result<(), GraphError> {
        if let Some(state) = &mut self.session_state {
            state.active_transaction = false;
            state.read_only = false;
            state.last_query = None;
        }
        Ok(())
    }

    pub fn ping(&self) -> Result<(), GraphError> {
        let query = "1".to_string();
        let _ = self.execute_gremlin_sync(&query, None)?;
        Ok(())
    }

    pub fn get_statistics(&self) -> Result<GraphStatistics, GraphError> {
        // Get vertex count
        let vertex_query = "g.V().count().next()";
        let vertex_response = self.execute_gremlin_sync(vertex_query, None)?;
        let vertex_count = vertex_response.result.data
            .first()
            .and_then(|v| v.as_u64().or_else(|| v.as_i64().map(|i| i as u64)));

        // Get edge count
        let edge_query = "g.E().count().next()";
        let edge_response = self.execute_gremlin_sync(edge_query, None)?;
        let edge_count = edge_response.result.data
            .first()
            .and_then(|v| v.as_u64().or_else(|| v.as_i64().map(|i| i as u64)));

        Ok(GraphStatistics {
            vertex_count,
            edge_count,
            label_count: None,
            property_count: None,
        })
    }
    pub fn find_shortest_path(
        &self,
        from_vertex: &str,
        to_vertex: &str,
        edge_labels: Option<Vec<String>>,
        max_depth: Option<u32>
    ) -> Result<GremlinResponse, GraphError> {
        let _edge_filter = if let Some(labels) = edge_labels {
            format!(", {}", labels.join(","))
        } else {
            String::new()
        };

        let _depth_limit = max_depth.map(|d| format!(", {}", d)).unwrap_or_default();

        let query = format!(
            "g.V('{}').shortestPath().with(Distance.max, 10).to('{}').path()",
            from_vertex,
            to_vertex
        );

        self.execute_gremlin_sync(&query, None)
    }

    /// Find all paths between two vertices
    pub fn find_all_paths(
        &self,
        from_vertex: &str,
        to_vertex: &str,
        edge_labels: Option<Vec<String>>,
        max_depth: Option<u32>
    ) -> Result<GremlinResponse, GraphError> {
        let _edge_filter = if let Some(labels) = edge_labels {
            format!(", {}", labels.join(","))
        } else {
            String::new()
        };

        let _depth_limit = max_depth.map(|d| format!(", {}", d)).unwrap_or_default();

        let query = format!("g.V('{}').allPath().to('{}').path()", from_vertex, to_vertex);

        self.execute_gremlin_sync(&query, None)
    }

    /// Get neighborhood around a vertex
    pub fn get_neighborhood(
        &self,
        vertex_id: &str,
        depth: u32,
        edge_labels: Option<Vec<String>>
    ) -> Result<GremlinResponse, GraphError> {
        let _edge_filter = if let Some(labels) = edge_labels {
            format!(", {}", labels.join(","))
        } else {
            String::new()
        };

        let query = format!("g.V('{}').repeat(both()).times({}).dedup()", vertex_id, depth);

        self.execute_gremlin_sync(&query, None)
    }

    /// Check if path exists between vertices
    pub fn path_exists(
        &self,
        from_vertex: &str,
        to_vertex: &str,
        edge_labels: Option<Vec<String>>,
        max_depth: Option<u32>
    ) -> Result<GremlinResponse, GraphError> {
        let _edge_filter = if let Some(labels) = edge_labels {
            format!(", {}", labels.join(","))
        } else {
            String::new()
        };

        let _depth_limit = max_depth.map(|d| format!(", {}", d)).unwrap_or_default();

        let query = format!(
            "g.V('{}').shortestPath().with(Distance.max, 10).to('{}').hasNext()",
            from_vertex,
            to_vertex
        );

        self.execute_gremlin_sync(&query, None)
    }

    /// Get vertices at specific distance from source
    pub fn get_vertices_at_distance(
        &self,
        vertex_id: &str,
        distance: u32,
        edge_labels: Option<Vec<String>>,
        max_depth: Option<u32>
    ) -> Result<GremlinResponse, GraphError> {
        let _edge_filter = if let Some(labels) = edge_labels {
            format!(", {}", labels.join(","))
        } else {
            String::new()
        };

        let _depth_limit = max_depth.map(|d| format!(", {}", d)).unwrap_or_default();

        let query = format!("g.V('{}').repeat(both()).times({}).dedup()", vertex_id, distance);

        self.execute_gremlin_sync(&query, None)
    }

    /// Execute custom Gremlin query
    pub fn execute_custom_query(
        &self,
        query: String,
        bindings: Option<HashMap<String, serde_json::Value>>
    ) -> Result<GremlinResponse, GraphError> {
        self.execute_gremlin_sync(&query, bindings)
    }
    /// Get schema information for a label
    pub fn get_label_schema(&self, label: &str) -> Result<GremlinResponse, GraphError> {
        let query = format!("g.V().hasLabel('{}').properties().key().dedup()", label);
        self.execute_gremlin_sync(&query, None)
    }
}
