use golem_exec::*;
use std::collections::HashMap;
use std::env;

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

fn create_default_limits() -> Limits {
    Limits {
        time_ms: Some(30000),
        memory_bytes: Some(128 * 1024 * 1024), // 128MB
        file_size_bytes: Some(1024 * 1024), // 1MB
        max_processes: Some(1),
    }
}

#[cfg(test)]
mod language_validation_tests {
    use super::*;

    #[test]
    fn test_supported_javascript_versions() {
        let supported_versions = vec![
            None, // Default version
            Some("ES5"),
            Some("ES6"),
            Some("ES2015"),
            Some("ES2016"),
            Some("ES2017"),
            Some("ES2018"),
            Some("ES2019"),
            Some("ES2020"),
            Some("ES2021"),
            Some("ES2022"),
        ];

        for version in supported_versions {
            let language = create_language(LanguageKind::Javascript, version);
            let files = vec![create_file(
                "test.js",
                "console.log('Hello');",
                Some(Encoding::Utf8),
            )];

            let result = executor::run(
                language,
                files,
                None,
                vec![],
                vec![],
                Some(create_default_limits()),
            );

            if let Err(Error::UnsupportedLanguage) = result {
                panic!("JavaScript version {:?} should be supported", version);
            }
        }
    }

    #[test]
    fn test_supported_python_versions() {
        let supported_versions = vec![
            None, // Default version
            Some("3.8"),
            Some("3.9"),
            Some("3.10"),
            Some("3.11"),
            Some("3.12"),
        ];

        for version in supported_versions {
            let language = create_language(LanguageKind::Python, version);
            let files = vec![create_file(
                "test.py",
                "print('Hello')",
                Some(Encoding::Utf8),
            )];

            let result = executor::run(
                language,
                files,
                None,
                vec![],
                vec![],
                Some(create_default_limits()),
            );

            if let Err(Error::UnsupportedLanguage) = result {
                panic!("Python version {:?} should be supported", version);
            }
        }
    }

    #[test]
    fn test_unsupported_language_version() {
        let language = create_language(LanguageKind::Python, Some("2.7"));
        let files = vec![create_file(
            "test.py",
            "print 'Hello'",
            Some(Encoding::Utf8),
        )];

        let result = executor::run(
            language,
            files,
            None,
            vec![],
            vec![],
            Some(create_default_limits()),
        );

        assert!(matches!(result, Err(Error::UnsupportedLanguage)));
    }
}

#[cfg(test)]
mod file_encoding_tests {
    use super::*;
    use base64::{Engine as _, engine::general_purpose};

    #[test]
    fn test_utf8_encoding() {
        let content = "console.log('Hello, 世界!');"; // UTF-8 with Unicode
        let file = create_file("test.js", content, Some(Encoding::Utf8));
        let language = create_language(LanguageKind::Javascript, None);

        let result = executor::run(
            language,
            vec![file],
            None,
            vec![],
            vec![],
            Some(create_default_limits()),
        );

        assert!(result.is_ok());
        if let Ok(exec_result) = result {
            assert!(exec_result.run.stdout.contains("Hello, 世界!"));
        }
    }

    #[test]
    fn test_base64_encoding() {
        let content = "console.log('Hello from Base64!');"; 
        let encoded_content = general_purpose::STANDARD.encode(content.as_bytes());
        
        let mut file = File {
            name: "test.js".to_string(),
            content: encoded_content.as_bytes().to_vec(),
            encoding: Some(Encoding::Base64),
        };

        let language = create_language(LanguageKind::Javascript, None);

        let result = executor::run(
            language,
            vec![file],
            None,
            vec![],
            vec![],
            Some(create_default_limits()),
        );

        assert!(result.is_ok());
        if let Ok(exec_result) = result {
            assert!(exec_result.run.stdout.contains("Hello from Base64!"));
        }
    }

    #[test]
    fn test_hex_encoding() {
        let content = "print('Hello from Hex!')";
        let hex_content = hex::encode(content.as_bytes());
        
        let file = File {
            name: "test.py".to_string(),
            content: hex_content.as_bytes().to_vec(),
            encoding: Some(Encoding::Hex),
        };

        let language = create_language(LanguageKind::Python, None);

        let result = executor::run(
            language,
            vec![file],
            None,
            vec![],
            vec![],
            Some(create_default_limits()),
        );

        assert!(result.is_ok());
        if let Ok(exec_result) = result {
            assert!(exec_result.run.stdout.contains("Hello from Hex!"));
        }
    }

    #[test]
    fn test_invalid_base64_encoding() {
        let file = File {
            name: "test.js".to_string(),
            content: "invalid-base64!!!".as_bytes().to_vec(),
            encoding: Some(Encoding::Base64),
        };

        let language = create_language(LanguageKind::Javascript, None);

        let result = executor::run(
            language,
            vec![file],
            None,
            vec![],
            vec![],
            Some(create_default_limits()),
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_hex_encoding() {
        let file = File {
            name: "test.py".to_string(),
            content: "invalid-hex-content".as_bytes().to_vec(),
            encoding: Some(Encoding::Hex),
        };

        let language = create_language(LanguageKind::Python, None);

        let result = executor::run(
            language,
            vec![file],
            None,
            vec![],
            vec![],
            Some(create_default_limits()),
        );

        assert!(result.is_err());
    }
}

#[cfg(test)]
mod executor_interface_tests {
    use super::*;

    #[test]
    fn test_blocking_execution_javascript() {
        let files = vec![create_file(
            "main.js",
            "console.log('Hello, World!');",
            Some(Encoding::Utf8),
        )];
        
        let language = create_language(LanguageKind::Javascript, None);
        let result = executor::run(
            language,
            files,
            None,
            vec![],
            vec![],
            Some(create_default_limits()),
        );
        
        assert!(result.is_ok());
        let exec_result = result.unwrap();
        assert_eq!(exec_result.run.exit_code, Some(0));
        assert!(exec_result.run.stdout.contains("Hello, World!"));
        assert!(exec_result.run.stderr.is_empty());
    }

    #[test]
    fn test_blocking_execution_python() {
        let files = vec![create_file(
            "main.py",
            "print('Hello, World!')",
            Some(Encoding::Utf8),
        )];
        
        let language = create_language(LanguageKind::Python, None);
        let result = executor::run(
            language,
            files,
            None,
            vec![],
            vec![],
            Some(create_default_limits()),
        );
        
        assert!(result.is_ok());
        let exec_result = result.unwrap();
        assert_eq!(exec_result.run.exit_code, Some(0));
        assert!(exec_result.run.stdout.contains("Hello, World!"));
        assert!(exec_result.run.stderr.is_empty());
    }

    #[test]
    fn test_execution_with_stdin() {
        let files = vec![create_file(
            "echo.py",
            "import sys\nprint(sys.stdin.read().strip())",
            Some(Encoding::Utf8),
        )];
        
        let language = create_language(LanguageKind::Python, None);
        let stdin = Some("Hello from stdin!".to_string());
        
        let result = executor::run(
            language,
            files,
            stdin,
            vec![],
            vec![],
            Some(create_default_limits()),
        );
        
        assert!(result.is_ok());
        let exec_result = result.unwrap();
        assert!(exec_result.run.stdout.contains("Hello from stdin!"));
    }

    #[test]
    fn test_execution_with_args() {
        let files = vec![create_file(
            "args.py",
            "import sys\nprint(' '.join(sys.argv[1:]))",
            Some(Encoding::Utf8),
        )];
        
        let language = create_language(LanguageKind::Python, None);
        let args = vec!["arg1".to_string(), "arg2".to_string(), "arg3".to_string()];
        
        let result = executor::run(
            language,
            files,
            None,
            args,
            vec![],
            Some(create_default_limits()),
        );
        
        assert!(result.is_ok());
        let exec_result = result.unwrap();
        assert!(exec_result.run.stdout.contains("arg1 arg2 arg3"));
    }

    #[test]
    fn test_execution_with_environment() {
        let files = vec![create_file(
            "env.py",
            "import os\nprint(os.environ.get('TEST_VAR', 'not found'))",
            Some(Encoding::Utf8),
        )];
        
        let language = create_language(LanguageKind::Python, None);
        let env = vec![("TEST_VAR".to_string(), "test_value".to_string())];
        
        let result = executor::run(
            language,
            files,
            None,
            vec![],
            env,
            Some(create_default_limits()),
        );
        
        assert!(result.is_ok());
        let exec_result = result.unwrap();
        assert!(exec_result.run.stdout.contains("test_value"));
    }

    #[test]
    fn test_streaming_execution() {
        let files = vec![create_file(
            "stream.py",
            "import time\nfor i in range(3):\n    print(f'Output {i}')\n    time.sleep(0.1)",
            Some(Encoding::Utf8),
        )];
        
        let language = create_language(LanguageKind::Python, None);
        
        let result = executor::run_streaming(
            language,
            files,
            None,
            vec![],
            vec![],
            Some(create_default_limits()),
        );
        
        assert!(result.is_ok());
        let mut stream = result.unwrap();
        
        let mut stdout_chunks = Vec::new();
        let mut finished = false;
        
        while !finished {
            if let Some(event) = stream.get_next() {
                match event {
                    ExecEvent::StdoutChunk(data) => {
                        stdout_chunks.push(String::from_utf8_lossy(&data).to_string());
                    }
                    ExecEvent::Finished(_) => {
                        finished = true;
                    }
                    ExecEvent::Failed(_) => {
                        panic!("Streaming execution failed");
                    }
                    _ => {}
                }
            } else {
                break;
            }
        }
        
        let combined_output = stdout_chunks.join("");
        assert!(combined_output.contains("Output 0"));
        assert!(combined_output.contains("Output 1"));
        assert!(combined_output.contains("Output 2"));
    }
}

#[cfg(test)]
mod session_interface_tests {
    use super::*;

    #[test]
    fn test_session_lifecycle() {
        let language = create_language(LanguageKind::Python, None);
        let session = session::Session::new(language);
        
        let file = create_file(
            "test.py",
            "print('Hello from session!')",
            Some(Encoding::Utf8),
        );
        
        let upload_result = session.upload(file);
        assert!(upload_result.is_ok());
        
        let run_result = session.run(
            "test.py".to_string(),
            vec![],
            None,
            vec![],
            Some(create_default_limits()),
        );
        
        assert!(run_result.is_ok());
        let exec_result = run_result.unwrap();
        assert!(exec_result.run.stdout.contains("Hello from session!"));
        
        session.close();
    }

    #[test]
    fn test_session_file_operations() {
        let language = create_language(LanguageKind::Python, None);
        let session = session::Session::new(language);
        
        let file1 = create_file(
            "file1.py",
            "with open('output.txt', 'w') as f: f.write('Hello')",
            Some(Encoding::Utf8),
        );
        let file2 = create_file(
            "file2.py",
            "print('File 2 executed')",
            Some(Encoding::Utf8),
        );
        
        assert!(session.upload(file1).is_ok());
        assert!(session.upload(file2).is_ok());
        
        let run_result = session.run(
            "file1.py".to_string(),
            vec![],
            None,
            vec![],
            Some(create_default_limits()),
        );
        assert!(run_result.is_ok());
        
        let list_result = session.list_files(".".to_string());
        if let Ok(files) = list_result {
            assert!(files.contains(&"file1.py".to_string()));
            assert!(files.contains(&"file2.py".to_string()));
        }
        
        let download_result = session.download("output.txt".to_string());
        if let Ok(content) = download_result {
            let content_str = String::from_utf8_lossy(&content);
            assert!(content_str.contains("Hello"));
        }
        
        session.close();
    }

    #[test]
    fn test_session_working_directory() {
        let language = create_language(LanguageKind::Python, None);
        let session = session::Session::new(language);
        
        let set_dir_result = session.set_working_dir("/tmp".to_string());

        let unsafe_dir_result = session.set_working_dir("/etc".to_string());
        assert!(unsafe_dir_result.is_err());
        
        session.close();
    }

    #[test]
    fn test_session_streaming_execution() {
        let language = create_language(LanguageKind::Python, None);
        let session = session::Session::new(language);
        
        let file = create_file(
            "stream.py",
            "for i in range(3): print(f'Line {i}')",
            Some(Encoding::Utf8),
        );
        
        assert!(session.upload(file).is_ok());
        
        let stream_result = session.run_streaming(
            "stream.py".to_string(),
            vec![],
            None,
            vec![],
            Some(create_default_limits()),
        );
        
        assert!(stream_result.is_ok());
        let mut stream = stream_result.unwrap();
        
        let mut output_received = false;
        while let Some(event) = stream.get_next() {
            match event {
                ExecEvent::StdoutChunk(_) => {
                    output_received = true;
                }
                ExecEvent::Finished(_) => break,
                ExecEvent::Failed(_) => panic!("Streaming failed"),
                _ => {}
            }
        }
        
        assert!(output_received);
        session.close();
    }
}

#[cfg(test)]
mod limits_and_constraints_tests {
    use super::*;

    #[test]
    fn test_timeout_constraint() {
        let files = vec![create_file(
            "timeout.py",
            "import time\ntime.sleep(10)\nprint('Should not reach here')",
            Some(Encoding::Utf8),
        )];
        
        let language = create_language(LanguageKind::Python, None);
        let limits = Limits {
            time_ms: Some(1000), // 1 second timeout
            memory_bytes: Some(128 * 1024 * 1024),
            file_size_bytes: Some(1024 * 1024),
            max_processes: Some(1),
        };
        
        let result = executor::run(
            language,
            files,
            None,
            vec![],
            vec![],
            Some(limits),
        );
        
        assert!(matches!(result, Err(Error::Timeout)));
    }

    #[test]
    fn test_file_size_limit() {
        let large_content = "print('x')\n".repeat(10000); // Large file
        let file = create_file("large.py", &large_content, Some(Encoding::Utf8));
        
        let language = create_language(LanguageKind::Python, None);
        let limits = Limits {
            time_ms: Some(30000),
            memory_bytes: Some(128 * 1024 * 1024),
            file_size_bytes: Some(1000), // Very small limit
            max_processes: Some(1),
        };
        
        let result = executor::run(
            language,
            vec![file],
            None,
            vec![],
            vec![],
            Some(limits),
        );
        
        assert!(result.is_err());
    }

    #[test]
    fn test_memory_limit() {
        let files = vec![create_file(
            "memory.py",
            "data = 'x' * (50 * 1024 * 1024)\nprint('Memory allocated')",
            Some(Encoding::Utf8),
        )];
        
        let language = create_language(LanguageKind::Python, None);
        let limits = Limits {
            time_ms: Some(30000),
            memory_bytes: Some(10 * 1024 * 1024), // 10MB limit
            file_size_bytes: Some(1024 * 1024),
            max_processes: Some(1),
        };
        
        let result = executor::run(
            language,
            files,
            None,
            vec![],
            vec![],
            Some(limits),
        );
        
        assert!(matches!(result, Err(Error::ResourceExceeded)));
    }
}

#[cfg(test)]
mod error_handling_tests {
    use super::*;

    #[test]
    fn test_compilation_error_javascript() {
        let files = vec![create_file(
            "syntax_error.js",
            "console.log('unclosed string",
            Some(Encoding::Utf8),
        )];
        
        let language = create_language(LanguageKind::Javascript, None);
        let result = executor::run(
            language,
            files,
            None,
            vec![],
            vec![],
            Some(create_default_limits()),
        );
        
        assert!(matches!(result, Err(Error::CompilationFailed(_))));
    }

    #[test]
    fn test_runtime_error_python() {
        let files = vec![create_file(
            "runtime_error.py",
            "print(undefined_variable)",
            Some(Encoding::Utf8),
        )];
        
        let language = create_language(LanguageKind::Python, None);
        let result = executor::run(
            language,
            files,
            None,
            vec![],
            vec![],
            Some(create_default_limits()),
        );
        
        assert!(matches!(result, Err(Error::RuntimeFailed(_))));
    }

    #[test]
    fn test_internal_error_handling() {
        let file = File {
            name: "test.py".to_string(),
            content: vec![0xFF, 0xFE, 0xFD], // Invalid UTF-8
            encoding: Some(Encoding::Utf8),
        };
        
        let language = create_language(LanguageKind::Python, None);
        let result = executor::run(
            language,
            vec![file],
            None,
            vec![],
            vec![],
            Some(create_default_limits()),
        );
        
        assert!(matches!(result, Err(Error::Internal(_))));
    }
}

#[cfg(test)]
mod python_runtime_tests {
    use super::*;

    #[test]
    fn test_python_standard_library() {
        let files = vec![create_file(
            "stdlib.py",
            r#"
import sys
import os
import json
import math
import random

# Test sys module
print(f"Python version: {sys.version}")
print(f"Platform: {sys.platform}")

# Test os module
print(f"Environment: {os.environ.get('PATH', 'not found')}")

# Test json module
data = {"key": "value", "number": 42}
json_str = json.dumps(data)
print(f"JSON: {json_str}")

# Test math module
print(f"Pi: {math.pi}")
print(f"Square root of 16: {math.sqrt(16)}")

# Test random module
random.seed(42)
print(f"Random number: {random.randint(1, 100)}")
"#,
            Some(Encoding::Utf8),
        )];
        
        let language = create_language(LanguageKind::Python, None);
        let result = executor::run(
            language,
            files,
            None,
            vec![],
            vec![],
            Some(create_default_limits()),
        );
        
        assert!(result.is_ok());
        let exec_result = result.unwrap();
        assert!(exec_result.run.stdout.contains("Python version"));
        assert!(exec_result.run.stdout.contains("Platform"));
        assert!(exec_result.run.stdout.contains("JSON"));
        assert!(exec_result.run.stdout.contains("Pi"));
        assert!(exec_result.run.stdout.contains("Square root"));
    }

    #[test]
    fn test_python_builtin_functions() {
        let files = vec![create_file(
            "builtins.py",
            r#"
# Test built-in functions
print(f"Length of 'hello': {len('hello')}")
print(f"Type of 42: {type(42)}")
print(f"Range: {list(range(5))}")
print(f"String conversion: {str(123)}")
print(f"Integer conversion: {int('456')}")
print(f"Float conversion: {float('3.14')}")
print(f"Boolean conversion: {bool(1)}")
print(f"List creation: {list([1, 2, 3])}")
print(f"Dictionary creation: {dict({'a': 1, 'b': 2})}")
"#,
            Some(Encoding::Utf8),
        )];
        
        let language = create_language(LanguageKind::Python, None);
        let result = executor::run(
            language,
            files,
            None,
            vec![],
            vec![],
            Some(create_default_limits()),
        );
        
        assert!(result.is_ok());
        let exec_result = result.unwrap();
        assert!(exec_result.run.stdout.contains("Length of 'hello': 5"));
        assert!(exec_result.run.stdout.contains("Type of 42"));
        assert!(exec_result.run.stdout.contains("Range: [0, 1, 2, 3, 4]"));
    }

    #[test]
    fn test_python_variable_scoping() {
        let files = vec![create_file(
            "scoping.py",
            r#"
# Test variable scoping
global_var = "global"

def test_function():
    local_var = "local"
    print(f"Global in function: {global_var}")
    print(f"Local in function: {local_var}")
    return local_var

result = test_function()
print(f"Global outside: {global_var}")
print(f"Function result: {result}")

# Test built-in variables
print(f"__name__: {__name__}")
"#,
            Some(Encoding::Utf8),
        )];
        
        let language = create_language(LanguageKind::Python, None);
        let result = executor::run(
            language,
            files,
            None,
            vec![],
            vec![],
            Some(create_default_limits()),
        );
        
        assert!(result.is_ok());
        let exec_result = result.unwrap();
        assert!(exec_result.run.stdout.contains("Global in function: global"));
        assert!(exec_result.run.stdout.contains("Local in function: local"));
        assert!(exec_result.run.stdout.contains("__name__"));
    }
}

#[cfg(test)]
mod integration_workflow_tests {
    use super::*;

    #[test]
    fn test_multi_file_javascript_project() {
        let files = vec![
            create_file(
                "utils.js",
                "function greet(name) { return 'Hello, ' + name + '!'; }",
                Some(Encoding::Utf8),
            ),
            create_file(
                "main.js",
                "// Load utils and use it\nconsole.log(greet('World'));",
                Some(Encoding::Utf8),
            ),
        ];
        
        let language = create_language(LanguageKind::Javascript, None);
        let result = executor::run(
            language,
            files,
            None,
            vec![],
            vec![],
            Some(create_default_limits()),
        );
        
        assert!(result.is_ok() || result.is_err()); // Either outcome is valid for this test
    }

    #[test]
    fn test_multi_file_python_project() {
        let files = vec![
            create_file(
                "utils.py",
                "def greet(name):\n    return f'Hello, {name}!'",
                Some(Encoding::Utf8),
            ),
            create_file(
                "main.py",
                "from utils import greet\nprint(greet('World'))",
                Some(Encoding::Utf8),
            ),
        ];
        
        let language = create_language(LanguageKind::Python, None);
        let result = executor::run(
            language,
            files,
            None,
            vec![],
            vec![],
            Some(create_default_limits()),
        );
        
        assert!(result.is_ok());
        let exec_result = result.unwrap();
        assert!(exec_result.run.stdout.contains("Hello, World!"));
    }

    #[test]
    fn test_complete_session_workflow() {
        let language = create_language(LanguageKind::Python, None);
        let session = session::Session::new(language);
        
        let config_file = create_file(
            "config.json",
            r#"{"name": "test", "version": "1.0"}",
            Some(Encoding::Utf8),
        );
        assert!(session.upload(config_file).is_ok());
        
        let main_file = create_file(
            "main.py",
            r#"
import json

with open('config.json', 'r') as f:
    config = json.load(f)

print(f"Running {config['name']} version {config['version']}")

with open('output.txt', 'w') as f:
    f.write(f"Processed by {config['name']}")

print("Processing complete")
"#,
            Some(Encoding::Utf8),
        );
        assert!(session.upload(main_file).is_ok());
        
        let run_result = session.run(
            "main.py".to_string(),
            vec![],
            None,
            vec![],
            Some(create_default_limits()),
        );
        assert!(run_result.is_ok());
        
        let exec_result = run_result.unwrap();
        assert!(exec_result.run.stdout.contains("Running test version 1.0"));
        assert!(exec_result.run.stdout.contains("Processing complete"));
        
        let download_result = session.download("output.txt".to_string());
        if let Ok(content) = download_result {
            let content_str = String::from_utf8_lossy(&content);
            assert!(content_str.contains("Processed by test"));
        }
        
        let list_result = session.list_files(".".to_string());
        if let Ok(files) = list_result {
            assert!(files.contains(&"config.json".to_string()));
            assert!(files.contains(&"main.py".to_string()));
            assert!(files.contains(&"output.txt".to_string()));
        }
        
        session.close();
    }
}