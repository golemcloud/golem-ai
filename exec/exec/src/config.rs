use std::env;
use std::path::PathBuf;
use std::time::Duration;

/// Global configuration for the execution environment
#[derive(Debug, Clone)]
pub struct ExecGlobalConfig {
    /// Default timeout for execution in milliseconds
    pub default_timeout_ms: u64,
    /// Default memory limit in megabytes
    pub default_memory_limit_mb: Option<u64>,
    /// Maximum file size in bytes
    pub max_file_size_bytes: usize,
    /// Maximum number of processes
    pub max_processes: Option<u32>,
    /// JavaScript runtime configuration
    pub javascript: JavaScriptConfig,
    /// Python runtime configuration
    pub python: PythonConfig,
    /// Enable debug logging
    pub debug_logging: bool,
}

/// JavaScript runtime configuration
#[derive(Debug, Clone)]
pub struct JavaScriptConfig {
    /// Path to QuickJS executable
    pub quickjs_path: Option<PathBuf>,
    /// Default QuickJS executable name
    pub default_executable: String,
    /// Enable QuickJS modules
    pub enable_modules: bool,
    /// Enable QuickJS BigInt support
    pub enable_bigint: bool,
}

/// Python runtime configuration
#[derive(Debug, Clone)]
pub struct PythonConfig {
    /// Path to Python WASI executable
    pub python_wasi_path: Option<PathBuf>,
    /// Default Python executable name
    pub default_executable: String,
    /// Python version to use
    pub default_version: String,
    /// Enable Python optimization
    pub enable_optimization: bool,
}

impl Default for ExecGlobalConfig {
    fn default() -> Self {
        Self {
            default_timeout_ms: 30000,          // 30 seconds
            default_memory_limit_mb: Some(128), // 128 MB
            max_file_size_bytes: 1024 * 1024,   // 1 MB
            max_processes: Some(1),
            javascript: JavaScriptConfig::default(),
            python: PythonConfig::default(),
            debug_logging: false,
        }
    }
}

impl Default for JavaScriptConfig {
    fn default() -> Self {
        Self {
            quickjs_path: None,
            default_executable: "qjs".to_string(),
            enable_modules: true,
            enable_bigint: true,
        }
    }
}

impl JavaScriptConfig {
    pub fn get_executable_path(&self) -> PathBuf {
        if let Some(ref path) = self.quickjs_path {
            PathBuf::from(path)
        } else {
            // Try to find in PATH or use default
            crate::runtime::utils::get_runtime_path(
                "EXEC_JS_QUICKJS_PATH",
                &self.default_executable,
            )
        }
    }
}

impl Default for PythonConfig {
    fn default() -> Self {
        Self {
            python_wasi_path: None,
            default_executable: "python".to_string(),
            default_version: "3.11".to_string(),
            enable_optimization: false,
        }
    }
}

impl PythonConfig {
    pub fn get_executable_path(&self) -> PathBuf {
        if let Some(ref path) = self.python_wasi_path {
            PathBuf::from(path)
        } else {
            // Try to find in PATH or use default
            crate::runtime::utils::get_runtime_path(
                "EXEC_PYTHON_WASI_PATH",
                &self.default_executable,
            )
        }
    }
}

impl ExecGlobalConfig {
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(timeout_str) = env::var("EXEC_TIMEOUT_MS") {
            if let Ok(timeout) = timeout_str.parse::<u64>() {
                config.default_timeout_ms = timeout;
            }
        }

        if let Ok(memory_str) = env::var("EXEC_MEMORY_LIMIT_MB") {
            if let Ok(memory) = memory_str.parse::<u64>() {
                config.default_memory_limit_mb = Some(memory);
            }
        }

        if let Ok(size_str) = env::var("EXEC_MAX_FILE_SIZE_BYTES") {
            if let Ok(size) = size_str.parse::<usize>() {
                config.max_file_size_bytes = size;
            }
        }

        if let Ok(proc_str) = env::var("EXEC_MAX_PROCESSES") {
            if let Ok(procs) = proc_str.parse::<u32>() {
                config.max_processes = Some(procs);
            }
        }

        if let Ok(js_path) = env::var("EXEC_JS_QUICKJS_PATH") {
            config.javascript.quickjs_path = Some(PathBuf::from(js_path));
        }

        if let Ok(js_exe) = env::var("EXEC_JS_EXECUTABLE") {
            config.javascript.default_executable = js_exe;
        }

        if let Ok(modules_str) = env::var("EXEC_JS_ENABLE_MODULES") {
            config.javascript.enable_modules = modules_str.parse().unwrap_or(true);
        }

        if let Ok(bigint_str) = env::var("EXEC_JS_ENABLE_BIGINT") {
            config.javascript.enable_bigint = bigint_str.parse().unwrap_or(true);
        }

        if let Ok(py_path) = env::var("EXEC_PYTHON_WASI_PATH") {
            config.python.python_wasi_path = Some(PathBuf::from(py_path));
        }

        if let Ok(py_exe) = env::var("EXEC_PYTHON_EXECUTABLE") {
            config.python.default_executable = py_exe;
        }

        if let Ok(py_version) = env::var("EXEC_PYTHON_VERSION") {
            config.python.default_version = py_version;
        }

        if let Ok(opt_str) = env::var("EXEC_PYTHON_ENABLE_OPTIMIZATION") {
            config.python.enable_optimization = opt_str.parse().unwrap_or(false);
        }

        config.debug_logging = env::var("GOLEM_EXEC_LOG").is_ok() || env::var("EXEC_DEBUG").is_ok();

        config
    }

    pub fn timeout_duration(&self) -> Duration {
        Duration::from_millis(self.default_timeout_ms)
    }

    pub fn javascript_executable(&self) -> PathBuf {
        self.javascript
            .quickjs_path
            .clone()
            .unwrap_or_else(|| PathBuf::from(&self.javascript.default_executable))
    }

    pub fn python_executable(&self) -> PathBuf {
        self.python
            .python_wasi_path
            .clone()
            .unwrap_or_else(|| PathBuf::from(&self.python.default_executable))
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.default_timeout_ms == 0 {
            return Err("Timeout must be greater than 0".to_string());
        }

        if let Some(memory) = self.default_memory_limit_mb {
            if memory == 0 {
                return Err("Memory limit must be greater than 0".to_string());
            }
        }

        if self.max_file_size_bytes == 0 {
            return Err("File size limit must be greater than 0".to_string());
        }

        if let Some(procs) = self.max_processes {
            if procs == 0 {
                return Err("Process limit must be greater than 0".to_string());
            }
        }

        Ok(())
    }

    pub fn print_summary(&self) {
        if self.debug_logging {
            log::info!("Golem Exec Configuration:");
            log::info!("  Timeout: {}ms", self.default_timeout_ms);
            log::info!("  Memory Limit: {:?}MB", self.default_memory_limit_mb);
            log::info!("  Max File Size: {} bytes", self.max_file_size_bytes);
            log::info!("  Max Processes: {:?}", self.max_processes);
            log::info!(
                "  JavaScript Executable: {:?}",
                self.javascript_executable()
            );
            log::info!("  Python Executable: {:?}", self.python_executable());
            log::info!("  Debug Logging: {}", self.debug_logging);
        }
    }
}

/// Environment variable names used by golem-exec
pub mod env_vars {
    /// Default execution timeout in milliseconds
    pub const EXEC_TIMEOUT_MS: &str = "EXEC_TIMEOUT_MS";

    /// Default memory limit in megabytes
    pub const EXEC_MEMORY_LIMIT_MB: &str = "EXEC_MEMORY_LIMIT_MB";

    /// Maximum file size in bytes
    pub const EXEC_MAX_FILE_SIZE_BYTES: &str = "EXEC_MAX_FILE_SIZE_BYTES";

    /// Maximum number of processes
    pub const EXEC_MAX_PROCESSES: &str = "EXEC_MAX_PROCESSES";

    /// Path to QuickJS executable
    pub const EXEC_JS_QUICKJS_PATH: &str = "EXEC_JS_QUICKJS_PATH";

    /// JavaScript executable name
    pub const EXEC_JS_EXECUTABLE: &str = "EXEC_JS_EXECUTABLE";

    /// Enable JavaScript modules
    pub const EXEC_JS_ENABLE_MODULES: &str = "EXEC_JS_ENABLE_MODULES";

    /// Enable JavaScript BigInt support
    pub const EXEC_JS_ENABLE_BIGINT: &str = "EXEC_JS_ENABLE_BIGINT";

    /// Path to Python WASI executable
    pub const EXEC_PYTHON_WASI_PATH: &str = "EXEC_PYTHON_WASI_PATH";

    /// Python executable name
    pub const EXEC_PYTHON_EXECUTABLE: &str = "EXEC_PYTHON_EXECUTABLE";

    /// Python version to use
    pub const EXEC_PYTHON_VERSION: &str = "EXEC_PYTHON_VERSION";

    /// Enable Python optimization
    pub const EXEC_PYTHON_ENABLE_OPTIMIZATION: &str = "EXEC_PYTHON_ENABLE_OPTIMIZATION";

    /// Enable debug logging
    pub const GOLEM_EXEC_LOG: &str = "GOLEM_EXEC_LOG";

    /// Alternative debug logging flag
    pub const EXEC_DEBUG: &str = "EXEC_DEBUG";
}

#[cfg(test)]
use std::sync::Mutex;
#[cfg(test)]
pub static ENV_MUTEX: Mutex<()> = Mutex::new(());

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_default_config() {
        let config = ExecGlobalConfig::default();
        assert_eq!(config.default_timeout_ms, 30000);
        assert_eq!(config.default_memory_limit_mb, Some(128));
        assert_eq!(config.max_file_size_bytes, 1024 * 1024);
        assert_eq!(config.max_processes, Some(1));
        assert!(!config.debug_logging);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation() {
        let mut config = ExecGlobalConfig::default();
        assert!(config.validate().is_ok());

        config.default_timeout_ms = 0;
        assert!(config.validate().is_err());

        config.default_timeout_ms = 1000;
        config.default_memory_limit_mb = Some(0);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_env_loading() {
        let _guard = ENV_MUTEX.lock().unwrap();

        let vars_to_clear = [
            "EXEC_TIMEOUT_MS",
            "EXEC_MEMORY_LIMIT_MB",
            "EXEC_MAX_FILE_SIZE_BYTES",
            "EXEC_MAX_PROCESSES",
            "GOLEM_EXEC_LOG",
            "EXEC_DEBUG",
            "EXEC_JS_QUICKJS_PATH",
            "EXEC_PYTHON_WASI_PATH",
        ];

        let original_values: Vec<_> = vars_to_clear
            .iter()
            .map(|var| (*var, env::var(var).ok()))
            .collect();

        for var in &vars_to_clear {
            env::remove_var(var);
        }

        env::set_var("EXEC_TIMEOUT_MS", "5000");
        env::set_var("EXEC_MEMORY_LIMIT_MB", "256");
        env::set_var("EXEC_DEBUG", "1");

        let config = ExecGlobalConfig::from_env();
        assert_eq!(config.default_timeout_ms, 5000);
        assert_eq!(config.default_memory_limit_mb, Some(256));
        assert!(config.debug_logging);

        for (var, original_value) in original_values {
            match original_value {
                Some(value) => env::set_var(var, value),
                None => env::remove_var(var),
            }
        }
    }
}
