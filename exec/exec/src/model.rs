use crate::ExecutionSession;

#[repr(u8)]
#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    Ord,
    PartialEq,
    PartialOrd,
    golem_rust::FromValueAndType,
    golem_rust::IntoValue,
)]
pub enum LanguageKind {
    Javascript,
    Python,
}

#[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
pub struct Language {
    pub kind: LanguageKind,
    pub version: Option<String>,
}

#[repr(u8)]
#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    Ord,
    PartialEq,
    PartialOrd,
    golem_rust::FromValueAndType,
    golem_rust::IntoValue,
)]
pub enum Encoding {
    Utf8,
    Base64,
    Hex,
}

#[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
pub struct File {
    pub name: String,
    pub content: Vec<u8>,
    pub encoding: Option<Encoding>,
}

#[derive(Clone, Copy, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
pub struct Limits {
    pub time_ms: Option<u64>,
    pub memory_bytes: Option<u64>,
    pub file_size_bytes: Option<u64>,
    pub max_processes: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
pub struct StageResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub signal: Option<String>,
}

#[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
pub struct ExecResult {
    pub compile: Option<StageResult>,
    pub run: StageResult,
    pub time_ms: Option<u64>,
    pub memory_bytes: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
pub enum Error {
    UnsupportedLanguage,
    CompilationFailed(StageResult),
    RuntimeFailed(StageResult),
    Timeout,
    ResourceExceeded,
    Internal(String),
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for Error {}

#[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
pub struct RunOptions {
    pub stdin: Option<String>,
    pub args: Option<Vec<String>>,
    pub env: Option<Vec<(String, String)>>,
    pub limits: Option<Limits>,
}

pub struct Session {
    inner: Box<dyn ExecutionSession>,
}

impl Session {
    pub fn new<T: ExecutionSession>(val: T) -> Self {
        Self {
            inner: Box::new(val),
        }
    }

    pub fn get<T: ExecutionSession>(&self) -> &T {
        self.inner
            .as_any()
            .downcast_ref::<T>()
            .expect("Session type mismatch")
    }

    pub fn get_mut<T: ExecutionSession>(&mut self) -> &mut T {
        self.inner
            .as_any_mut()
            .downcast_mut::<T>()
            .expect("Session type mismatch")
    }
}

impl std::ops::Deref for Session {
    type Target = dyn ExecutionSession;

    fn deref(&self) -> &Self::Target {
        &*self.inner
    }
}

impl std::ops::DerefMut for Session {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.inner
    }
}

impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Session").finish()
    }
}
