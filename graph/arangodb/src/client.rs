use golem_graph::exports::golem::graph::connection::GraphStatistics;
use golem_graph::exports::golem::graph::errors::GraphError;
use reqwest::Method;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArangoRequest {
    pub query: String,
    #[serde(rename = "bindVars")]
    pub bind_vars: Option<HashMap<String, serde_json::Value>>,
    #[serde(rename = "batchSize")]
    pub batch_size: Option<u32>,
    pub count: Option<bool>,
    #[serde(rename = "ttl")]
    pub ttl: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArangoResponse {
    pub result: Vec<serde_json::Value>,
    #[serde(rename = "hasMore")]
    pub has_more: bool,
    pub count: Option<u64>,
    pub error: bool,
    pub code: u16,
    #[serde(rename = "errorMessage")]
    pub error_message: Option<String>,
    #[serde(rename = "errorNum")]
    pub error_num: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArangoAuthRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArangoAuthResponse {
    pub jwt: String,
    #[serde(rename = "must_change_password")]
    pub must_change_password: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct SessionState {}

#[derive(Debug, Clone)]
pub struct ArangoClient {
    client: reqwest::Client,
    base_url: String,
    database: String,
    username: String,
    password: String,
    jwt_token: Option<String>,
    session_state: Option<SessionState>,
}

impl ArangoClient {
    pub fn new(base_url: String, username: String, password: String, database: String) -> Self {
        let client = reqwest::Client::new();
        Self {
            client,
            base_url,
            database,
            username,
            password,
            jwt_token: None,
            session_state: None,
        }
    }

    fn authenticate(&mut self) -> Result<(), GraphError> {
        if self.jwt_token.is_some() {
            return Ok(());
        }

        let auth_url = format!("{}/_open/auth", self.base_url);
        let auth_request = ArangoAuthRequest {
            username: self.username.clone(),
            password: self.password.clone(),
        };

        let response = self
            .client
            .request(Method::POST, auth_url)
            .json(&auth_request)
            .send()
            .map_err(|err| from_reqwest_error("Authentication failed", err))?;

        if response.status().is_success() {
            let auth_response: ArangoAuthResponse = response.json().map_err(|err| {
                GraphError::InternalError(format!("Failed to parse auth response: {err}"))
            })?;
            self.jwt_token = Some(auth_response.jwt);
            Ok(())
        } else {
            Err(GraphError::InternalError(
                "Authentication failed".to_string(),
            ))
        }
    }

    pub fn execute_query(
        &self,
        query: &str,
        bind_vars: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<ArangoResponse, GraphError> {
        let mut temp_client = self.clone();
        temp_client.authenticate()?;

        let url = format!("{}/_db/{}/_api/cursor", self.base_url, self.database);
        let request = ArangoRequest {
            query: query.to_string(),
            bind_vars,
            batch_size: Some(1000),
            count: Some(true),
            ttl: Some(60),
        };

        let response = temp_client
            .client
            .request(Method::POST, url)
            .header(
                "Authorization",
                format!("Bearer {}", temp_client.jwt_token.unwrap()),
            )
            .json(&request)
            .send()
            .map_err(|err| from_reqwest_error("Query execution failed", err))?;

        if response.status().is_success() {
            let arango_response: ArangoResponse = response.json().map_err(|err| {
                GraphError::InternalError(format!("Failed to parse response: {err}"))
            })?;

            if arango_response.error {
                let error_msg = arango_response
                    .error_message
                    .unwrap_or_else(|| "Unknown error".to_string());
                return Err(GraphError::InvalidQuery(error_msg));
            }

            Ok(arango_response)
        } else {
            Err(GraphError::InternalError(format!(
                "HTTP error: {}",
                response.status()
            )))
        }
    }

    pub fn begin_transaction(&mut self) -> Result<String, GraphError> {
        self.authenticate()?;

        let url = format!("{}/_db/{}/_api/transaction", self.base_url, self.database);
        let request = serde_json::json!({
            "collections": {
                "write": ["*"],
                "read": ["*"]
            }
        });

        let response = self
            .client
            .request(Method::POST, url)
            .header(
                "Authorization",
                format!("Bearer {}", self.jwt_token.as_ref().unwrap()),
            )
            .json(&request)
            .send()
            .map_err(|err| from_reqwest_error("Begin transaction failed", err))?;

        if response.status().is_success() {
            let result: serde_json::Value = response.json().map_err(|err| {
                GraphError::InternalError(format!("Failed to parse response: {err}"))
            })?;

            let tx_id = result["result"]["id"]
                .as_str()
                .ok_or_else(|| {
                    GraphError::InternalError("No transaction ID in response".to_string())
                })?
                .to_string();

            self.session_state = Some(SessionState {});

            Ok(tx_id)
        } else {
            Err(GraphError::TransactionFailed(
                "Begin transaction failed".to_string(),
            ))
        }
    }

    pub fn begin_read_transaction(&mut self) -> Result<String, GraphError> {
        self.authenticate()?;

        let url = format!("{}/_db/{}/_api/transaction", self.base_url, self.database);
        let request = serde_json::json!({
            "collections": {
                "read": ["*"]
            }
        });

        let response = self
            .client
            .request(Method::POST, url)
            .header(
                "Authorization",
                format!("Bearer {}", self.jwt_token.as_ref().unwrap()),
            )
            .json(&request)
            .send()
            .map_err(|err| from_reqwest_error("Begin read transaction failed", err))?;

        if response.status().is_success() {
            let result: serde_json::Value = response.json().map_err(|err| {
                GraphError::InternalError(format!("Failed to parse response: {err}"))
            })?;

            let tx_id = result["result"]["id"]
                .as_str()
                .ok_or_else(|| {
                    GraphError::InternalError("No transaction ID in response".to_string())
                })?
                .to_string();

            self.session_state = Some(SessionState {});

            Ok(tx_id)
        } else {
            Err(GraphError::TransactionFailed(
                "Begin read transaction failed".to_string(),
            ))
        }
    }

    pub fn commit_transaction(&mut self, tx_id: &str) -> Result<(), GraphError> {
        self.authenticate()?;

        let url = format!(
            "{}/_db/{}/_api/transaction/{}",
            self.base_url, self.database, tx_id
        );

        let response = self
            .client
            .request(Method::PUT, url)
            .header(
                "Authorization",
                format!("Bearer {}", self.jwt_token.as_ref().unwrap()),
            )
            .send()
            .map_err(|err| from_reqwest_error("Commit failed", err))?;

        if response.status().is_success() {
            self.session_state = None;
            Ok(())
        } else {
            Err(GraphError::TransactionFailed("Commit failed".to_string()))
        }
    }

    pub fn rollback_transaction(&mut self, tx_id: &str) -> Result<(), GraphError> {
        self.authenticate()?;

        let url = format!(
            "{}/_db/{}/_api/transaction/{}",
            self.base_url, self.database, tx_id
        );

        let response = self
            .client
            .request(Method::DELETE, url)
            .header(
                "Authorization",
                format!("Bearer {}", self.jwt_token.as_ref().unwrap()),
            )
            .send()
            .map_err(|err| from_reqwest_error("Rollback failed", err))?;

        if response.status().is_success() {
            self.session_state = None;
            Ok(())
        } else {
            Err(GraphError::TransactionFailed("Rollback failed".to_string()))
        }
    }

    pub fn create_vertex(
        &mut self,
        collection: &str,
        properties: serde_json::Value,
    ) -> Result<ArangoResponse, GraphError> {
        self.authenticate()?;

        let url = format!(
            "{}/_db/{}/_api/document/{}",
            self.base_url, self.database, collection
        );

        let response = self
            .client
            .request(Method::POST, url)
            .header(
                "Authorization",
                format!("Bearer {}", self.jwt_token.as_ref().unwrap()),
            )
            .json(&properties)
            .send()
            .map_err(|err| from_reqwest_error("Create vertex failed", err))?;

        if response.status().is_success() {
            let result: serde_json::Value = response.json().map_err(|err| {
                GraphError::InternalError(format!("Failed to parse response: {err}"))
            })?;

            Ok(ArangoResponse {
                result: vec![result],
                has_more: false,
                count: Some(1),
                error: false,
                code: 200,
                error_message: None,
                error_num: None,
            })
        } else {
            Err(GraphError::InternalError(
                "Create vertex failed".to_string(),
            ))
        }
    }

    pub fn get_vertex(&self, id: &str) -> Result<ArangoResponse, GraphError> {
        let mut temp_client = self.clone();
        temp_client.authenticate()?;

        let url = format!(
            "{}/_db/{}/_api/document/{}",
            self.base_url, self.database, id
        );

        let response = temp_client
            .client
            .request(Method::GET, url)
            .header(
                "Authorization",
                format!("Bearer {}", temp_client.jwt_token.unwrap()),
            )
            .send()
            .map_err(|err| from_reqwest_error("Get vertex failed", err))?;

        if response.status().is_success() {
            let result: serde_json::Value = response.json().map_err(|err| {
                GraphError::InternalError(format!("Failed to parse response: {err}"))
            })?;

            Ok(ArangoResponse {
                result: vec![result],
                has_more: false,
                count: Some(1),
                error: false,
                code: 200,
                error_message: None,
                error_num: None,
            })
        } else if response.status().as_u16() == 404 {
            Ok(ArangoResponse {
                result: vec![],
                has_more: false,
                count: Some(0),
                error: false,
                code: 404,
                error_message: None,
                error_num: None,
            })
        } else {
            Err(GraphError::InternalError("Get vertex failed".to_string()))
        }
    }

    pub fn update_vertex(
        &mut self,
        id: &str,
        properties: serde_json::Value,
    ) -> Result<ArangoResponse, GraphError> {
        self.authenticate()?;

        let url = format!(
            "{}/_db/{}/_api/document/{}",
            self.base_url, self.database, id
        );

        let response = self
            .client
            .request(Method::PATCH, url)
            .header(
                "Authorization",
                format!("Bearer {}", self.jwt_token.as_ref().unwrap()),
            )
            .json(&properties)
            .send()
            .map_err(|err| from_reqwest_error("Update vertex failed", err))?;

        if response.status().is_success() {
            let result: serde_json::Value = response.json().map_err(|err| {
                GraphError::InternalError(format!("Failed to parse response: {err}"))
            })?;

            Ok(ArangoResponse {
                result: vec![result],
                has_more: false,
                count: Some(1),
                error: false,
                code: 200,
                error_message: None,
                error_num: None,
            })
        } else {
            Err(GraphError::InternalError(
                "Update vertex failed".to_string(),
            ))
        }
    }

    pub fn delete_vertex(&mut self, id: &str, _delete_edges: bool) -> Result<(), GraphError> {
        self.authenticate()?;

        let url = format!(
            "{}/_db/{}/_api/document/{}",
            self.base_url, self.database, id
        );

        let response = self
            .client
            .request(Method::DELETE, url)
            .header(
                "Authorization",
                format!("Bearer {}", self.jwt_token.as_ref().unwrap()),
            )
            .send()
            .map_err(|err| from_reqwest_error("Delete vertex failed", err))?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(GraphError::InternalError(
                "Delete vertex failed".to_string(),
            ))
        }
    }

    pub fn create_edge(
        &mut self,
        collection: &str,
        from_id: &str,
        to_id: &str,
        properties: serde_json::Value,
    ) -> Result<ArangoResponse, GraphError> {
        self.authenticate()?;

        let mut edge_properties = properties.as_object().unwrap().clone();
        edge_properties.insert(
            "_from".to_string(),
            serde_json::Value::String(from_id.to_string()),
        );
        edge_properties.insert(
            "_to".to_string(),
            serde_json::Value::String(to_id.to_string()),
        );

        let url = format!(
            "{}/_db/{}/_api/document/{}",
            self.base_url, self.database, collection
        );

        let response = self
            .client
            .request(Method::POST, url)
            .header(
                "Authorization",
                format!("Bearer {}", self.jwt_token.as_ref().unwrap()),
            )
            .json(&edge_properties)
            .send()
            .map_err(|err| from_reqwest_error("Create edge failed", err))?;

        if response.status().is_success() {
            let result: serde_json::Value = response.json().map_err(|err| {
                GraphError::InternalError(format!("Failed to parse response: {err}"))
            })?;

            Ok(ArangoResponse {
                result: vec![result],
                has_more: false,
                count: Some(1),
                error: false,
                code: 200,
                error_message: None,
                error_num: None,
            })
        } else {
            Err(GraphError::InternalError("Create edge failed".to_string()))
        }
    }

    pub fn get_edge(&self, id: &str) -> Result<ArangoResponse, GraphError> {
        let mut temp_client = self.clone();
        temp_client.authenticate()?;

        let url = format!(
            "{}/_db/{}/_api/document/{}",
            self.base_url, self.database, id
        );

        let response = temp_client
            .client
            .request(Method::GET, url)
            .header(
                "Authorization",
                format!("Bearer {}", temp_client.jwt_token.unwrap()),
            )
            .send()
            .map_err(|err| from_reqwest_error("Get edge failed", err))?;

        if response.status().is_success() {
            let result: serde_json::Value = response.json().map_err(|err| {
                GraphError::InternalError(format!("Failed to parse response: {err}"))
            })?;

            Ok(ArangoResponse {
                result: vec![result],
                has_more: false,
                count: Some(1),
                error: false,
                code: 200,
                error_message: None,
                error_num: None,
            })
        } else if response.status().as_u16() == 404 {
            Ok(ArangoResponse {
                result: vec![],
                has_more: false,
                count: Some(0),
                error: false,
                code: 404,
                error_message: None,
                error_num: None,
            })
        } else {
            Err(GraphError::InternalError("Get edge failed".to_string()))
        }
    }

    pub fn update_edge(
        &mut self,
        id: &str,
        properties: serde_json::Value,
    ) -> Result<ArangoResponse, GraphError> {
        self.authenticate()?;

        let url = format!(
            "{}/_db/{}/_api/document/{}",
            self.base_url, self.database, id
        );

        let response = self
            .client
            .request(Method::PATCH, url)
            .header(
                "Authorization",
                format!("Bearer {}", self.jwt_token.as_ref().unwrap()),
            )
            .json(&properties)
            .send()
            .map_err(|err| from_reqwest_error("Update edge failed", err))?;

        if response.status().is_success() {
            let result: serde_json::Value = response.json().map_err(|err| {
                GraphError::InternalError(format!("Failed to parse response: {err}"))
            })?;

            Ok(ArangoResponse {
                result: vec![result],
                has_more: false,
                count: Some(1),
                error: false,
                code: 200,
                error_message: None,
                error_num: None,
            })
        } else {
            Err(GraphError::InternalError("Update edge failed".to_string()))
        }
    }

    pub fn delete_edge(&mut self, id: &str) -> Result<(), GraphError> {
        self.authenticate()?;

        let url = format!(
            "{}/_db/{}/_api/document/{}",
            self.base_url, self.database, id
        );

        let response = self
            .client
            .request(Method::DELETE, url)
            .header(
                "Authorization",
                format!("Bearer {}", self.jwt_token.as_ref().unwrap()),
            )
            .send()
            .map_err(|err| from_reqwest_error("Delete edge failed", err))?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(GraphError::InternalError("Delete edge failed".to_string()))
        }
    }

    pub fn ping(&self) -> Result<(), GraphError> {
        let url = format!("{}/_api/version", self.base_url);

        let response = self
            .client
            .request(Method::GET, url)
            .send()
            .map_err(|err| from_reqwest_error("Ping failed", err))?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(GraphError::InternalError("Ping failed".to_string()))
        }
    }

    pub fn close(&mut self) -> Result<(), GraphError> {
        self.session_state = None;
        self.jwt_token = None;
        Ok(())
    }

    pub fn get_statistics(&self) -> Result<GraphStatistics, GraphError> {
        let query =
            "RETURN { vertex_count: LENGTH(_collections), edge_count: 0, label_count: LENGTH(_collections) }";
        let response = self.execute_query(query, None)?;

        if response.result.is_empty() {
            return Ok(GraphStatistics {
                vertex_count: Some(0),
                edge_count: Some(0),
                label_count: Some(0),
                property_count: Some(0),
            });
        }

        let stats = &response.result[0];
        let vertex_count = stats["vertex_count"].as_u64().unwrap_or(0);
        let edge_count = stats["edge_count"].as_u64().unwrap_or(0);
        let label_count = stats["label_count"].as_u64().unwrap_or(0) as u32;

        Ok(GraphStatistics {
            vertex_count: Some(vertex_count),
            edge_count: Some(edge_count),
            label_count: Some(label_count),
            property_count: Some(0),
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

    pub fn get_database(&self) -> String {
        self.database.clone()
    }

    pub fn _execute_batch(&self, queries: Vec<String>) -> Result<Vec<ArangoResponse>, GraphError> {
        let mut responses = Vec::new();
        for query in queries {
            let response = self.execute_query(&query, None)?;
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
    ) -> Result<ArangoResponse, GraphError> {
        let query = format!(
            "FOR v, e, p IN 1..{depth_limit} SHORTEST_PATH '{from_vertex}' TO '{to_vertex}' GRAPH 'default' FILTER {edge_filter} RETURN p"
        );
        self.execute_query(&query, None)
    }

    pub fn _find_all_paths(
        &self,
        from_vertex: &str,
        to_vertex: &str,
        edge_filter: &str,
        depth_limit: u32,
    ) -> Result<ArangoResponse, GraphError> {
        let query = format!(
            "FOR v, e, p IN 1..{depth_limit} ALL SHORTEST_PATHS '{from_vertex}' TO '{to_vertex}' GRAPH 'default' FILTER {edge_filter} RETURN p"
        );
        self.execute_query(&query, None)
    }

    pub fn _get_neighborhood(
        &self,
        center_vertex: &str,
        direction: &str,
        depth: u32,
        vertex_limit: u32,
    ) -> Result<ArangoResponse, GraphError> {
        let _direction_used = direction;
        let query = format!(
            "FOR v, e, p IN 1..{depth} {direction} '{center_vertex}' GRAPH 'default' LIMIT {vertex_limit} RETURN v"
        );
        self.execute_query(&query, None)
    }

    pub fn _path_exists(
        &self,
        from_vertex: &str,
        to_vertex: &str,
        edge_filter: &str,
        depth_limit: u32,
    ) -> Result<ArangoResponse, GraphError> {
        let query = format!(
            "FOR v, e, p IN 1..{depth_limit} SHORTEST_PATH '{from_vertex}' TO '{to_vertex}' GRAPH 'default' FILTER {edge_filter} LIMIT 1 RETURN LENGTH(p) > 0"
        );
        self.execute_query(&query, None)
    }

    pub fn _get_vertices_at_distance(
        &self,
        source_vertex: &str,
        distance: u32,
        direction: &str,
        edge_types: Option<Vec<String>>,
        vertex_limit: u32,
    ) -> Result<ArangoResponse, GraphError> {
        let _direction_used = direction;
        let edge_filter = if let Some(types) = edge_types {
            format!("e.type IN {}", serde_json::to_string(&types).unwrap())
        } else {
            "true".to_string()
        };

        let query = format!(
            "FOR v, e, p IN {distance} {direction} '{source_vertex}' GRAPH 'default' FILTER {edge_filter} LIMIT {vertex_limit} RETURN v"
        );
        self.execute_query(&query, None)
    }

    pub fn _execute_custom_query(&self, query: String) -> Result<ArangoResponse, GraphError> {
        self.execute_query(&query, None)
    }

    pub fn _create_index(
        &self,
        collection: &str,
        fields: Vec<String>,
        index_type: &str,
    ) -> Result<ArangoResponse, GraphError> {
        let fields_json = serde_json::to_string(&fields).unwrap();
        let query = format!(
            "db.{collection}.ensureIndex({{ fields: {fields_json}, type: '{index_type}' }})"
        );
        self.execute_query(&query, None)
    }

    pub fn _drop_index(&self, index_id: &str) -> Result<ArangoResponse, GraphError> {
        let query = format!("db._dropIndex('{index_id}')");
        self.execute_query(&query, None)
    }

    pub fn _list_indexes(&self) -> Result<ArangoResponse, GraphError> {
        let query = "FOR index IN db._indexes() RETURN index";
        self.execute_query(query, None)
    }

    pub fn _get_index(&self, index_name: &str) -> Result<ArangoResponse, GraphError> {
        let query =
            format!("FOR index IN db._indexes() FILTER index.name == '{index_name}' RETURN index");
        self.execute_query(&query, None)
    }

    pub fn _get_collection_schema(&self, collection: &str) -> Result<ArangoResponse, GraphError> {
        let query = format!("FOR doc IN {collection} LIMIT 1 RETURN doc");
        self.execute_query(&query, None)
    }

    pub fn _create_collection(
        &self,
        name: &str,
        collection_type: &str,
    ) -> Result<ArangoResponse, GraphError> {
        let query = format!("db._create('{name}', {{ type: '{collection_type}' }})");
        self.execute_query(&query, None)
    }

    pub fn _drop_collection(&self, name: &str) -> Result<ArangoResponse, GraphError> {
        let query = format!("db._drop('{name}')");
        self.execute_query(&query, None)
    }

    pub fn list_collections(&self) -> Result<ArangoResponse, GraphError> {
        let query = "FOR collection IN db._collections() RETURN collection";
        self.execute_query(query, None)
    }
}

fn from_reqwest_error(context: &str, err: reqwest::Error) -> GraphError {
    GraphError::InternalError(format!("{context}: {err}"))
}
