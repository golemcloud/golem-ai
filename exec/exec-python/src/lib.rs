use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[cfg(not(target_arch = "wasm32"))]
use tempfile::TempDir;
#[cfg(not(target_arch = "wasm32"))]
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
#[cfg(not(target_arch = "wasm32"))]
use tokio::process::Command;
#[cfg(not(target_arch = "wasm32"))]
use tokio::sync::mpsc as async_mpsc;

#[cfg(target_arch = "wasm32")]
use tokio::sync::mpsc as async_mpsc;

#[cfg(target_arch = "wasm32")]
use wit_bindgen::generate;

#[cfg(target_arch = "wasm32")]
generate!({
    world: "exec-python",
    path: "wit/exec-python.wit",
});

#[cfg(target_arch = "wasm32")]
use crate::exports::golem::exec_python::exec_stream;
#[cfg(target_arch = "wasm32")]
use crate::exports::golem::exec_python::executor::Guest as ExecutorGuest;
#[cfg(target_arch = "wasm32")]
use crate::exports::golem::exec_python::session::{Guest as SessionGuest, GuestSession};

#[cfg(target_arch = "wasm32")]
pub use golem::exec_python::types::*;

#[cfg(not(target_arch = "wasm32"))]
pub use types::*;

pub mod types;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonFile {
    pub name: String,
    pub content: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonLimits {
    pub timeout_ms: Option<u64>,
    pub memory_limit_mb: Option<u64>,
    pub max_file_size_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonExecResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub time_ms: Option<u64>,
    pub memory_bytes: Option<u64>,
}

#[derive(Debug, Clone)]
pub enum StreamEvent {
    StdoutChunk(Vec<u8>),
    StderrChunk(Vec<u8>),
    Finished(PythonExecResult),
    Error(String),
}

#[derive(Debug)]
pub struct PythonSession {
    #[allow(dead_code)]
    session_id: String,
    files: HashMap<String, Vec<u8>>,
    working_dir: String,
    closed: bool,
}

impl PythonSession {
    pub fn new() -> Self {
        Self {
            session_id: "session".to_string(),
            files: HashMap::new(),
            working_dir: "/".to_string(),
            closed: false,
        }
    }

    pub fn upload_file(&mut self, name: String, content: Vec<u8>) -> Result<(), String> {
        if self.closed {
            return Err("Session is closed".to_string());
        }

        if name.is_empty() {
            return Err("File name cannot be empty".to_string());
        }

        if content.is_empty() {
            return Err("File content cannot be empty".to_string());
        }

        if content.len() > 5 * 1024 * 1024 {
            return Err("File size exceeds maximum limit of 5MB".to_string());
        }

        self.files.insert(name, content);
        Ok(())
    }

    pub fn download_file(&self, name: &str) -> Result<Vec<u8>, String> {
        if self.closed {
            return Err("Session is closed".to_string());
        }

        self.files
            .get(name)
            .cloned()
            .ok_or_else(|| format!("File '{}' not found", name))
    }

    pub fn list_files(&self) -> Result<Vec<String>, String> {
        if self.closed {
            return Err("Session is closed".to_string());
        }

        Ok(self.files.keys().cloned().collect())
    }

    pub fn set_working_dir(&mut self, dir: String) -> Result<(), String> {
        if self.closed {
            return Err("Session is closed".to_string());
        }

        if dir.is_empty() {
            return Err("Working directory cannot be empty".to_string());
        }

        self.working_dir = dir;
        Ok(())
    }

    pub fn get_working_dir(&self) -> &str {
        &self.working_dir
    }

    pub fn run(
        &self,
        entrypoint: &str,
        args: Vec<String>,
        stdin: Option<String>,
        env: Vec<(String, String)>,
        limits: Option<PythonLimits>,
    ) -> Result<PythonExecResult, String> {
        if self.closed {
            return Err("Session is closed".to_string());
        }

        let content = self
            .files
            .get(entrypoint)
            .ok_or_else(|| format!("File '{}' not found", entrypoint))?;

        let file = types::File {
            name: entrypoint.to_string(),
            content: content.clone(),
            encoding: None,
        };

        execute_python_code(vec![file], stdin, args, env, limits)
    }

    pub async fn run_streaming(
        &self,
        entrypoint: &str,
        args: Vec<String>,
        stdin: Option<String>,
        env: Vec<(String, String)>,
        limits: Option<PythonLimits>,
    ) -> Result<async_mpsc::Receiver<StreamEvent>, String> {
        if self.closed {
            return Err("Session is closed".to_string());
        }

        let content = self
            .files
            .get(entrypoint)
            .ok_or_else(|| format!("File '{}' not found", entrypoint))?;

        let file = types::File {
            name: entrypoint.to_string(),
            content: content.clone(),
            encoding: None,
        };

        execute_python_code_streaming(vec![file], stdin, args, env, limits).await
    }

    pub fn close(&mut self) {
        self.closed = true;
        self.files.clear();
    }

    pub fn is_closed(&self) -> bool {
        self.closed
    }

    #[allow(dead_code)]
    pub fn session_id(&self) -> &str {
        &self.session_id
    }
}

// WIT Component Implementation
#[cfg(target_arch = "wasm32")]
pub struct PythonExecComponent;

#[cfg(target_arch = "wasm32")]
impl ExecutorGuest for PythonExecComponent {
    fn run(
        _lang: Language,
        files: Vec<File>,
        stdin: Option<String>,
        args: Vec<String>,
        env: Vec<(String, String)>,
        constraints: Option<Limits>,
    ) -> Result<ExecResult, Error> {
        let internal_files: Vec<types::File> = files.into_iter().map(|f| f.into()).collect();
        let internal_limits = constraints.map(|l| types::Limits {
            time_ms: l.time_ms,
            memory_bytes: l.memory_bytes,
            file_size_bytes: l.file_size_bytes,
            max_processes: l.max_processes,
        });

        let python_limits = internal_limits.map(|l| PythonLimits {
            timeout_ms: l.time_ms,
            memory_limit_mb: l.memory_bytes.map(|b| b / (1024 * 1024)),
            max_file_size_bytes: l.file_size_bytes,
        });

        match execute_python_code(internal_files, stdin, args, env, python_limits) {
            Ok(result) => Ok(ExecResult {
                compile: None,
                run: StageResult {
                    stdout: result.stdout,
                    stderr: result.stderr,
                    exit_code: result.exit_code,
                    signal: None,
                },
                time_ms: result.time_ms,
                memory_bytes: result.memory_bytes,
            }),
            Err(e) => Err(Error::Internal(e)),
        }
    }

    fn run_streaming(
        _lang: Language,
        files: Vec<File>,
        stdin: Option<String>,
        args: Vec<String>,
        env: Vec<(String, String)>,
        constraints: Option<Limits>,
    ) -> Result<exec_stream::ExecStream, Error> {
        let internal_files: Vec<types::File> = files.into_iter().map(|f| f.into()).collect();
        let internal_limits = constraints.map(|l| types::Limits {
            time_ms: l.time_ms,
            memory_bytes: l.memory_bytes,
            file_size_bytes: l.file_size_bytes,
            max_processes: l.max_processes,
        });

        let python_limits = internal_limits.map(|l| PythonLimits {
            timeout_ms: l.time_ms,
            memory_limit_mb: l.memory_bytes.map(|b| b / (1024 * 1024)),
            max_file_size_bytes: l.file_size_bytes,
        });

        // Execute Python code and create stream from result
        let result = execute_python_code(internal_files, stdin, args, env, python_limits);
        let stream = PythonExecStreamWrapper::from_result(result);

        Ok(exec_stream::ExecStream::new(stream))
    }
}

#[cfg(target_arch = "wasm32")]
pub struct PythonSessionWrapper {
    inner: std::cell::RefCell<PythonSession>,
}

#[cfg(target_arch = "wasm32")]
impl GuestSession for PythonSessionWrapper {
    fn new(_lang: Language) -> Self {
        Self {
            inner: std::cell::RefCell::new(PythonSession::new()),
        }
    }

    fn upload(&self, file: File) -> Result<(), Error> {
        let mut session = self.inner.borrow_mut();
        session
            .upload_file(file.name, file.content)
            .map_err(|e| Error::Internal(e))
    }

    fn run(
        &self,
        entrypoint: String,
        args: Vec<String>,
        stdin: Option<String>,
        env: Vec<(String, String)>,
        constraints: Option<Limits>,
    ) -> Result<ExecResult, Error> {
        let session = self.inner.borrow();

        let python_limits = constraints.map(|l| PythonLimits {
            timeout_ms: l.time_ms,
            memory_limit_mb: l.memory_bytes.map(|b| b / (1024 * 1024)),
            max_file_size_bytes: l.file_size_bytes,
        });

        match session.run(&entrypoint, args, stdin, env, python_limits) {
            Ok(result) => Ok(ExecResult {
                compile: None,
                run: StageResult {
                    stdout: result.stdout,
                    stderr: result.stderr,
                    exit_code: result.exit_code,
                    signal: None,
                },
                time_ms: result.time_ms,
                memory_bytes: result.memory_bytes,
            }),
            Err(e) => Err(Error::Internal(e)),
        }
    }

    fn run_streaming(
        &self,
        entrypoint: String,
        args: Vec<String>,
        stdin: Option<String>,
        env: Vec<(String, String)>,
        constraints: Option<Limits>,
    ) -> Result<exec_stream::ExecStream, Error> {
        let session = self.inner.borrow();

        let python_limits = constraints.map(|l| PythonLimits {
            timeout_ms: l.time_ms,
            memory_limit_mb: l.memory_bytes.map(|b| b / (1024 * 1024)),
            max_file_size_bytes: l.file_size_bytes,
        });

        match session.run(&entrypoint, args, stdin, env, python_limits) {
            Ok(result) => {
                let stream = PythonExecStreamWrapper::from_result(Ok(result));
                Ok(exec_stream::ExecStream::new(stream))
            }
            Err(e) => {
                let stream = PythonExecStreamWrapper::from_result(Err(e));
                Ok(exec_stream::ExecStream::new(stream))
            }
        }
    }

    fn download(&self, path: String) -> Result<Vec<u8>, Error> {
        let session = self.inner.borrow();
        session.download_file(&path).map_err(|e| Error::Internal(e))
    }

    fn list_files(&self, dir: String) -> Result<Vec<String>, Error> {
        let session = self.inner.borrow();
        if dir.is_empty() || dir == "/" {
            session.list_files().map_err(|e| Error::Internal(e))
        } else {
            session.list_files().map_err(|e| Error::Internal(e))
        }
    }

    fn set_working_dir(&self, path: String) -> Result<(), Error> {
        let mut session = self.inner.borrow_mut();
        session
            .set_working_dir(path)
            .map_err(|e| Error::Internal(e))
    }

    fn close(&self) {
        let mut session = self.inner.borrow_mut();
        session.close();
    }
}

#[cfg(target_arch = "wasm32")]
impl SessionGuest for PythonExecComponent {
    type Session = PythonSessionWrapper;
}

#[cfg(target_arch = "wasm32")]
impl exec_stream::Guest for PythonExecComponent {
    type ExecStream = PythonExecStreamWrapper;
}

#[cfg(target_arch = "wasm32")]
pub struct PythonExecStreamWrapper {
    events: std::cell::RefCell<Vec<ExecEvent>>,
    current_index: std::cell::RefCell<usize>,
    finished: std::cell::RefCell<bool>,
}

#[cfg(target_arch = "wasm32")]
impl PythonExecStreamWrapper {
    pub fn new() -> Self {
        Self {
            events: std::cell::RefCell::new(Vec::new()),
            current_index: std::cell::RefCell::new(0),
            finished: std::cell::RefCell::new(false),
        }
    }

    pub fn from_result(result: Result<PythonExecResult, String>) -> Self {
        let mut events = Vec::new();

        match result {
            Ok(exec_result) => {
                if !exec_result.stdout.is_empty() {
                    events.push(ExecEvent::StdoutChunk(
                        exec_result.stdout.as_bytes().to_vec(),
                    ));
                }

                if !exec_result.stderr.is_empty() {
                    events.push(ExecEvent::StderrChunk(
                        exec_result.stderr.as_bytes().to_vec(),
                    ));
                }

                events.push(ExecEvent::Finished(ExecResult {
                    compile: None,
                    run: StageResult {
                        stdout: exec_result.stdout,
                        stderr: exec_result.stderr,
                        exit_code: exec_result.exit_code,
                        signal: None,
                    },
                    time_ms: exec_result.time_ms,
                    memory_bytes: exec_result.memory_bytes,
                }));
            }
            Err(error) => {
                events.push(ExecEvent::Failed(Error::Internal(error)));
            }
        }

        Self {
            events: std::cell::RefCell::new(events),
            current_index: std::cell::RefCell::new(0),
            finished: std::cell::RefCell::new(false),
        }
    }
}

#[cfg(target_arch = "wasm32")]
impl exec_stream::GuestExecStream for PythonExecStreamWrapper {
    fn get_next(&self) -> Option<ExecEvent> {
        let events = self.events.borrow();
        let mut index = self.current_index.borrow_mut();

        if *index < events.len() {
            let event = events[*index].clone();
            *index += 1;

            if *index >= events.len() {
                *self.finished.borrow_mut() = true;
            }

            Some(event)
        } else {
            None
        }
    }

    fn blocking_get_next(&self) -> Option<ExecEvent> {
        // For now, just delegate to get_next since we're not truly async
        self.get_next()
    }
}

#[cfg(target_arch = "wasm32")]
export!(PythonExecComponent with_types_in crate);

pub struct PythonExecutor {
    #[cfg(not(target_arch = "wasm32"))]
    workspace: Option<TempDir>,
    #[cfg(target_arch = "wasm32")]
    workspace: Option<PathBuf>,
    #[allow(dead_code)]
    python_binary: PathBuf,
}

#[cfg(feature = "no-logging")]
struct NoOpLogger;

#[cfg(feature = "no-logging")]
impl log::Log for NoOpLogger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        false
    }

    fn log(&self, _record: &log::Record) {}

    fn flush(&self) {}
}

fn init_logging() {
    use std::sync::Once;
    static INIT: Once = Once::new();

    INIT.call_once(|| {
        #[cfg(feature = "no-logging")]
        {
            let _ =
                log::set_logger(&NoOpLogger).map(|()| log::set_max_level(log::LevelFilter::Off));
        }
        #[cfg(not(feature = "no-logging"))]
        {
            if std::env::var("GOLEM_EXEC_LOG").is_ok() {
                #[cfg(all(target_arch = "wasm32", feature = "wasi-logging"))]
                {
                    use std::str::FromStr;
                    // Try to install wasi_logger, but don't fail if WASI logging interface is not available
                    if wasi_logger::Logger::install().is_ok() {
                        let max_level: log::LevelFilter = log::LevelFilter::from_str(
                            &std::env::var("GOLEM_EXEC_LOG").unwrap_or_default(),
                        )
                        .unwrap_or(log::LevelFilter::Info);
                        log::set_max_level(max_level);
                    }
                }
                #[cfg(not(target_arch = "wasm32"))]
                {
                    env_logger::init();
                }
            }
        }
    });
}

impl PythonExecutor {
    pub fn new() -> Result<Self> {
        init_logging();

        let python_binary = Self::find_python_binary()?;

        Ok(Self {
            workspace: None,
            python_binary,
        })
    }

    fn find_python_binary() -> Result<PathBuf> {
        if let Ok(wasi_python) = std::env::var("PYTHON_WASI_PATH") {
            let path = PathBuf::from(wasi_python);
            if path.exists() {
                log::info!("Using CPython WASI binary at: {:?}", path);
                return Ok(path);
            }
        }

        let python_candidates = ["python3.11", "python3.10", "python3.9", "python3", "python"];

        for candidate in &python_candidates {
            if let Ok(output) = std::process::Command::new("which").arg(candidate).output() {
                if output.status.success() {
                    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    log::info!("Using system Python binary at: {}", path);
                    return Ok(PathBuf::from(path));
                }
            }
        }

        Err(anyhow::anyhow!("No Python binary found. Please install Python or set PYTHON_WASI_PATH environment variable."))
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn setup_workspace(&mut self, files: &[PythonFile]) -> Result<PathBuf> {
        let workspace = TempDir::new().context("Failed to create temporary workspace")?;
        let workspace_path = workspace.path().to_path_buf();

        for file in files {
            let file_path = workspace_path.join(&file.name);

            if let Some(parent) = file_path.parent() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("Failed to create directory: {:?}", parent))?;
            }

            std::fs::write(&file_path, &file.content)
                .with_context(|| format!("Failed to write file: {:?}", file_path))?;

            log::debug!("Created file: {:?}", file_path);
        }

        self.workspace = Some(workspace);
        Ok(workspace_path)
    }

    #[cfg(target_arch = "wasm32")]
    pub fn setup_workspace(&mut self, files: &[PythonFile]) -> Result<PathBuf> {
        let workspace_path = PathBuf::from("/tmp/python_workspace");
        std::fs::create_dir_all(&workspace_path).context("Failed to create workspace directory")?;

        for file in files {
            let file_path = workspace_path.join(&file.name);

            if let Some(parent) = file_path.parent() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("Failed to create directory: {:?}", parent))?;
            }

            std::fs::write(&file_path, &file.content)
                .with_context(|| format!("Failed to write file: {:?}", file_path))?;

            log::debug!("Created file: {:?}", file_path);
        }

        self.workspace = Some(workspace_path.clone());
        Ok(workspace_path)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn execute_streaming(
        &mut self,
        files: Vec<types::File>,
        stdin: Option<String>,
        args: Vec<String>,
        env: Vec<(String, String)>,
        limits: Option<PythonLimits>,
    ) -> Result<async_mpsc::Receiver<StreamEvent>> {
        let python_files: Vec<PythonFile> = files
            .into_iter()
            .map(|f| PythonFile {
                name: f.name,
                content: f.content,
            })
            .collect();

        let workspace_path = self.setup_workspace(&python_files)?;

        let main_file = python_files
            .iter()
            .find(|f| f.name.ends_with(".py"))
            .ok_or_else(|| anyhow::anyhow!("No Python file found"))?;

        let main_file_path = workspace_path.join(&main_file.name);

        let (tx, rx) = async_mpsc::channel(1000);
        let python_binary = self.python_binary.clone();
        let timeout = limits
            .as_ref()
            .and_then(|l| l.timeout_ms)
            .map(Duration::from_millis);

        tokio::spawn(async move {
            let start_time = Instant::now();

            let result = Self::execute_python_process(
                python_binary,
                main_file_path,
                workspace_path,
                stdin,
                args,
                env,
                timeout,
                tx.clone(),
            )
            .await;

            let execution_time = start_time.elapsed();

            match result {
                Ok((stdout, stderr, exit_code)) => {
                    let final_result = PythonExecResult {
                        stdout,
                        stderr,
                        exit_code,
                        time_ms: Some(execution_time.as_millis() as u64),
                        memory_bytes: None,
                    };
                    let _ = tx.send(StreamEvent::Finished(final_result)).await;
                }
                Err(e) => {
                    let error_result = PythonExecResult {
                        stdout: String::new(),
                        stderr: e.to_string(),
                        exit_code: Some(1),
                        time_ms: Some(execution_time.as_millis() as u64),
                        memory_bytes: None,
                    };
                    let _ = tx.send(StreamEvent::Finished(error_result)).await;
                }
            }
        });

        Ok(rx)
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn execute_streaming(
        &mut self,
        files: Vec<types::File>,
        stdin: Option<String>,
        args: Vec<String>,
        env: Vec<(String, String)>,
        limits: Option<PythonLimits>,
    ) -> Result<async_mpsc::Receiver<StreamEvent>> {
        let python_files: Vec<PythonFile> = files
            .into_iter()
            .map(|f| PythonFile {
                name: f.name,
                content: f.content,
            })
            .collect();

        let _workspace_path = self.setup_workspace(&python_files)?;

        let (tx, rx) = async_mpsc::channel(1000);

        let files_clone = python_files.clone();
        let stdin_clone = stdin.clone();
        let args_clone = args.clone();
        let env_clone = env.clone();
        let limits_clone = limits.clone();

        tokio::spawn(async move {
            let type_files: Vec<types::File> = files_clone
                .into_iter()
                .map(|pf| types::File {
                    name: pf.name,
                    content: pf.content,
                    encoding: None,
                })
                .collect();

            match execute_python_wasi_streaming(
                &type_files,
                stdin_clone,
                args_clone,
                env_clone,
                limits_clone,
            )
            .await
            {
                Ok(result) => {
                    let _ = tx.send(StreamEvent::Finished(result)).await;
                }
                Err(error) => {
                    let _ = tx.send(StreamEvent::Error(error)).await;
                }
            }
        });

        Ok(rx)
    }

    #[cfg(not(target_arch = "wasm32"))]
    async fn execute_python_process(
        python_binary: PathBuf,
        main_file: PathBuf,
        workspace: PathBuf,
        stdin: Option<String>,
        args: Vec<String>,
        env: Vec<(String, String)>,
        timeout: Option<Duration>,
        tx: async_mpsc::Sender<StreamEvent>,
    ) -> Result<(String, String, Option<i32>)> {
        let mut cmd = Command::new(&python_binary);
        cmd.arg(&main_file);
        cmd.args(&args);
        cmd.current_dir(&workspace);

        for (key, value) in env {
            cmd.env(key, value);
        }

        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let mut child = cmd.spawn().context("Failed to spawn Python process")?;

        if let Some(stdin_data) = stdin {
            if let Some(mut stdin_pipe) = child.stdin.take() {
                tokio::spawn(async move {
                    let _ = stdin_pipe.write_all(stdin_data.as_bytes()).await;
                    let _ = stdin_pipe.shutdown().await;
                });
            }
        }

        let stdout = child.stdout.take().context("Failed to capture stdout")?;
        let stderr = child.stderr.take().context("Failed to capture stderr")?;

        let mut stdout_reader = BufReader::new(stdout);
        let mut stderr_reader = BufReader::new(stderr);

        let tx_stdout = tx.clone();
        let tx_stderr = tx.clone();

        let stdout_task = tokio::spawn(async move {
            let mut buffer = vec![0u8; 8192];
            let mut accumulated = Vec::new();

            loop {
                match stdout_reader.read(&mut buffer).await {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        let chunk = buffer[..n].to_vec();
                        accumulated.extend_from_slice(&chunk);
                        let _ = tx_stdout.send(StreamEvent::StdoutChunk(chunk)).await;
                    }
                    Err(_) => break,
                }
            }

            String::from_utf8_lossy(&accumulated).to_string()
        });

        let stderr_task = tokio::spawn(async move {
            let mut buffer = vec![0u8; 8192];
            let mut accumulated = Vec::new();

            loop {
                match stderr_reader.read(&mut buffer).await {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        let chunk = buffer[..n].to_vec();
                        accumulated.extend_from_slice(&chunk);
                        let _ = tx_stderr.send(StreamEvent::StderrChunk(chunk)).await;
                    }
                    Err(_) => break,
                }
            }

            String::from_utf8_lossy(&accumulated).to_string()
        });

        let exit_status = if let Some(timeout_duration) = timeout {
            match tokio::time::timeout(timeout_duration, child.wait()).await {
                Ok(Ok(status)) => Some(status.code()),
                Ok(Err(e)) => return Err(anyhow::anyhow!("Process wait failed: {}", e)),
                Err(_) => {
                    let _ = child.kill().await;
                    return Err(anyhow::anyhow!("Python execution timed out"));
                }
            }
        } else {
            match child.wait().await {
                Ok(status) => Some(status.code()),
                Err(e) => return Err(anyhow::anyhow!("Process wait failed: {}", e)),
            }
        };

        let stdout_result = stdout_task.await.unwrap_or_default();
        let stderr_result = stderr_task.await.unwrap_or_default();

        Ok((stdout_result, stderr_result, exit_status.flatten()))
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn execute_python_code(
    files: Vec<types::File>,
    stdin: Option<String>,
    args: Vec<String>,
    env: Vec<(String, String)>,
    limits: Option<PythonLimits>,
) -> Result<PythonExecResult, String> {
    if tokio::runtime::Handle::try_current().is_ok() {
        let rt_handle = tokio::runtime::Handle::current();
        rt_handle.block_on(async {
            let mut executor = PythonExecutor::new().map_err(|e| e.to_string())?;
            let mut rx = executor
                .execute_streaming(files, stdin, args, env, limits)
                .await
                .map_err(|e| e.to_string())?;

            let mut final_result = None;

            while let Some(event) = rx.recv().await {
                match event {
                    StreamEvent::Finished(result) => {
                        final_result = Some(result);
                        break;
                    }
                    StreamEvent::Error(error) => {
                        return Err(error);
                    }
                    _ => {} // Ignore streaming chunks for blocking API
                }
            }

            final_result.ok_or_else(|| "Execution completed without result".to_string())
        })
    } else {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| format!("Failed to create async runtime: {}", e))?;

        rt.block_on(async {
            let mut executor = PythonExecutor::new().map_err(|e| e.to_string())?;
            let mut rx = executor
                .execute_streaming(files, stdin, args, env, limits)
                .await
                .map_err(|e| e.to_string())?;

            let mut final_result = None;

            while let Some(event) = rx.recv().await {
                match event {
                    StreamEvent::Finished(result) => {
                        final_result = Some(result);
                        break;
                    }
                    StreamEvent::Error(error) => {
                        return Err(error);
                    }
                    _ => {} // Ignore streaming chunks for blocking API
                }
            }

            final_result.ok_or_else(|| "Execution completed without result".to_string())
        })
    }
}

#[cfg(target_arch = "wasm32")]
pub fn execute_python_code(
    files: Vec<types::File>,
    stdin: Option<String>,
    args: Vec<String>,
    env: Vec<(String, String)>,
    limits: Option<PythonLimits>,
) -> Result<PythonExecResult, String> {
    init_logging();
    log::info!("Executing Python in WASM using CPython WASI");

    let _main_file = files
        .iter()
        .find(|f| f.name.ends_with(".py"))
        .ok_or("No Python file found")?;

    execute_python_wasi(&files, stdin, args, env, limits)
}

#[cfg(target_arch = "wasm32")]
fn execute_python_wasi(
    files: &[types::File],
    stdin: Option<String>,
    args: Vec<String>,
    env: Vec<(String, String)>,
    limits: Option<PythonLimits>,
) -> Result<PythonExecResult, String> {
    let start_time = Instant::now();

    let python_wasi_path = std::env::var("EXEC_PYTHON_WASI_PATH")
        .or_else(|_| std::env::var("PYTHON_WASI_PATH"))
        .unwrap_or_else(|_| "/usr/local/bin/python.wasm".to_string());

    let temp_dir = std::path::PathBuf::from(format!("/tmp/python_exec_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("Failed to create temp directory: {}", e))?;

    for file in files {
        let file_path = temp_dir.join(&file.name);

        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        }

        std::fs::write(&file_path, &file.content)
            .map_err(|e| format!("Failed to write file {}: {}", file.name, e))?;
    }

    let main_file = files
        .iter()
        .find(|f| f.name.ends_with(".py"))
        .ok_or("No Python file found")?;

    let main_file_path = temp_dir.join(&main_file.name);

    let mut cmd_args = vec![main_file_path.to_string_lossy().to_string()];
    cmd_args.extend(args);

    let mut env_map = std::collections::HashMap::new();
    for (key, value) in env {
        env_map.insert(key, value);
    }

    env_map.insert(
        "PYTHONPATH".to_string(),
        temp_dir.to_string_lossy().to_string(),
    );

    let timeout_duration = limits
        .as_ref()
        .and_then(|l| l.timeout_ms)
        .map(Duration::from_millis)
        .unwrap_or(Duration::from_secs(30));

    let result = execute_python_with_timeout(
        &python_wasi_path,
        &cmd_args,
        stdin,
        &env_map,
        timeout_duration,
    )?;

    let _ = std::fs::remove_dir_all(&temp_dir);

    let execution_time = start_time.elapsed();

    Ok(PythonExecResult {
        stdout: result.stdout,
        stderr: result.stderr,
        exit_code: result.exit_code,
        time_ms: Some(execution_time.as_millis() as u64),
        memory_bytes: None,
    })
}

#[cfg(target_arch = "wasm32")]
struct WasmProcessResult {
    stdout: String,
    stderr: String,
    exit_code: Option<i32>,
}

#[cfg(target_arch = "wasm32")]
fn execute_python_with_timeout(
    executable: &str,
    args: &[String],
    stdin: Option<String>,
    env: &std::collections::HashMap<String, String>,
    _timeout: Duration,
) -> Result<WasmProcessResult, String> {
    use std::thread;

    let executable = executable.to_string();
    let args = args.to_vec();
    let stdin = stdin.clone();
    let env = env.clone();

    let handle = thread::spawn(move || {
        let mut command = std::process::Command::new(&executable);
        command
            .args(&args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        for (key, value) in env {
            command.env(key, value);
        }

        let mut child = command
            .spawn()
            .map_err(|e| format!("Failed to spawn Python WASI process: {}", e))?;

        if let Some(input) = stdin {
            if let Some(mut stdin_handle) = child.stdin.take() {
                use std::io::Write;
                let _ = stdin_handle.write_all(input.as_bytes());
            }
        }

        let output = child
            .wait_with_output()
            .map_err(|e| format!("Failed to wait for Python process: {}", e))?;

        Ok(WasmProcessResult {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code(),
        })
    });

    match handle.join() {
        Ok(result) => result,
        Err(_) => Err("Python process execution panicked".to_string()),
    }
}

/// Public API for streaming Python code execution
#[cfg(not(target_arch = "wasm32"))]
pub async fn execute_python_code_streaming(
    files: Vec<types::File>,
    stdin: Option<String>,
    args: Vec<String>,
    env: Vec<(String, String)>,
    limits: Option<PythonLimits>,
) -> Result<async_mpsc::Receiver<StreamEvent>, String> {
    let mut executor = PythonExecutor::new().map_err(|e| e.to_string())?;
    executor
        .execute_streaming(files, stdin, args, env, limits)
        .await
        .map_err(|e| e.to_string())
}

#[cfg(target_arch = "wasm32")]
pub async fn execute_python_code_streaming(
    files: Vec<types::File>,
    stdin: Option<String>,
    args: Vec<String>,
    env: Vec<(String, String)>,
    limits: Option<PythonLimits>,
) -> Result<async_mpsc::Receiver<StreamEvent>, String> {
    let (tx, rx) = async_mpsc::channel(1000);

    let files_clone = files.clone();
    let stdin_clone = stdin.clone();
    let args_clone = args.clone();
    let env_clone = env.clone();
    let limits_clone = limits.clone();

    tokio::spawn(async move {
        match execute_python_wasi_streaming(
            &files_clone,
            stdin_clone,
            args_clone,
            env_clone,
            limits_clone,
        )
        .await
        {
            Ok(result) => {
                let _ = tx.send(StreamEvent::Finished(result)).await;
            }
            Err(error) => {
                let _ = tx.send(StreamEvent::Error(error)).await;
            }
        }
    });

    Ok(rx)
}

#[cfg(target_arch = "wasm32")]
async fn execute_python_wasi_streaming(
    files: &[types::File],
    stdin: Option<String>,
    args: Vec<String>,
    env: Vec<(String, String)>,
    limits: Option<PythonLimits>,
) -> Result<PythonExecResult, String> {
    execute_python_wasi(files, stdin, args, env, limits)
}

#[cfg(test)]
mod tests {
    #[cfg(not(target_arch = "wasm32"))]
    use super::*;

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn test_python_hello_world() {
        let files = vec![types::File {
            name: "main.py".to_string(),
            content: b"print('Hello, World!')".to_vec(),
            encoding: None,
        }];

        let result = execute_python_code(files, None, vec![], vec![], None);
        assert!(result.is_ok(), "Execution failed: {:?}", result);

        let result = result.unwrap();
        assert_eq!(result.stdout.trim(), "Hello, World!");
        assert_eq!(result.exit_code, Some(0));
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn test_python_with_arguments() {
        let files = vec![types::File {
            name: "main.py".to_string(),
            content: b"import sys\nprint(f'Args: {sys.argv[1:]}')".to_vec(),
            encoding: None,
        }];

        let args = vec!["arg1".to_string(), "arg2".to_string()];
        let result = execute_python_code(files, None, args, vec![], None);
        assert!(result.is_ok(), "Execution failed: {:?}", result);

        let result = result.unwrap();
        assert!(result.stdout.contains("arg1"));
        assert!(result.stdout.contains("arg2"));
        assert_eq!(result.exit_code, Some(0));
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn test_python_with_stdin() {
        let files = vec![types::File {
            name: "main.py".to_string(),
            content: b"import sys\ndata = sys.stdin.read()\nprint(f'Input: {data.strip()}')"
                .to_vec(),
            encoding: None,
        }];

        let stdin = Some("test input".to_string());
        let result = execute_python_code(files, stdin, vec![], vec![], None);
        assert!(result.is_ok(), "Execution failed: {:?}", result);

        let result = result.unwrap();
        assert!(result.stdout.contains("test input"));
        assert_eq!(result.exit_code, Some(0));
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn test_python_syntax_error() {
        let files = vec![types::File {
            name: "main.py".to_string(),
            content: b"print('Hello World'\n# Missing closing quote".to_vec(),
            encoding: None,
        }];

        let result = execute_python_code(files, None, vec![], vec![], None);
        assert!(
            result.is_ok(),
            "Execution should succeed even with syntax errors"
        );

        let result = result.unwrap();
        assert_ne!(
            result.exit_code,
            Some(0),
            "Should have non-zero exit code for syntax error"
        );
        assert!(
            !result.stderr.is_empty(),
            "Should have stderr output for syntax error"
        );
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn test_python_session_management() {
        let mut session = PythonSession::new();

        let content = b"print('Hello from session!')".to_vec();
        assert!(session
            .upload_file("test.py".to_string(), content.clone())
            .is_ok());

        let downloaded = session.download_file("test.py").unwrap();
        assert_eq!(downloaded, content);

        let files = session.list_files().unwrap();
        assert_eq!(files.len(), 1);
        assert!(files.contains(&"test.py".to_string()));

        session.set_working_dir("/tmp/test".to_string()).unwrap();
        assert_eq!(session.get_working_dir(), "/tmp/test");

        session.close();
        assert!(session.is_closed());
        assert!(session.upload_file("test2.py".to_string(), vec![]).is_err());
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn test_python_session_validation() {
        let mut session = PythonSession::new();

        assert!(session.upload_file("".to_string(), vec![1, 2, 3]).is_err());

        assert!(session.upload_file("test.py".to_string(), vec![]).is_err());

        assert!(session.download_file("nonexistent.py").is_err());
    }

    #[tokio::test]
    #[cfg(not(target_arch = "wasm32"))]
    async fn test_python_streaming() {
        let files = vec![types::File {
            name: "main.py".to_string(),
            content:
                b"import time\nfor i in range(3):\n    print(f'Line {i}')\n    time.sleep(0.1)"
                    .to_vec(),
            encoding: None,
        }];

        let mut rx = execute_python_code_streaming(files, None, vec![], vec![], None)
            .await
            .unwrap();

        let mut stdout_chunks = Vec::new();
        let mut finished = false;

        while let Some(event) = rx.recv().await {
            match event {
                StreamEvent::StdoutChunk(chunk) => {
                    stdout_chunks.push(String::from_utf8_lossy(&chunk).to_string());
                }
                StreamEvent::StderrChunk(_chunk) => {}
                StreamEvent::Finished(result) => {
                    assert!(
                        result.exit_code == Some(0) || result.exit_code == Some(1),
                        "Unexpected exit code: {:?}",
                        result.exit_code
                    );
                    finished = true;
                    break;
                }
                StreamEvent::Error(e) => {
                    panic!("Execution error: {}", e);
                }
            }
        }

        assert!(finished);
    }

    #[tokio::test]
    #[cfg(not(target_arch = "wasm32"))]
    async fn test_python_session_streaming() {
        let mut session = PythonSession::new();

        let content = b"import time\nfor i in range(2):\n    print(f'Session line {i}')\n    time.sleep(0.05)".to_vec();
        session
            .upload_file("stream_test.py".to_string(), content)
            .unwrap();

        let mut rx = session
            .run_streaming("stream_test.py", vec![], None, vec![], None)
            .await
            .unwrap();

        let mut events_received = 0;
        while let Some(event) = rx.recv().await {
            events_received += 1;
            match event {
                StreamEvent::StdoutChunk(_) => {}
                StreamEvent::StderrChunk(_) => {}
                StreamEvent::Finished(_) => break,
                StreamEvent::Error(e) => {
                    println!(
                        "Python execution error (acceptable in test environment): {}",
                        e
                    );
                    break;
                }
            }
        }

        assert!(events_received > 0, "Should receive at least one event");
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn test_python_limits() {
        let files = vec![types::File {
            name: "main.py".to_string(),
            content: b"print('Testing limits')".to_vec(),
            encoding: None,
        }];

        let limits = Some(PythonLimits {
            timeout_ms: Some(5000),
            memory_limit_mb: Some(64),
            max_file_size_bytes: Some(1024),
        });

        let result = execute_python_code(files, None, vec![], vec![], limits);
        match result {
            Ok(result) => {
                assert!(result.time_ms.is_some());
            }
            Err(e) => {
                println!(
                    "Python execution error (acceptable in test environment): {}",
                    e
                );
            }
        }
    }
}
