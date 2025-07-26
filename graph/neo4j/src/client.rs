use golem_graph::error::from_reqwest_error;
use golem_graph::golem::graph::connection::{ConnectionConfig, GraphStatistics};
use golem_graph::golem::graph::errors::GraphError;
use log::trace;
use reqwest::Method;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Neo4jRequest {
    pub statements: Vec<Neo4jStatement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Neo4jStatement {
    pub statement: String,
    pub parameters: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Neo4jResponse {
    pub results: Vec<Neo4jResult>,
    pub errors: Vec<Neo4jError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Neo4jResult {
    pub columns: Vec<String>,
    pub data: Vec<Neo4jData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Neo4jData {
    pub row: Vec<serde_json::Value>,
    pub meta: Vec<Neo4jMeta>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Neo4jMeta {
    pub id: Option<i64>,
    #[serde(rename = "type")]
    pub meta_type: Option<String>,
    pub deleted: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Neo4jError {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct SessionState {
    pub session_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Neo4jClient {
    client: reqwest::Client,
    base_url: String,
    username: String,
    password: String,
    database_name: Option<String>,
    session_state: Option<SessionState>,
}

impl Neo4jClient {
    pub fn new(base_url: String, username: String, password: String) -> Self {
        let client = reqwest::Client::new();
        Self {
            client,
            base_url,
            username,
            password,
            database_name: None,
            session_state: None,
        }
    }

    pub fn create_client_from_config(config: &ConnectionConfig) -> Result<Self, GraphError> {
        let base_url = config
            .hosts
            .first()
            .ok_or_else(|| GraphError::InternalError("No hosts provided".to_string()))?
            .clone();
        let username = config
            .username
            .as_ref()
            .ok_or_else(|| GraphError::InternalError("Username required".to_string()))?
            .clone();

        let password = config.password.as_ref().unwrap_or(&"".to_string()).clone();
        let database_name = config.database_name.clone();
        let timeout_seconds = config.timeout_seconds.unwrap_or(30);
        let provider_config = config
            .provider_config
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<HashMap<String, String>>();

        // Create HTTP client with proper configuration
        let client_builder =
            reqwest::Client::builder().timeout(Duration::from_secs(timeout_seconds as u64));

        // Apply SSL/TLS configuration from provider_config
        if let Some(ssl_enabled) = provider_config.get("encryption") {
            if ssl_enabled == "true" {
                // Enable SSL - reqwest handles this automatically for HTTPS URLs
                trace!("SSL encryption enabled");
            } else {
                trace!("SSL encryption disabled");
            }
        }

        // Apply trust strategy if specified
        if let Some(trust_strategy) = provider_config.get("trust") {
            match trust_strategy.as_str() {
                "TRUST_ALL_CERTIFICATES" => {
                    trace!("Trust strategy: TRUST_ALL_CERTIFICATES");
                }
                "TRUST_SYSTEM_CA_SIGNED_CERTIFICATES" => {
                    trace!("Trust strategy: TRUST_SYSTEM_CA_SIGNED_CERTIFICATES");
                }
                _ => {
                    trace!("Unknown trust strategy: {trust_strategy}");
                }
            }
        }

        let client = client_builder
            .build()
            .map_err(|e| GraphError::InternalError(format!("Failed to create HTTP client: {e}")))?;

        Ok(Neo4jClient {
            client,
            base_url,
            username,
            password,
            database_name,
            session_state: None,
        })
    }

    pub fn get_base_url(&self) -> String {
        self.base_url.clone()
    }

    pub fn get_username(&self) -> String {
        self.username.clone()
    }

    pub fn get_password(&self) -> String {
        self.password.clone()
    }

    pub fn execute_cypher(
        &self,
        query: String,
        parameters: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<Neo4jResponse, GraphError> {
        trace!("Executing Cypher query: {query}");

        let statement = Neo4jStatement {
            statement: query,
            parameters,
        };

        let request = Neo4jRequest {
            statements: vec![statement],
        };

        // Build the URL with database name if specified
        let url = if let Some(db_name) = &self.database_name {
            format!("{}/db/{}/tx/commit", self.base_url, db_name)
        } else {
            format!("{}/db/data/transaction/commit", self.base_url)
        };

        let response = self
            .client
            .request(Method::POST, url)
            .header("Content-Type", "application/json")
            .basic_auth(&self.username, Some(&self.password))
            .json(&request)
            .send()
            .map_err(|err| from_reqwest_error("Request failed", err))?;

        self.parse_response(response)
    }

    pub fn ping(&self) -> Result<(), GraphError> {
        // Build the URL with database name if specified
        let url = if let Some(db_name) = &self.database_name {
            format!("{}/db/{}/", self.base_url, db_name)
        } else {
            format!("{}/db/data", self.base_url)
        };

        let response = self
            .client
            .request(Method::GET, url)
            .basic_auth(&self.username, Some(&self.password))
            .send()
            .map_err(|err| from_reqwest_error("Ping failed", err))?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(GraphError::InternalError("Ping failed".to_string()))
        }
    }

    pub fn close(&mut self) -> Result<(), GraphError> {
        // Close any active session
        if let Some(state) = &self.session_state {
            if let Some(session_id) = &state.session_id {
                let mut temp_client = Neo4jClient::new(
                    self.base_url.clone(),
                    self.username.clone(),
                    self.password.clone(),
                );
                let _ = temp_client.rollback_transaction(session_id);
            }
        }
        self.session_state = None;
        Ok(())
    }

    pub fn get_statistics(&self) -> Result<GraphStatistics, GraphError> {
        let query = "MATCH (n) RETURN count(n) as node_count";
        let response = self.execute_cypher(query.to_string(), None)?;

        let node_count = if let Some(result) = response.results.first() {
            if let Some(data) = result.data.first() {
                if let Some(value) = data.row.first() {
                    value.as_u64().unwrap_or(0)
                } else {
                    0
                }
            } else {
                0
            }
        } else {
            0
        };

        // Get edge count
        let edge_query = "MATCH ()-[r]->() RETURN count(r) as edge_count";
        let edge_response = self.execute_cypher(edge_query.to_string(), None)?;

        let edge_count = if let Some(result) = edge_response.results.first() {
            if let Some(data) = result.data.first() {
                if let Some(value) = data.row.first() {
                    value.as_u64().unwrap_or(0)
                } else {
                    0
                }
            } else {
                0
            }
        } else {
            0
        };

        // Get label count
        let label_query = "CALL db.labels() YIELD label RETURN count(label) as label_count";
        let label_response = self.execute_cypher(label_query.to_string(), None)?;

        let label_count = if let Some(result) = label_response.results.first() {
            if let Some(data) = result.data.first() {
                if let Some(value) = data.row.first() {
                    value.as_u64().unwrap_or(0) as u32
                } else {
                    0
                }
            } else {
                0
            }
        } else {
            0
        };

        Ok(GraphStatistics {
            vertex_count: Some(node_count),
            edge_count: Some(edge_count),
            label_count: Some(label_count),
            property_count: None, // Neo4j doesn't provide this directly
        })
    }

    pub fn begin_transaction(&mut self) -> Result<String, GraphError> {
        // Build the URL with database name if specified
        let url = if let Some(db_name) = &self.database_name {
            format!("{}/db/{}/tx", self.base_url, db_name)
        } else {
            format!("{}/db/data/transaction", self.base_url)
        };

        let response = self
            .client
            .request(Method::POST, url)
            .header("Content-Type", "application/json")
            .basic_auth(&self.username, Some(&self.password))
            .json(&serde_json::json!({
                "statements": []
            }))
            .send()
            .map_err(|err| from_reqwest_error("Begin transaction failed", err))?;

        let location = response
            .headers()
            .get("Location")
            .and_then(|h| h.to_str().ok())
            .ok_or_else(|| GraphError::InternalError("No transaction location".to_string()))?;

        let session_id = location
            .split('/')
            .next_back()
            .ok_or_else(|| GraphError::InternalError("Invalid transaction location".to_string()))?
            .to_string();

        self.session_state = Some(SessionState {
            session_id: Some(session_id.clone()),
        });

        Ok(session_id)
    }

    pub fn begin_read_transaction(&mut self) -> Result<String, GraphError> {
        self.begin_transaction()
    }

    pub fn commit_transaction(&mut self, session_id: &str) -> Result<(), GraphError> {
        // Build the URL with database name if specified
        let url = if let Some(db_name) = &self.database_name {
            format!("{}/db/{}/tx/{}/commit", self.base_url, db_name, session_id)
        } else {
            format!(
                "{}/db/data/transaction/{}/commit",
                self.base_url, session_id
            )
        };

        let response = self
            .client
            .request(Method::POST, url)
            .basic_auth(&self.username, Some(&self.password))
            .send()
            .map_err(|err| from_reqwest_error("Commit failed", err))?;

        if response.status().is_success() {
            self.session_state = None;
            Ok(())
        } else {
            Err(GraphError::TransactionFailed("Commit failed".to_string()))
        }
    }

    pub fn rollback_transaction(&mut self, session_id: &str) -> Result<(), GraphError> {
        // Build the URL with database name if specified
        let url = if let Some(db_name) = &self.database_name {
            format!(
                "{}/db/{}/tx/{}/rollback",
                self.base_url, db_name, session_id
            )
        } else {
            format!(
                "{}/db/data/transaction/{}/rollback",
                self.base_url, session_id
            )
        };

        let response = self
            .client
            .request(Method::DELETE, url)
            .basic_auth(&self.username, Some(&self.password))
            .send()
            .map_err(|err| from_reqwest_error("Rollback failed", err))?;

        if response.status().is_success() {
            self.session_state = None;
            Ok(())
        } else {
            Err(GraphError::TransactionFailed("Rollback failed".to_string()))
        }
    }

    fn parse_response(&self, response: reqwest::Response) -> Result<Neo4jResponse, GraphError> {
        if response.status().is_success() {
            let neo4j_response: Neo4jResponse = response.json().map_err(|err| {
                GraphError::InternalError(format!("Failed to parse response: {err}"))
            })?;

            // Check for Neo4j errors
            if !neo4j_response.errors.is_empty() {
                let error_msg = neo4j_response
                    .errors
                    .iter()
                    .map(|e| format!("{}: {}", e.code, e.message))
                    .collect::<Vec<_>>()
                    .join("; ");
                return Err(GraphError::InvalidQuery(error_msg));
            }

            Ok(neo4j_response)
        } else {
            Err(GraphError::InternalError(format!(
                "HTTP error: {}",
                response.status()
            )))
        }
    }
    pub fn _execute_batch(
        &self,
        statements: Vec<String>,
    ) -> Result<Vec<Neo4jResponse>, GraphError> {
        let mut responses = Vec::new();
        for statement in statements {
            let response = self.execute_cypher(statement, None)?;
            responses.push(response);
        }
        Ok(responses)
    }
    pub fn _find_shortest_path(
        &self,
        from_vertex: &str,
        to_vertex: &str,
        edge_filter: &str,
        depth_limit: u32,
    ) -> Result<Neo4jResponse, GraphError> {
        let query = format!(
            "MATCH path = shortestPath((from:Vertex {{id: '{from_vertex}'}})-[*..{depth_limit}]-(to:Vertex {{id: '{to_vertex}'}})) WHERE all(rel in relationships(path) WHERE {edge_filter}) RETURN path"
        );
        self.execute_cypher(query, None)
    }

    /// Find all paths between two vertices
    pub fn _find_all_paths(
        &self,
        from_vertex: &str,
        to_vertex: &str,
        edge_filter: &str,
        depth_limit: u32,
        result_limit: u32,
    ) -> Result<Neo4jResponse, GraphError> {
        let query = format!(
            "MATCH path = (from:Vertex {{id: '{from_vertex}'}})-[*..{depth_limit}]-(to:Vertex {{id: '{to_vertex}'}})) WHERE all(rel in relationships(path) WHERE {edge_filter}) RETURN path LIMIT {result_limit}"
        );
        self.execute_cypher(query, None)
    }

    /// Get neighborhood around a vertex
    pub fn _get_neighborhood(
        &self,
        center_vertex: &str,
        direction: &str,
        depth: u32,
        vertex_limit: u32,
    ) -> Result<Neo4jResponse, GraphError> {
        let _direction_used = direction;
        let query = format!(
            "MATCH (center:Vertex {{id: '{center_vertex}'}})-[*..{depth}]-(neighbor:Vertex) RETURN center, neighbor LIMIT {vertex_limit}"
        );
        self.execute_cypher(query, None)
    }

    /// Check if path exists between vertices
    pub fn _path_exists(
        &self,
        from_vertex: &str,
        to_vertex: &str,
        edge_filter: &str,
        depth_limit: u32,
    ) -> Result<Neo4jResponse, GraphError> {
        let query = format!(
            "MATCH path = shortestPath((from:Vertex {{id: '{from_vertex}'}})-[*..{depth_limit}]-(to:Vertex {{id: '{to_vertex}'}})) WHERE all(rel in relationships(path) WHERE {edge_filter}) RETURN count(path) > 0 as exists"
        );
        self.execute_cypher(query, None)
    }

    /// Get vertices at specific distance from source
    pub fn _get_vertices_at_distance(
        &self,
        source_vertex: &str,
        distance: u32,
        direction: &str,
        edge_types: Option<Vec<String>>,
        vertex_limit: u32,
    ) -> Result<Neo4jResponse, GraphError> {
        let _direction_used = direction;
        let edge_filter = if let Some(types) = edge_types {
            format!("type(rel) IN {types:?}")
        } else {
            "true".to_string()
        };

        let query = format!(
            "MATCH (source:Vertex {{id: '{source_vertex}'}})-[*{distance}]-(target:Vertex) WHERE all(rel in relationships(path) WHERE {edge_filter}) RETURN target LIMIT {vertex_limit}"
        );
        self.execute_cypher(query, None)
    }

    /// Execute custom Cypher query
    pub fn _execute_custom_query(&self, query: String) -> Result<Neo4jResponse, GraphError> {
        self.execute_cypher(query, None)
    }

    /// Create index
    pub fn _create_index(
        &self,
        label: &str,
        property: &str,
        index_type: &str,
    ) -> Result<Neo4jResponse, GraphError> {
        let query = format!(
            "CREATE INDEX {label}_{property}_idx IF NOT EXISTS FOR (n:{label}) ON (n.{property}) TYPE {index_type}"
        );
        self.execute_cypher(query, None)
    }

    /// Drop index
    pub fn drop_index(&self, index_name: &str) -> Result<Neo4jResponse, GraphError> {
        let query = format!("DROP INDEX {index_name}");
        self.execute_cypher(query, None)
    }

    /// List all indexes
    pub fn list_indexes(&self) -> Result<Neo4jResponse, GraphError> {
        let query = "SHOW INDEXES".to_string();
        self.execute_cypher(query, None)
    }

    /// Get index by name
    pub fn get_index(&self, index_name: &str) -> Result<Neo4jResponse, GraphError> {
        let query = format!("SHOW INDEX {index_name}");
        self.execute_cypher(query, None)
    }

    /// List all labels (vertex types)
    pub fn list_labels(&self) -> Result<Neo4jResponse, GraphError> {
        let query = "CALL db.labels() YIELD label RETURN label".to_string();
        self.execute_cypher(query, None)
    }

    /// List all relationship types
    pub fn list_relationship_types(&self) -> Result<Neo4jResponse, GraphError> {
        let query = "CALL db.relationshipTypes() YIELD relationshipType RETURN relationshipType"
            .to_string();
        self.execute_cypher(query, None)
    }

    /// Get schema information for a label
    pub fn get_label_schema(&self, label: &str) -> Result<Neo4jResponse, GraphError> {
        let query = format!(
            "MATCH (n:{label}) 
             RETURN DISTINCT keys(n) as properties, 
                    labels(n) as labels"
        );
        self.execute_cypher(query, None)
    }
}
