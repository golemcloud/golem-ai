use base64::{engine::general_purpose, Engine as _};
use golem_graph::golem::graph::errors::GraphError;
use golem_graph::golem::graph::schema::{
    ContainerInfo, ContainerType, EdgeTypeDefinition, IndexDefinition, IndexType,
};
use golem_graph::golem::graph::types::ElementId;
use log::trace;
use reqwest::{Client, Method, Response};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};

pub struct ArangoDbApi {
    base_url: String,
    client: Client,
    auth_header: String,
}

impl ArangoDbApi {
    pub fn new(host: &str, port: u16, username: &str, password: &str, database_name: &str) -> Self {
        trace!(
            "Initializing ArangoDbApi for host: {host}, port: {port}, database: {database_name}"
        );
        let base_url = format!("http://{host}:{port}/_db/{database_name}");
        let auth_header = format!(
            "Basic {}",
            general_purpose::STANDARD.encode(format!("{username}:{password}"))
        );

        let client = Client::builder()
            .build()
            .expect("Failed to initialize HTTP client");

        Self {
            base_url,
            client,
            auth_header,
        }
    }

    fn execute<T: DeserializeOwned>(
        &self,
        method: Method,
        endpoint: &str,
        body: Option<&Value>,
    ) -> Result<T, GraphError> {
        let url = format!("{}{}", self.base_url, endpoint);

        let mut request_builder = self
            .client
            .request(method, url)
            .header("authorization", &self.auth_header);

        if let Some(body_value) = body {
            let body_string = serde_json::to_string(body_value).map_err(|e| {
                GraphError::InternalError(format!("Failed to serialize request body: {e}"))
            })?;

            request_builder = request_builder
                .header("content-type", "application/json")
                .header("content-length", body_string.len().to_string())
                .body(body_string);
        }

        let response = request_builder
            .send()
            .map_err(|e| self.handle_arango_reqwest_error("Request failed", e))?;

        self.handle_response(response)
    }

    fn handle_response<T: DeserializeOwned>(&self, response: Response) -> Result<T, GraphError> {
        let status = response.status();
        let status_code = status.as_u16();

        if status.is_success() {
            let response_body: Value = response.json().map_err(|e| {
                GraphError::InternalError(format!("Failed to parse response body: {e}"))
            })?;

            if let Some(result) = response_body.get("result") {
                serde_json::from_value(result.clone()).map_err(|e| {
                    GraphError::InternalError(format!(
                        "Failed to deserialize successful response: {e}"
                    ))
                })
            } else {
                serde_json::from_value(response_body).map_err(|e| {
                    GraphError::InternalError(format!(
                        "Failed to deserialize successful response: {e}"
                    ))
                })
            }
        } else {
            let error_body: ArangoErrorResponse = response.json().map_err(|e| {
                GraphError::InternalError(format!("Failed to read error response: {e}"))
            })?;

            let error_msg = error_body
                .error_message
                .as_deref()
                .unwrap_or("Unknown error");

            let mut error = if let Some(code) = error_body.error_num {
                from_arangodb_error_code(code, error_msg)
            } else {
                map_arangodb_http_status(status_code, error_msg, &error_body)
            };

            error = self.enhance_arangodb_error(error, &error_body);

            Err(error)
        }
    }

    #[allow(dead_code)]
    pub fn begin_transaction(&self, read_only: bool) -> Result<String, GraphError> {
        trace!("Begin transaction (read_only={read_only})");
        let existing_collections = self.list_collections().unwrap_or_default();
        let collection_names: Vec<String> = existing_collections
            .iter()
            .map(|c| c.name.clone())
            .collect();

        let collections = if read_only {
            json!({ "read": collection_names })
        } else {
            json!({ "write": collection_names })
        };

        let body = json!({ "collections": collections });
        let result: TransactionBeginResponse =
            self.execute(Method::POST, "/_api/transaction/begin", Some(&body))?;
        Ok(result.id)
    }

    #[allow(dead_code)]
    pub fn begin_transaction_with_collections(
        &self,
        read_only: bool,
        collections: Vec<String>,
    ) -> Result<String, GraphError> {
        trace!(
            "Begin transaction with collections (read_only={read_only}, collections={collections:?})"
        );
        let collections_spec = if read_only {
            json!({ "read": collections })
        } else {
            json!({ "write": collections })
        };

        let body = json!({ "collections": collections_spec });
        let result: TransactionBeginResponse =
            self.execute(Method::POST, "/_api/transaction/begin", Some(&body))?;
        Ok(result.id)
    }

    pub fn commit_transaction(&self, transaction_id: &str) -> Result<(), GraphError> {
        trace!("Commit transaction: {transaction_id}");
        let endpoint = format!("/_api/transaction/{transaction_id}");
        let _: Value = self.execute(Method::PUT, &endpoint, None)?;
        Ok(())
    }

    pub fn rollback_transaction(&self, transaction_id: &str) -> Result<(), GraphError> {
        trace!("Rollback transaction: {transaction_id}");
        let endpoint = format!("/_api/transaction/{transaction_id}");
        let _: Value = self.execute(Method::DELETE, &endpoint, None)?;
        Ok(())
    }

    pub fn execute_in_transaction(
        &self,
        transaction_id: &str,
        query: Value,
    ) -> Result<Value, GraphError> {
        trace!("Execute in transaction: {transaction_id}");
        let url = format!("{}/_api/cursor", self.base_url);

        let body_string = serde_json::to_string(&query)
            .map_err(|e| GraphError::InternalError(format!("Failed to serialize query: {e}")))?;

        let response = self
            .client
            .request(Method::POST, url)
            .header("authorization", &self.auth_header)
            .header("content-type", "application/json")
            .header("content-length", body_string.len().to_string())
            .header("x-arango-trx-id", transaction_id)
            .body(body_string)
            .send()
            .map_err(|e| self.handle_arango_reqwest_error("Transaction query failed", e))?;

        self.handle_response(response)
    }

    pub fn ping(&self) -> Result<(), GraphError> {
        trace!("Ping ArangoDB");
        let _: Value = self.execute(Method::GET, "/_api/version", None)?;
        Ok(())
    }

    fn enhance_arangodb_error(
        &self,
        error: GraphError,
        error_body: &ArangoErrorResponse,
    ) -> GraphError {
        match &error {
            GraphError::InternalError(_)
                if self.is_arangodb_document_not_found_error(error_body) =>
            {
                if let Some(element_id) = self.extract_arangodb_element_id(error_body) {
                    GraphError::ElementNotFound(element_id)
                } else {
                    error
                }
            }
            GraphError::ConstraintViolation(_)
                if self.is_arangodb_unique_constraint_error(error_body) =>
            {
                if let Some(element_id) = self.extract_arangodb_element_id(error_body) {
                    GraphError::DuplicateElement(element_id)
                } else {
                    error
                }
            }
            _ => error,
        }
    }

    fn is_arangodb_document_not_found_error(&self, error_body: &ArangoErrorResponse) -> bool {
        error_body.error_num == Some(1202)
    }

    fn is_arangodb_unique_constraint_error(&self, error_body: &ArangoErrorResponse) -> bool {
        error_body.error_num == Some(1210)
    }

    fn extract_arangodb_element_id(&self, error_body: &ArangoErrorResponse) -> Option<ElementId> {
        if let Some(doc_id) = &error_body.id {
            return Some(ElementId::StringValue(doc_id.clone()));
        }

        if let Some(doc_key) = &error_body.key {
            return Some(ElementId::StringValue(doc_key.clone()));
        }

        if let Some(error_msg) = &error_body.error_message {
            if let Some(element_id) = self.extract_arangodb_id_from_message(error_msg) {
                return Some(element_id);
            }
        }
        None
    }

    fn extract_arangodb_id_from_message(&self, message: &str) -> Option<ElementId> {
        if let Some(start) = message.find('"') {
            if let Some(end) = message[start + 1..].find('"') {
                let potential_id = &message[start + 1..start + 1 + end];
                if potential_id.contains('/') && potential_id.len() > 3 {
                    return Some(ElementId::StringValue(potential_id.to_string()));
                }
            }
        }

        if message.contains('/') {
            let words: Vec<&str> = message.split_whitespace().collect();
            for word in words {
                if word.contains('/') && word.matches('/').count() == 1 {
                    let parts: Vec<&str> = word.split('/').collect();
                    if parts.len() == 2 && !parts[0].is_empty() && !parts[1].is_empty() {
                        return Some(ElementId::StringValue(word.to_string()));
                    }
                }
            }
        }
        None
    }

    fn handle_arango_reqwest_error(&self, details: &str, err: reqwest::Error) -> GraphError {
        if err.is_timeout() {
            return GraphError::Timeout;
        }

        if err.is_request() {
            return GraphError::ConnectionFailed(format!(
                "ArangoDB request failed ({details}): {err}"
            ));
        }

        if err.is_decode() {
            return GraphError::InternalError(format!(
                "ArangoDB response decode failed ({details}): {err}"
            ));
        }

        if err.is_status() {
            if let Some(status) = err.status() {
                let error_msg = format!(
                    "ArangoDB HTTP error {} ({}): {}",
                    status.as_u16(),
                    details,
                    err
                );
                let error_body = ArangoErrorResponse {
                    error_message: Some(error_msg.clone()),
                    error_num: None,
                    id: None,
                    key: None,
                };
                return map_arangodb_http_status(status.as_u16(), &error_msg, &error_body);
            }
        }
        GraphError::InternalError(format!("ArangoDB request error ({details}): {err}"))
    }

    // Schema operations
    pub fn create_collection(
        &self,
        name: &str,
        container_type: ContainerType,
    ) -> Result<(), GraphError> {
        trace!("Create collection: {name}, type: {container_type:?}");
        let collection_type = match container_type {
            ContainerType::VertexContainer => 2,
            ContainerType::EdgeContainer => 3,
        };
        let body = json!({ "name": name, "type": collection_type });
        let _: Value = self.execute(Method::POST, "/_api/collection", Some(&body))?;
        Ok(())
    }

    pub fn list_collections(&self) -> Result<Vec<ContainerInfo>, GraphError> {
        trace!("List collections");
        let collections: Vec<CollectionInfo> =
            self.execute(Method::GET, "/_api/collection", None)?;

        let collections = collections
            .into_iter()
            .filter(|c| !c.is_system)
            .map(|c| {
                let container_type = if c.collection_type == 3 {
                    ContainerType::EdgeContainer
                } else {
                    ContainerType::VertexContainer
                };
                ContainerInfo {
                    name: c.name,
                    container_type,
                    element_count: None,
                }
            })
            .collect();
        Ok(collections)
    }

    pub fn create_index(
        &self,
        collection: String,
        fields: Vec<String>,
        unique: bool,
        index_type: IndexType,
        name: Option<String>,
    ) -> Result<(), GraphError> {
        trace!(
            "Create index on collection: {collection}, fields: {fields:?}, unique: {unique}, type: {index_type:?}, name: {name:?}"
        );
        let type_str = match index_type {
            IndexType::Exact => "persistent",
            IndexType::Range => "persistent",
            IndexType::Text => "inverted",
            IndexType::Geospatial => "geo",
        };

        let mut body = json!({
            "type": type_str,
            "fields": fields,
            "unique": unique,
        });

        if let Some(index_name) = name {
            body["name"] = json!(index_name);
        }

        let endpoint = format!("/_api/index?collection={collection}");
        let _: Value = self.execute(Method::POST, &endpoint, Some(&body))?;
        Ok(())
    }

    pub fn drop_index(&self, name: &str) -> Result<(), GraphError> {
        trace!("Drop index: {name}");
        let collections = self.list_collections()?;

        for collection in collections {
            let endpoint = format!("/_api/index?collection={}", collection.name);

            if let Ok(response) = self.execute::<IndexListResponse>(Method::GET, &endpoint, None) {
                for idx in response.indexes {
                    if let Some(idx_name) = &idx.name {
                        if idx_name == name {
                            if let Some(idx_id) = &idx.id {
                                let delete_endpoint = format!("/_api/index/{idx_id}");
                                let _: Value =
                                    self.execute(Method::DELETE, &delete_endpoint, None)?;
                                return Ok(());
                            }
                        }
                    }
                }
            }
        }

        Err(GraphError::InternalError(format!(
            "Index '{name}' not found"
        )))
    }

    pub fn list_indexes(&self) -> Result<Vec<IndexDefinition>, GraphError> {
        trace!("List indexes");
        let collections = self.list_collections()?;
        let mut all_indexes = Vec::new();

        for collection in collections {
            let endpoint = format!("/_api/index?collection={}", collection.name);

            match self.execute::<IndexListResponse>(Method::GET, &endpoint, None) {
                Ok(response) => {
                    for index in response.indexes {
                        if index.index_type == "primary" || index.index_type == "edge" {
                            continue;
                        }

                        if index.fields.is_empty() {
                            continue;
                        }

                        let unique = index.unique.unwrap_or(false);
                        let index_type = match index.index_type.as_str() {
                            "geo" => golem_graph::golem::graph::schema::IndexType::Geospatial,
                            "inverted" => golem_graph::golem::graph::schema::IndexType::Text,
                            _ => golem_graph::golem::graph::schema::IndexType::Exact,
                        };

                        let logical_name = if index.fields.len() == 1 {
                            format!("idx_{}_{}", collection.name, index.fields[0])
                        } else {
                            format!("idx_{}_{}", collection.name, index.fields.join("_"))
                        };

                        let final_name = index.name.or(index.id).unwrap_or(logical_name);

                        all_indexes.push(IndexDefinition {
                            name: final_name,
                            label: collection.name.clone(),
                            container: Some(collection.name.clone()),
                            properties: index.fields,
                            unique,
                            index_type,
                        });
                    }
                }
                Err(_) => {
                    continue;
                }
            }
        }

        Ok(all_indexes)
    }

    pub fn get_index(&self, name: &str) -> Result<Option<IndexDefinition>, GraphError> {
        trace!("Get index: {name}");
        let all_indexes = self.list_indexes()?;

        if let Some(index) = all_indexes.iter().find(|idx| idx.name == name) {
            return Ok(Some(index.clone()));
        }

        if name.starts_with("idx_") {
            let parts: Vec<&str> = name.split('_').collect();
            if parts.len() >= 3 {
                let collection_part = parts[1];
                let field_part = parts[2..].join("_");

                if let Some(index) = all_indexes.iter().find(|idx| {
                    idx.label == collection_part
                        && idx.properties.len() == 1
                        && idx.properties[0] == field_part
                }) {
                    return Ok(Some(index.clone()));
                }
            }
        }

        Ok(None)
    }

    pub fn define_edge_type(&self, definition: EdgeTypeDefinition) -> Result<(), GraphError> {
        trace!("Define edge type: {definition:?}");
        self.create_collection(&definition.collection, ContainerType::EdgeContainer)?;
        // Note: ArangoDB doesn't enforce from/to collection constraints like some other graph databases
        // The constraints in EdgeTypeDefinition are mainly for application-level validation
        Ok(())
    }

    pub fn list_edge_types(&self) -> Result<Vec<EdgeTypeDefinition>, GraphError> {
        trace!("List edge types");
        // In ArangoDB, we return edge collections as edge types
        // Since ArangoDB doesn't enforce from/to constraints at the DB level,
        // we return edge collections with empty from/to collections
        let collections = self.list_collections()?;
        let edge_types = collections
            .into_iter()
            .filter(|c| matches!(c.container_type, ContainerType::EdgeContainer))
            .map(|c| EdgeTypeDefinition {
                collection: c.name,
                from_collections: vec![],
                to_collections: vec![], // ArangoDB doesn't store these constraints
            })
            .collect();
        Ok(edge_types)
    }

    pub fn get_transaction_status(&self, transaction_id: &str) -> Result<String, GraphError> {
        trace!("Get transaction status: {transaction_id}");
        let endpoint = format!("/_api/transaction/{transaction_id}");
        let response: TransactionStatusResponse = self.execute(Method::GET, &endpoint, None)?;
        Ok(response.status)
    }

    pub fn get_database_statistics(&self) -> Result<DatabaseStatistics, GraphError> {
        trace!("Get database statistics");
        let collections: Vec<CollectionInfoShort> =
            self.execute(Method::GET, "/_api/collection?excludeSystem=true", None)?;

        let mut total_vertex_count = 0;
        let mut total_edge_count = 0;

        for collection_info in collections {
            let properties_endpoint =
                format!("/_api/collection/{}/properties", collection_info.name);
            let properties: CollectionPropertiesResponse =
                self.execute(Method::GET, &properties_endpoint, None)?;

            if properties.collection_type == ArangoCollectionType::Edge {
                total_edge_count += properties.count;
            } else {
                total_vertex_count += properties.count;
            }
        }

        Ok(DatabaseStatistics {
            vertex_count: total_vertex_count,
            edge_count: total_edge_count,
        })
    }

    #[allow(dead_code)]
    pub fn execute_query(&self, query: Value) -> Result<Value, GraphError> {
        trace!("Execute query");
        self.execute(Method::POST, "/_api/cursor", Some(&query))
    }

    pub fn begin_dynamic_transaction(&self, read_only: bool) -> Result<String, GraphError> {
        trace!("Begin dynamic transaction (read_only={read_only})");

        let existing_collections = self.list_collections().unwrap_or_default();
        let all_collections: Vec<String> = existing_collections
            .iter()
            .map(|c| c.name.clone())
            .collect();

        let collections = if read_only {
            json!({ "read": all_collections })
        } else {
            json!({ "write": all_collections })
        };

        let body = json!({ "collections": collections });
        let result: TransactionBeginResponse =
            self.execute(Method::POST, "/_api/transaction/begin", Some(&body))?;
        Ok(result.id)
    }
}

#[derive(serde::Deserialize, Debug)]
pub struct ArangoErrorResponse {
    #[serde(rename = "errorMessage")]
    pub error_message: Option<String>,
    #[serde(rename = "errorNum")]
    pub error_num: Option<i64>,
    #[serde(rename = "_id")]
    pub id: Option<String>,
    #[serde(rename = "_key")]
    pub key: Option<String>,
}

#[derive(serde::Deserialize, Debug)]
struct TransactionBeginResponse {
    pub id: String,
}

#[derive(serde::Deserialize, Debug)]
struct TransactionStatusResponse {
    #[serde(rename = "id")]
    _id: String,
    status: String,
}

#[derive(serde::Deserialize, Debug)]
pub struct DatabaseStatistics {
    pub vertex_count: u64,
    pub edge_count: u64,
}

#[derive(serde::Deserialize, Debug)]
struct CollectionInfoShort {
    name: String,
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct CollectionInfo {
    pub name: String,
    #[serde(rename = "type")]
    pub collection_type: u8,
    #[serde(rename = "isSystem")]
    pub is_system: bool,
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct CollectionPropertiesResponse {
    count: u64,
    #[serde(rename = "type")]
    collection_type: ArangoCollectionType,
}

#[derive(serde::Deserialize, Debug)]
struct IndexListResponse {
    pub indexes: Vec<IndexInfo>,
}

#[derive(serde::Deserialize, Debug)]
struct IndexInfo {
    pub id: Option<String>,
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub index_type: String,
    pub fields: Vec<String>,
    pub unique: Option<bool>,
}

#[derive(Debug, PartialEq)]
enum ArangoCollectionType {
    Document,
    Edge,
    Unknown(u8),
}

impl From<u8> for ArangoCollectionType {
    fn from(value: u8) -> Self {
        match value {
            2 => ArangoCollectionType::Document,
            3 => ArangoCollectionType::Edge,
            _ => ArangoCollectionType::Unknown(value),
        }
    }
}

impl<'de> serde::Deserialize<'de> for ArangoCollectionType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = u8::deserialize(deserializer)?;
        Ok(ArangoCollectionType::from(value))
    }
}

pub fn from_arangodb_error_code(error_code: i64, message: &str) -> GraphError {
    match error_code {
        // document/element errors (1200-1299)
        1202 => GraphError::InternalError(format!("Document not found: {message}")),
        1210 => GraphError::ConstraintViolation(format!("Unique constraint violated: {message}")),
        1213 => GraphError::SchemaViolation(format!("Collection not found: {message}")),
        1218 => GraphError::SchemaViolation(format!("Document handle bad: {message}")),
        1221 => GraphError::InvalidPropertyType(format!("Illegal document key: {message}")),
        1229 => GraphError::ConstraintViolation(format!("Document key missing: {message}")),
        1232 => GraphError::InternalError(format!("Document not found in collection: {message}")),
        1233 => GraphError::ConstraintViolation(format!("Collection read-only: {message}")),
        1234 => GraphError::ConstraintViolation(format!("Document key exists: {message}")),

        // query errors (1500-1599)
        1501 => GraphError::InvalidQuery(format!("Query parse error: {message}")),
        1502 => GraphError::InvalidQuery(format!("Query empty: {message}")),
        1503 => GraphError::InvalidQuery(format!("Query runtime error: {message}")),
        1504 => GraphError::InvalidQuery(format!("Query number out of range: {message}")),
        1505 => GraphError::InvalidQuery(format!("Query geo index violation: {message}")),
        1510 => GraphError::InvalidQuery(format!("Query fulltext index missing: {message}")),
        1521 => GraphError::InvalidQuery(format!("AQL function not found: {message}")),
        1522 => {
            GraphError::InvalidQuery(format!("AQL function argument number mismatch: {message}"))
        }
        1540 => GraphError::InvalidPropertyType(format!("Invalid bind parameter type: {message}")),
        1541 => GraphError::InvalidQuery(format!("No bind parameter value: {message}")),
        1562 => GraphError::InvalidQuery(format!("Variable already declared: {message}")),
        1563 => GraphError::InvalidQuery(format!("Variable not declared: {message}")),
        1570 => GraphError::InvalidQuery(format!("Query killed: {message}")),
        1579 => GraphError::Timeout,
        1580 => GraphError::InvalidQuery(format!("Query warning: {message}")),

        // transaction errors (1650-1699)
        1650 => GraphError::TransactionFailed(format!("Transaction not found: {message}")),
        1651 => GraphError::TransactionFailed(format!("Transaction already started: {message}")),
        1652 => GraphError::TransactionFailed(format!("Transaction not started: {message}")),
        1653 => GraphError::TransactionFailed(format!(
            "Transaction already committed/aborted: {message}"
        )),
        1654 => GraphError::TransactionFailed(format!("Transaction nested: {message}")),
        1655 => GraphError::TransactionTimeout,
        1656 => GraphError::DeadlockDetected,
        1658 => GraphError::TransactionConflict,
        1659 => GraphError::TransactionFailed(format!("Transaction internal: {message}")),
        1660 => {
            GraphError::TransactionFailed(format!("Transaction unregistered collection: {message}"))
        }
        1661 => {
            GraphError::TransactionFailed(format!("Transaction disallowed operation: {message}"))
        }

        // schema/collection errors
        1207 => GraphError::SchemaViolation(format!("Collection must be unloaded: {message}")),
        1228 => GraphError::SchemaViolation(format!("Document revision bad: {message}")),
        1220 => GraphError::ConstraintViolation(format!("Conflict: {message}")),
        1200 => GraphError::InternalError(format!("Arango error: {message}")),
        1203 => GraphError::SchemaViolation(format!("Collection name invalid: {message}")),
        1208 => GraphError::SchemaViolation(format!("Collection corrupted: {message}")),

        // Index errors (1201, 1204-1206, etc.)
        1204 => GraphError::SchemaViolation(format!("Collection can't be dropped: {message}")),
        1205 => GraphError::SchemaViolation(format!("Collection can't be renamed: {message}")),
        1206 => GraphError::SchemaViolation(format!("Collection needs to be loaded: {message}")),
        1212 => {
            GraphError::SchemaViolation(format!("Cross-collection request forbidden: {message}"))
        }
        1230 => GraphError::SchemaViolation(format!("Datafile sealed: {message}")),

        // resource errors
        32 => GraphError::ResourceExhausted(format!("Out of memory: {message}")),
        1104 => GraphError::ResourceExhausted(format!("Collection full: {message}")),
        1105 => GraphError::ResourceExhausted(format!("Collection empty: {message}")),

        // Cluster/replication errors
        1447 => GraphError::ServiceUnavailable(format!("Cluster backend unavailable: {message}")),
        1448 => GraphError::TransactionConflict,
        1449 => GraphError::ServiceUnavailable(format!("Cluster coordinator error: {message}")),
        1450 => GraphError::ServiceUnavailable(format!("Cluster reading plan agency: {message}")),
        1451 => GraphError::ServiceUnavailable(format!(
            "Cluster could not create collection in plan: {message}"
        )),
        1452 => GraphError::ServiceUnavailable(format!(
            "Cluster could not read current collection: {message}"
        )),
        1453 => GraphError::ServiceUnavailable(format!(
            "Cluster could not create collection: {message}"
        )),
        1454 => GraphError::Timeout,
        1455 => GraphError::ServiceUnavailable(format!(
            "Cluster leadership challenge ongoing: {message}"
        )),

        // Authentication and authorization errors (11xx)
        1100 => GraphError::AuthenticationFailed(format!("Forbidden: {message}")),
        1401 => GraphError::AuthenticationFailed(format!("Authentication required: {message}")),
        1402 => GraphError::AuthenticationFailed(format!("Database name missing: {message}")),
        1403 => GraphError::AuthenticationFailed(format!("User name missing: {message}")),
        1404 => GraphError::AuthenticationFailed(format!("Password missing: {message}")),
        1405 => GraphError::AuthorizationFailed(format!("Invalid password: {message}")),
        1406 => GraphError::AuthorizationFailed(format!("User active: {message}")),
        1407 => GraphError::AuthorizationFailed(format!("User not found: {message}")),
        1410 => GraphError::AuthorizationFailed(format!("User duplicate: {message}")),
        1430 => GraphError::AuthorizationFailed(format!("Insufficient rights: {message}")),

        // Graph specific errors (1901-1999)
        1901 => GraphError::SchemaViolation(format!("Graph invalid graph: {message}")),
        1902 => GraphError::SchemaViolation(format!("Graph could not create graph: {message}")),
        1903 => GraphError::SchemaViolation(format!("Graph invalid vertex: {message}")),
        1904 => GraphError::SchemaViolation(format!("Graph could not create vertex: {message}")),
        1905 => GraphError::SchemaViolation(format!("Graph invalid edge: {message}")),
        1906 => GraphError::SchemaViolation(format!("Graph could not create edge: {message}")),
        1907 => GraphError::SchemaViolation(format!("Graph too many iterations: {message}")),
        1908 => GraphError::SchemaViolation(format!("Graph invalid filter result: {message}")),
        1909 => GraphError::SchemaViolation(format!("Graph collection multi use: {message}")),
        1910 => GraphError::SchemaViolation(format!("Graph edge collection not used: {message}")),
        1920 => GraphError::InvalidQuery(format!("Graph edge col does not exist: {message}")),
        1921 => GraphError::InvalidQuery(format!("Graph wrong collection type edge: {message}")),
        1922 => GraphError::InvalidQuery(format!("Graph not found: {message}")),
        1924 => GraphError::InvalidQuery(format!("Graph vertex col does not exist: {message}")),
        1925 => GraphError::InvalidQuery(format!("Graph wrong collection type vertex: {message}")),

        _ => GraphError::InternalError(format!("ArangoDB error [{error_code}]: {message}")),
    }
}

fn map_arangodb_http_status(
    status: u16,
    message: &str,
    error_body: &ArangoErrorResponse,
) -> GraphError {
    match status {
        // Authentication and Authorization
        401 => {
            GraphError::AuthenticationFailed(format!("ArangoDB authentication failed: {message}"))
        }
        403 => GraphError::AuthorizationFailed(format!("ArangoDB authorization failed: {message}")),

        // Client errors specific to ArangoDB context
        400 => GraphError::InvalidQuery(format!("ArangoDB bad request: {message}")),
        404 => GraphError::ServiceUnavailable(format!("ArangoDB resource not found: {message}")),
        409 => GraphError::TransactionConflict,
        412 => GraphError::ConstraintViolation(format!("ArangoDB precondition failed: {message}")),
        413 => GraphError::ResourceExhausted(format!("ArangoDB request too large: {message}")),
        422 => GraphError::SchemaViolation(format!("ArangoDB unprocessable entity: {message}")),
        429 => GraphError::ResourceExhausted(format!("ArangoDB rate limit exceeded: {message}")),

        // Server errors
        500 => GraphError::InternalError(format!("ArangoDB internal server error: {message}")),
        502 => GraphError::ServiceUnavailable(format!("ArangoDB bad gateway: {message}")),
        503 => GraphError::ServiceUnavailable(format!("ArangoDB service unavailable: {message}")),
        504 => GraphError::Timeout,
        507 => GraphError::ResourceExhausted(format!("ArangoDB insufficient storage: {message}")),

        _ => {
            let debug_info = format!(
                "ArangoDB HTTP error [{}]: {} | Error code: {:?}",
                status, message, error_body.error_num
            );
            GraphError::InternalError(debug_info)
        }
    }
}
