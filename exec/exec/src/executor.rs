use crate::encoding::*;
use crate::error::{runtime, validation};
use crate::types::*;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct ExecConfig {
    pub global_config: crate::config::ExecGlobalConfig,
}

impl ExecConfig {
    pub fn from_env() -> Self {
        let global_config = crate::config::ExecGlobalConfig::from_env();

        global_config.print_summary();

        if let Err(err) = global_config.validate() {
            log::error!("Invalid configuration: {err}");
        }

        Self { global_config }
    }

    pub fn apply_defaults(&self, constraints: Option<Limits>) -> Option<Limits> {
        let mut limits = constraints.unwrap_or(Limits {
            time_ms: None,
            memory_bytes: None,
            file_size_bytes: None,
            max_processes: None,
        });

        if limits.time_ms.is_none() {
            limits.time_ms = Some(self.global_config.default_timeout_ms);
        }

        if limits.memory_bytes.is_none() {
            if let Some(mb) = self.global_config.default_memory_limit_mb {
                limits.memory_bytes = Some(mb * 1024 * 1024); // Convert MB to bytes
            }
        }

        if limits.max_processes.is_none() {
            limits.max_processes = self.global_config.max_processes;
        }

        if limits.time_ms.is_none()
            && limits.memory_bytes.is_none()
            && limits.file_size_bytes.is_none()
        {
            None
        } else {
            Some(limits)
        }
    }
}

pub fn run(
    lang: Language,
    files: Vec<File>,
    stdin: Option<String>,
    args: Vec<String>,
    env: Vec<(String, String)>,
    constraints: Option<Limits>,
) -> crate::error::ExecResult<ExecResult> {
    let _start_time = Instant::now();

    validate_language(&lang)?;
    validate_files(&files)?;

    let config = ExecConfig::from_env();
    let effective_constraints = config.apply_defaults(constraints);

    if let Some(ref limits) = effective_constraints {
        crate::limits::utils::validate_limits(limits)?;
    }

    let factory = crate::runtime::RuntimeFactory::new();

    let runtime = factory.create_for_language(lang.kind);

    let result = runtime.execute_blocking(files, stdin, args, env, effective_constraints)?;

    Ok(result)
}

pub fn run_streaming(
    lang: Language,
    files: Vec<File>,
    stdin: Option<String>,
    args: Vec<String>,
    env: Vec<(String, String)>,
    constraints: Option<Limits>,
) -> crate::error::ExecResult<crate::stream::ExecStream> {
    validate_language(&lang)?;
    validate_files(&files)?;

    let config = ExecConfig::from_env();
    let effective_constraints = config.apply_defaults(constraints);

    if let Some(ref limits) = effective_constraints {
        crate::limits::utils::validate_limits(limits)?;
    }

    let factory = crate::runtime::RuntimeFactory::new();

    let runtime = factory.create_for_language(lang.kind);

    let stream = runtime.execute_streaming(files, stdin, args, env, effective_constraints)?;

    Ok(stream)
}

fn validate_language(lang: &Language) -> crate::error::ExecResult<()> {
    match lang.kind {
        LanguageKind::Javascript => {
            if let Some(ref version) = lang.version {
                match version.as_str() {
                    "ES5" | "ES6" | "ES2015" | "ES2016" | "ES2017" | "ES2018" | "ES2019"
                    | "ES2020" | "ES2021" | "ES2022" => Ok(()),
                    _ => Err(validation::unsupported_language_version(
                        "JavaScript",
                        version,
                    )),
                }
            } else {
                Ok(()) // Default to latest supported version
            }
        }
        LanguageKind::Python => {
            if let Some(ref version) = lang.version {
                match version.as_str() {
                    "3.8" | "3.9" | "3.10" | "3.11" | "3.12" => Ok(()),
                    _ => Err(validation::unsupported_language_version("Python", version)),
                }
            } else {
                Ok(()) // Default to Python 3.11
            }
        }
    }
}

fn validate_files(files: &[File]) -> crate::error::ExecResult<()> {
    if files.is_empty() {
        return Err(validation::no_files_provided());
    }

    for file in files {
        if file.name.is_empty() {
            return Err(validation::empty_filename());
        }

        if file.content.is_empty() {
            return Err(validation::empty_file_content(&file.name));
        }

        let config = crate::config::ExecGlobalConfig::from_env();
        let max_file_size = config.max_file_size_bytes;

        if file.content.len() > max_file_size {
            return Err(validation::file_size_exceeded(
                &file.name,
                file.content.len(),
                max_file_size,
            ));
        }

        validate_file_encoding(file)?
    }

    Ok(())
}

pub fn find_entry_point<'a>(files: &'a [File], lang: &Language) -> Result<&'a File, Error> {
    let main_files = match lang.kind {
        LanguageKind::Javascript => vec!["main.js", "index.js", "app.js"],
        LanguageKind::Python => vec!["main.py", "__main__.py", "app.py"],
    };

    for main_name in &main_files {
        if let Some(file) = files.iter().find(|f| f.name == *main_name) {
            return Ok(file);
        }
    }

    let extensions = match lang.kind {
        LanguageKind::Javascript => vec![".js", ".mjs"],
        LanguageKind::Python => vec![".py"],
    };

    for ext in &extensions {
        if let Some(file) = files.iter().find(|f| f.name.ends_with(ext)) {
            return Ok(file);
        }
    }

    files.first().ok_or_else(validation::no_entry_point_found)
}

pub fn prepare_environment(
    files: &[File],
    working_dir: &str,
) -> Result<std::collections::HashMap<String, Vec<u8>>, Error> {
    let mut file_system = std::collections::HashMap::new();

    for file in files {
        let encoding = file.encoding.unwrap_or(Encoding::Utf8);
        let content_str = String::from_utf8_lossy(&file.content);
        let content = decode_content(&content_str, encoding)?;

        let file_path = if file.name.starts_with('/') {
            file.name.clone()
        } else {
            format!("{}/{}", working_dir.trim_end_matches('/'), file.name)
        };

        file_system.insert(file_path, content);
    }

    Ok(file_system)
}

pub fn create_stage_result(
    stdout: String,
    stderr: String,
    exit_code: Option<i32>,
    signal: Option<String>,
) -> StageResult {
    StageResult {
        stdout,
        stderr,
        exit_code,
        signal,
    }
}

pub fn create_success_result(
    compile: Option<StageResult>,
    run: StageResult,
    time_ms: Option<u64>,
    memory_bytes: Option<u64>,
) -> ExecResult {
    ExecResult {
        compile,
        run,
        time_ms,
        memory_bytes,
    }
}

pub fn handle_execution_error(error: std::io::Error, context: &str) -> Error {
    match error.kind() {
        std::io::ErrorKind::TimedOut => Error::Timeout,
        std::io::ErrorKind::PermissionDenied => runtime::spawn_failed(context, error.to_string()),
        std::io::ErrorKind::NotFound => runtime::spawn_failed(context, error.to_string()),
        _ => runtime::spawn_failed(context, error.to_string()),
    }
}

pub mod utils {
    use super::*;
    use std::collections::HashMap;

    pub fn create_test_context(
        lang_kind: LanguageKind,
        code: &str,
        filename: &str,
    ) -> ExecutionContext {
        let language = Language {
            kind: lang_kind,
            version: None,
        };

        let file = File {
            name: filename.to_string(),
            content: code.as_bytes().to_vec(),
            encoding: Some(Encoding::Utf8),
        };

        ExecutionContext::new(language, vec![file], None, vec![], vec![], None)
    }

    pub fn extract_output(result: &ExecResult) -> (String, String) {
        (result.run.stdout.clone(), result.run.stderr.clone())
    }

    pub fn is_success(result: &ExecResult) -> bool {
        result.run.exit_code.unwrap_or(-1) == 0
    }

    pub fn get_duration_ms(result: &ExecResult) -> u64 {
        result.time_ms.unwrap_or(0)
    }

    pub fn create_env_map(env: &[(String, String)]) -> HashMap<String, String> {
        env.iter().cloned().collect()
    }

    pub fn sanitize_filename(name: &str) -> Result<String, Error> {
        let sanitized = name
            .replace("..", "")
            .replace("//", "/")
            .trim_start_matches('/')
            .to_string();

        if sanitized.is_empty() {
            return Err(validation::invalid_filename(
                "Invalid filename after sanitization",
            ));
        }

        if sanitized.len() > 255 {
            return Err(validation::filename_too_long());
        }

        Ok(sanitized)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_language() {
        let js_lang = Language {
            kind: LanguageKind::Javascript,
            version: Some("ES2020".to_string()),
        };
        assert!(validate_language(&js_lang).is_ok());

        let py_lang = Language {
            kind: LanguageKind::Python,
            version: Some("3.9".to_string()),
        };
        assert!(validate_language(&py_lang).is_ok());
    }

    #[test]
    fn test_validate_files() {
        let valid_file = File {
            name: "test.js".to_string(),
            content: "console.log('hello');".as_bytes().to_vec(),
            encoding: Some(Encoding::Utf8),
        };
        assert!(validate_files(&[valid_file]).is_ok());

        assert!(validate_files(&[]).is_err());

        let invalid_file = File {
            name: "".to_string(),
            content: "test".as_bytes().to_vec(),
            encoding: Some(Encoding::Utf8),
        };
        assert!(validate_files(&[invalid_file]).is_err());
    }

    #[test]
    fn test_find_entry_point() {
        let files = vec![
            File {
                name: "helper.js".to_string(),
                content: "// helper".as_bytes().to_vec(),
                encoding: Some(Encoding::Utf8),
            },
            File {
                name: "main.js".to_string(),
                content: "// main".as_bytes().to_vec(),
                encoding: Some(Encoding::Utf8),
            },
        ];

        let lang = Language {
            kind: LanguageKind::Javascript,
            version: None,
        };

        let entry = find_entry_point(&files, &lang).unwrap();
        assert_eq!(entry.name, "main.js");
    }

    #[test]
    fn test_utils() {
        let context =
            utils::create_test_context(LanguageKind::Javascript, "console.log('test');", "test.js");

        assert_eq!(context.language.kind, LanguageKind::Javascript);
        assert_eq!(context.files.len(), 1);
        assert_eq!(context.files[0].name, "test.js");

        let sanitized = utils::sanitize_filename("../../../etc/passwd").unwrap();
        assert_eq!(sanitized, "etc/passwd");
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn test_javascript_streaming_execution() {
        let lang = Language {
            kind: LanguageKind::Javascript,
            version: None,
        };

        let files = vec![File {
            name: "test.js".to_string(),
            content: b"console.log('Hello from streaming!');".to_vec(),
            encoding: Some(Encoding::Utf8),
        }];

        let result = run_streaming(lang, files, None, vec![], vec![], None);
        assert!(result.is_ok(), "Streaming execution should succeed");

        let mut stream = result.unwrap();
        let mut events = Vec::new();

        while let Ok(Some(event)) = stream.get_next() {
            events.push(event);
        }

        assert!(!events.is_empty(), "Stream should produce events");

        let has_output = events.iter().any(|event| {
            matches!(
                event,
                crate::types::ExecEvent::StdoutChunk(_) | crate::types::ExecEvent::Finished(_)
            )
        });

        assert!(has_output, "Stream should contain output or finished event");
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn test_python_streaming_execution() {
        let lang = Language {
            kind: LanguageKind::Python,
            version: None,
        };

        let files = vec![File {
            name: "test.py".to_string(),
            content: b"print('Hello from Python streaming!')".to_vec(),
            encoding: Some(Encoding::Utf8),
        }];

        let result = run_streaming(lang, files, None, vec![], vec![], None);
        assert!(result.is_ok(), "Python streaming execution should succeed");

        let mut stream = result.unwrap();
        let mut events = Vec::new();

        while let Ok(Some(event)) = stream.get_next() {
            events.push(event);
        }

        assert!(!events.is_empty(), "Stream should produce events");

        let has_output = events.iter().any(|event| {
            matches!(
                event,
                crate::types::ExecEvent::StdoutChunk(_) | crate::types::ExecEvent::Finished(_)
            )
        });

        assert!(has_output, "Stream should contain output or finished event");
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn test_streaming_error_handling() {
        let lang = Language {
            kind: LanguageKind::Javascript,
            version: None,
        };

        let files = vec![File {
            name: "error.js".to_string(),
            content: b"throw new Error('Test error');".to_vec(),
            encoding: Some(Encoding::Utf8),
        }];

        let result = run_streaming(lang, files, None, vec![], vec![], None);
        assert!(
            result.is_ok(),
            "Streaming should succeed even with runtime errors"
        );

        let mut stream = result.unwrap();
        let mut events = Vec::new();

        while let Ok(Some(event)) = stream.get_next() {
            events.push(event);
        }

        assert!(
            !events.is_empty(),
            "Stream should produce events even for errors"
        );

        let has_error_indication = events.iter().any(|event| match event {
            crate::types::ExecEvent::StderrChunk(_) => true,
            crate::types::ExecEvent::Finished(result) => result.run.exit_code != Some(0),
            crate::types::ExecEvent::Failed(_) => true,
            _ => false,
        });

        assert!(
            has_error_indication,
            "Stream should indicate error occurred"
        );
    }

    #[test]
    #[cfg(target_arch = "wasm32")]
    fn test_streaming_interface_wasm() {
        let lang = Language {
            kind: LanguageKind::Javascript,
            version: None,
        };

        let files = vec![File {
            name: "test.js".to_string(),
            content: b"console.log('Hello');".to_vec(),
            encoding: Some(Encoding::Utf8),
        }];

        let result = run_streaming(lang, files, None, vec![], vec![], None);

        match result {
            Ok(stream) => {
                let mut events = Vec::new();
                while let Ok(Some(event)) = stream.get_next() {
                    events.push(event);
                }

                assert!(
                    !events.is_empty(),
                    "Stream should produce events even in WASM"
                );

                let has_error = events
                    .iter()
                    .any(|event| matches!(event, ExecEvent::Failed(_) | ExecEvent::StderrChunk(_)));

                assert!(
                    has_error,
                    "Stream should indicate WASM execution limitation"
                );
            }
            Err(_) => {}
        }
    }

    #[test]
    #[cfg(target_arch = "wasm32")]
    fn test_validation_works_in_wasm() {
        let valid_lang = Language {
            kind: LanguageKind::Javascript,
            version: Some("ES2020".to_string()),
        };
        assert!(validate_language(&valid_lang).is_ok());

        let valid_files = vec![File {
            name: "test.js".to_string(),
            content: b"console.log('test');".to_vec(),
            encoding: Some(Encoding::Utf8),
        }];
        assert!(validate_files(&valid_files).is_ok());

        let invalid_files: Vec<File> = vec![];
        assert!(validate_files(&invalid_files).is_err());
    }
}
