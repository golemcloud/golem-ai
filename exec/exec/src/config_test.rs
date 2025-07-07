use crate::config::*;
use std::env;
use std::path::PathBuf;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ENV_MUTEX;

    #[test]
    fn test_default_config() {
        let config = ExecGlobalConfig::default();
        assert_eq!(config.default_timeout_ms, 30000);
        assert_eq!(config.default_memory_limit_mb, Some(128));
        assert_eq!(config.max_file_size_bytes, 1048576);
        assert_eq!(config.max_processes, Some(1));
        assert!(!config.debug_logging);

        assert_eq!(config.javascript.default_executable, "qjs");
        assert!(config.javascript.enable_modules);
        assert!(config.javascript.quickjs_path.is_none());

        assert_eq!(config.python.default_executable, "python");
        assert_eq!(config.python.default_version, "3.11");
        assert!(!config.python.enable_optimization);
        assert!(config.python.python_wasi_path.is_none());
    }

    #[test]
    fn test_config_from_env() {
        let _guard = ENV_MUTEX.lock().unwrap();

        let env_vars = [
            "EXEC_TIMEOUT_MS",
            "EXEC_MEMORY_LIMIT_MB",
            "EXEC_MAX_FILE_SIZE_BYTES",
            "EXEC_MAX_PROCESSES",
            "GOLEM_EXEC_LOG",
            "EXEC_JS_QUICKJS_PATH",
            "EXEC_PYTHON_WASI_PATH",
        ];
        let original_values: Vec<_> = env_vars
            .iter()
            .map(|var| (*var, env::var(var).ok()))
            .collect();

        env::set_var("EXEC_TIMEOUT_MS", "60000");
        env::set_var("EXEC_MEMORY_LIMIT_MB", "256");
        env::set_var("EXEC_MAX_FILE_SIZE_BYTES", "2097152");
        env::set_var("EXEC_MAX_PROCESSES", "2");
        env::set_var("GOLEM_EXEC_LOG", "true");
        env::set_var("EXEC_JS_QUICKJS_PATH", "/custom/path/to/qjs");
        env::set_var("EXEC_PYTHON_WASI_PATH", "/custom/path/to/python");

        let config = ExecGlobalConfig::from_env();

        assert_eq!(config.default_timeout_ms, 60000);
        assert_eq!(config.default_memory_limit_mb, Some(256));
        assert_eq!(config.max_file_size_bytes, 2097152);
        assert_eq!(config.max_processes, Some(2));
        assert!(config.debug_logging);
        assert_eq!(
            config.javascript.quickjs_path,
            Some(PathBuf::from("/custom/path/to/qjs"))
        );
        assert_eq!(
            config.python.python_wasi_path,
            Some(PathBuf::from("/custom/path/to/python"))
        );

        for (var, original_value) in original_values {
            match original_value {
                Some(value) => env::set_var(var, value),
                None => env::remove_var(var),
            }
        }
    }

    #[test]
    fn test_config_validation() {
        let mut config = ExecGlobalConfig::default();

        assert!(config.validate().is_ok());

        config.default_timeout_ms = 0;
        assert!(config.validate().is_err());

        config = ExecGlobalConfig::default();
        config.max_file_size_bytes = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_get_timeout_duration() {
        let config = ExecGlobalConfig::default();
        let duration = config.timeout_duration();
        assert_eq!(duration.as_millis(), 30000);
    }

    #[test]
    fn test_javascript_config_executable_path() {
        let mut js_config = JavaScriptConfig::default();

        js_config.quickjs_path = Some(PathBuf::from("/custom/qjs"));
        let path = js_config.get_executable_path();
        assert_eq!(path, PathBuf::from("/custom/qjs"));

        js_config.quickjs_path = None;
        let path = js_config.get_executable_path();
        assert!(!path.as_os_str().is_empty());
    }

    #[test]
    fn test_python_config_executable_path() {
        let mut py_config = PythonConfig::default();

        py_config.python_wasi_path = Some(PathBuf::from("/custom/python"));
        let path = py_config.get_executable_path();
        assert_eq!(path, PathBuf::from("/custom/python"));

        py_config.python_wasi_path = None;
        let path = py_config.get_executable_path();
        assert!(!path.as_os_str().is_empty());
    }

    #[test]
    fn test_env_vars_constants() {
        assert!(!env_vars::EXEC_TIMEOUT_MS.is_empty());
        assert!(!env_vars::EXEC_MEMORY_LIMIT_MB.is_empty());
        assert!(!env_vars::EXEC_MAX_FILE_SIZE_BYTES.is_empty());
        assert!(!env_vars::EXEC_MAX_PROCESSES.is_empty());
        assert!(!env_vars::GOLEM_EXEC_LOG.is_empty());
        assert!(!env_vars::EXEC_JS_QUICKJS_PATH.is_empty());
        assert!(!env_vars::EXEC_PYTHON_WASI_PATH.is_empty());
    }

    #[test]
    fn test_config_print_summary() {
        let config = ExecGlobalConfig::default();
        config.print_summary();
    }
}
