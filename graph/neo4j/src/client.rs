use base64::{engine::general_purpose::STANDARD, Engine as _};
use golem_graph::golem::graph::errors::GraphError;
use log::trace;
use reqwest::{Client, Response};
use serde_json::Value;

#[derive(Clone)]
pub(crate) struct Neo4jApi {
    base_url: String,
    database: String,
    auth_header: String,
    client: Client,
}

impl Neo4jApi {
    pub(crate) fn new(
        host: &str,
        port: u16,
        database: &str,
        username: &str,
        password: &str,
    ) -> Self {
        trace!("Initializing Neo4jApi for host: {host}, port: {port}, database: {database}");
        let base_url = format!("http://{host}:{port}");
        let auth = format!("{username}:{password}");
        let auth_header = format!("Basic {}", STANDARD.encode(auth.as_bytes()));
        let client = Client::builder()
            .build()
            .expect("Failed to initialize HTTP client");

        Neo4jApi {
            base_url,
            database: database.to_string(),
            auth_header,
            client,
        }
    }

    fn tx_endpoint(&self) -> String {
        format!("/db/{}/tx", self.database)
    }

    fn handle_neo4j_reqwest_error(&self, details: &str, err: reqwest::Error) -> GraphError {
        if err.is_timeout() {
            return GraphError::Timeout;
        }

        if err.is_request() {
            return GraphError::ConnectionFailed(format!(
                "Neo4j request failed ({details}): {err}"
            ));
        }

        if err.is_decode() {
            return GraphError::InternalError(format!(
                "Neo4j response decode failed ({details}): {err}"
            ));
        }

        if err.is_status() {
            if let Some(status) = err.status() {
                let error_msg = format!(
                    "Neo4j HTTP error {} ({}): {}",
                    status.as_u16(),
                    details,
                    err
                );
                return Self::map_neo4j_http_status(
                    status.as_u16(),
                    &error_msg,
                    &serde_json::Value::Null,
                );
            }
        }

        GraphError::InternalError(format!("Neo4j request error ({details}): {err}"))
    }

    pub(crate) fn begin_transaction(&self) -> Result<String, GraphError> {
        trace!("Begin Neo4j transaction for database: {}", self.database);
        let url = format!("{}{}", self.base_url, self.tx_endpoint());
        let resp = self
            .client
            .post(&url)
            .header("Authorization", &self.auth_header)
            .send()
            .map_err(|e| self.handle_neo4j_reqwest_error("Neo4j begin transaction failed", e))?;
        Self::ensure_success_and_get_location(resp)
    }

    pub(crate) fn execute_in_transaction(
        &self,
        tx_url: &str,
        statements: Value,
    ) -> Result<Value, GraphError> {
        trace!("Execute in Neo4j transaction: {tx_url}");
        trace!("[Neo4jApi] Cypher request: {statements}");
        let resp = self
            .client
            .post(tx_url)
            .header("Authorization", &self.auth_header)
            .header("Content-Type", "application/json")
            .body(statements.to_string())
            .send()
            .map_err(|e| {
                self.handle_neo4j_reqwest_error("Neo4j execute in transaction failed", e)
            })?;
        let json = Self::ensure_success_and_json(resp)?;
        trace!("[Neo4jApi] Cypher response: {json}");
        Ok(json)
    }

    pub(crate) fn commit_transaction(&self, tx_url: &str) -> Result<(), GraphError> {
        trace!("Commit Neo4j transaction: {tx_url}");
        let commit_url = format!("{tx_url}/commit");
        let resp = self
            .client
            .post(&commit_url)
            .header("Authorization", &self.auth_header)
            .send()
            .map_err(|e| self.handle_neo4j_reqwest_error("Neo4j commit transaction failed", e))?;
        Self::ensure_success(resp).map(|_| ())
    }

    pub(crate) fn rollback_transaction(&self, tx_url: &str) -> Result<(), GraphError> {
        trace!("Rollback Neo4j transaction: {tx_url}");
        let resp = self
            .client
            .delete(tx_url)
            .header("Authorization", &self.auth_header)
            .send()
            .map_err(|e| self.handle_neo4j_reqwest_error("Neo4j rollback transaction failed", e))?;
        Self::ensure_success(resp).map(|_| ())
    }

    // Helpers

    fn ensure_success(response: Response) -> Result<Response, GraphError> {
        if response.status().is_success() {
            Ok(response)
        } else {
            let status_code = response.status().as_u16();
            let text = response.text().map_err(|e| {
                GraphError::InternalError(format!("Failed to read Neo4j response body: {e}"))
            })?;
            let error_body: Value = serde_json::from_str(&text)
                .unwrap_or_else(|_| serde_json::json!({"message": text, "raw_body": text}));

            let error_msg = error_body
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown Neo4j error");

            Err(Self::map_neo4j_error(status_code, error_msg, &error_body))
        }
    }

    fn ensure_success_and_json(response: Response) -> Result<Value, GraphError> {
        if response.status().is_success() {
            response.json().map_err(|e| {
                GraphError::InternalError(format!("Failed to parse Neo4j response JSON: {e}"))
            })
        } else {
            let status_code = response.status().as_u16();
            let text = response.text().map_err(|e| {
                GraphError::InternalError(format!("Failed to read Neo4j response body: {e}"))
            })?;
            let error_body: Value = serde_json::from_str(&text)
                .unwrap_or_else(|_| serde_json::json!({"message": text, "raw_body": text}));

            let error_msg = error_body
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown Neo4j error");

            Err(Self::map_neo4j_error(status_code, error_msg, &error_body))
        }
    }

    fn ensure_success_and_get_location(response: Response) -> Result<String, GraphError> {
        if response.status().is_success() {
            response
                .headers()
                .get("Location")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string())
                .ok_or_else(|| GraphError::InternalError("Missing Location header".into()))
        } else {
            let status_code = response.status().as_u16();
            let text = response.text().map_err(|e| {
                GraphError::InternalError(format!("Failed to read Neo4j response body: {e}"))
            })?;
            let error_body: Value = serde_json::from_str(&text)
                .unwrap_or_else(|_| serde_json::json!({"message": text, "raw_body": text}));

            let error_msg = error_body
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown Neo4j error");

            Err(Self::map_neo4j_error(status_code, error_msg, &error_body))
        }
    }

    fn map_neo4j_error(status_code: u16, message: &str, error_body: &Value) -> GraphError {
        if let Some(errors) = error_body.get("errors").and_then(|e| e.as_array()) {
            if let Some(first_error) = errors.first() {
                if let Some(neo4j_code) = first_error.get("code").and_then(|c| c.as_str()) {
                    if let Some(neo4j_message) = first_error.get("message").and_then(|m| m.as_str())
                    {
                        return Self::from_neo4j_error_code(neo4j_code, neo4j_message, error_body);
                    }
                }
            }
        }

        let enhanced_error_body = if error_body.get("raw_body").is_some() {
            error_body.clone()
        } else {
            let mut enhanced = error_body.clone();
            enhanced["debug_info"] = serde_json::json!({
                "original_message": message,
                "status_code": status_code,
                "error_body_sample": error_body.to_string().chars().take(200).collect::<String>()
            });
            enhanced
        };

        Self::map_neo4j_http_status(status_code, message, &enhanced_error_body)
    }

    fn from_neo4j_error_code(code: &str, message: &str, error_body: &Value) -> GraphError {
        match code {
            //  authentication and authorization
            "Neo.ClientError.Security.Unauthorized" => {
                GraphError::AuthenticationFailed(format!("Neo4j authentication failed: {message}"))
            }
            "Neo.ClientError.Security.Forbidden" => {
                GraphError::AuthorizationFailed(format!("Neo4j authorization failed: {message}"))
            }
            "Neo.ClientError.Security.AuthenticationRateLimit" => GraphError::ResourceExhausted(
                format!("Authentication rate limit exceeded: {message}"),
            ),
            "Neo.ClientError.Security.CredentialsExpired" => {
                GraphError::AuthenticationFailed(format!("Neo4j credentials expired: {message}"))
            }
            "Neo.ClientError.Security.TokenExpired" => {
                GraphError::AuthenticationFailed(format!("Neo4j token expired: {message}"))
            }

            // request issues
            "Neo.ClientError.Request.Invalid" => {
                GraphError::InvalidQuery(format!("Invalid Neo4j request: {message}"))
            }
            "Neo.ClientError.Request.InvalidFormat" => {
                GraphError::InvalidQuery(format!("Invalid request format: {message}"))
            }
            "Neo.ClientError.Request.InvalidUsage" => {
                GraphError::InvalidQuery(format!("Invalid usage: {message}"))
            }

            // statement/query issues
            "Neo.ClientError.Statement.SyntaxError" => {
                GraphError::InvalidQuery(format!("Cypher syntax error: {message}"))
            }
            "Neo.ClientError.Statement.SemanticError" => {
                GraphError::InvalidQuery(format!("Cypher semantic error: {message}"))
            }
            "Neo.ClientError.Statement.ParameterMissing" => {
                GraphError::InvalidQuery(format!("Missing parameter: {message}"))
            }
            "Neo.ClientError.Statement.TypeError" => {
                GraphError::InvalidPropertyType(format!("Type error: {message}"))
            }
            "Neo.ClientError.Statement.ArgumentError" => {
                GraphError::InvalidQuery(format!("Argument error: {message}"))
            }
            "Neo.ClientError.Statement.EntityNotFound" => {
                if let Some(element_id) =
                    golem_graph::error::mapping::extract_element_id_from_message(message)
                {
                    GraphError::ElementNotFound(element_id)
                } else {
                    GraphError::InternalError(format!("Entity not found: {message}"))
                }
            }

            // schema and constraints
            "Neo.ClientError.Schema.ConstraintValidationFailed" => {
                GraphError::ConstraintViolation(format!("Constraint validation failed: {message}"))
            }
            "Neo.ClientError.Schema.ConstraintViolation" => {
                GraphError::ConstraintViolation(format!("Constraint violation: {message}"))
            }
            "Neo.ClientError.Schema.ConstraintAlreadyExists" => {
                GraphError::ConstraintViolation(format!("Constraint already exists: {message}"))
            }
            "Neo.ClientError.Schema.IndexNotFound" => {
                GraphError::SchemaViolation(format!("Index not found: {message}"))
            }
            "Neo.ClientError.Schema.IndexAlreadyExists" => {
                GraphError::SchemaViolation(format!("Index already exists: {message}"))
            }
            "Neo.ClientError.Schema.LabelNotFound" => {
                GraphError::SchemaViolation(format!("Label not found: {message}"))
            }
            "Neo.ClientError.Schema.PropertyKeyNotFound" => {
                GraphError::SchemaViolation(format!("Property key not found: {message}"))
            }
            "Neo.ClientError.Schema.RelationshipTypeNotFound" => {
                GraphError::SchemaViolation(format!("Relationship type not found: {message}"))
            }

            // procedure issues
            "Neo.ClientError.Procedure.ProcedureNotFound" => {
                GraphError::InvalidQuery(format!("Procedure not found: {message}"))
            }
            "Neo.ClientError.Procedure.ProcedureCallFailed" => {
                GraphError::InvalidQuery(format!("Procedure call failed: {message}"))
            }
            "Neo.ClientError.Procedure.TypeError" => {
                GraphError::InvalidPropertyType(format!("Procedure type error: {message}"))
            }

            // transaction issues
            "Neo.ClientError.Transaction.InvalidType" => {
                GraphError::TransactionFailed(format!("Invalid transaction type: {message}"))
            }
            "Neo.ClientError.Transaction.ForbiddenDueToTransactionType" => {
                GraphError::TransactionFailed(format!(
                    "Operation forbidden in transaction: {message}"
                ))
            }
            "Neo.ClientError.Transaction.MarkedAsFailed" => {
                GraphError::TransactionFailed(format!("Transaction marked as failed: {message}"))
            }
            "Neo.ClientError.Transaction.InvalidBookmark" => {
                GraphError::TransactionFailed(format!("Invalid bookmark: {message}"))
            }
            "Neo.ClientError.Transaction.BookmarkTimeout" => GraphError::TransactionTimeout,

            // transient errors - database issues
            "Neo.TransientError.Database.DatabaseUnavailable" => {
                GraphError::ServiceUnavailable(format!("Database unavailable: {message}"))
            }
            "Neo.TransientError.General.DatabaseUnavailable" => {
                GraphError::ServiceUnavailable(format!("General database unavailable: {message}"))
            }
            "Neo.TransientError.Network.UnknownFailure" => {
                GraphError::ConnectionFailed(format!("Network failure: {message}"))
            }

            // transient errors - transaction issues
            "Neo.TransientError.Transaction.DeadlockDetected" => GraphError::DeadlockDetected,
            "Neo.TransientError.Transaction.LockClientStopped" => GraphError::TransactionConflict,
            "Neo.TransientError.Transaction.Terminated" => {
                GraphError::TransactionFailed(format!("Transaction terminated: {message}"))
            }
            "Neo.TransientError.Transaction.LockWaitTimeout" => GraphError::TransactionTimeout,
            "Neo.TransientError.Transaction.ConstraintsChanged" => GraphError::TransactionConflict,

            // database errors -general
            "Neo.DatabaseError.General.UnknownError" => {
                GraphError::InternalError(format!("Neo4j unknown error: {message}"))
            }
            "Neo.DatabaseError.General.CorruptSchemaRule" => {
                GraphError::SchemaViolation(format!("Corrupt schema rule: {message}"))
            }
            "Neo.DatabaseError.Statement.ExecutionFailed" => {
                GraphError::InternalError(format!("Statement execution failed: {message}"))
            }

            // database errors - schema
            "Neo.DatabaseError.Schema.ConstraintCreationFailed" => {
                GraphError::SchemaViolation(format!("Constraint creation failed: {message}"))
            }
            "Neo.DatabaseError.Schema.IndexCreationFailed" => {
                GraphError::SchemaViolation(format!("Index creation failed: {message}"))
            }
            "Neo.DatabaseError.Schema.SchemaRuleNotFound" => {
                GraphError::SchemaViolation(format!("Schema rule not found: {message}"))
            }

            // database errors - transaction
            "Neo.DatabaseError.Transaction.TransactionLogError" => {
                GraphError::TransactionFailed(format!("Transaction log error: {message}"))
            }
            "Neo.DatabaseError.Transaction.TransactionValidationFailed" => {
                GraphError::TransactionFailed(format!("Transaction validation failed: {message}"))
            }
            "Neo.DatabaseError.Transaction.TransactionCommitFailed" => {
                GraphError::TransactionFailed(format!("Transaction commit failed: {message}"))
            }

            _ => {
                let enhanced_message = format!("Neo4j error [{code}]: {message}");
                let mut debug_error_body = error_body.clone();
                debug_error_body["neo4j_error_code"] = serde_json::Value::String(code.to_string());
                debug_error_body["neo4j_message"] = serde_json::Value::String(message.to_string());

                GraphError::InternalError(format!(
                    "{} | Debug info: {}",
                    enhanced_message,
                    debug_error_body
                        .to_string()
                        .chars()
                        .take(300)
                        .collect::<String>()
                ))
            }
        }
    }

    fn map_neo4j_http_status(
        status: u16,
        message: &str,
        error_body: &serde_json::Value,
    ) -> GraphError {
        match status {
            // Authentication and Authorization
            401 => {
                GraphError::AuthenticationFailed(format!("Neo4j authentication failed: {message}"))
            }
            403 => {
                GraphError::AuthorizationFailed(format!("Neo4j authorization failed: {message}"))
            }

            // Client errors specific to Neo4j context
            400 => GraphError::InvalidQuery(format!("Neo4j bad request: {message}")),
            404 => GraphError::ServiceUnavailable(format!("Neo4j resource not found: {message}")),
            409 => GraphError::TransactionConflict,
            412 => GraphError::ConstraintViolation(format!("Neo4j precondition failed: {message}")),
            413 => GraphError::ResourceExhausted(format!("Neo4j request too large: {message}")),
            422 => GraphError::InvalidQuery(format!("Neo4j unprocessable entity: {message}")),
            429 => GraphError::ResourceExhausted(format!("Neo4j rate limit exceeded: {message}")),

            // Server errors
            500 => GraphError::InternalError(format!("Neo4j internal server error: {message}")),
            502 => GraphError::ServiceUnavailable(format!("Neo4j bad gateway: {message}")),
            503 => GraphError::ServiceUnavailable(format!("Neo4j service unavailable: {message}")),
            504 => GraphError::Timeout,
            507 => GraphError::ResourceExhausted(format!("Neo4j insufficient storage: {message}")),

            _ => {
                let debug_info = format!(
                    "Neo4j HTTP error [{}]: {} | Error body sample: {}",
                    status,
                    message,
                    error_body.to_string().chars().take(200).collect::<String>()
                );
                GraphError::InternalError(debug_info)
            }
        }
    }
}
