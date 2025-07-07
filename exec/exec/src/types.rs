use crate::error::{fs, validation};
use base64;
use std::collections::HashMap;
use std::time::Duration;

#[cfg(target_arch = "wasm32")]
pub use crate::golem::exec::types::*;

#[cfg(not(target_arch = "wasm32"))]
pub use local_types::*;

mod local_types {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub enum LanguageKind {
        Javascript,
        Python,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct Language {
        pub kind: LanguageKind,
        pub version: Option<String>,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
    pub enum Encoding {
        Utf8,
        Base64,
        Hex,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct File {
        pub name: String,
        pub content: Vec<u8>,
        pub encoding: Option<Encoding>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct Limits {
        pub time_ms: Option<u64>,
        pub memory_bytes: Option<u64>,
        pub file_size_bytes: Option<u64>,
        pub max_processes: Option<u32>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct StageResult {
        pub stdout: String,
        pub stderr: String,
        pub exit_code: Option<i32>,
        pub signal: Option<String>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct ExecResult {
        pub compile: Option<StageResult>,
        pub run: StageResult,
        pub time_ms: Option<u64>,
        pub memory_bytes: Option<u64>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub enum Error {
        Timeout,
        MemoryLimit,
        ProcessLimit,
        FileSizeLimit,
        InvalidInput(String),
        RuntimeError(String),
        CompileError(String),
        IoError(String),
        ValidationError(String),
        UnsupportedLanguage,
        ConfigurationError(String),
        Internal(String),
        CompilationFailed(StageResult),
        RuntimeFailed(StageResult),
        ResourceExceeded,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub enum ExecEvent {
        StdoutChunk(Vec<u8>),
        StderrChunk(Vec<u8>),
        Finished(ExecResult),
        Failed(Error),
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct FileInfo {
        pub path: String,
        pub size: u64,
        pub is_directory: bool,
        pub created_at: u64,
        pub modified_at: u64,
    }
}

#[cfg(target_arch = "wasm32")]
mod conversions {
    use super::local_types;
    use crate::golem::exec::types as wit_types;

    impl From<local_types::ExecResult> for wit_types::ExecResult {
        fn from(local: local_types::ExecResult) -> Self {
            wit_types::ExecResult {
                compile: local.compile.map(|stage| stage.into()),
                run: local.run.into(),
                time_ms: local.time_ms,
                memory_bytes: local.memory_bytes,
            }
        }
    }

    impl From<local_types::StageResult> for wit_types::StageResult {
        fn from(local: local_types::StageResult) -> Self {
            wit_types::StageResult {
                stdout: local.stdout,
                stderr: local.stderr,
                exit_code: local.exit_code,
                signal: local.signal,
            }
        }
    }

    impl From<local_types::Error> for wit_types::Error {
        fn from(local: local_types::Error) -> Self {
            match local {
                local_types::Error::UnsupportedLanguage => wit_types::Error::UnsupportedLanguage,
                local_types::Error::CompilationFailed(stage) => {
                    wit_types::Error::CompilationFailed(stage.into())
                }
                local_types::Error::RuntimeFailed(stage) => {
                    wit_types::Error::RuntimeFailed(stage.into())
                }
                local_types::Error::Timeout => wit_types::Error::Timeout,
                local_types::Error::ResourceExceeded => wit_types::Error::ResourceExceeded,
                local_types::Error::Internal(msg) => wit_types::Error::Internal(msg),
                local_types::Error::MemoryLimit => {
                    wit_types::Error::Internal("Memory limit exceeded".to_string())
                }
                local_types::Error::ProcessLimit => {
                    wit_types::Error::Internal("Process limit exceeded".to_string())
                }
                local_types::Error::FileSizeLimit => {
                    wit_types::Error::Internal("File size limit exceeded".to_string())
                }
                local_types::Error::InvalidInput(msg) => {
                    wit_types::Error::Internal(format!("Invalid input: {msg}"))
                }
                local_types::Error::RuntimeError(msg) => {
                    wit_types::Error::Internal(format!("Runtime error: {msg}"))
                }
                local_types::Error::CompileError(msg) => {
                    wit_types::Error::Internal(format!("Compile error: {msg}"))
                }
                local_types::Error::IoError(msg) => {
                    wit_types::Error::Internal(format!("IO error: {msg}"))
                }
                local_types::Error::ValidationError(msg) => {
                    wit_types::Error::Internal(format!("Validation error: {msg}"))
                }
                local_types::Error::ConfigurationError(msg) => {
                    wit_types::Error::Internal(format!("Configuration error: {msg}"))
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct SessionState {
    pub files: HashMap<String, File>,
    pub working_dir: String,
    pub closed: bool,
}

impl Default for SessionState {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionState {
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
            working_dir: "/tmp".to_string(),
            closed: false,
        }
    }

    pub fn add_file(&mut self, file: File) {
        self.files.insert(file.name.clone(), file);
    }

    pub fn remove_file(&mut self, name: &str) {
        self.files.remove(name);
    }

    pub fn list_files(&self) -> Vec<String> {
        self.files.keys().cloned().collect()
    }

    pub fn get_files(&self) -> Vec<File> {
        self.files.values().cloned().collect()
    }

    pub fn clear(&mut self) {
        self.files.clear();
    }

    pub fn set_working_dir(&mut self, path: String) {
        self.working_dir = path;
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub timeout_ms: u64,
    pub memory_limit_mb: Option<u64>,
    pub js_quickjs_path: Option<String>,
    pub python_wasi_path: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            timeout_ms: std::env::var("EXEC_TIMEOUT_MS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(5000),
            memory_limit_mb: std::env::var("EXEC_MEMORY_LIMIT_MB")
                .ok()
                .and_then(|s| s.parse().ok()),
            js_quickjs_path: std::env::var("EXEC_JS_QUICKJS_PATH").ok(),
            python_wasi_path: std::env::var("EXEC_PYTHON_WASI_PATH").ok(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub language: Language,
    pub files: Vec<File>,
    pub stdin: Option<String>,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub constraints: Option<Limits>,
    pub config: Config,
}

impl ExecutionContext {
    pub fn new(
        language: Language,
        files: Vec<File>,
        stdin: Option<String>,
        args: Vec<String>,
        env: Vec<(String, String)>,
        constraints: Option<Limits>,
    ) -> Self {
        Self {
            language,
            files,
            stdin,
            args,
            env,
            constraints,
            config: Config::default(),
        }
    }

    pub fn get_timeout(&self) -> Duration {
        let timeout_ms = self
            .constraints
            .as_ref()
            .and_then(|c| c.time_ms)
            .unwrap_or(self.config.timeout_ms);
        Duration::from_millis(timeout_ms)
    }

    pub fn get_memory_limit(&self) -> Option<u64> {
        self.constraints
            .as_ref()
            .and_then(|c| c.memory_bytes)
            .or_else(|| self.config.memory_limit_mb.map(|mb| mb * 1024 * 1024))
    }
}

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub result: ExecResult,
    pub duration: Duration,
}

pub trait IntoExecError {
    fn into_exec_error(self) -> Error;
}

impl IntoExecError for std::io::Error {
    fn into_exec_error(self) -> Error {
        fs::io_operation_failed("io operation", "", &self.to_string())
    }
}

impl IntoExecError for serde_json::Error {
    fn into_exec_error(self) -> Error {
        validation::json_parse_failed(&self.to_string())
    }
}

impl IntoExecError for base64::DecodeError {
    fn into_exec_error(self) -> Error {
        validation::base64_decode_failed(&self.to_string())
    }
}

pub mod utils {
    use super::*;
    use crate::encoding::*;

    pub fn decode_file_content(file: &File) -> Result<Vec<u8>, Error> {
        let encoding = file.encoding.unwrap_or(Encoding::Utf8);
        let content_str = String::from_utf8_lossy(&file.content);
        decode_content(&content_str, encoding)
    }

    pub fn encode_file_content(content: &[u8], encoding: Encoding) -> Result<Vec<u8>, Error> {
        let encoded_str = encode_content(content, encoding)?;
        Ok(encoded_str.as_bytes().to_vec())
    }

    pub fn create_stage_result(
        stdout: String,
        stderr: String,
        exit_code: Option<i32>,
        signal: Option<String>,
    ) -> StageResult {
        StageResult {
            stdout,
            stderr,
            exit_code,
            signal,
        }
    }

    pub fn create_error_result(error: Error) -> Result<ExecResult, Error> {
        Err(error)
    }

    pub fn create_success_result(
        compile: Option<StageResult>,
        run: StageResult,
        time_ms: Option<u64>,
        memory_bytes: Option<u64>,
    ) -> ExecResult {
        ExecResult {
            compile,
            run,
            time_ms,
            memory_bytes,
        }
    }
}
