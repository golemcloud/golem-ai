//! Explicit, provider-neutral access to registered Golem tools.
//!
//! A [`GolemToolkit`] advertises only commands selected by the application. It
//! does not call an LLM, approve calls, or run a conversation loop. Declared
//! tool-domain errors are returned as successful [`ToolResult`] values with a
//! `{"status":"error",...}` payload; discovery, validation, transport, and
//! protocol failures remain Rust errors.

use crate::model::{ToolCall, ToolDefinition, ToolResult, ToolSuccess};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use golem_rust::agentic::{
    get_tool_type, InputStream, ReflectedToolCustomError, ToolArgument, ToolArgumentKind,
    ToolCommand, ToolError, ToolInvocation, ToolInvocationOutput, ToolReflectionError, ToolType,
};
use golem_rust::golem_agentic::golem::tool::streams::ByteStreamFailure;
use golem_rust::schema::render::{to_json_value, to_reflection_json_schema};
use golem_rust::SchemaValue;
use serde_json::{json, Map, Value};
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};

const STDIN_FIELD: &str = "_stdin";

/// One explicitly selected registered command.
#[derive(Clone, Debug)]
pub struct GolemToolSelection {
    source: ToolSource,
    path: Vec<String>,
    name: Option<String>,
    max_stdout_bytes: Option<usize>,
    max_stderr_bytes: Option<usize>,
}

#[derive(Clone, Debug)]
enum ToolSource {
    Registered(String),
    Reflected(ToolType),
}

impl GolemToolSelection {
    /// Selects a command from the tool registered under `lookup_name`.
    pub fn new(
        lookup_name: impl Into<String>,
        path: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            source: ToolSource::Registered(lookup_name.into()),
            path: path.into_iter().map(Into::into).collect(),
            name: None,
            max_stdout_bytes: None,
            max_stderr_bytes: None,
        }
    }

    /// Selects a command from an already reflected tool type.
    pub fn from_tool_type(
        tool_type: ToolType,
        path: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            source: ToolSource::Reflected(tool_type),
            path: path.into_iter().map(Into::into).collect(),
            name: None,
            max_stdout_bytes: None,
            max_stderr_bytes: None,
        }
    }

    /// Overrides the deterministic model-facing command name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets the retained stdout prefix limit. Required when stdout is declared.
    pub fn max_stdout_bytes(mut self, limit: usize) -> Self {
        self.max_stdout_bytes = Some(limit);
        self
    }

    /// Sets the retained stderr prefix limit. Required when stderr is declared.
    pub fn max_stderr_bytes(mut self, limit: usize) -> Self {
        self.max_stderr_bytes = Some(limit);
        self
    }
}

/// A fixed allowlist of reflected Golem commands and their LLM definitions.
pub struct GolemToolkit {
    definitions: Vec<ToolDefinition>,
    commands: HashMap<String, PreparedCommand>,
}

struct PreparedCommand {
    command: ToolCommand,
    defaults: Map<String, Value>,
    max_stdout_bytes: Option<usize>,
    max_stderr_bytes: Option<usize>,
    stdout_textual: bool,
    stderr_textual: bool,
}

impl GolemToolkit {
    /// Resolves and validates all selections before any definition is sent to a model.
    pub fn new(
        selections: impl IntoIterator<Item = GolemToolSelection>,
    ) -> Result<Self, GolemToolError> {
        let mut definitions = Vec::new();
        let mut commands = HashMap::new();
        for selection in selections {
            let tool = match selection.source {
                ToolSource::Registered(name) => get_tool_type(&name)?,
                ToolSource::Reflected(tool) => tool,
            };
            let requested_path = selection
                .path
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>();
            let node = tool.node(&requested_path).ok_or_else(|| {
                GolemToolError::InvalidSelection(format!(
                    "tool `{}` has no command path `{}`",
                    tool.lookup_name(),
                    selection.path.join(" ")
                ))
            })?;
            if !node.is_callable() {
                return Err(GolemToolError::InvalidSelection(format!(
                    "command namespace `{}` is not callable",
                    selection.path.join(" ")
                )));
            }
            let command = node.command()?;
            let canonical_tool_name = tool.definition().name().ok_or_else(|| {
                GolemToolError::InvalidSelection("reflected tool has no root name".to_string())
            })?;
            let name = selection
                .name
                .unwrap_or_else(|| default_model_name(canonical_tool_name, command.path()));
            validate_model_name(&name)?;
            if commands.contains_key(&name) {
                return Err(GolemToolError::DuplicateName(name));
            }
            prepare_command(
                name,
                command,
                selection.max_stdout_bytes,
                selection.max_stderr_bytes,
                &mut definitions,
                &mut commands,
            )?;
        }
        Ok(Self {
            definitions,
            commands,
        })
    }

    /// Definitions to combine with any application-owned manual definitions.
    ///
    /// The application must ensure names are unique across the combined set.
    pub fn definitions(&self) -> &[ToolDefinition] {
        &self.definitions
    }

    /// Returns whether this toolkit owns a model-facing tool name.
    pub fn contains(&self, name: &str) -> bool {
        self.commands.contains_key(name)
    }

    /// Executes a complete model-requested call owned by this toolkit.
    ///
    /// Unknown or unselected names fail before reaching the native transport.
    pub async fn execute(&self, call: &ToolCall) -> Result<ToolResult, GolemToolError> {
        let prepared = self
            .commands
            .get(&call.name)
            .ok_or_else(|| GolemToolError::UnknownCall(call.name.clone()))?;
        let raw: Value = serde_json::from_str(&call.arguments_json)
            .map_err(|error| GolemToolError::InvalidArguments(error.to_string()))?;
        let mut arguments = raw.as_object().cloned().ok_or_else(|| {
            GolemToolError::InvalidArguments("tool arguments must be a JSON object".to_string())
        })?;
        let stdin = if prepared.command.body().stdin.is_some() {
            decode_stdin(arguments.remove(STDIN_FIELD))?
        } else {
            None
        };
        normalize_defaults(&prepared.defaults, &mut arguments);
        let packed = prepared.command.pack_json(&Value::Object(arguments))?;
        let invocation = prepared.command.start_value(packed, stdin).await?;
        let outcome = execute_started(prepared, invocation).await?;
        Ok(ToolResult::Success(ToolSuccess {
            id: call.id.clone(),
            name: call.name.clone(),
            result_json: serde_json::to_string(&outcome)?,
            execution_time_ms: None,
        }))
    }
}

fn prepare_command(
    name: String,
    command: ToolCommand,
    max_stdout_bytes: Option<usize>,
    max_stderr_bytes: Option<usize>,
    definitions: &mut Vec<ToolDefinition>,
    commands: &mut HashMap<String, PreparedCommand>,
) -> Result<(), GolemToolError> {
    let body = command.body();
    let max_stdout_bytes = capture_limit(body.stdout.is_some(), max_stdout_bytes, "stdout", &name)?;
    let max_stderr_bytes = capture_limit(body.stderr.is_some(), max_stderr_bytes, "stderr", &name)?;
    if body.stdin.is_some()
        && command.arguments().iter().any(|argument| {
            argument.name == STDIN_FIELD
                || argument.aliases.iter().any(|alias| alias == STDIN_FIELD)
        })
    {
        return Err(GolemToolError::InvalidSelection(format!(
            "command `{name}` parameter `{STDIN_FIELD}` conflicts with stdin"
        )));
    }

    ensure_json_eligible(
        &command.input_schema().to_json_schema(false),
        "parameter",
        &name,
    )?;
    if let Some(schema) = command.output_schema() {
        ensure_json_eligible(&schema.to_json_schema(false), "result", &name)?;
    }
    for error in &body.errors {
        if let Some(payload) = &error.payload {
            let schema = to_reflection_json_schema(command.input_schema().graph(), payload, false);
            ensure_json_eligible(&schema, "error payload", &name)?;
        }
    }

    let schema = parameter_schema(&command)?;
    let defaults = command_defaults(command.arguments(), &name)?;
    let description = command_description(&command);
    let stdout_textual = body
        .stdout
        .as_ref()
        .is_some_and(|stream| textual_mime(&stream.mime));
    let stderr_textual = body
        .stderr
        .as_ref()
        .is_some_and(|stream| textual_mime(&stream.mime));
    definitions.push(ToolDefinition {
        name: name.clone(),
        description,
        parameters_schema: serde_json::to_string(&schema)?,
    });
    commands.insert(
        name,
        PreparedCommand {
            command,
            defaults,
            max_stdout_bytes,
            max_stderr_bytes,
            stdout_textual,
            stderr_textual,
        },
    );
    Ok(())
}

fn capture_limit(
    declared: bool,
    configured: Option<usize>,
    channel: &'static str,
    command_name: &str,
) -> Result<Option<usize>, GolemToolError> {
    if declared {
        configured.map(Some).ok_or_else(|| {
            GolemToolError::InvalidSelection(format!(
                "command `{command_name}` requires max_{channel}_bytes"
            ))
        })
    } else {
        Ok(None)
    }
}

fn default_model_name(tool: &str, path: &[String]) -> String {
    if path.is_empty() {
        tool.to_string()
    } else {
        format!("{tool}__{}", path.join("__"))
    }
}

fn validate_model_name(name: &str) -> Result<(), GolemToolError> {
    if name.is_empty()
        || name.len() > 64
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
    {
        return Err(GolemToolError::InvalidName(name.to_string()));
    }
    Ok(())
}

fn parameter_schema(command: &ToolCommand) -> Result<Value, GolemToolError> {
    let schema = command.input_schema().to_json_schema(false);
    let optional = command
        .arguments()
        .iter()
        .filter(|argument| !argument.required)
        .map(|argument| argument.name.as_str())
        .collect::<HashSet<_>>();
    adapt_parameter_schema(
        schema,
        &optional,
        command.body().stdin.is_some(),
        command
            .body()
            .stdin
            .as_ref()
            .is_some_and(|stdin| stdin.required),
    )
}

fn adapt_parameter_schema(
    mut schema: Value,
    optional: &HashSet<&str>,
    stdin_declared: bool,
    stdin_required: bool,
) -> Result<Value, GolemToolError> {
    let object = schema.as_object_mut().ok_or_else(|| {
        GolemToolError::InvalidSelection(
            "reflected command input schema must be an object".to_string(),
        )
    })?;
    let properties = object
        .entry("properties")
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .ok_or_else(|| {
            GolemToolError::InvalidSelection(
                "reflected command properties schema must be an object".to_string(),
            )
        })?;
    if stdin_declared {
        properties.insert(
            STDIN_FIELD.to_string(),
            json!({
                "type": "object",
                "properties": {
                    "data": { "type": "string" },
                    "encoding": { "enum": ["utf8", "base64"] }
                },
                "required": ["data", "encoding"],
                "additionalProperties": false
            }),
        );
    }

    let required = object
        .entry("required")
        .or_insert_with(|| Value::Array(Vec::new()))
        .as_array_mut()
        .ok_or_else(|| {
            GolemToolError::InvalidSelection(
                "reflected command required schema must be an array".to_string(),
            )
        })?;
    required.retain(|field| field.as_str().is_none_or(|field| !optional.contains(field)));
    if stdin_required {
        required.push(Value::String(STDIN_FIELD.to_string()));
    }
    Ok(schema)
}

fn command_defaults(
    arguments: &[ToolArgument],
    command_name: &str,
) -> Result<Map<String, Value>, GolemToolError> {
    let mut defaults = Map::new();
    for argument in arguments {
        if !argument.required {
            if let Some(default) = &argument.default {
                let value = argument.schema.unpack_json(default).map_err(|error| {
                    GolemToolError::InvalidSelection(format!(
                        "command `{command_name}` argument `{}` has a non-JSON default: {error}",
                        argument.name
                    ))
                })?;
                defaults.insert(argument.name.clone(), value);
            } else if matches!(
                argument.kind,
                ToolArgumentKind::Tail | ToolArgumentKind::Option
            ) && argument.schema.pack_json(&Value::Null).is_err()
            {
                // Optional scalar arguments use an option carrier and can be omitted.
                // Tail and repeatable options collect into list/map values instead.
                defaults.insert(argument.name.clone(), Value::Array(Vec::new()));
            }
        }
    }
    Ok(defaults)
}

fn normalize_defaults(defaults: &Map<String, Value>, arguments: &mut Map<String, Value>) {
    for (name, default) in defaults {
        if !arguments.contains_key(name) {
            arguments.insert(name.clone(), default.clone());
        }
    }
}

fn command_description(command: &ToolCommand) -> Option<String> {
    let mut paragraphs = Vec::new();
    append_doc(
        &mut paragraphs,
        &command.doc().summary,
        &command.doc().description,
    );
    if let Some(stdin) = &command.body().stdin {
        let mut description = format!(
            "Stdin is {}; pass {STDIN_FIELD} as {{ data, encoding: \"utf8\" | \"base64\" }}.",
            if stdin.required {
                "required"
            } else {
                "optional"
            }
        );
        append_mime_hint(&mut description, &stdin.mime);
        paragraphs.push(description);
        append_doc(&mut paragraphs, &stdin.doc.summary, &stdin.doc.description);
    }
    if let Some(stdout) = &command.body().stdout {
        let mut description =
            "Stdout is returned as bounded captured bytes after the stream is fully drained."
                .to_string();
        append_mime_hint(&mut description, &stdout.mime);
        paragraphs.push(description);
        append_doc(
            &mut paragraphs,
            &stdout.doc.summary,
            &stdout.doc.description,
        );
    }
    if let Some(stderr) = &command.body().stderr {
        let mut description =
            "Stderr is returned as bounded captured bytes after the stream is fully drained."
                .to_string();
        append_mime_hint(&mut description, &stderr.mime);
        paragraphs.push(description);
        append_doc(
            &mut paragraphs,
            &stderr.doc.summary,
            &stderr.doc.description,
        );
    }
    (!paragraphs.is_empty()).then(|| paragraphs.join("\n\n"))
}

fn append_doc(paragraphs: &mut Vec<String>, summary: &str, description: &str) {
    if !summary.trim().is_empty() {
        paragraphs.push(summary.trim().to_string());
    }
    if !description.trim().is_empty() && description.trim() != summary.trim() {
        paragraphs.push(description.trim().to_string());
    }
}

fn append_mime_hint(description: &mut String, mime: &[String]) {
    if !mime.is_empty() {
        description.push_str(" Declared MIME hints: ");
        description.push_str(&mime.join(", "));
        description.push('.');
    }
}

fn ensure_json_eligible(
    schema: &Value,
    kind: &'static str,
    name: &str,
) -> Result<(), GolemToolError> {
    if schema_accepts_no_values(schema) {
        return Err(GolemToolError::InvalidSelection(format!(
            "command `{name}` has a non-JSON {kind} schema"
        )));
    }
    Ok(())
}

fn schema_accepts_no_values(schema: &Value) -> bool {
    fn visit(
        value: &Value,
        defs: Option<&Map<String, Value>>,
        visited: &mut HashSet<String>,
    ) -> bool {
        let Some(object) = value.as_object() else {
            return false;
        };
        if object
            .get("not")
            .and_then(Value::as_object)
            .is_some_and(Map::is_empty)
        {
            return true;
        }
        if let Some(reference) = object.get("$ref").and_then(Value::as_str) {
            if let Some(key) = reference.strip_prefix("#/$defs/") {
                let key = key.replace("~1", "/").replace("~0", "~");
                if visited.insert(key.clone()) {
                    let rejects_values = defs
                        .and_then(|defs| defs.get(&key))
                        .is_some_and(|target| visit(target, defs, visited));
                    visited.remove(&key);
                    if rejects_values {
                        return true;
                    }
                }
            }
        }
        object.iter().any(|(key, child)| {
            key != "$defs"
                && match child {
                    Value::Object(_) => visit(child, defs, visited),
                    Value::Array(items) => items.iter().any(|item| visit(item, defs, visited)),
                    _ => false,
                }
        })
    }
    let defs = schema.get("$defs").and_then(Value::as_object);
    visit(schema, defs, &mut HashSet::new())
}

fn decode_stdin(value: Option<Value>) -> Result<Option<InputStream>, GolemToolError> {
    let Some(bytes) = decode_stdin_bytes(value)? else {
        return Ok(None);
    };
    let (mut writer, reader) =
        golem_rust::golem_agentic::wit_stream::new::<Result<Vec<u8>, ByteStreamFailure>>();
    golem_rust::agentic::spawn_local(async move {
        let _ = writer.write_all(vec![Ok(bytes)]).await;
    });
    Ok(Some(reader))
}

fn decode_stdin_bytes(value: Option<Value>) -> Result<Option<Vec<u8>>, GolemToolError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let object = value
        .as_object()
        .ok_or_else(|| GolemToolError::InvalidArguments("stdin must be an object".to_string()))?;
    if object.len() != 2 {
        return Err(GolemToolError::InvalidArguments(
            "stdin must contain only data and encoding".to_string(),
        ));
    }
    let data = object.get("data").and_then(Value::as_str).ok_or_else(|| {
        GolemToolError::InvalidArguments("stdin data must be a string".to_string())
    })?;
    let encoding = object
        .get("encoding")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            GolemToolError::InvalidArguments("stdin encoding must be a string".to_string())
        })?;
    let bytes = match encoding {
        "utf8" => data.as_bytes().to_vec(),
        "base64" => {
            let decoded = BASE64.decode(data).map_err(|_| {
                GolemToolError::InvalidArguments("invalid base64 stdin".to_string())
            })?;
            if BASE64.encode(&decoded) != data {
                return Err(GolemToolError::InvalidArguments(
                    "non-canonical base64 stdin".to_string(),
                ));
            }
            decoded
        }
        _ => {
            return Err(GolemToolError::InvalidArguments(
                "stdin encoding must be 'utf8' or 'base64'".to_string(),
            ));
        }
    };
    Ok(Some(bytes))
}

struct InvocationGuard<T, E> {
    invocation: ToolInvocation<T, E>,
    completed: bool,
}

impl<T, E> Drop for InvocationGuard<T, E> {
    fn drop(&mut self) {
        if !self.completed {
            self.invocation.cancel();
        }
    }
}

async fn execute_started(
    prepared: &PreparedCommand,
    invocation: ToolInvocation<Option<SchemaValue>, ReflectedToolCustomError>,
) -> Result<Value, GolemToolError> {
    let mut guard = InvocationGuard {
        invocation,
        completed: false,
    };
    let result = guard.invocation.result();
    let stdout = capture_output(
        guard.invocation.stdout.take(),
        prepared.max_stdout_bytes,
        prepared.stdout_textual,
        "stdout",
    );
    let stderr = capture_output(
        guard.invocation.stderr.take(),
        prepared.max_stderr_bytes,
        prepared.stderr_textual,
        "stderr",
    );
    let (result, stdout, stderr) = futures::join!(result, stdout, stderr);
    guard.completed = true;

    match result {
        Err(ToolError::Tool(domain)) => {
            let error = domain_error_json(prepared, domain)?;
            Ok(domain_error_envelope(error, stdout, stderr))
        }
        Err(error) => Err(GolemToolError::Invocation(error.to_string())),
        Ok(result) => {
            let result = result
                .map(|result| {
                    let schema = prepared.command.output_schema().ok_or_else(|| {
                        GolemToolError::Invocation("tool returned an undeclared result".to_string())
                    })?;
                    schema
                        .unpack_json(&result)
                        .map_err(|error| {
                            ToolReflectionError::Tool(ToolError::MalformedRemoteOutput(
                                error.to_string(),
                            ))
                        })
                        .map_err(Into::into)
                })
                .transpose();
            finish_success(result, stdout, stderr)
        }
    }
}

fn finish_success(
    result: Result<Option<Value>, GolemToolError>,
    stdout: Result<Option<Value>, GolemToolError>,
    stderr: Result<Option<Value>, GolemToolError>,
) -> Result<Value, GolemToolError> {
    let result = result?;
    let stdout = stdout?;
    let stderr = stderr?;
    Ok(success_envelope(result, stdout, stderr))
}

fn success_envelope(result: Option<Value>, stdout: Option<Value>, stderr: Option<Value>) -> Value {
    let mut envelope = Map::new();
    envelope.insert("status".to_string(), Value::String("success".to_string()));
    if let Some(result) = result {
        envelope.insert("result".to_string(), result);
    }
    if let Some(stdout) = stdout {
        envelope.insert("stdout".to_string(), stdout);
    }
    if let Some(stderr) = stderr {
        envelope.insert("stderr".to_string(), stderr);
    }
    Value::Object(envelope)
}

fn domain_error_json(
    prepared: &PreparedCommand,
    domain: ReflectedToolCustomError,
) -> Result<Value, GolemToolError> {
    let declared = prepared
        .command
        .body()
        .errors
        .iter()
        .find(|error| error.name == domain.name)
        .ok_or_else(|| GolemToolError::Invocation("undeclared tool error".to_string()))?;
    let payload = if declared.payload.is_some() {
        Some(to_json_value(
            domain.payload.graph(),
            &domain.payload.graph().root,
            domain.payload.value(),
        )?)
    } else {
        None
    };
    Ok(domain_error_value(domain.name, payload))
}

fn domain_error_value(name: String, payload: Option<Value>) -> Value {
    let mut error = Map::new();
    error.insert("name".to_string(), Value::String(name));
    if let Some(payload) = payload {
        error.insert("value".to_string(), payload);
    }
    Value::Object(error)
}

fn domain_error_envelope(
    error: Value,
    stdout: Result<Option<Value>, GolemToolError>,
    stderr: Result<Option<Value>, GolemToolError>,
) -> Value {
    let mut envelope = Map::new();
    envelope.insert("status".to_string(), Value::String("error".to_string()));
    envelope.insert("error".to_string(), error);
    if let Ok(Some(stdout)) = stdout {
        envelope.insert("stdout".to_string(), stdout);
    }
    if let Ok(Some(stderr)) = stderr {
        envelope.insert("stderr".to_string(), stderr);
    }
    Value::Object(envelope)
}

async fn capture_output(
    mut output: Option<ToolInvocationOutput>,
    limit: Option<usize>,
    textual: bool,
    channel: &'static str,
) -> Result<Option<Value>, GolemToolError> {
    let Some(ref mut output) = output else {
        if limit.is_some() {
            return Err(GolemToolError::Invocation(format!(
                "declared tool {channel} was not attached"
            )));
        }
        return Ok(None);
    };
    let limit = limit.ok_or_else(|| {
        GolemToolError::Invocation(format!("tool {channel} has no capture limit"))
    })?;
    let mut capture = CaptureBuffer::new(limit);
    while let Some(item) = output.next().await {
        match item {
            Ok(chunk) => capture.push(&chunk),
            Err(reason) => {
                return Err(GolemToolError::Stream {
                    channel,
                    message: format!("{reason:?}"),
                });
            }
        }
    }
    Ok(Some(capture.finish(textual)))
}

struct CaptureBuffer {
    retained: Vec<u8>,
    limit: usize,
    total: u64,
}

impl CaptureBuffer {
    fn new(limit: usize) -> Self {
        Self {
            retained: Vec::with_capacity(limit.min(8192)),
            limit,
            total: 0,
        }
    }

    fn push(&mut self, chunk: &[u8]) {
        self.total = self.total.saturating_add(chunk.len() as u64);
        let take = chunk
            .len()
            .min(self.limit.saturating_sub(self.retained.len()));
        self.retained.extend_from_slice(&chunk[..take]);
    }

    fn finish(self, textual: bool) -> Value {
        let retained_len = self.retained.len() as u64;
        let (data, encoding) = if textual {
            match String::from_utf8(self.retained) {
                Ok(text) => (text, "utf8"),
                Err(error) => (BASE64.encode(error.into_bytes()), "base64"),
            }
        } else {
            (BASE64.encode(self.retained), "base64")
        };
        json!({
            "data": data,
            "encoding": encoding,
            "truncated": self.total > retained_len,
            "totalBytes": self.total,
        })
    }
}

fn textual_mime(mime: &[String]) -> bool {
    mime.iter().any(|declared| {
        let value = declared.to_ascii_lowercase();
        let essence = value.split(';').next().unwrap_or_default();
        essence.starts_with("text/")
            || essence == "application/json"
            || essence.ends_with("+json")
            || essence == "application/xml"
            || essence.ends_with("+xml")
            || essence == "application/javascript"
    })
}

/// Failures that must remain visible to application policy and retry logic.
#[derive(Debug)]
pub enum GolemToolError {
    Reflection(Box<ToolReflectionError>),
    InvalidSelection(String),
    InvalidName(String),
    DuplicateName(String),
    UnknownCall(String),
    InvalidArguments(String),
    Invocation(String),
    Stream {
        channel: &'static str,
        message: String,
    },
    Json(serde_json::Error),
    Schema(golem_rust::schema::render::RenderError),
}

impl Display for GolemToolError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Reflection(error) => error.fmt(f),
            Self::InvalidSelection(message) => write!(f, "invalid Golem tool selection: {message}"),
            Self::InvalidName(name) => write!(f, "invalid model-facing tool name `{name}`"),
            Self::DuplicateName(name) => write!(f, "duplicate model-facing tool name `{name}`"),
            Self::UnknownCall(name) => write!(f, "tool call `{name}` is not selected"),
            Self::InvalidArguments(message) => write!(f, "invalid tool arguments: {message}"),
            Self::Invocation(message) => write!(f, "tool invocation failed: {message}"),
            Self::Stream { channel, message } => {
                write!(f, "tool {channel} stream failed: {message}")
            }
            Self::Json(error) => write!(f, "tool JSON error: {error}"),
            Self::Schema(error) => write!(f, "tool schema rendering error: {error}"),
        }
    }
}

impl std::error::Error for GolemToolError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Reflection(error) => Some(error.as_ref()),
            Self::Json(error) => Some(error),
            Self::Schema(error) => Some(error),
            _ => None,
        }
    }
}

impl From<ToolReflectionError> for GolemToolError {
    fn from(value: ToolReflectionError) -> Self {
        Self::Reflection(Box::new(value))
    }
}

impl From<serde_json::Error> for GolemToolError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

impl From<golem_rust::schema::render::RenderError> for GolemToolError {
    fn from(value: golem_rust::schema::render::RenderError) -> Self {
        Self::Schema(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use golem_rust::{SchemaGraph, SchemaRef, SchemaType};

    fn argument(
        kind: ToolArgumentKind,
        name: &str,
        schema: SchemaType,
        required: bool,
        default: Option<SchemaValue>,
    ) -> ToolArgument {
        ToolArgument {
            kind,
            name: name.to_string(),
            aliases: Vec::new(),
            short: None,
            schema: SchemaRef::new(SchemaGraph::anonymous(schema)),
            required,
            default,
        }
    }

    #[test]
    fn validates_provider_compatible_names() {
        for valid in ["tool", "tool_name-2", &"a".repeat(64)] {
            assert!(validate_model_name(valid).is_ok(), "{valid}");
        }
        for invalid in ["", "has space", "slash/name", &"a".repeat(65)] {
            assert!(validate_model_name(invalid).is_err(), "{invalid}");
        }
    }

    #[test]
    fn default_names_distinguish_root_and_nested_commands() {
        assert_eq!(default_model_name("git", &[]), "git");
        assert_eq!(
            default_model_name("git", &["remote".into(), "add".into()]),
            "git__remote__add"
        );
    }

    #[test]
    fn capture_limits_are_required_only_for_declared_channels() {
        assert_eq!(capture_limit(false, None, "stdout", "tool").unwrap(), None);
        assert_eq!(
            capture_limit(false, Some(10), "stdout", "tool").unwrap(),
            None
        );
        assert!(capture_limit(true, None, "stdout", "tool").is_err());
        assert_eq!(
            capture_limit(true, Some(0), "stdout", "tool").unwrap(),
            Some(0)
        );
    }

    #[test]
    fn parameter_adaptation_preserves_defs_and_adds_required_stdin() {
        let schema = json!({
            "type": "object",
            "properties": {
                "required": { "$ref": "#/$defs/item" },
                "optional": { "type": ["string", "null"] }
            },
            "required": ["required", "optional"],
            "$defs": { "item": { "type": "string", "minLength": 2 } },
            "additionalProperties": false
        });
        let optional = HashSet::from(["optional"]);
        let adapted = adapt_parameter_schema(schema, &optional, true, true).unwrap();
        assert_eq!(
            adapted["$defs"],
            json!({ "item": { "type": "string", "minLength": 2 } })
        );
        assert_eq!(adapted["required"], json!(["required", "_stdin"]));
        assert_eq!(
            adapted["properties"][STDIN_FIELD]["required"],
            json!(["data", "encoding"])
        );
    }

    #[test]
    fn sdk_argument_defaults_cover_scalars_options_and_collectors() {
        let arguments = vec![
            argument(
                ToolArgumentKind::Option,
                "defaulted",
                SchemaType::string(),
                false,
                Some(SchemaValue::String("value".to_string())),
            ),
            argument(
                ToolArgumentKind::Option,
                "optional",
                SchemaType::option(SchemaType::string()),
                false,
                None,
            ),
            argument(
                ToolArgumentKind::Tail,
                "tail",
                SchemaType::list(SchemaType::string()),
                false,
                None,
            ),
            argument(
                ToolArgumentKind::Option,
                "map",
                SchemaType::map(SchemaType::string(), SchemaType::string()),
                false,
                None,
            ),
        ];
        assert_eq!(
            command_defaults(&arguments, "tool").unwrap(),
            Map::from_iter([
                ("defaulted".to_string(), json!("value")),
                ("tail".to_string(), json!([])),
                ("map".to_string(), json!([])),
            ])
        );
    }

    #[test]
    fn non_json_declared_defaults_are_rejected_before_execution() {
        let arguments = [argument(
            ToolArgumentKind::Option,
            "number",
            SchemaType::f64(),
            false,
            Some(SchemaValue::F64(f64::NAN)),
        )];
        assert!(command_defaults(&arguments, "tool").is_err());
    }

    #[test]
    fn eligibility_follows_reachable_refs_but_ignores_unrelated_defs() {
        let eligible = json!({
            "$defs": {
                "used": { "type": "string" },
                "unrelated": { "not": {} }
            },
            "type": "object",
            "properties": { "value": { "$ref": "#/$defs/used" } }
        });
        assert!(!schema_accepts_no_values(&eligible));
        let rejected = json!({
            "$defs": { "used": { "not": {} } },
            "$ref": "#/$defs/used"
        });
        assert!(schema_accepts_no_values(&rejected));
    }

    #[test]
    fn strict_base64_rejects_noncanonical_and_malformed_input() {
        assert_eq!(
            decode_stdin_bytes(Some(json!({ "data": "aGVsbG8=", "encoding": "base64" }))).unwrap(),
            Some(b"hello".to_vec())
        );
        for data in ["aGVsbG8", "aGVsbG8===", "aGVsbG9="] {
            assert!(
                decode_stdin_bytes(Some(json!({ "data": data, "encoding": "base64" }))).is_err(),
                "{data}"
            );
        }
    }

    #[test]
    fn stdin_distinguishes_omitted_and_empty_and_rejects_malformed_objects() {
        assert_eq!(decode_stdin_bytes(None).unwrap(), None);
        assert_eq!(
            decode_stdin_bytes(Some(json!({ "data": "", "encoding": "utf8" }))).unwrap(),
            Some(Vec::new())
        );
        for malformed in [
            json!(null),
            json!("text"),
            json!({ "data": "text" }),
            json!({ "data": 1, "encoding": "utf8" }),
            json!({ "data": "text", "encoding": "hex" }),
            json!({ "data": "text", "encoding": "utf8", "extra": true }),
        ] {
            assert!(decode_stdin_bytes(Some(malformed)).is_err());
        }
    }

    #[test]
    fn capture_is_bounded_but_counts_every_byte() {
        let mut capture = CaptureBuffer::new(3);
        capture.push(b"ab");
        capture.push(b"cdef");
        assert_eq!(
            capture.finish(true),
            json!({
                "data": "abc",
                "encoding": "utf8",
                "truncated": true,
                "totalBytes": 6
            })
        );
    }

    #[test]
    fn capture_handles_zero_exact_empty_binary_and_split_utf8_limits() {
        let mut zero = CaptureBuffer::new(0);
        zero.push(b"x");
        assert_eq!(
            zero.finish(true),
            json!({
                "data": "", "encoding": "utf8", "truncated": true, "totalBytes": 1
            })
        );

        let mut exact = CaptureBuffer::new(2);
        exact.push(b"ok");
        assert_eq!(
            exact.finish(true),
            json!({
                "data": "ok", "encoding": "utf8", "truncated": false, "totalBytes": 2
            })
        );

        assert_eq!(
            CaptureBuffer::new(4).finish(false),
            json!({
                "data": "", "encoding": "base64", "truncated": false, "totalBytes": 0
            })
        );

        let mut split = CaptureBuffer::new(2);
        split.push("€x".as_bytes());
        assert_eq!(
            split.finish(true),
            json!({
                "data": "4oI=", "encoding": "base64", "truncated": true, "totalBytes": 4
            })
        );

        let mut binary = CaptureBuffer::new(4);
        binary.push(b"text");
        assert_eq!(
            binary.finish(false),
            json!({
                "data": "dGV4dA==", "encoding": "base64", "truncated": false, "totalBytes": 4
            })
        );
    }

    #[test]
    fn textual_mime_matches_declared_text_formats() {
        assert!(textual_mime(&["text/plain".into()]));
        assert!(textual_mime(&[
            "application/problem+json; charset=utf-8".into()
        ]));
        assert!(!textual_mime(&["application/octet-stream".into()]));
    }

    #[test]
    fn success_envelope_preserves_absent_null_and_falsy_results() {
        assert_eq!(
            success_envelope(None, None, None),
            json!({ "status": "success" })
        );
        for result in [
            json!(null),
            json!(false),
            json!(0),
            json!(""),
            json!([]),
            json!({}),
        ] {
            assert_eq!(
                success_envelope(Some(result.clone()), None, None),
                json!({ "status": "success", "result": result })
            );
        }
    }

    #[test]
    fn structured_completion_errors_precede_stream_errors() {
        let schema = SchemaRef::new(SchemaGraph::anonymous(SchemaType::f64()));
        let structured = schema
            .unpack_json(&SchemaValue::F64(f64::NAN))
            .map(Some)
            .map_err(|error| GolemToolError::Invocation(error.to_string()));
        let result = finish_success(
            structured,
            Err(GolemToolError::Stream {
                channel: "stdout",
                message: "stream".to_string(),
            }),
            Ok(None),
        );
        assert!(matches!(result, Err(GolemToolError::Invocation(_))));

        let result = finish_success(
            Ok(Some(json!(1))),
            Err(GolemToolError::Stream {
                channel: "stdout",
                message: "stream".to_string(),
            }),
            Ok(None),
        );
        assert!(matches!(
            result,
            Err(GolemToolError::Stream {
                channel: "stdout",
                ..
            })
        ));
    }

    #[test]
    fn domain_errors_preserve_payload_presence_and_only_successful_streams() {
        assert_eq!(
            domain_error_value("absent".to_string(), None),
            json!({ "name": "absent" })
        );
        for payload in [json!(null), json!(false), json!(0)] {
            assert_eq!(
                domain_error_value("present".to_string(), Some(payload.clone())),
                json!({ "name": "present", "value": payload })
            );
        }

        let stdout = json!({ "data": "ok" });
        assert_eq!(
            domain_error_envelope(
                json!({ "name": "failed" }),
                Ok(Some(stdout.clone())),
                Err(GolemToolError::Stream {
                    channel: "stderr",
                    message: "failed".to_string(),
                }),
            ),
            json!({
                "status": "error",
                "error": { "name": "failed" },
                "stdout": stdout
            })
        );
    }

    #[test]
    fn defaults_fill_only_omitted_arguments() {
        let defaults = Map::from_iter([
            ("omitted".to_string(), json!("default")),
            ("null".to_string(), json!("default")),
            ("false".to_string(), json!(true)),
        ]);
        let mut arguments = Map::from_iter([
            ("null".to_string(), Value::Null),
            ("false".to_string(), Value::Bool(false)),
        ]);
        normalize_defaults(&defaults, &mut arguments);
        assert_eq!(
            arguments,
            Map::from_iter([
                ("null".to_string(), Value::Null),
                ("false".to_string(), Value::Bool(false)),
                ("omitted".to_string(), json!("default")),
            ])
        );
    }
}
