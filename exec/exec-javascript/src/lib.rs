use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

#[cfg(not(target_arch = "wasm32"))]
use rquickjs::{Context, Runtime};

#[cfg(target_arch = "wasm32")]
use wit_bindgen::generate;

#[cfg(target_arch = "wasm32")]
use std::collections::HashMap;

#[cfg(target_arch = "wasm32")]
generate!({
    world: "exec-javascript",
    path: "wit",
});

#[cfg(target_arch = "wasm32")]
use crate::exports::golem::exec_javascript::exec_stream;
#[cfg(target_arch = "wasm32")]
use crate::exports::golem::exec_javascript::executor::Guest as ExecutorGuest;
#[cfg(target_arch = "wasm32")]
use crate::exports::golem::exec_javascript::session::{Guest as SessionGuest, GuestSession};

#[cfg(target_arch = "wasm32")]
pub use golem::exec_javascript::types::*;

#[cfg(not(target_arch = "wasm32"))]
pub use types::*;

pub mod bindings;
pub mod types;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaScriptFile {
    pub name: String,
    pub content: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaScriptLimits {
    pub timeout_ms: Option<u64>,
    pub memory_limit_mb: Option<u64>,
    pub max_file_size_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaScriptExecResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub time_ms: Option<u64>,
    pub memory_bytes: Option<u64>,
}

// WIT Component Implementation
#[cfg(target_arch = "wasm32")]
pub struct JavaScriptExecComponent;

#[cfg(target_arch = "wasm32")]
impl ExecutorGuest for JavaScriptExecComponent {
    fn run(
        lang: Language,
        files: Vec<File>,
        stdin: Option<String>,
        args: Vec<String>,
        env: Vec<(String, String)>,
        constraints: Option<Limits>,
    ) -> Result<ExecResult, Error> {
        let _internal_lang = types::Language::from(lang);
        let internal_files: Vec<types::File> = files.into_iter().map(|f| f.into()).collect();
        let internal_limits = constraints.map(|l| types::Limits {
            time_ms: l.time_ms,
            memory_bytes: l.memory_bytes,
            file_size_bytes: l.file_size_bytes,
            max_processes: l.max_processes,
        });

        let js_limits = internal_limits.map(|l| JavaScriptLimits {
            timeout_ms: l.time_ms,
            memory_limit_mb: l.memory_bytes.map(|bytes| bytes / (1024 * 1024)),
            max_file_size_bytes: l.file_size_bytes,
        });

        // Execute JavaScript code
        match execute_javascript_code(internal_files, stdin, args, env, js_limits) {
            Ok(result) => Ok(types::ExecResult {
                compile: None,
                run: types::StageResult {
                    stdout: result.stdout,
                    stderr: result.stderr,
                    exit_code: result.exit_code,
                    signal: None,
                },
                time_ms: result.time_ms,
                memory_bytes: result.memory_bytes,
            }
            .into()),
            Err(e) => Err(types::Error::ExecutionFailed(e).into()),
        }
    }

    fn run_streaming(
        lang: Language,
        files: Vec<File>,
        stdin: Option<String>,
        args: Vec<String>,
        env: Vec<(String, String)>,
        constraints: Option<Limits>,
    ) -> Result<exec_stream::ExecStream, Error> {
        let _internal_lang = types::Language::from(lang);
        let internal_files: Vec<types::File> = files.into_iter().map(|f| f.into()).collect();
        let internal_limits = constraints.map(|l| types::Limits {
            time_ms: l.time_ms,
            memory_bytes: l.memory_bytes,
            file_size_bytes: l.file_size_bytes,
            max_processes: l.max_processes,
        });

        let js_limits = internal_limits.map(|l| JavaScriptLimits {
            timeout_ms: l.time_ms,
            memory_limit_mb: l.memory_bytes.map(|bytes| bytes / (1024 * 1024)),
            max_file_size_bytes: l.file_size_bytes,
        });

        // Execute JavaScript code and create stream from result
        let result = execute_javascript_code(internal_files, stdin, args, env, js_limits);
        let stream = JavaScriptExecStreamWrapper::from_result(result);

        Ok(exec_stream::ExecStream::new(stream))
    }
}

#[cfg(target_arch = "wasm32")]
pub struct JavaScriptSession {
    files: std::cell::RefCell<HashMap<String, Vec<u8>>>,
    working_dir: std::cell::RefCell<String>,
    #[allow(dead_code)]
    session_id: String,
    closed: std::cell::RefCell<bool>,
}

#[cfg(target_arch = "wasm32")]
impl GuestSession for JavaScriptSession {
    fn new(_lang: Language) -> Self {
        Self {
            files: std::cell::RefCell::new(HashMap::new()),
            working_dir: std::cell::RefCell::new("/tmp".to_string()),
            session_id: "session".to_string(),
            closed: std::cell::RefCell::new(false),
        }
    }

    fn upload(&self, file: File) -> Result<(), Error> {
        if *self.closed.borrow() {
            return Err(types::Error::Internal("Session is closed".to_string()).into());
        }

        if file.name.is_empty() {
            return Err(types::Error::InvalidFile("Empty filename".to_string()).into());
        }

        if file.content.is_empty() {
            return Err(types::Error::InvalidFile("Empty file content".to_string()).into());
        }

        self.files.borrow_mut().insert(file.name, file.content);
        Ok(())
    }

    fn run(
        &self,
        entrypoint: String,
        args: Vec<String>,
        stdin: Option<String>,
        env: Vec<(String, String)>,
        constraints: Option<Limits>,
    ) -> Result<ExecResult, Error> {
        if *self.closed.borrow() {
            return Err(types::Error::Internal("Session is closed".to_string()).into());
        }

        let files = self.files.borrow();
        let content = files
            .get(&entrypoint)
            .ok_or_else(|| types::Error::FileNotFound(entrypoint.clone()))?;

        let file = types::File {
            name: entrypoint,
            content: content.clone(),
            encoding: None,
        };

        let internal_limits = constraints.map(|l| types::Limits {
            time_ms: l.time_ms,
            memory_bytes: l.memory_bytes,
            file_size_bytes: l.file_size_bytes,
            max_processes: l.max_processes,
        });

        let js_limits = internal_limits.map(|l| JavaScriptLimits {
            timeout_ms: l.time_ms,
            memory_limit_mb: l.memory_bytes.map(|bytes| bytes / (1024 * 1024)),
            max_file_size_bytes: l.file_size_bytes,
        });

        match execute_javascript_code(vec![file], stdin, args, env, js_limits) {
            Ok(result) => Ok(types::ExecResult {
                compile: None,
                run: types::StageResult {
                    stdout: result.stdout,
                    stderr: result.stderr,
                    exit_code: result.exit_code,
                    signal: None,
                },
                time_ms: result.time_ms,
                memory_bytes: result.memory_bytes,
            }
            .into()),
            Err(e) => Err(types::Error::ExecutionFailed(e).into()),
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
        if *self.closed.borrow() {
            return Err(types::Error::Internal("Session is closed".to_string()).into());
        }

        let files = self.files.borrow();
        let content = files
            .get(&entrypoint)
            .ok_or_else(|| types::Error::FileNotFound(entrypoint.clone()))?;

        let file = types::File {
            name: entrypoint,
            content: content.clone(),
            encoding: None,
        };

        let internal_limits = constraints.map(|l| types::Limits {
            time_ms: l.time_ms,
            memory_bytes: l.memory_bytes,
            file_size_bytes: l.file_size_bytes,
            max_processes: l.max_processes,
        });

        let js_limits = internal_limits.map(|l| JavaScriptLimits {
            timeout_ms: l.time_ms,
            memory_limit_mb: l.memory_bytes.map(|bytes| bytes / (1024 * 1024)),
            max_file_size_bytes: l.file_size_bytes,
        });

        let result = execute_javascript_code(vec![file], stdin, args, env, js_limits);
        let stream = JavaScriptExecStreamWrapper::from_result(result);

        Ok(exec_stream::ExecStream::new(stream))
    }

    fn download(&self, path: String) -> Result<Vec<u8>, Error> {
        let files = self.files.borrow();
        files
            .get(&path)
            .cloned()
            .ok_or_else(|| types::Error::FileNotFound(path).into())
    }

    fn list_files(&self, dir: String) -> Result<Vec<String>, Error> {
        let files = self.files.borrow();

        if dir.is_empty() || dir == "/" {
            Ok(files.keys().cloned().collect())
        } else {
            let dir_prefix = if dir.ends_with('/') {
                dir
            } else {
                format!("{}/", dir)
            };
            let matching_files: Vec<String> = files
                .keys()
                .filter(|path| path.starts_with(&dir_prefix))
                .map(|path| {
                    let relative = &path[dir_prefix.len()..];
                    if let Some(slash_pos) = relative.find('/') {
                        format!("{}/", &relative[..slash_pos])
                    } else {
                        relative.to_string()
                    }
                })
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();

            Ok(matching_files)
        }
    }

    fn set_working_dir(&self, path: String) -> Result<(), Error> {
        if *self.closed.borrow() {
            return Err(types::Error::Internal("Session is closed".to_string()).into());
        }

        if path.is_empty() {
            return Err(types::Error::InvalidFile("Empty path".to_string()).into());
        }

        *self.working_dir.borrow_mut() = path;
        Ok(())
    }

    fn close(&self) {
        if *self.closed.borrow() {
            return;
        }

        self.files.borrow_mut().clear();
        *self.closed.borrow_mut() = true;
    }
}

#[cfg(target_arch = "wasm32")]
impl SessionGuest for JavaScriptExecComponent {
    type Session = JavaScriptSession;
}

// Placeholder implementation for exec_stream::Guest
#[cfg(target_arch = "wasm32")]
pub struct JavaScriptExecStreamWrapper {
    events: std::cell::RefCell<Vec<ExecEvent>>,
    current_index: std::cell::RefCell<usize>,
    finished: std::cell::RefCell<bool>,
}

#[cfg(target_arch = "wasm32")]
impl JavaScriptExecStreamWrapper {
    pub fn new() -> Self {
        Self {
            events: std::cell::RefCell::new(Vec::new()),
            current_index: std::cell::RefCell::new(0),
            finished: std::cell::RefCell::new(false),
        }
    }

    pub fn from_result(result: Result<JavaScriptExecResult, String>) -> Self {
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

                events.push(ExecEvent::Finished(
                    types::ExecResult {
                        compile: None,
                        run: types::StageResult {
                            stdout: exec_result.stdout,
                            stderr: exec_result.stderr,
                            exit_code: exec_result.exit_code,
                            signal: None,
                        },
                        time_ms: exec_result.time_ms,
                        memory_bytes: exec_result.memory_bytes,
                    }
                    .into(),
                ));
            }
            Err(error) => {
                events.push(ExecEvent::Failed(
                    types::Error::ExecutionFailed(error).into(),
                ));
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
impl exec_stream::GuestExecStream for JavaScriptExecStreamWrapper {
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
        self.get_next()
    }
}

#[cfg(target_arch = "wasm32")]
impl exec_stream::Guest for JavaScriptExecComponent {
    type ExecStream = JavaScriptExecStreamWrapper;
}

#[cfg(target_arch = "wasm32")]
export!(JavaScriptExecComponent with_types_in crate);

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

#[cfg(not(target_arch = "wasm32"))]
pub fn execute_javascript_code(
    files: Vec<File>,
    stdin: Option<String>,
    args: Vec<String>,
    env: Vec<(String, String)>,
    limits: Option<JavaScriptLimits>,
) -> Result<JavaScriptExecResult, String> {
    init_logging();
    log::info!("Executing JavaScript code");

    let main_file = files
        .iter()
        .find(|f| f.name.ends_with(".js") || f.name.ends_with(".mjs"))
        .ok_or("No JavaScript file found")?;

    let code = String::from_utf8(main_file.content.clone())
        .map_err(|e| format!("Invalid UTF-8 in JavaScript file: {}", e))?;

    execute_javascript(&code, stdin, args, env, limits)
}

#[cfg(target_arch = "wasm32")]
pub fn execute_javascript_code(
    files: Vec<types::File>,
    stdin: Option<String>,
    args: Vec<String>,
    env: Vec<(String, String)>,
    limits: Option<JavaScriptLimits>,
) -> Result<JavaScriptExecResult, String> {
    init_logging();
    log::info!("Executing JavaScript in WASM using QuickJS WASI");

    let main_file = files
        .iter()
        .find(|f| f.name.ends_with(".js") || f.name.ends_with(".mjs"))
        .ok_or("No JavaScript file found")?;

    let code = String::from_utf8(main_file.content.clone())
        .map_err(|e| format!("Invalid UTF-8 in JavaScript file: {}", e))?;

    execute_javascript_wasi(&code, stdin, args, env, limits)
}

#[cfg(target_arch = "wasm32")]
fn execute_javascript_wasi(
    code: &str,
    stdin: Option<String>,
    args: Vec<String>,
    env: Vec<(String, String)>,
    limits: Option<JavaScriptLimits>,
) -> Result<JavaScriptExecResult, String> {
    let start_time = Instant::now();

    let quickjs_path =
        std::env::var("EXEC_JS_QUICKJS_PATH").unwrap_or_else(|_| "/usr/local/bin/qjs".to_string());

    let temp_dir = std::path::PathBuf::from("/tmp/js_exec_");
    let script_path = temp_dir.join("script.js");

    std::fs::write(&script_path, code)
        .map_err(|e| format!("Failed to write script file: {}", e))?;

    let mut cmd_args = vec![script_path.to_string_lossy().to_string()];
    cmd_args.extend(args);

    let mut env_map = HashMap::new();
    for (key, value) in env {
        env_map.insert(key, value);
    }

    let timeout_duration = limits
        .as_ref()
        .and_then(|l| l.timeout_ms)
        .map(Duration::from_millis)
        .unwrap_or(Duration::from_secs(30));

    let result =
        execute_with_timeout_wasm(&quickjs_path, &cmd_args, stdin, &env_map, timeout_duration)?;

    let _ = std::fs::remove_file(&script_path);

    let execution_time = start_time.elapsed();

    Ok(JavaScriptExecResult {
        stdout: result.stdout,
        stderr: result.stderr,
        exit_code: result.exit_code,
        time_ms: Some(execution_time.as_millis() as u64),
        memory_bytes: None,
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn execute_javascript(
    code: &str,
    _stdin: Option<String>,
    _args: Vec<String>,
    _env: Vec<(String, String)>,
    limits: Option<JavaScriptLimits>,
) -> Result<JavaScriptExecResult, String> {
    let start_time = Instant::now();

    let rt = Runtime::new().map_err(|e| format!("Failed to create runtime: {e:?}"))?;
    let ctx = Context::full(&rt).map_err(|e| format!("Failed to create context: {e:?}"))?;

    let _timeout_duration = limits
        .as_ref()
        .and_then(|l| l.timeout_ms)
        .map(Duration::from_millis);

    let (stdout, stderr, exit_code) = ctx.with(|ctx| match ctx.eval::<String, _>(code) {
        Ok(result) => (format!("Result: {}", result), String::new(), Some(0)),
        Err(e) => (
            String::new(),
            format!("JavaScript execution error: {}", e),
            Some(1),
        ),
    });

    let execution_time = start_time.elapsed();

    Ok(JavaScriptExecResult {
        stdout,
        stderr,
        exit_code,
        time_ms: Some(execution_time.as_millis() as u64),
        memory_bytes: None,
    })
}

#[cfg(target_arch = "wasm32")]
struct ProcessResult {
    stdout: String,
    stderr: String,
    exit_code: Option<i32>,
}

#[cfg(target_arch = "wasm32")]
fn execute_with_timeout_wasm(
    _executable: &str,
    _args: &[String],
    _stdin: Option<String>,
    _env: &HashMap<String, String>,
    _timeout: Duration,
) -> Result<ProcessResult, String> {
    Err("Process execution not supported in WASM environment".to_string())
}
