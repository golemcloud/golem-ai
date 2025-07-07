use crate::config::ExecGlobalConfig;
use crate::error::runtime;
use crate::runtime::Runtime;
use crate::stream::ExecStream;
use crate::types::*;

pub struct JavaScriptRuntime {
    #[allow(dead_code)]
    config: ExecGlobalConfig,
}

impl Default for JavaScriptRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl JavaScriptRuntime {
    pub fn new() -> Self {
        Self {
            config: ExecGlobalConfig::from_env(),
        }
    }

    pub fn with_config(config: ExecGlobalConfig) -> Self {
        Self { config }
    }

    pub fn name(&self) -> &str {
        "javascript-exec-library"
    }

    pub fn extensions(&self) -> Vec<&str> {
        vec![".js", ".mjs"]
    }
}

impl Runtime for JavaScriptRuntime {
    fn execute_blocking(
        &self,
        files: Vec<File>,
        stdin: Option<String>,
        args: Vec<String>,
        env: Vec<(String, String)>,
        limits: Option<Limits>,
    ) -> crate::error::ExecResult<ExecResult> {
        log::info!("Executing JavaScript code using exec-javascript library");

        let js_files: Vec<exec_javascript::types::File> = files
            .into_iter()
            .map(|f| exec_javascript::types::File {
                name: f.name,
                content: f.content,
                encoding: None,
            })
            .collect();

        let js_limits = limits.map(|l| exec_javascript::JavaScriptLimits {
            timeout_ms: l.time_ms,
            memory_limit_mb: l.memory_bytes.map(|b| b / (1024 * 1024)),
            max_file_size_bytes: l.file_size_bytes,
        });

        let result =
            exec_javascript::execute_javascript_code(js_files, stdin, args, env, js_limits)
                .map_err(|e| runtime::execution_failed("javascript", e))?;

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
        log::info!("Executing JavaScript code using exec-javascript library (streaming)");

        // For now, just execute blocking and return the result as a stream
        let result = self.execute_blocking(files, stdin, args, env, limits)?;
        let stream = ExecStream::new();

        if !result.run.stdout.is_empty() {
            stream.push_event(ExecEvent::StdoutChunk(
                result.run.stdout.as_bytes().to_vec(),
            ))?;
        }
        if !result.run.stderr.is_empty() {
            stream.push_event(ExecEvent::StderrChunk(
                result.run.stderr.as_bytes().to_vec(),
            ))?;
        }
        stream.push_event(ExecEvent::Finished(result))?;

        Ok(stream)
    }
}

pub mod utils {
    #[allow(dead_code)]
    pub fn validate_syntax(code: &str) -> crate::error::ExecResult<()> {
        if code.trim().is_empty() {
            return Err(crate::error::validation::empty_file_content("javascript"));
        }

        Ok(())
    }
}
