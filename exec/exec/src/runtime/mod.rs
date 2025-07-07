//! Runtime implementations for different programming languages

pub mod common;
pub mod javascript;
pub mod python;

use crate::error::runtime;
use crate::types::*;
use std::collections::HashMap;
use tempfile;
use which;

#[cfg(not(target_arch = "wasm32"))]
use tokio::time::timeout;

/// Configuration for runtime execution
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub timeout_ms: Option<u64>,
    pub memory_mb: Option<u64>,
    pub processes: Option<u32>,
    pub working_dir: Option<String>,
    pub env_vars: HashMap<String, String>,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            timeout_ms: Some(30000), // 30 seconds
            memory_mb: Some(128),    // 128 MB
            processes: Some(1),      // 1 process
            working_dir: None,
            env_vars: HashMap::new(),
        }
    }
}

/// Trait for language-specific runtime implementations
pub trait Runtime {
    fn execute_blocking(
        &self,
        files: Vec<File>,
        stdin: Option<String>,
        args: Vec<String>,
        env: Vec<(String, String)>,
        limits: Option<Limits>,
    ) -> crate::error::ExecResult<ExecResult>;

    fn execute_streaming(
        &self,
        files: Vec<File>,
        stdin: Option<String>,
        args: Vec<String>,
        env: Vec<(String, String)>,
        limits: Option<Limits>,
    ) -> crate::error::ExecResult<crate::stream::ExecStream>;
}

/// Factory for creating runtime instances
pub struct RuntimeFactory;

impl Default for RuntimeFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl RuntimeFactory {
    pub fn new() -> Self {
        Self
    }

    pub fn create_javascript(&self) -> Box<dyn Runtime> {
        Box::new(javascript::JavaScriptRuntime::new())
    }

    pub fn create_python(&self) -> Box<dyn Runtime> {
        Box::new(python::PythonRuntime::new())
    }

    pub fn create_for_language(&self, lang: LanguageKind) -> Box<dyn Runtime> {
        match lang {
            LanguageKind::Javascript => self.create_javascript(),
            LanguageKind::Python => self.create_python(),
        }
    }
}

pub mod utils {
    use super::*;

    #[cfg(not(target_arch = "wasm32"))]
    use std::process::Stdio;
    #[cfg(not(target_arch = "wasm32"))]
    use tokio::process::Command;

    /// Execute a command with monitoring
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn execute_with_monitoring(
        mut cmd: Command,
        limits: &Limits,
        _monitor: &crate::limits::ResourceMonitor,
    ) -> crate::error::ExecResult<std::process::Output> {
        use std::time::Duration;

        cmd.stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::piped());

        let timeout_duration = Duration::from_millis(limits.time_ms.unwrap_or(30000));

        let child = cmd
            .spawn()
            .map_err(|e| runtime::spawn_failed("runtime", e))?;

        let output = timeout(timeout_duration, child.wait_with_output())
            .await
            .map_err(|_| Error::Timeout)?
            .map_err(|e| runtime::execution_failed("runtime", e))?;

        Ok(output)
    }

    /// Create a temporary directory for execution
    pub fn create_temp_dir(prefix: &str) -> crate::error::ExecResult<std::path::PathBuf> {
        let temp_dir_handle = tempfile::Builder::new()
            .prefix(prefix)
            .tempdir()
            .map_err(crate::error::fs::temp_dir_creation_failed)?;
        let temp_dir = temp_dir_handle.keep();
        Ok(temp_dir)
    }

    /// Write files to a directory
    pub fn write_files_to_dir(
        files: &[File],
        dir: &std::path::Path,
    ) -> crate::error::ExecResult<()> {
        for file in files {
            let file_path = dir.join(&file.name);

            // Create parent directories if needed
            if let Some(parent) = file_path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    crate::error::fs::dir_creation_failed(&parent.to_string_lossy(), e)
                })?;
            }

            let content_str = String::from_utf8(file.content.clone())
                .map_err(|e| crate::error::convert::utf8_error("output conversion", e))?;
            let content = crate::encoding::decode_content(
                &content_str,
                file.encoding.unwrap_or(Encoding::Utf8),
            )?;
            std::fs::write(&file_path, content)
                .map_err(|e| crate::error::fs::file_write_failed(&file.name, e))?;
        }
        Ok(())
    }

    /// Clean up temporary directory
    pub fn cleanup_temp_dir(dir: &std::path::Path) {
        if let Err(e) = std::fs::remove_dir_all(dir) {
            log::warn!(
                "Failed to cleanup temporary directory {}: {}",
                dir.display(),
                e
            );
        }
    }

    /// Get runtime path from environment or default
    pub fn get_runtime_path(env_var: &str, default_name: &str) -> std::path::PathBuf {
        std::env::var(env_var)
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::path::PathBuf::from(default_name))
    }

    /// Check if an executable exists
    pub fn check_executable(path: &std::path::Path) -> bool {
        path.exists() && path.is_file()
    }

    /// Find executable in PATH
    pub fn find_in_path(name: &str) -> Option<std::path::PathBuf> {
        which::which(name).ok()
    }

    /// Find the executable path for a runtime
    pub fn find_executable(name: &str, env_var: &str) -> Result<String, Error> {
        // First check environment variable
        if let Ok(path) = std::env::var(env_var) {
            return Ok(path);
        }

        // Then check PATH
        which::which(name)
            .map(|p| p.to_string_lossy().to_string())
            .map_err(|_| runtime::executable_not_found(name, env_var))
    }
}
