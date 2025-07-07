use crate::config::ExecGlobalConfig;
use crate::error::runtime;
use crate::runtime::Runtime;
use crate::stream::ExecStream;
use crate::types::*;
#[allow(unused_imports)]
use exec_python::{execute_python_code_streaming, PythonLimits, StreamEvent as PythonStreamEvent};

pub struct PythonRuntime {
    #[allow(dead_code)]
    config: ExecGlobalConfig,
}

impl Default for PythonRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl PythonRuntime {
    pub fn new() -> Self {
        Self {
            config: ExecGlobalConfig::from_env(),
        }
    }

    pub fn with_config(config: ExecGlobalConfig) -> Self {
        Self { config }
    }

    pub fn name(&self) -> &str {
        "python-exec-library"
    }

    pub fn extensions(&self) -> Vec<&str> {
        vec![".py"]
    }
}

impl Runtime for PythonRuntime {
    fn execute_blocking(
        &self,
        files: Vec<File>,
        stdin: Option<String>,
        args: Vec<String>,
        env: Vec<(String, String)>,
        limits: Option<Limits>,
    ) -> crate::error::ExecResult<ExecResult> {
        log::info!("Executing Python code using exec-python library");

        let py_files: Vec<exec_python::types::File> = files
            .into_iter()
            .map(|f| exec_python::types::File {
                name: f.name,
                content: f.content,
                encoding: None,
            })
            .collect();

        let py_limits = limits.map(|l| PythonLimits {
            timeout_ms: l.time_ms,
            memory_limit_mb: l.memory_bytes.map(|b| b / (1024 * 1024)),
            max_file_size_bytes: l.file_size_bytes,
        });

        let result = exec_python::execute_python_code(py_files, stdin, args, env, py_limits)
            .map_err(|e| runtime::execution_failed("python", e))?;

        Ok(ExecResult {
            compile: Some(StageResult {
                stdout: String::new(),
                stderr: String::new(),
                exit_code: Some(0),
                signal: None,
            }),
            run: StageResult {
                stdout: result.stdout,
                stderr: result.stderr,
                exit_code: result.exit_code,
                signal: None,
            },
            time_ms: result.time_ms,
            memory_bytes: result.memory_bytes,
        })
    }

    fn execute_streaming(
        &self,
        files: Vec<File>,
        stdin: Option<String>,
        args: Vec<String>,
        env: Vec<(String, String)>,
        limits: Option<Limits>,
    ) -> crate::error::ExecResult<ExecStream> {
        log::info!("Executing Python code using exec-python library (streaming)");

        let stream = ExecStream::new();

        let _python_files: Vec<exec_python::types::File> = files
            .iter()
            .map(|f| exec_python::types::File {
                name: f.name.clone(),
                content: f.content.clone(),
                encoding: None,
            })
            .collect();

        let _python_limits = limits.map(|l| PythonLimits {
            timeout_ms: l.time_ms,
            memory_limit_mb: l.memory_bytes.map(|b| b / (1024 * 1024)),
            max_file_size_bytes: l.file_size_bytes,
        });

        #[cfg(not(target_arch = "wasm32"))]
        let rt =
            tokio::runtime::Runtime::new().map_err(|e| runtime::execution_failed("python", e))?;

        #[cfg(target_arch = "wasm32")]
        {
            use exec_python::types::File as PythonFile;
            use exec_python::{execute_python_code, PythonLimits};

            let wasm_python_files: Vec<PythonFile> = files
                .iter()
                .map(|f| PythonFile {
                    name: f.name.clone(),
                    content: f.content.clone(),
                    encoding: None,
                })
                .collect();

            let wasm_python_limits = limits.map(|l| PythonLimits {
                timeout_ms: l.time_ms,
                memory_limit_mb: l.memory_bytes.map(|b| b / (1024 * 1024)),
                max_file_size_bytes: l.file_size_bytes,
            });

            match execute_python_code(wasm_python_files, stdin, args, env, wasm_python_limits) {
                Ok(result) => {
                    if !result.stdout.is_empty() {
                        stream.push_event(ExecEvent::StdoutChunk(
                            result.stdout.as_bytes().to_vec(),
                        ))?;
                    }
                    if !result.stderr.is_empty() {
                        stream.push_event(ExecEvent::StderrChunk(
                            result.stderr.as_bytes().to_vec(),
                        ))?;
                    }

                    stream.push_event(ExecEvent::Finished(ExecResult {
                        compile: None, // Python doesn't have a separate compile stage
                        run: StageResult {
                            stdout: result.stdout,
                            stderr: result.stderr,
                            exit_code: result.exit_code,
                            signal: None,
                        },
                        time_ms: result.time_ms,
                        memory_bytes: result.memory_bytes,
                    }))?;
                }
                Err(error) => {
                    stream.push_event(ExecEvent::Failed(Error::Internal(error)))?;
                }
            }
            return Ok(stream);
        }

        #[cfg(not(target_arch = "wasm32"))]
        rt.block_on(async {
            match execute_python_code_streaming(_python_files, stdin, args, env, _python_limits)
                .await
            {
                Ok(mut rx) => {
                    while let Some(event) = rx.recv().await {
                        match event {
                            PythonStreamEvent::StdoutChunk(chunk) => {
                                stream.push_event(ExecEvent::StdoutChunk(chunk))?;
                            }
                            PythonStreamEvent::StderrChunk(chunk) => {
                                stream.push_event(ExecEvent::StderrChunk(chunk))?;
                            }
                            PythonStreamEvent::Finished(result) => {
                                let exec_result = ExecResult {
                                    compile: Some(StageResult {
                                        stdout: String::new(),
                                        stderr: String::new(),
                                        exit_code: Some(0),
                                        signal: None,
                                    }),
                                    run: StageResult {
                                        stdout: result.stdout,
                                        stderr: result.stderr,
                                        exit_code: result.exit_code,
                                        signal: None,
                                    },
                                    time_ms: result.time_ms,
                                    memory_bytes: result.memory_bytes,
                                };
                                stream.push_event(ExecEvent::Finished(exec_result))?;
                                break;
                            }
                            PythonStreamEvent::Error(error) => {
                                return Err(runtime::execution_failed("python", error));
                            }
                        }
                    }
                }
                Err(e) => {
                    return Err(runtime::execution_failed("python", e));
                }
            }
            Ok(())
        })?;

        #[cfg(not(target_arch = "wasm32"))]
        Ok(stream)
    }
}

pub mod utils {
    #[allow(dead_code)]
    pub fn validate_syntax(code: &str) -> crate::error::ExecResult<()> {
        if code.trim().is_empty() {
            return Err(crate::error::validation::empty_file_content("python"));
        }

        Ok(())
    }
}
