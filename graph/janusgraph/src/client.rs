use golem_graph::golem::graph::errors::GraphError;
use log::trace;
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Deserialize, Serialize, Debug)]
pub struct GremlinResponse {
    #[serde(rename = "requestId")]
    pub request_id: Option<String>,
    pub status: Option<GremlinStatus>,
    pub result: Option<GremlinResult>,
    pub exceptions: Option<Vec<Value>>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct GremlinStatus {
    pub message: Option<String>,
    pub code: Option<u16>,
    pub attributes: Option<HashMap<String, Value>>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct GremlinResult {
    pub data: Option<Value>,
    pub meta: Option<HashMap<String, Value>>,
}

#[derive(Deserialize, Debug)]
pub struct GremlinErrorResponse {
    pub message: Option<String>,
    pub status: Option<GremlinStatus>,
    pub result: Option<GremlinErrorResult>,
    pub exceptions: Option<Vec<String>>,
}

#[derive(Deserialize, Debug)]
pub struct GremlinErrorResult {
    pub data: Option<GremlinErrorData>,
}

#[derive(Deserialize, Debug)]
pub struct GremlinErrorData {
    #[serde(rename = "@type")]
    pub at_type: Option<String>,
    #[serde(rename = "detailedMessage")]
    pub detailed_message: Option<String>,
    #[serde(rename = "java.lang.Class")]
    pub java_class: Option<String>,
    #[serde(rename = "exceptionType")]
    pub exception_type: Option<String>,
    #[serde(rename = "stackTrace")]
    pub stack_trace: Option<String>,
    #[serde(rename = "responseStatusCode")]
    pub response_status_code: Option<u64>,
    pub message: Option<String>,
}

#[derive(Serialize, Debug)]
struct GremlinRequest {
    gremlin: String,
    bindings: Value,
    session: String,
    processor: String,
    op: String,
}

#[derive(Serialize, Debug)]
struct GremlinCloseRequest {
    session: String,
    op: String,
    processor: String,
}

#[derive(Clone)]
pub struct JanusGraphApi {
    endpoint: String,
    client: Client,
    session_id: String,
}

impl JanusGraphApi {
    pub fn new(
        host: &str,
        port: u16,
        _username: Option<&str>,
        _password: Option<&str>,
    ) -> Result<Self, GraphError> {
        trace!("Initializing JanusGraphApi for host: {host}, port: {port}");
        let endpoint = format!("http://{host}:{port}/gremlin");
        let client = Client::builder()
            .build()
            .expect("Failed to initialize HTTP client");
        let session_id = Uuid::new_v4().to_string();
        Ok(JanusGraphApi {
            endpoint,
            client,
            session_id,
        })
    }

    pub fn new_with_session(
        host: &str,
        port: u16,
        _username: Option<&str>,
        _password: Option<&str>,
        session_id: String,
    ) -> Result<Self, GraphError> {
        trace!(
            "Initializing JanusGraphApi with session for host: {host}, port: {port}, session_id: {session_id}"
        );
        let endpoint = format!("http://{host}:{port}/gremlin");
        let client = Client::builder()
            .build()
            .expect("Failed to initialize HTTP client");
        Ok(JanusGraphApi {
            endpoint,
            client,
            session_id,
        })
    }

    pub fn commit(&self) -> Result<(), GraphError> {
        trace!("Commit transaction");
        self.execute("g.tx().commit()", None)?;
        self.execute("g.tx().open()", None)?;
        Ok(())
    }

    pub fn rollback(&self) -> Result<(), GraphError> {
        trace!("Rollback transaction");
        self.execute("g.tx().rollback()", None)?;
        self.execute("g.tx().open()", None)?;
        Ok(())
    }

    pub fn execute(&self, gremlin: &str, bindings: Option<Value>) -> Result<Value, GraphError> {
        trace!("Execute Gremlin query: {gremlin}");
        let bindings = bindings.unwrap_or_else(|| json!({}));
        let request = GremlinRequest {
            gremlin: gremlin.to_string(),
            bindings,
            session: self.session_id.clone(),
            processor: "session".to_string(),
            op: "eval".to_string(),
        };

        trace!("[JanusGraphApi] DEBUG - Full request details:");
        trace!("[JanusGraphApi] Endpoint: {}", self.endpoint);
        trace!("[JanusGraphApi] Session ID: {}", self.session_id);
        trace!("[JanusGraphApi] Gremlin Query: {gremlin}");

        let body_string = serde_json::to_string(&request).map_err(|e| {
            GraphError::InternalError(format!("Failed to serialize request body: {e}"))
        })?;

        trace!(
            "[JanusGraphApi] Request Body: {}",
            serde_json::to_string_pretty(&request)
                .unwrap_or_else(|_| "Failed to serialize".to_string())
        );

        trace!(
            "[JanusGraphApi] Sending POST request to: {} with body length: {}",
            self.endpoint,
            body_string.len()
        );
        let response = self
            .client
            .post(&self.endpoint)
            .header("Content-Type", "application/json")
            .header("Content-Length", body_string.len().to_string())
            .body(body_string)
            .send()
            .map_err(|e| {
                log::error!("[JanusGraphApi] ERROR - Request failed: {e}");
                self.handle_janusgraph_reqwest_error("JanusGraph request failed", e)
            })?;

        log::info!(
            "[JanusGraphApi] Got response with status: {}",
            response.status()
        );
        Self::handle_response(response)
    }

    fn _read(&self, gremlin: &str, bindings: Option<Value>) -> Result<Value, GraphError> {
        trace!("Read Gremlin query: {gremlin}");
        let bindings = bindings.unwrap_or_else(|| json!({}));
        let request = GremlinRequest {
            gremlin: gremlin.to_string(),
            bindings,
            session: String::new(),
            processor: "".to_string(),
            op: "eval".to_string(),
        };

        let body_string = serde_json::to_string(&request).map_err(|e| {
            GraphError::InternalError(format!("Failed to serialize request body: {e}"))
        })?;

        let response = self
            .client
            .post(&self.endpoint)
            .header("Content-Type", "application/json")
            .header("Content-Length", body_string.len().to_string())
            .body(body_string)
            .send()
            .map_err(|e| {
                self.handle_janusgraph_reqwest_error("JanusGraph read request failed", e)
            })?;
        Self::handle_response(response)
    }

    pub fn close_session(&self) -> Result<(), GraphError> {
        trace!("Close session: {}", self.session_id);
        let request = GremlinCloseRequest {
            session: self.session_id.clone(),
            op: "close".to_string(),
            processor: "session".to_string(),
        };

        let body_string = serde_json::to_string(&request).map_err(|e| {
            GraphError::InternalError(format!("Failed to serialize request body: {e}"))
        })?;

        let response = self
            .client
            .post(&self.endpoint)
            .header("Content-Type", "application/json")
            .header("Content-Length", body_string.len().to_string())
            .body(body_string)
            .send()
            .map_err(|e| {
                self.handle_janusgraph_reqwest_error("JanusGraph close session failed", e)
            })?;
        Self::handle_response(response).map(|_| ())
    }

    pub fn session_id(&self) -> &str {
        trace!("Get session ID: {}", self.session_id);
        &self.session_id
    }

    pub fn is_session_active(&self) -> bool {
        trace!("Check session active status: {}", self.session_id);
        match self.execute("1", None) {
            Ok(_) => true,
            Err(e) => {
                trace!("Session {} appears inactive: {}", self.session_id, e);
                false
            }
        }
    }

    fn handle_janusgraph_reqwest_error(&self, details: &str, err: reqwest::Error) -> GraphError {
        if err.is_timeout() {
            return GraphError::Timeout;
        }

        if err.is_request() {
            return GraphError::ConnectionFailed(format!(
                "JanusGraph request failed ({details}): {err}"
            ));
        }

        if err.is_decode() {
            return GraphError::InternalError(format!(
                "JanusGraph response decode failed ({details}): {err}"
            ));
        }

        if err.is_status() {
            if let Some(status) = err.status() {
                let error_msg = format!(
                    "JanusGraph HTTP error {} ({}): {}",
                    status.as_u16(),
                    details,
                    err
                );
                let error_body = GremlinErrorResponse {
                    message: Some(error_msg.clone()),
                    status: Some(GremlinStatus {
                        message: Some(error_msg.clone()),
                        code: Some(status.as_u16()),
                        attributes: None,
                    }),
                    result: None,
                    exceptions: None,
                };
                return Self::map_janusgraph_http_status(status.as_u16(), &error_msg, &error_body);
            }
        }

        GraphError::InternalError(format!("JanusGraph request error ({details}): {err}"))
    }

    fn handle_response(response: Response) -> Result<Value, GraphError> {
        let status = response.status();
        let status_code = status.as_u16();

        if status.is_success() {
            let response_body: GremlinResponse = response.json().map_err(|e| {
                GraphError::InternalError(format!("Failed to parse response body: {e}"))
            })?;

            if let Some(result) = response_body.result {
                if let Some(data) = result.data {
                    Ok(data)
                } else {
                    Ok(serde_json::to_value(result).unwrap_or(Value::Null))
                }
            } else {
                Ok(serde_json::to_value(response_body).unwrap_or(Value::Null))
            }
        } else {
            let error_body: GremlinErrorResponse = response.json().map_err(|e| {
                GraphError::InternalError(format!("Failed to read error response: {e}"))
            })?;

            let error_msg = error_body.message.as_deref().unwrap_or("Unknown error");

            Err(Self::map_janusgraph_error(
                status_code,
                error_msg,
                &error_body,
            ))
        }
    }

    fn map_janusgraph_error(
        status_code: u16,
        message: &str,
        error_body: &GremlinErrorResponse,
    ) -> GraphError {
        if let Some(status) = &error_body.status {
            if let Some(code) = status.code {
                return Self::from_gremlin_status_code(code, message, error_body);
            }
        }

        if let Some(result) = &error_body.result {
            if let Some(data) = &result.data {
                if let Some(detailed_msg) = &data.detailed_message {
                    return Self::from_janusgraph_detailed_error(detailed_msg, error_body);
                }
            }
        }

        if let Some(exceptions) = &error_body.exceptions {
            if let Some(first_exception) = exceptions.first() {
                return Self::from_janusgraph_exception(first_exception, error_body);
            }
        }

        Self::map_janusgraph_http_status(status_code, message, error_body)
    }

    fn from_gremlin_status_code(
        code: u16,
        message: &str,
        error_body: &GremlinErrorResponse,
    ) -> GraphError {
        let detailed_error = Self::extract_gremlin_exception_info(error_body);

        match code {
            // Authentication and Authorization
            401 => GraphError::AuthenticationFailed(format!(
                "Gremlin authentication failed: {message}"
            )),
            403 => {
                GraphError::AuthorizationFailed(format!("Gremlin authorization failed: {message}"))
            }
            407 => GraphError::AuthenticationFailed(format!(
                "Gremlin authentication challenge: {message}"
            )),

            // Client Request Errors
            498 => {
                if let Some(detailed) = detailed_error {
                    return detailed;
                }
                GraphError::InvalidQuery(format!("Gremlin malformed request: {message}"))
            }
            499 => {
                GraphError::InvalidQuery(format!("Gremlin invalid request arguments: {message}"))
            }

            // Server Errors
            500 => {
                if let Some(detailed) = detailed_error {
                    return detailed;
                }
                GraphError::InternalError(format!("Gremlin server error: {message}"))
            }
            597 => {
                if let Some(detailed) = detailed_error {
                    return detailed;
                }
                GraphError::InvalidQuery(format!("Gremlin script evaluation error: {message}"))
            }
            598 => GraphError::Timeout,
            599 => GraphError::InternalError(format!("Gremlin serialization error: {message}")),

            // Default fallback
            _ => {
                if let Some(detailed) = detailed_error {
                    return detailed;
                }
                let debug_info = format!(
                    "Gremlin status code [{code}]: {message} | Status: {:?}",
                    error_body.status
                );
                GraphError::InternalError(debug_info)
            }
        }
    }

    fn extract_gremlin_exception_info(error_body: &GremlinErrorResponse) -> Option<GraphError> {
        if let Some(result) = &error_body.result {
            if let Some(data) = &result.data {
                if data.at_type.as_deref() == Some("g:Map") {
                    if let Some(class_name) = &data.java_class {
                        return Self::map_java_exception_class(class_name, data);
                    }

                    if let Some(stack_trace) = &data.stack_trace {
                        return Self::extract_from_stack_trace(stack_trace);
                    }
                }

                if let Some(ex_type) = &data.exception_type {
                    return Self::map_java_exception_class(ex_type, data);
                }
            }
        }

        if let Some(exceptions) = &error_body.exceptions {
            if let Some(first_exception) = exceptions.first() {
                return Self::extract_from_stack_trace(first_exception);
            }
        }

        None
    }

    fn map_java_exception_class(
        class_name: &str,
        exception_data: &GremlinErrorData,
    ) -> Option<GraphError> {
        let message = exception_data.message.as_deref().unwrap_or(class_name);

        match class_name {
            // JanusGraph specific exceptions
            "org.janusgraph.core.SchemaViolationException" => {
                Some(GraphError::SchemaViolation(format!("JanusGraph schema violation: {message}")))
            }
            "org.janusgraph.core.JanusGraphException" => {
                Some(GraphError::InternalError(format!("JanusGraph error: {message}")))
            }
            "org.janusgraph.diskstorage.TemporaryBackendException" => {
                Some(GraphError::ServiceUnavailable(format!("JanusGraph backend temporarily unavailable: {message}")))
            }
            "org.janusgraph.diskstorage.PermanentBackendException" => {
                Some(GraphError::InternalError(format!("JanusGraph backend permanent error: {message}")))
            }
            // Gremlin/TinkerPop exceptions
            "org.apache.tinkerpop.gremlin.process.traversal.strategy.verification.VerificationException" => {
                Some(GraphError::InvalidQuery(format!("Gremlin verification error: {message}")))
            }
            "org.apache.tinkerpop.gremlin.groovy.CompilationFailedException" => {
                Some(GraphError::InvalidQuery(format!("Gremlin compilation error: {message}")))
            }
            "org.apache.tinkerpop.gremlin.driver.exception.ResponseException" => {
                if let Some(code) = exception_data.response_status_code {
                    return Some(Self::from_gremlin_status_code(code as u16, message, &GremlinErrorResponse {
                        message: Some(message.to_string()),
                        status: None,
                        result: None,
                        exceptions: None,
                    }));
                }
                Some(GraphError::InternalError(format!("Gremlin response error: {message}")))
            }
            // Standard Java exceptions with graph context
            "java.util.concurrent.TimeoutException" => {
                Some(GraphError::Timeout)
            }
            "java.lang.IllegalArgumentException" => {
                Some(GraphError::InvalidQuery(format!("Invalid argument: {message}")))
            }
            "java.lang.UnsupportedOperationException" => {
                Some(GraphError::UnsupportedOperation(format!("Unsupported operation: {message}")))
            }
            "java.lang.IllegalStateException" => {
                Some(GraphError::TransactionFailed(format!("Illegal state: {message}")))
            }
            "java.util.NoSuchElementException" => {
                if let Some(element_id) = golem_graph::error::mapping::extract_element_id_from_message(message) {
                    Some(GraphError::ElementNotFound(element_id))
                } else {
                    Some(GraphError::InternalError(format!("Element not found: {message}")))
                }
            }
            // Transaction related exceptions
            "org.apache.tinkerpop.gremlin.structure.util.TransactionException" => {
                Some(GraphError::TransactionFailed(format!("Transaction error: {message}")))
            }
            _ => None

        }
    }

    fn extract_from_stack_trace(stack_trace: &str) -> Option<GraphError> {
        let first_line = stack_trace.lines().next()?;

        if let Some(colon_pos) = first_line.find(':') {
            let exception_part = &first_line[..colon_pos];
            let message_part = &first_line[colon_pos + 1..].trim();

            if let Some(last_dot) = exception_part.rfind('.') {
                let _class_name = &exception_part[last_dot + 1..];
                let full_class_name = exception_part;

                let exception_data = GremlinErrorData {
                    at_type: None,
                    detailed_message: None,
                    java_class: Some(full_class_name.to_string()),
                    exception_type: Some(full_class_name.to_string()),
                    stack_trace: Some(stack_trace.to_string()),
                    response_status_code: None,
                    message: Some(message_part.to_string()),
                };

                return Self::map_java_exception_class(full_class_name, &exception_data);
            }
        }

        None
    }

    fn from_janusgraph_detailed_error(
        detailed_message: &str,
        error_body: &GremlinErrorResponse,
    ) -> GraphError {
        if let Some(detailed_error) = Self::extract_gremlin_exception_info(error_body) {
            return detailed_error;
        }

        if let Some(exception_error) = Self::extract_from_stack_trace(detailed_message) {
            return exception_error;
        }

        GraphError::InternalError(format!("JanusGraph detailed error: {detailed_message}"))
    }

    fn from_janusgraph_exception(
        exception_message: &str,
        error_body: &GremlinErrorResponse,
    ) -> GraphError {
        if let Some(detailed_error) = Self::extract_gremlin_exception_info(error_body) {
            return detailed_error;
        }

        if let Some(exception_error) = Self::extract_from_stack_trace(exception_message) {
            return exception_error;
        }

        GraphError::InternalError(format!("JanusGraph exception: {exception_message}"))
    }

    fn map_janusgraph_http_status(
        status: u16,
        message: &str,
        error_body: &GremlinErrorResponse,
    ) -> GraphError {
        match status {
            // Authentication and Authorization
            401 => GraphError::AuthenticationFailed(format!(
                "JanusGraph authentication failed: {message}"
            )),
            403 => GraphError::AuthorizationFailed(format!(
                "JanusGraph authorization failed: {message}"
            )),

            // Client errors
            400 => {
                if let Some(detailed_error) = Self::extract_gremlin_exception_info(error_body) {
                    return detailed_error;
                }
                GraphError::InvalidQuery(format!("JanusGraph bad request: {message}"))
            }
            404 => {
                GraphError::ServiceUnavailable(format!("JanusGraph resource not found: {message}"))
            }
            409 => GraphError::TransactionConflict,
            413 => {
                GraphError::ResourceExhausted(format!("JanusGraph request too large: {message}"))
            }
            429 => {
                GraphError::ResourceExhausted(format!("JanusGraph rate limit exceeded: {message}"))
            }

            // Server errors
            500 => {
                if let Some(detailed_error) = Self::extract_gremlin_exception_info(error_body) {
                    return detailed_error;
                }
                GraphError::InternalError(format!("JanusGraph internal server error: {message}"))
            }
            502 => GraphError::ServiceUnavailable(format!("JanusGraph bad gateway: {message}")),
            503 => {
                GraphError::ServiceUnavailable(format!("JanusGraph service unavailable: {message}"))
            }
            504 => GraphError::Timeout,
            507 => {
                GraphError::ResourceExhausted(format!("JanusGraph insufficient storage: {message}"))
            }

            _ => {
                if let Some(detailed_error) = Self::extract_gremlin_exception_info(error_body) {
                    return detailed_error;
                }

                let debug_info = format!(
                    "JanusGraph HTTP error [{}]: {} | Status: {:?}",
                    status, message, error_body.status
                );
                GraphError::InternalError(debug_info)
            }
        }
    }
}
