#[cfg(target_arch = "wasm32")]
use crate::golem::exec::types::Error as ExecError;

#[cfg(not(target_arch = "wasm32"))]
use crate::types::Error as ExecError;

pub type ExecResult<T> = Result<T, ExecError>;

pub mod messages {
    pub fn format_error(context: &str, error: impl std::fmt::Display) -> String {
        format!("{context}: {error}")
    }

    pub fn file_error(operation: &str, filename: &str, error: impl std::fmt::Display) -> String {
        format!("Failed to {operation} file '{filename}': {error}")
    }

    pub fn runtime_error(runtime: &str, operation: &str, error: impl std::fmt::Display) -> String {
        format!("{runtime} runtime error during {operation}: {error}")
    }

    pub fn validation_error(field: &str, reason: &str) -> String {
        format!("Validation failed for {field}: {reason}")
    }

    pub fn limit_error(
        resource: &str,
        limit: impl std::fmt::Display,
        actual: impl std::fmt::Display,
    ) -> String {
        format!("{resource} limit exceeded: {actual} > {limit}")
    }

    pub fn config_error(setting: &str, reason: &str) -> String {
        format!("Configuration error for {setting}: {reason}")
    }
}

pub mod convert {
    use super::*;

    pub fn io_error(context: &str, error: std::io::Error) -> ExecError {
        match error.kind() {
            std::io::ErrorKind::PermissionDenied => ExecError::Internal(messages::format_error(
                &format!("Permission denied during {context}"),
                error,
            )),
            std::io::ErrorKind::NotFound => ExecError::Internal(messages::format_error(
                &format!("Required component not found during {context}"),
                error,
            )),
            _ => ExecError::Internal(messages::format_error(
                &format!("IO error during {context}"),
                error,
            )),
        }
    }

    pub fn utf8_error(context: &str, error: std::string::FromUtf8Error) -> ExecError {
        ExecError::Internal(messages::format_error(
            &format!("Invalid UTF-8 in {context}"),
            error,
        ))
    }

    pub fn base64_error(context: &str, error: base64::DecodeError) -> ExecError {
        ExecError::Internal(messages::format_error(
            &format!("Base64 decode error in {context}"),
            error,
        ))
    }

    pub fn hex_error(context: &str, error: hex::FromHexError) -> ExecError {
        ExecError::Internal(messages::format_error(
            &format!("Hex decode error in {context}"),
            error,
        ))
    }

    pub fn json_error(context: &str, error: serde_json::Error) -> ExecError {
        ExecError::Internal(messages::format_error(
            &format!("JSON error in {context}"),
            error,
        ))
    }
}

pub mod validation {
    use super::*;

    pub fn empty_filename() -> ExecError {
        ExecError::Internal(messages::validation_error("filename", "cannot be empty"))
    }

    pub fn empty_file_content(filename: &str) -> ExecError {
        ExecError::Internal(messages::validation_error(
            &format!("file '{filename}' content"),
            "cannot be empty",
        ))
    }

    pub fn file_size_exceeded(filename: &str, size: usize, limit: usize) -> ExecError {
        ExecError::Internal(messages::limit_error(
            &format!("File '{filename}' size"),
            limit,
            size,
        ))
    }

    pub fn invalid_encoding(filename: &str, encoding: &str, reason: &str) -> ExecError {
        ExecError::Internal(messages::validation_error(
            &format!("file '{filename}' {encoding} encoding"),
            reason,
        ))
    }

    pub fn timeout_zero() -> ExecError {
        ExecError::Internal(messages::validation_error(
            "timeout",
            "must be greater than 0",
        ))
    }

    pub fn timeout_too_large(max_seconds: u64) -> ExecError {
        ExecError::Internal(messages::validation_error(
            "timeout",
            &format!("exceeds maximum allowed ({max_seconds} seconds)"),
        ))
    }

    pub fn memory_zero() -> ExecError {
        ExecError::Internal(messages::validation_error(
            "memory limit",
            "must be greater than 0",
        ))
    }

    pub fn memory_too_large(max_gb: u64) -> ExecError {
        ExecError::Internal(messages::validation_error(
            "memory limit",
            &format!("exceeds maximum allowed ({max_gb}GB)"),
        ))
    }

    pub fn process_count_zero() -> ExecError {
        ExecError::Internal(messages::validation_error(
            "process limit",
            "must be greater than 0",
        ))
    }

    pub fn process_count_too_large(max_processes: u32) -> ExecError {
        ExecError::Internal(messages::validation_error(
            "process limit",
            &format!("exceeds maximum allowed ({max_processes})"),
        ))
    }

    pub fn session_closed() -> ExecError {
        ExecError::Internal("Session is closed".to_string())
    }

    pub fn file_not_found(filename: &str) -> ExecError {
        ExecError::Internal(messages::validation_error(
            "file lookup",
            &format!("file '{filename}' not found"),
        ))
    }

    pub fn no_files_provided() -> ExecError {
        ExecError::Internal(messages::validation_error("input", "no files provided"))
    }

    pub fn no_entry_point() -> ExecError {
        ExecError::Internal(messages::validation_error(
            "entry point",
            "no suitable entry point file found",
        ))
    }

    pub fn path_traversal() -> ExecError {
        ExecError::Internal(messages::validation_error(
            "path",
            "path traversal not allowed",
        ))
    }

    pub fn absolute_path() -> ExecError {
        ExecError::Internal(messages::validation_error(
            "path",
            "absolute paths not allowed",
        ))
    }

    pub fn empty_path() -> ExecError {
        ExecError::Internal(messages::validation_error("path", "cannot be empty"))
    }

    pub fn invalid_path(path: &str) -> ExecError {
        ExecError::Internal(messages::validation_error(
            "path",
            &format!("'{path}' is not valid"),
        ))
    }

    pub fn filename_too_long() -> ExecError {
        ExecError::Internal(messages::validation_error("filename", "too long"))
    }

    pub fn invalid_working_dir() -> ExecError {
        ExecError::Internal(messages::validation_error(
            "working directory",
            "absolute paths outside /tmp and /workspace are not allowed",
        ))
    }

    pub fn empty_working_dir() -> ExecError {
        ExecError::Internal(messages::validation_error(
            "working directory",
            "path cannot be empty",
        ))
    }

    pub fn base64_decode_failed(reason: &str) -> ExecError {
        ExecError::Internal(messages::validation_error("base64 decoding", reason))
    }

    pub fn hex_decode_failed(reason: &str) -> ExecError {
        ExecError::Internal(messages::validation_error("hex decoding", reason))
    }

    pub fn json_parse_failed(reason: &str) -> ExecError {
        ExecError::Internal(messages::validation_error("JSON parsing", reason))
    }

    pub fn unsupported_language_version(language: &str, version: &str) -> ExecError {
        ExecError::Internal(messages::validation_error(
            &format!("{language} version"),
            &format!("unsupported version: {version}"),
        ))
    }

    pub fn decode_error(
        filename: &str,
        encoding: &str,
        error: impl std::fmt::Display,
    ) -> ExecError {
        ExecError::Internal(messages::validation_error(
            &format!("file '{filename}' {encoding} decoding"),
            &error.to_string(),
        ))
    }

    pub fn no_entry_point_found() -> ExecError {
        ExecError::Internal(messages::validation_error(
            "entry point",
            "no suitable entry point file found",
        ))
    }

    pub fn invalid_filename(reason: &str) -> ExecError {
        ExecError::Internal(messages::validation_error("filename", reason))
    }

    pub fn invalid_limit(reason: &str) -> ExecError {
        ExecError::Internal(messages::validation_error("limit", reason))
    }

    pub fn syntax_error(reason: &str) -> ExecError {
        ExecError::Internal(messages::validation_error("syntax", reason))
    }
}

pub mod runtime {
    use super::*;

    pub fn executable_not_found(runtime: &str, env_var: &str) -> ExecError {
        ExecError::Internal(messages::config_error(
            runtime,
            &format!("executable not found. Please install {runtime} or set {env_var} environment variable")
        ))
    }

    pub fn execution_failed(runtime: &str, error: impl std::fmt::Display) -> ExecError {
        ExecError::Internal(messages::runtime_error(runtime, "execution", error))
    }

    pub fn spawn_failed(runtime: &str, error: impl std::fmt::Display) -> ExecError {
        ExecError::Internal(messages::runtime_error(runtime, "process spawn", error))
    }

    pub fn wrapper_script_failed(runtime: &str, error: impl std::fmt::Display) -> ExecError {
        ExecError::Internal(messages::runtime_error(
            runtime,
            "wrapper script creation",
            error,
        ))
    }

    pub fn syntax_error(runtime: &str, details: &str) -> ExecError {
        ExecError::Internal(messages::runtime_error(
            runtime,
            "syntax validation",
            details,
        ))
    }

    pub fn execution_timeout() -> ExecError {
        ExecError::Timeout
    }

    pub fn memory_limit_exceeded(usage: u64, limit: u64) -> ExecError {
        ExecError::Internal(messages::limit_error("Memory", limit, usage))
    }

    pub fn process_limit_exceeded(count: u32, limit: u32) -> ExecError {
        ExecError::Internal(messages::limit_error("Process count", limit, count))
    }

    pub fn thread_disconnected() -> ExecError {
        ExecError::Internal("Thread disconnected".to_string())
    }

    pub fn invalid_timeout(reason: &str) -> ExecError {
        ExecError::Internal(messages::validation_error("timeout", reason))
    }

    pub fn invalid_memory_limit(reason: &str) -> ExecError {
        ExecError::Internal(messages::validation_error("memory limit", reason))
    }

    pub fn invalid_process_limit(reason: &str) -> ExecError {
        ExecError::Internal(messages::validation_error("process limit", reason))
    }

    pub fn invalid_file_size_limit(reason: &str) -> ExecError {
        ExecError::Internal(messages::validation_error("file size limit", reason))
    }

    pub fn file_size_limit_exceeded(filename: &str, size: u64, limit: u64) -> ExecError {
        ExecError::Internal(messages::limit_error(
            &format!("File '{}' size", filename),
            limit,
            size,
        ))
    }
}

pub mod stream {
    use super::*;

    pub fn stdout_capture_failed() -> ExecError {
        ExecError::Internal("Failed to capture stdout".to_string())
    }

    pub fn stderr_capture_failed() -> ExecError {
        ExecError::Internal("Failed to capture stderr".to_string())
    }

    pub fn stream_finished_without_result() -> ExecError {
        ExecError::Internal("Stream finished without result".to_string())
    }

    pub fn thread_disconnected() -> ExecError {
        ExecError::Internal("Thread disconnected".to_string())
    }

    pub fn process_error(error: impl std::fmt::Display) -> ExecError {
        ExecError::Internal(messages::format_error("Process error", error))
    }

    pub fn mutex_lock_failed() -> ExecError {
        ExecError::Internal("Failed to acquire mutex lock on stream".to_string())
    }
}

pub mod fs {
    use super::*;

    pub fn temp_dir_creation_failed(error: impl std::fmt::Display) -> ExecError {
        ExecError::Internal(messages::format_error(
            "Failed to create temporary directory",
            error,
        ))
    }

    pub fn dir_creation_failed(path: &str, error: impl std::fmt::Display) -> ExecError {
        ExecError::Internal(messages::format_error(
            &format!("Failed to create directory {path}"),
            error,
        ))
    }

    pub fn file_write_failed(filename: &str, error: impl std::fmt::Display) -> ExecError {
        ExecError::Internal(messages::file_error("write", filename, error))
    }

    pub fn file_read_failed(filename: &str, error: impl std::fmt::Display) -> ExecError {
        ExecError::Internal(messages::file_error("read", filename, error))
    }

    pub fn metadata_failed(path: &str, error: impl std::fmt::Display) -> ExecError {
        ExecError::Internal(messages::format_error(
            &format!("Failed to get metadata for {path}"),
            error,
        ))
    }

    pub fn permissions_failed(path: &str, error: impl std::fmt::Display) -> ExecError {
        ExecError::Internal(messages::format_error(
            &format!("Failed to set permissions for {path}"),
            error,
        ))
    }

    pub fn io_operation_failed(operation: &str, path: &str, error: &str) -> ExecError {
        ExecError::Internal(format!(
            "IO operation '{operation}' failed for {path}: {error}"
        ))
    }
}
