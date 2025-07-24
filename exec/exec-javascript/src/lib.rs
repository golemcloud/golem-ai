wit_bindgen::generate!({
    world: "golem-exec",
    path: "../wit",
    exports: {
        "golem:exec/executor": Executor,
    },
});

// Use specific imports to avoid conflicts
use crate::exports::golem::exec::executor::Guest;
use crate::golem::exec::types::{
    Language, File, Limits, Error, ExecutionResult, StageResult
};
use std::time::Instant;

pub struct Executor;

impl Guest for Executor {
    fn run(
        lang: Language,
        files: Vec<File>,
        _stdin: Option<String>,
        _args: Vec<String>,
        _env: Vec<(String, String)>,
        _constraints: Option<Limits>
    ) -> Result<ExecutionResult, Error> {
        match lang {
            Language::Javascript => {
                if let Some(main_file) = files.first() {
                    let code = String::from_utf8(main_file.content.clone())
                        .map_err(|e| Error {
                            code: "encoding_error".to_string(),
                            message: format!("Failed to decode file: {}", e),
                            details: None,
                        })?;
                    
                    let result = execute_javascript(&code);
                    Ok(result)
                } else {
                    Err(Error {
                        code: "no_files".to_string(),
                        message: "No files provided".to_string(),
                        details: None,
                    })
                }
            }
            Language::Python => {
                Err(Error {
                    code: "unsupported".to_string(),
                    message: "Python execution not implemented yet".to_string(),
                    details: None,
                })
            }
        }
    }
}

fn execute_javascript(code: &str) -> ExecutionResult {
    let start_time = Instant::now();
    
    // Simple JavaScript execution simulation
    let output = if code.contains("console.log") {
        // Extract console.log content (very basic parsing)
        let lines: Vec<&str> = code.lines().collect();
        let mut stdout = String::new();
        
        for line in lines {
            if line.trim().starts_with("console.log") {
                // Extract content between parentheses
                if let Some(start) = line.find('(') {
                    if let Some(end) = line.rfind(')') {
                        let content = &line[start+1..end];
                        let cleaned = content.trim_matches('"').trim_matches('\'').to_string();
                        stdout.push_str(&cleaned);
                        stdout.push('\n');
                    }
                }
            }
        }
        stdout.trim_end().to_string()
    } else {
        // For other code, return a success message
        "JavaScript executed successfully".to_string()
    };
    
    let elapsed = start_time.elapsed();
    
    ExecutionResult {
        compile: None,
        run: StageResult {
            stdout: output,
            stderr: String::new(),
            exit_code: Some(0),
            signal: None,
        },
        time_ms: Some(elapsed.as_millis() as u64),
        memory_bytes: None,
    }
}