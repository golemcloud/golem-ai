use golem_exec::config::ExecGlobalConfig;
use golem_exec::*;
use std::env;
use tempfile::TempDir;

fn create_file(name: &str, content: &str, encoding: Option<Encoding>) -> File {
    File {
        name: name.to_string(),
        content: content.as_bytes().to_vec(),
        encoding,
    }
}

fn create_language(kind: LanguageKind, version: Option<&str>) -> Language {
    Language {
        kind,
        version: version.map(|v| v.to_string()),
    }
}

#[cfg(test)]
mod javascript_tests {
    use super::*;

    #[test]
    fn test_javascript_hello_world() {
        let files = vec![create_file(
            "main.js",
            "console.log('Hello, World!');",
            Some(Encoding::Utf8),
        )];
        
        let language = create_language(LanguageKind::Javascript, None);
        let args = vec![];
        let env = vec![];
        let stdin = None;
        let limits = Some(Limits {
            time_ms: Some(5000),
            memory_bytes: Some(64 * 1024 * 1024), // 64MB in bytes
            file_size_bytes: Some(1024 * 1024),
            max_processes: Some(1),
        });
        
        let result = executor::run(language, files, stdin, args, env, limits);
        
        match result {
            Ok(exec_result) => {
                assert_eq!(exec_result.run.exit_code, Some(0));
                assert!(exec_result.run.stdout.contains("Hello, World!"));
                assert!(exec_result.run.stderr.is_empty());
            }
            Err(e) => {
                // If JavaScript runtime is not available, skip the test
                match e {
                    Error::Internal(msg) if msg.contains("not found") => {
                        println!("Skipping JavaScript test: runtime not available");
                        return;
                    }
                    _ => panic!("Unexpected error: {:?}", e),
                }
            }
        }
    }
    
    #[test]
    fn test_javascript_with_arguments() {
        let files = vec![create_file(
            "args.js",
            "console.log('Arguments:', process.argv.slice(2).join(' '));",
            Some(Encoding::Utf8),
        )];
        
        let language = create_language(LanguageKind::Javascript, None);
        let args = vec!["arg1".to_string(), "arg2".to_string()];
        let env = vec![];
        let stdin = None;
        let limits = Some(Limits {
            time_ms: Some(5000),
            memory_bytes: Some(64 * 1024 * 1024), // 64MB in bytes
            file_size_bytes: Some(1024 * 1024),
            max_processes: Some(1),
        });
        
        let result = executor::run(language, files, stdin, args, env, limits);
        
        match result {
            Ok(exec_result) => {
                assert_eq!(exec_result.run.exit_code, Some(0));
                assert!(exec_result.run.stdout.contains("arg1 arg2"));
            }
            Err(Error::Internal(msg)) if msg.contains("not found") => {
                println!("Skipping JavaScript test: runtime not available");
            }
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    }
    
    #[test]
    fn test_javascript_with_environment() {
        let files = vec![create_file(
            "env.js",
            "console.log('TEST_VAR:', process.env.TEST_VAR || 'not set');",
            Some(Encoding::Utf8),
        )];
        
        let language = create_language(LanguageKind::Javascript, None);
        let args = vec![];
        let env = vec![("TEST_VAR".to_string(), "test_value".to_string())];
        let stdin = None;
        let limits = Some(Limits {
            time_ms: Some(5000),
            memory_bytes: Some(64 * 1024 * 1024), // 64MB in bytes
            file_size_bytes: Some(1024 * 1024),
            max_processes: Some(1),
        });
        
        let result = executor::run(language, files, stdin, args, env, limits);
        
        match result {
            Ok(exec_result) => {
                assert_eq!(exec_result.run.exit_code, Some(0));
                assert!(exec_result.run.stdout.contains("test_value"));
            }
            Err(Error::Internal(msg)) if msg.contains("not found") => {
                println!("Skipping JavaScript test: runtime not available");
            }
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    }
    
    #[test]
    fn test_javascript_syntax_error() {
        let files = vec![create_file(
            "error.js",
            "console.log('Hello' // Missing closing parenthesis",
            Some(Encoding::Utf8),
        )];
        
        let language = create_language(LanguageKind::Javascript, None);
        let args = vec![];
        let env = vec![];
        let stdin = None;
        let limits = Some(Limits {
            time_ms: Some(5000),
            memory_bytes: Some(64 * 1024 * 1024), // 64MB in bytes
            file_size_bytes: Some(1024 * 1024),
            max_processes: Some(1),
        });
        
        let result = executor::run(language, files, stdin, args, env, limits);
        
        match result {
            Ok(exec_result) => {
                // Should have non-zero exit code for syntax error
                assert_ne!(exec_result.run.exit_code, Some(0));
            }
            Err(Error::Internal(msg)) if msg.contains("not found") => {
                println!("Skipping JavaScript test: runtime not available");
            }
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    }
}

#[cfg(test)]
mod python_tests {
    use super::*;

    #[test]
    fn test_python_hello_world() {
        let files = vec![create_file(
            "main.py",
            "print('Hello, World!')",
            Some(Encoding::Utf8),
        )];
        
        let language = create_language(LanguageKind::Python, Some("3.11"));
        let args = vec![];
        let env = vec![];
        let stdin = None;
        let limits = Some(Limits {
            time_ms: Some(5000),
            memory_bytes: Some(64 * 1024 * 1024), // 64MB in bytes
            file_size_bytes: Some(1024 * 1024),
            max_processes: Some(1),
        });
        
        let result = executor::run(language, files, stdin, args, env, limits);
        
        match result {
            Ok(exec_result) => {
                assert_eq!(exec_result.run.exit_code, Some(0));
                assert!(exec_result.run.stdout.contains("Hello, World!"));
                assert!(exec_result.run.stderr.is_empty());
            }
            Err(Error::Internal(msg)) if msg.contains("not found") => {
                println!("Skipping Python test: runtime not available");
            }
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    }
    
    #[test]
    fn test_python_with_stdin() {
        let files = vec![create_file(
            "stdin.py",
            "import sys\nfor line in sys.stdin:\n    print(f'Input: {line.strip()}')",
            Some(Encoding::Utf8),
        )];
        
        let language = create_language(LanguageKind::Python, Some("3.11"));
        let args = vec![];
        let env = vec![];
        let stdin = Some("test input\n".to_string());
        let limits = Some(Limits {
            time_ms: Some(5000),
            memory_bytes: Some(64 * 1024 * 1024), // 64MB in bytes
            file_size_bytes: Some(1024 * 1024),
            max_processes: Some(1),
        });
        
        let result = executor::run(language, files, stdin, args, env, limits);
        
        match result {
            Ok(exec_result) => {
                assert_eq!(exec_result.run.exit_code, Some(0));
                assert!(exec_result.run.stdout.contains("Input: test input"));
            }
            Err(Error::Internal(msg)) if msg.contains("not found") => {
                println!("Skipping Python test: runtime not available");
            }
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    }
    
    #[test]
    fn test_python_with_arguments() {
        let files = vec![create_file(
            "args.py",
            "import sys\nprint('Arguments:', ' '.join(sys.argv[1:]))",
            Some(Encoding::Utf8),
        )];
        
        let language = create_language(LanguageKind::Python, Some("3.11"));
        let args = vec!["arg1".to_string(), "arg2".to_string()];
        let env = vec![];
        let stdin = None;
        let limits = Some(Limits {
            time_ms: Some(5000),
            memory_bytes: Some(64 * 1024 * 1024), // 64MB in bytes
            file_size_bytes: Some(1024 * 1024),
            max_processes: Some(1),
        });
        
        let result = executor::run(language, files, stdin, args, env, limits);
        
        match result {
            Ok(exec_result) => {
                assert_eq!(exec_result.run.exit_code, Some(0));
                assert!(exec_result.run.stdout.contains("arg1 arg2"));
            }
            Err(Error::Internal(msg)) if msg.contains("not found") => {
                println!("Skipping Python test: runtime not available");
            }
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    }
    
    #[test]
    fn test_python_syntax_error() {
        let files = vec![create_file(
            "error.py",
            "print('Hello'\n    invalid_syntax",
            Some(Encoding::Utf8),
        )];
        
        let language = create_language(LanguageKind::Python, Some("3.11"));
        let args = vec![];
        let env = vec![];
        let stdin = None;
        let limits = Some(Limits {
            time_ms: Some(5000),
            memory_bytes: Some(64 * 1024 * 1024), // 64MB in bytes
            file_size_bytes: Some(1024 * 1024),
            max_processes: Some(1),
        });
        
        let result = executor::run(language, files, stdin, args, env, limits);
        
        match result {
            Ok(exec_result) => {
                // Should have non-zero exit code for syntax error
                assert_ne!(exec_result.run.exit_code, Some(0));
            }
            Err(Error::Internal(msg)) if msg.contains("not found") => {
                println!("Skipping Python test: runtime not available");
            }
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    }
}

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
}

#[cfg(test)]
mod encoding_tests {
    use super::*;

    #[test]
    fn test_utf8_encoding() {
        let files = vec![create_file(
            "utf8.js",
            "console.log('UTF-8 test: 你好世界');",
            Some(Encoding::Utf8),
        )];
        
        let language = create_language(LanguageKind::Javascript, None);
        let result = executor::run(files, language, vec![], vec![], None, None);
        
        match result {
            Ok(exec_result) => {
                assert_eq!(exec_result.exit_code, 0);
                assert!(exec_result.stdout.contains("你好世界"));
            }
            Err(Error::Internal(msg)) if msg.contains("not found") => {
                println!("Skipping encoding test: runtime not available");
            }
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    }
    
    #[test]
    fn test_base64_encoding() {
        // "console.log('Base64 test');" encoded in base64
        let base64_content = "Y29uc29sZS5sb2coJ0Jhc2U2NCB0ZXN0Jyk7";
        
        let files = vec![create_file(
            "base64.js",
            base64_content,
            Some(Encoding::Base64),
        )];
        
        let language = create_language(LanguageKind::Javascript, None);
        let result = executor::run(files, language, vec![], vec![], None, None);
        
        match result {
            Ok(exec_result) => {
                assert_eq!(exec_result.exit_code, 0);
                assert!(exec_result.stdout.contains("Base64 test"));
            }
            Err(Error::Internal(msg)) if msg.contains("not found") => {
                println!("Skipping encoding test: runtime not available");
            }
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    }
    
    #[test]
    fn test_hex_encoding() {
        // "console.log('Hex test');" encoded in hex
        let hex_content = "636f6e736f6c652e6c6f67282748657820746573742729";
        
        let files = vec![create_file(
            "hex.js",
            hex_content,
            Some(Encoding::Hex),
        )];
        
        let language = create_language(LanguageKind::Javascript, None);
        let result = executor::run(files, language, vec![], vec![], None, None);
        
        match result {
            Ok(exec_result) => {
                assert_eq!(exec_result.exit_code, 0);
                assert!(exec_result.stdout.contains("Hex test"));
            }
            Err(Error::Internal(msg)) if msg.contains("not found") => {
                println!("Skipping encoding test: runtime not available");
            }
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    }
}

#[cfg(test)]
mod error_handling_tests {
    use super::*;

    #[test]
    fn test_empty_files() {
        let files = vec![];
        let language = create_language(LanguageKind::Javascript, None);
        let result = executor::run(files, language, vec![], vec![], None, None);
        
        assert!(matches!(result, Err(Error::Internal(_))));
    }
    
    #[test]
    fn test_empty_file_content() {
        let files = vec![create_file("empty.js", "", Some(Encoding::Utf8))];
        let language = create_language(LanguageKind::Javascript, None);
        let result = executor::run(files, language, vec![], vec![], None, None);
        
        assert!(matches!(result, Err(Error::Internal(_))));
    }
    
    #[test]
    fn test_file_size_limit() {
        // Create a large file that exceeds the default limit
        let large_content = "a".repeat(2 * 1024 * 1024); // 2MB
        let files = vec![create_file("large.js", &large_content, Some(Encoding::Utf8))];
        let language = create_language(LanguageKind::Javascript, None);
        let result = executor::run(files, language, vec![], vec![], None, None);
        
        assert!(matches!(result, Err(Error::Internal(_))));
    }
    
    #[test]
    fn test_invalid_base64() {
        let files = vec![create_file(
            "invalid.js",
            "invalid-base64-content!!!",
            Some(Encoding::Base64),
        )];
        let language = create_language(LanguageKind::Javascript, None);
        let result = executor::run(files, language, vec![], vec![], None, None);
        
        assert!(matches!(result, Err(Error::Internal(_))));
    }
    
    #[test]
    fn test_invalid_hex() {
        let files = vec![create_file(
            "invalid.js",
            "invalid-hex-content-xyz",
            Some(Encoding::Hex),
        )];
        let language = create_language(LanguageKind::Javascript, None);
        let result = executor::run(files, language, vec![], vec![], None, None);
        
        assert!(matches!(result, Err(Error::Internal(_))));
    }
    
    #[test]
    fn test_unsupported_language_version() {
        let files = vec![create_file("test.js", "console.log('test');", Some(Encoding::Utf8))];
        let language = create_language(LanguageKind::Javascript, Some("ES1999"));
        let result = executor::run(files, language, vec![], vec![], None, None);
        
        assert!(matches!(result, Err(Error::UnsupportedLanguage)));
    }
}

#[cfg(test)]
mod timeout_tests {
    use super::*;

    #[test]
    fn test_timeout_enforcement() {
        // Create a script that runs longer than the timeout
        let files = vec![create_file(
            "timeout.js",
            "while(true) { /* infinite loop */ }",
            Some(Encoding::Utf8),
        )];
        
        let language = create_language(LanguageKind::Javascript, None);
        let limits = Some(Limits {
            time_ms: Some(1000), // 1 second timeout
            memory_mb: Some(64),
            file_size_bytes: Some(1024 * 1024),
            max_processes: Some(1),
        });
        
        let result = executor::run(files, language, vec![], vec![], None, limits);
        
        match result {
            Ok(exec_result) => {
                // Should timeout and have non-zero exit code
                assert_ne!(exec_result.exit_code, 0);
            }
            Err(Error::Timeout) => {
                // Timeout error is also acceptable
            }
            Err(Error::Internal(msg)) if msg.contains("not found") => {
                println!("Skipping timeout test: runtime not available");
            }
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    }
}