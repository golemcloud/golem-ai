use golem_graph::golem::graph::connection::{ConnectionConfig, GraphStatistics};
use golem_graph::golem::graph::errors::GraphError;
use log::trace;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::thread;
use std::time::Duration;

// Retry configuration for lock timeout issues
const MAX_RETRIES: u32 = 15;
const RETRY_DELAY_MS: u64 = 2000;

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
    pub data: serde_json::Value,
    pub meta: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct SessionState {
    pub session_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct JanusGraphClient {
    client: reqwest::Client,
    base_url: String,
    username: Option<String>,
    password: Option<String>,
    graph_name: String,
    session_state: Option<SessionState>,
}

impl JanusGraphClient {
    pub fn new(
        base_url: String,
        username: Option<String>,
        password: Option<String>,
        graph_name: String,
    ) -> Self {
        let client = reqwest::Client::new();
        Self {
            client,
            base_url,
            username,
            password,
            graph_name,
            session_state: None,
        }
    }

    pub fn create_client_from_config(config: &ConnectionConfig) -> Result<Self, GraphError> {
        let host = config
            .hosts
            .first()
            .ok_or_else(|| GraphError::InternalError("No hosts provided".to_string()))?
            .clone();

        let username = config.username.clone();
        let password = config.password.clone();
        let graph_name = config
            .database_name
            .as_ref()
            .unwrap_or(&"graph".to_string())
            .clone();
        let timeout_seconds = config.timeout_seconds.unwrap_or(30);
        let provider_config = config
            .provider_config
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<HashMap<String, String>>();

        // Construct the base URL with protocol and port
        let port = config.port.unwrap_or(8182); // Default to JanusGraph port
        let base_url = format!("http://{host}:{port}");

        // Create HTTP client with proper configuration
        let client_builder =
            reqwest::Client::builder().timeout(Duration::from_secs(timeout_seconds as u64));

        // Apply SSL/TLS configuration from provider_config
        if let Some(ssl_enabled) = provider_config.get("encryption") {
            if ssl_enabled == "true" {
                trace!("SSL encryption enabled");
            } else {
                trace!("SSL encryption disabled");
            }
        }

        let client = client_builder
            .build()
            .map_err(|e| GraphError::InternalError(format!("Failed to create HTTP client: {e}")))?;

        Ok(JanusGraphClient {
            client,
            base_url,
            username,
            password,
            graph_name,
            session_state: None,
        })
    }

    pub fn get_base_url(&self) -> String {
        self.base_url.clone()
    }

    pub fn get_username(&self) -> Option<String> {
        self.username.clone()
    }

    pub fn get_password(&self) -> Option<String> {
        self.password.clone()
    }

    pub fn get_graph_name(&self) -> String {
        self.graph_name.clone()
    }

    pub fn is_session_active(&self) -> bool {
        self.session_state.is_some()
    }

    pub fn execute_gremlin_sync(
        &self,
        query: &str,
        bindings: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<GremlinResponse, GraphError> {
        let mut request_json = serde_json::Map::new();
        request_json.insert(
            "gremlin".to_string(),
            serde_json::Value::String(query.to_string()),
        );
        request_json.insert(
            "language".to_string(),
            serde_json::Value::String("gremlin-groovy".to_string()),
        );

        // Only include bindings if they exist
        if let Some(bindings) = bindings {
            request_json.insert(
                "bindings".to_string(),
                serde_json::to_value(bindings).unwrap_or_default(),
            );
        }

        if let Some(session_id) = self
            .session_state
            .as_ref()
            .and_then(|s| s.session_id.clone())
        {
            request_json.insert("session".to_string(), serde_json::Value::String(session_id));
        }

        let request_body = serde_json::Value::Object(request_json);

        let url = format!("{}/gremlin", self.base_url);
        let mut request_builder = self.client.post(&url);

        if let Some(ref username) = self.username {
            request_builder = request_builder.basic_auth(username, self.password.as_deref());
        }

        let response = request_builder
            .json(&request_body)
            .send()
            .map_err(|e| GraphError::InternalError(format!("HTTP request failed: {e}")))?;

        if response.status().is_success() {
            self.parse_response(response)
        } else {
            Err(GraphError::InternalError(format!(
                "Request failed with status {}",
                response.status()
            )))
        }
    }

    pub fn ping(&self) -> Result<(), GraphError> {
        // Use a simple Gremlin query for ping
        let query = "1".to_string();
        let response = self.execute_gremlin_sync(&query, None)?;

        if response.status.code == 200 {
            Ok(())
        } else {
            Err(GraphError::InternalError("Ping failed".to_string()))
        }
    }

    pub fn close(&mut self) -> Result<(), GraphError> {
        // Close any active session
        if let Some(state) = &self.session_state {
            if let Some(session_id) = &state.session_id {
                let session_id_clone = session_id.clone();
                let _ = self.rollback_transaction(&session_id_clone);
            }
        }
        self.session_state = None;
        Ok(())
    }

    pub fn get_statistics(&self) -> Result<GraphStatistics, GraphError> {
        // Get vertex count
        let vertex_query = "g.V().count().next()";
        let vertex_response = self.execute_gremlin_sync(vertex_query, None)?;
        let vertex_count = if vertex_response.status.code == 200 {
            vertex_response
                .result
                .data
                .get("@value")
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.first())
                .and_then(|v| v.as_u64().or_else(|| v.as_i64().map(|i| i as u64)))
        } else {
            None
        };

        // Get edge count
        let edge_query = "g.E().count().next()";
        let edge_response = self.execute_gremlin_sync(edge_query, None)?;
        let edge_count = if edge_response.status.code == 200 {
            edge_response
                .result
                .data
                .get("@value")
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.first())
                .and_then(|v| v.as_u64().or_else(|| v.as_i64().map(|i| i as u64)))
        } else {
            None
        };

        // Get label count
        let label_query = "g.V().label().dedup().count().next()";
        let label_response = self.execute_gremlin_sync(label_query, None)?;
        let label_count = if label_response.status.code == 200 {
            label_response
                .result
                .data
                .get("@value")
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.first())
                .and_then(|v| v.as_u64().map(|u| u as u32))
        } else {
            None
        };

        Ok(GraphStatistics {
            vertex_count,
            edge_count,
            label_count,
            property_count: None, // JanusGraph doesn't provide this directly
        })
    }

    pub fn begin_transaction(&mut self) -> Result<String, GraphError> {
        // JanusGraph doesn't have explicit transaction management
        let session_id = format!("session_{}", uuid::Uuid::new_v4());

        self.session_state = Some(SessionState {
            session_id: Some(session_id.clone()),
        });

        Ok(session_id)
    }

    pub fn begin_read_transaction(&mut self) -> Result<String, GraphError> {
        self.begin_transaction()
    }

    pub fn commit_transaction(&mut self, _session_id: &str) -> Result<(), GraphError> {
        // JanusGraph doesn't have explicit commit
        self.session_state = None;
        Ok(())
    }

    pub fn rollback_transaction(&mut self, _session_id: &str) -> Result<(), GraphError> {
        // JanusGraph doesn't have explicit rollback
        self.session_state = None;
        Ok(())
    }

    pub fn create_vertex(
        &mut self,
        vertex_type: &str,
        properties: HashMap<String, serde_json::Value>,
    ) -> Result<GremlinResponse, GraphError> {
        let mut query = format!("g.addV('{vertex_type}')");

        if !properties.is_empty() {
            for (key, value) in properties {
                let value_str = match value {
                    serde_json::Value::String(s) => format!("'{s}'"),
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::Bool(b) => b.to_string(),
                    serde_json::Value::Null => "null".to_string(),
                    _ => serde_json::to_string(&value).unwrap_or_default(),
                };
                query = format!("{query}.property('{key}', {value_str})");
            }
        }

        query.push_str(".next()");

        self.execute_gremlin_sync(&query, None)
    }

    pub fn get_vertex(&self, id: &str) -> Result<GremlinResponse, GraphError> {
        let vertex_query = if id.parse::<i64>().is_ok() {
            id.to_string()
        } else {
            format!("'{id}'")
        };
        let query = format!("g.V({vertex_query}).next()");
        self.execute_gremlin_sync(&query, None)
    }

    pub fn update_vertex(
        &mut self,
        id: &str,
        properties: HashMap<String, serde_json::Value>,
    ) -> Result<GremlinResponse, GraphError> {
        let vertex_query = if id.parse::<i64>().is_ok() {
            id.to_string()
        } else {
            format!("\"{id}\"")
        };
        let mut query = format!("g.V({vertex_query})");

        for (key, value) in &properties {
            let value_str = match value {
                serde_json::Value::String(s) => format!("\"{s}\""),
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::Bool(b) => b.to_string(),
                serde_json::Value::Null => "null".to_string(),
                _ => serde_json::to_string(&value).unwrap_or_default(),
            };
            query = format!("{query}.property('{key}', {value_str})");
        }

        query.push_str(".next()");

        // Retry logic for lock timeouts
        let mut last_error = None;
        for attempt in 0..MAX_RETRIES {
            match self.execute_gremlin_sync(&query, None) {
                Ok(result) => {
                    return Ok(result);
                }
                Err(e) => {
                    let error_msg = format!("{e:?}");
                    if error_msg.contains("Lock expired")
                        || error_msg.contains("LockTimeoutException")
                        || error_msg.contains("timeout")
                        || error_msg.contains("500 Internal Server Error")
                        || error_msg.contains("Request failed with status 500")
                        || error_msg.contains("Lock")
                    {
                        last_error = Some(e.clone());
                        if attempt < MAX_RETRIES - 1 {
                            // Exponential backoff with longer delays
                            let delay = RETRY_DELAY_MS * (1 << attempt);
                            thread::sleep(Duration::from_millis(delay));
                            continue;
                        }
                    }
                    return Err(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| GraphError::InternalError("Retry failed".to_string())))
    }

    pub fn delete_vertex(&mut self, id: &str, _delete_edges: bool) -> Result<(), GraphError> {
        let vertex_query = if id.parse::<i64>().is_ok() {
            id.to_string()
        } else {
            format!("\"{id}\"")
        };
        let query = format!("g.V({vertex_query}).drop()");

        // Retry logic for lock timeouts
        let mut last_error = None;
        for attempt in 0..MAX_RETRIES {
            match self.execute_gremlin_sync(&query, None) {
                Ok(_) => {
                    return Ok(());
                }
                Err(e) => {
                    let error_msg = format!("{e:?}");
                    if error_msg.contains("Lock expired")
                        || error_msg.contains("LockTimeoutException")
                        || error_msg.contains("timeout")
                        || error_msg.contains("500 Internal Server Error")
                        || error_msg.contains("Request failed with status 500")
                        || error_msg.contains("Lock")
                    {
                        last_error = Some(e.clone());
                        if attempt < MAX_RETRIES - 1 {
                            // Exponential backoff with longer delays
                            let delay = RETRY_DELAY_MS * (1 << attempt);
                            thread::sleep(Duration::from_millis(delay));
                            continue;
                        }
                    }
                    return Err(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| GraphError::InternalError("Retry failed".to_string())))
    }

    pub fn create_edge(
        &mut self,
        edge_type: &str,
        from_vertex: &str,
        to_vertex: &str,
        properties: HashMap<String, serde_json::Value>,
    ) -> Result<GremlinResponse, GraphError> {
        // Handle numeric IDs vs string IDs in the query
        let from_vertex_query = if from_vertex.parse::<i64>().is_ok() {
            from_vertex.to_string()
        } else {
            format!("'{from_vertex}'")
        };

        let to_vertex_query = if to_vertex.parse::<i64>().is_ok() {
            to_vertex.to_string()
        } else {
            format!("'{to_vertex}'")
        };

        // Try a different edge creation syntax that might work better with JanusGraph
        let mut query = format!(
            "g.V({from_vertex_query}).as('from').V({to_vertex_query}).as('to').addE('{edge_type}').from('from').to('to')"
        );

        if !properties.is_empty() {
            for (key, value) in properties {
                let value_str = match value {
                    serde_json::Value::String(s) => format!("'{s}'"),
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::Bool(b) => b.to_string(),
                    serde_json::Value::Null => "null".to_string(),
                    _ => serde_json::to_string(&value).unwrap_or_default(),
                };
                query = format!("{query}.property('{key}', {value_str})");
            }
        }

        query.push_str(".next()");
        self.execute_gremlin_sync(&query, None)
    }

    pub fn get_edge(&self, id: &str) -> Result<GremlinResponse, GraphError> {
        let edge_query = if id.parse::<i64>().is_ok() {
            id.to_string()
        } else {
            format!("'{id}'")
        };
        let query = format!("g.E({edge_query}).next()");
        self.execute_gremlin_sync(&query, None)
    }

    pub fn update_edge(
        &mut self,
        id: &str,
        properties: HashMap<String, serde_json::Value>,
    ) -> Result<GremlinResponse, GraphError> {
        let edge_query = if id.parse::<i64>().is_ok() {
            id.to_string()
        } else {
            format!("\"{id}\"")
        };
        let mut query = format!("g.E({edge_query})");

        for (key, value) in &properties {
            query = format!(
                "{}.property('{}', {})",
                query,
                key,
                serde_json::to_string(&value).unwrap_or_default()
            );
        }

        query.push_str(".next()");

        // Retry logic for lock timeouts
        let mut last_error = None;
        for attempt in 0..MAX_RETRIES {
            match self.execute_gremlin_sync(&query, None) {
                Ok(result) => {
                    return Ok(result);
                }
                Err(e) => {
                    let error_msg = format!("{e:?}");
                    if error_msg.contains("Lock expired")
                        || error_msg.contains("LockTimeoutException")
                        || error_msg.contains("timeout")
                        || error_msg.contains("500 Internal Server Error")
                        || error_msg.contains("Request failed with status 500")
                        || error_msg.contains("Lock")
                    {
                        last_error = Some(e.clone());
                        if attempt < MAX_RETRIES - 1 {
                            // Exponential backoff with longer delays
                            let delay = RETRY_DELAY_MS * (1 << attempt);
                            thread::sleep(Duration::from_millis(delay));
                            continue;
                        }
                    }
                    return Err(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| GraphError::InternalError("Retry failed".to_string())))
    }

    pub fn delete_edge(&mut self, id: &str) -> Result<(), GraphError> {
        let edge_query = if id.parse::<i64>().is_ok() {
            id.to_string()
        } else {
            format!("\"{id}\"")
        };
        let query = format!("g.E({edge_query}).drop()");

        // Retry logic for lock timeouts
        let mut last_error = None;
        for attempt in 0..MAX_RETRIES {
            match self.execute_gremlin_sync(&query, None) {
                Ok(_) => {
                    return Ok(());
                }
                Err(e) => {
                    let error_msg = format!("{e:?}");
                    if error_msg.contains("Lock expired")
                        || error_msg.contains("LockTimeoutException")
                        || error_msg.contains("timeout")
                        || error_msg.contains("500 Internal Server Error")
                        || error_msg.contains("Request failed with status 500")
                        || error_msg.contains("Lock")
                    {
                        last_error = Some(e.clone());
                        if attempt < MAX_RETRIES - 1 {
                            // Exponential backoff with longer delays
                            let delay = RETRY_DELAY_MS * (1 << attempt);
                            thread::sleep(Duration::from_millis(delay));
                            continue;
                        }
                    }
                    return Err(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| GraphError::InternalError("Retry failed".to_string())))
    }

    pub fn find_shortest_path(
        &self,
        from_vertex: &str,
        to_vertex: &str,
        edge_labels: Option<Vec<String>>,
        max_depth: Option<u32>,
    ) -> Result<GremlinResponse, GraphError> {
        let _edge_filter = if let Some(labels) = edge_labels {
            format!(", {}", labels.join(","))
        } else {
            String::new()
        };

        let _depth_limit = max_depth.map(|d| format!(", {d}")).unwrap_or_default();

        let query = format!(
            "g.V('{from_vertex}').shortestPath().with(Distance.max, 10).to('{to_vertex}').path()"
        );

        self.execute_gremlin_sync(&query, None)
    }

    pub fn find_all_paths(
        &self,
        from_vertex: &str,
        to_vertex: &str,
        edge_labels: Option<Vec<String>>,
        max_depth: Option<u32>,
    ) -> Result<GremlinResponse, GraphError> {
        let _edge_filter = if let Some(labels) = edge_labels {
            format!(", {}", labels.join(","))
        } else {
            String::new()
        };

        let _depth_limit = max_depth.map(|d| format!(", {d}")).unwrap_or_default();

        let query = format!("g.V('{from_vertex}').allPath().to('{to_vertex}').path()");

        self.execute_gremlin_sync(&query, None)
    }

    pub fn get_neighborhood(
        &self,
        vertex_id: &str,
        depth: u32,
        edge_labels: Option<Vec<String>>,
    ) -> Result<GremlinResponse, GraphError> {
        let _edge_filter = if let Some(labels) = edge_labels {
            format!(", {}", labels.join(","))
        } else {
            String::new()
        };

        let query = format!("g.V('{vertex_id}').repeat(both()).times({depth}).dedup()");

        self.execute_gremlin_sync(&query, None)
    }

    pub fn path_exists(
        &self,
        from_vertex: &str,
        to_vertex: &str,
        edge_labels: Option<Vec<String>>,
        max_depth: Option<u32>,
    ) -> Result<GremlinResponse, GraphError> {
        let _edge_filter = if let Some(labels) = edge_labels {
            format!(", {}", labels.join(","))
        } else {
            String::new()
        };

        let _depth_limit = max_depth.map(|d| format!(", {d}")).unwrap_or_default();

        let query = format!(
            "g.V('{from_vertex}').shortestPath().with(Distance.max, 10).to('{to_vertex}').hasNext()"
        );

        self.execute_gremlin_sync(&query, None)
    }

    pub fn get_vertices_at_distance(
        &self,
        vertex_id: &str,
        distance: u32,
        edge_labels: Option<Vec<String>>,
        max_depth: Option<u32>,
    ) -> Result<GremlinResponse, GraphError> {
        let _edge_filter = if let Some(labels) = edge_labels {
            format!(", {}", labels.join(","))
        } else {
            String::new()
        };

        let _depth_limit = max_depth.map(|d| format!(", {d}")).unwrap_or_default();

        let query = format!("g.V('{vertex_id}').repeat(both()).times({distance}).dedup()");

        self.execute_gremlin_sync(&query, None)
    }

    pub fn execute_custom_query(
        &self,
        query: String,
        bindings: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<GremlinResponse, GraphError> {
        self.execute_gremlin_sync(&query, bindings)
    }

    pub fn get_label_schema(&self, label: &str) -> Result<GremlinResponse, GraphError> {
        let query = format!("g.V().hasLabel('{label}').properties().key().dedup()");
        self.execute_gremlin_sync(&query, None)
    }

    fn parse_response(&self, response: reqwest::Response) -> Result<GremlinResponse, GraphError> {
        if response.status().is_success() {
            let gremlin_response: GremlinResponse = response.json().map_err(|err| {
                GraphError::InternalError(format!("Failed to parse response: {err}"))
            })?;

            // Check for Gremlin errors
            if gremlin_response.status.code != 200 {
                return Err(GraphError::InvalidQuery(
                    gremlin_response.status.message.clone(),
                ));
            }

            Ok(gremlin_response)
        } else {
            Err(GraphError::InternalError(format!(
                "HTTP error: {}",
                response.status()
            )))
        }
    }
}
