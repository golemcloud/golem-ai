//! Configuration and utility tests that don't require WASM components
//!
//! These tests verify configuration handling, encoding/decoding,
//! and other functionality that doesn't depend on the WASM runtime.

use golem_exec::config::ExecGlobalConfig;
use std::env;

#[cfg(test)]
mod configuration_tests {
    use super::*;

    #[test]
    fn test_default_configuration() {
        let config = ExecGlobalConfig::default();
        assert_eq!(config.default_timeout_ms, 30000);
        assert_eq!(config.default_memory_limit_mb, Some(128));
        assert_eq!(config.max_file_size_bytes, 1024 * 1024);
        assert_eq!(config.max_processes, Some(1));
        assert!(!config.debug_logging);
    }
    
    #[test]
    fn test_environment_configuration() {
        let _guard = crate::config::ENV_MUTEX.lock().unwrap();
        
        // Set test environment variables
        env::set_var("EXEC_TIMEOUT_MS", "10000");
        env::set_var("EXEC_MEMORY_LIMIT_MB", "256");
        env::set_var("EXEC_MAX_FILE_SIZE_BYTES", "2097152");
        env::set_var("EXEC_MAX_PROCESSES", "2");
        env::set_var("EXEC_DEBUG", "1");
        
        let config = ExecGlobalConfig::from_env();
        
        assert_eq!(config.default_timeout_ms, 10000);
        assert_eq!(config.default_memory_limit_mb, Some(256));
        assert_eq!(config.max_file_size_bytes, 2097152);
        assert_eq!(config.max_processes, Some(2));
        assert!(config.debug_logging);
        
        // Clean up environment variables
        env::remove_var("EXEC_TIMEOUT_MS");
        env::remove_var("EXEC_MEMORY_LIMIT_MB");
        env::remove_var("EXEC_MAX_FILE_SIZE_BYTES");
        env::remove_var("EXEC_MAX_PROCESSES");
        env::remove_var("EXEC_DEBUG");
    }
    
    #[test]
    fn test_javascript_runtime_configuration() {
        let _guard = crate::config::ENV_MUTEX.lock().unwrap();
        
        env::set_var("EXEC_JS_QUICKJS_PATH", "/custom/path/to/qjs");
        env::set_var("EXEC_JS_EXECUTABLE", "custom-qjs");
        env::set_var("EXEC_JS_ENABLE_MODULES", "false");
        
        let config = ExecGlobalConfig::from_env();
        
        assert_eq!(config.javascript.quickjs_path, Some(std::path::PathBuf::from("/custom/path/to/qjs")));
        assert_eq!(config.javascript.default_executable, "custom-qjs");
        assert!(!config.javascript.enable_modules);
        
        env::remove_var("EXEC_JS_QUICKJS_PATH");
        env::remove_var("EXEC_JS_EXECUTABLE");
        env::remove_var("EXEC_JS_ENABLE_MODULES");
    }
    
    #[test]
    fn test_python_runtime_configuration() {
        let _guard = crate::config::ENV_MUTEX.lock().unwrap();
        
        env::set_var("EXEC_PYTHON_WASI_PATH", "/custom/path/to/python");
        env::set_var("EXEC_PYTHON_EXECUTABLE", "custom-python");
        env::set_var("EXEC_PYTHON_VERSION", "3.12");
        env::set_var("EXEC_PYTHON_ENABLE_OPTIMIZATION", "true");
        
        let config = ExecGlobalConfig::from_env();
        
        assert_eq!(config.python.python_wasi_path, Some(std::path::PathBuf::from("/custom/path/to/python")));
        assert_eq!(config.python.default_executable, "custom-python");
        assert_eq!(config.python.default_version, "3.12");
        assert!(config.python.enable_optimization);
        
        env::remove_var("EXEC_PYTHON_WASI_PATH");
        env::remove_var("EXEC_PYTHON_EXECUTABLE");
        env::remove_var("EXEC_PYTHON_VERSION");
        env::remove_var("EXEC_PYTHON_ENABLE_OPTIMIZATION");
    }
    
    #[test]
    fn test_configuration_validation() {
        let mut config = ExecGlobalConfig::default();
        assert!(config.validate().is_ok());
        
        // Test invalid timeout
        config.default_timeout_ms = 0;
        assert!(config.validate().is_err());
        
        // Reset and test invalid memory limit
        config = ExecGlobalConfig::default();
        config.default_memory_limit_mb = Some(0);
        assert!(config.validate().is_err());
        
        // Reset and test invalid file size
        config = ExecGlobalConfig::default();
        config.max_file_size_bytes = 0;
        assert!(config.validate().is_err());
        
        // Reset and test invalid process limit
        config = ExecGlobalConfig::default();
        config.max_processes = Some(0);
        assert!(config.validate().is_err());
    }
    
    #[test]
    fn test_config_summary() {
        let config = ExecGlobalConfig::default();
        // Just ensure the summary doesn't panic
        config.print_summary();
    }
    
    #[test]
    fn test_config_timeout_duration() {
        let config = ExecGlobalConfig::default();
        let duration = config.get_timeout_duration();
        assert_eq!(duration.as_millis(), 30000);
    }
    
    #[test]
    fn test_config_executable_paths() {
        let config = ExecGlobalConfig::default();
        
        // Test JavaScript executable path
        let js_path = config.javascript.get_executable_path();
        assert!(js_path.is_some());
        
        // Test Python executable path
        let py_path = config.python.get_executable_path();
        assert!(py_path.is_some());
    }
}

#[cfg(test)]
mod encoding_tests {
    use super::*;
    use base64;
    use hex;
    
    #[test]
    fn test_base64_encoding_decoding() {
        let original = "Hello, World! 🌍";
        let encoded = base64::encode(original.as_bytes());
        let decoded = base64::decode(&encoded).unwrap();
        let result = String::from_utf8(decoded).unwrap();
        assert_eq!(original, result);
    }
    
    #[test]
    fn test_hex_encoding_decoding() {
        let original = "Hello, World!";
        let encoded = hex::encode(original.as_bytes());
        let decoded = hex::decode(&encoded).unwrap();
        let result = String::from_utf8(decoded).unwrap();
        assert_eq!(original, result);
    }
    
    #[test]
    fn test_invalid_base64() {
        let invalid = "invalid-base64-content!!!";
        assert!(base64::decode(invalid).is_err());
    }
    
    #[test]
    fn test_invalid_hex() {
        let invalid = "invalid-hex-content-xyz";
        assert!(hex::decode(invalid).is_err());
    }
}

#[cfg(test)]
mod utility_tests {
    use super::*;
    use golem_exec::config::env_vars;
    
    #[test]
    fn test_environment_variable_names() {
        // Test that all environment variable names are defined
        assert!(!env_vars::EXEC_TIMEOUT_MS.is_empty());
        assert!(!env_vars::EXEC_MEMORY_LIMIT_MB.is_empty());
        assert!(!env_vars::EXEC_MAX_FILE_SIZE_BYTES.is_empty());
        assert!(!env_vars::EXEC_MAX_PROCESSES.is_empty());
        assert!(!env_vars::EXEC_DEBUG.is_empty());
        assert!(!env_vars::EXEC_JS_QUICKJS_PATH.is_empty());
        assert!(!env_vars::EXEC_JS_EXECUTABLE.is_empty());
        assert!(!env_vars::EXEC_JS_ENABLE_MODULES.is_empty());
        assert!(!env_vars::EXEC_PYTHON_WASI_PATH.is_empty());
        assert!(!env_vars::EXEC_PYTHON_EXECUTABLE.is_empty());
        assert!(!env_vars::EXEC_PYTHON_VERSION.is_empty());
        assert!(!env_vars::EXEC_PYTHON_ENABLE_OPTIMIZATION.is_empty());
    }
    
    #[test]
    fn test_config_from_env_with_invalid_values() {
        let _guard = crate::config::ENV_MUTEX.lock().unwrap();
        
        // Test with invalid numeric values
        env::set_var("EXEC_TIMEOUT_MS", "invalid");
        env::set_var("EXEC_MEMORY_LIMIT_MB", "not_a_number");
        
        let config = ExecGlobalConfig::from_env();
        
        // Should fall back to defaults for invalid values
        assert_eq!(config.default_timeout_ms, 30000); // default
        assert_eq!(config.default_memory_limit_mb, Some(128)); // default
        
        env::remove_var("EXEC_TIMEOUT_MS");
        env::remove_var("EXEC_MEMORY_LIMIT_MB");
    }
    
    #[test]
    fn test_config_boolean_parsing() {
        let _guard = crate::config::ENV_MUTEX.lock().unwrap();
        
        // Test various boolean representations
        let test_cases = vec![
            ("1", true),
            ("true", true),
            ("TRUE", true),
            ("yes", true),
            ("YES", true),
            ("0", false),
            ("false", false),
            ("FALSE", false),
            ("no", false),
            ("NO", false),
            ("invalid", false), // default to false for invalid values
        ];
        
        for (value, expected) in test_cases {
            env::set_var("EXEC_DEBUG", value);
            let config = ExecGlobalConfig::from_env();
            assert_eq!(config.debug_logging, expected, "Failed for value: {}", value);
        }
        
        env::remove_var("EXEC_DEBUG");
    }
}