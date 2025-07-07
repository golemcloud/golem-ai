use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LanguageKind {
    Javascript,
    Python,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Language {
    pub kind: LanguageKind,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Encoding {
    Utf8,
    Base64,
    Hex,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct File {
    pub name: String,
    pub content: Vec<u8>,
    pub encoding: Option<Encoding>,
}

#[derive(Debug, Clone)]
pub struct Limits {
    pub time_ms: Option<u64>,
    pub memory_bytes: Option<u64>,
    pub file_size_bytes: Option<u64>,
    pub max_processes: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub signal: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecResult {
    pub compile: Option<StageResult>,
    pub run: StageResult,
    pub time_ms: Option<u64>,
    pub memory_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Error {
    InvalidLanguage(String),
    InvalidFile(String),
    InvalidEncoding(String),
    InvalidLimits(String),
    CompilationFailed(String),
    ExecutionFailed(String),
    Timeout,
    MemoryLimitExceeded,
    OutputLimitExceeded,
    FileSizeLimitExceeded,
    TooManyFiles,
    FileNotFound(String),
    PermissionDenied(String),
    Internal(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecEvent {
    StdoutChunk(Vec<u8>),
    StderrChunk(Vec<u8>),
    Finished(ExecResult),
    Failed(Error),
}

// Conversion implementations for WIT compatibility
#[cfg(target_arch = "wasm32")]
mod wit_conversions {
    use super::*;
    use crate::golem::exec_javascript::types as wit_types;

    impl From<Language> for wit_types::Language {
        fn from(lang: Language) -> Self {
            wit_types::Language {
                kind: match lang.kind {
                    LanguageKind::Javascript => wit_types::LanguageKind::Javascript,
                    LanguageKind::Python => wit_types::LanguageKind::Python,
                },
                version: lang.version,
            }
        }
    }

    impl From<wit_types::Language> for Language {
        fn from(lang: wit_types::Language) -> Self {
            Language {
                kind: match lang.kind {
                    wit_types::LanguageKind::Javascript => LanguageKind::Javascript,
                    wit_types::LanguageKind::Python => LanguageKind::Python,
                },
                version: lang.version,
            }
        }
    }

    impl From<File> for wit_types::File {
        fn from(file: File) -> Self {
            wit_types::File {
                name: file.name,
                content: file.content,
                encoding: file.encoding.map(|e| match e {
                    Encoding::Utf8 => wit_types::Encoding::Utf8,
                    Encoding::Base64 => wit_types::Encoding::Base64,
                    Encoding::Hex => wit_types::Encoding::Hex,
                }),
            }
        }
    }

    impl From<wit_types::File> for File {
        fn from(file: wit_types::File) -> Self {
            File {
                name: file.name,
                content: file.content,
                encoding: file.encoding.map(|e| match e {
                    wit_types::Encoding::Utf8 => Encoding::Utf8,
                    wit_types::Encoding::Base64 => Encoding::Base64,
                    wit_types::Encoding::Hex => Encoding::Hex,
                }),
            }
        }
    }

    impl From<ExecResult> for wit_types::ExecResult {
        fn from(result: ExecResult) -> Self {
            wit_types::ExecResult {
                compile: result.compile.map(|s| wit_types::StageResult {
                    stdout: s.stdout,
                    stderr: s.stderr,
                    exit_code: s.exit_code,
                    signal: s.signal,
                }),
                run: wit_types::StageResult {
                    stdout: result.run.stdout,
                    stderr: result.run.stderr,
                    exit_code: result.run.exit_code,
                    signal: result.run.signal,
                },
                time_ms: result.time_ms,
                memory_bytes: result.memory_bytes,
            }
        }
    }

    impl From<Error> for wit_types::Error {
        fn from(error: Error) -> Self {
            match error {
                Error::InvalidLanguage(_) => wit_types::Error::UnsupportedLanguage,
                Error::InvalidFile(msg) => wit_types::Error::Internal(msg),
                Error::InvalidEncoding(msg) => wit_types::Error::Internal(msg),
                Error::InvalidLimits(msg) => wit_types::Error::Internal(msg),
                Error::CompilationFailed(msg) => {
                    let stage_result = wit_types::StageResult {
                        stdout: String::new(),
                        stderr: msg,
                        exit_code: Some(1),
                        signal: None,
                    };
                    wit_types::Error::CompilationFailed(stage_result)
                }
                Error::ExecutionFailed(msg) => {
                    let stage_result = wit_types::StageResult {
                        stdout: String::new(),
                        stderr: msg,
                        exit_code: Some(1),
                        signal: None,
                    };
                    wit_types::Error::RuntimeFailed(stage_result)
                }
                Error::Timeout => wit_types::Error::Timeout,
                Error::MemoryLimitExceeded => wit_types::Error::ResourceExceeded,
                Error::OutputLimitExceeded => wit_types::Error::ResourceExceeded,
                Error::FileSizeLimitExceeded => wit_types::Error::ResourceExceeded,
                Error::TooManyFiles => wit_types::Error::ResourceExceeded,
                Error::FileNotFound(msg) => wit_types::Error::Internal(msg),
                Error::PermissionDenied(msg) => wit_types::Error::Internal(msg),
                Error::Internal(msg) => wit_types::Error::Internal(msg),
            }
        }
    }

    impl From<ExecEvent> for wit_types::ExecEvent {
        fn from(event: ExecEvent) -> Self {
            match event {
                ExecEvent::StdoutChunk(data) => wit_types::ExecEvent::StdoutChunk(data),
                ExecEvent::StderrChunk(data) => wit_types::ExecEvent::StderrChunk(data),
                ExecEvent::Finished(result) => wit_types::ExecEvent::Finished(result.into()),
                ExecEvent::Failed(error) => wit_types::ExecEvent::Failed(error.into()),
            }
        }
    }

    impl From<wit_types::Limits> for Limits {
        fn from(limits: wit_types::Limits) -> Self {
            Self {
                time_ms: limits.time_ms,
                memory_bytes: limits.memory_bytes,
                file_size_bytes: limits.file_size_bytes,
                max_processes: limits.max_processes,
            }
        }
    }

    impl From<Limits> for wit_types::Limits {
        fn from(limits: Limits) -> Self {
            Self {
                time_ms: limits.time_ms,
                memory_bytes: limits.memory_bytes,
                file_size_bytes: limits.file_size_bytes,
                max_processes: limits.max_processes,
            }
        }
    }
}
