use crate::ExecutionSession;

#[repr(u8)]
#[cfg_attr(
    feature = "golem",
    derive(golem_rust::FromValueAndType, golem_rust::IntoValue)
)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum LanguageKind {
    Javascript,
    Python,
}

#[cfg_attr(
    feature = "golem",
    derive(golem_rust::FromValueAndType, golem_rust::IntoValue)
)]
#[derive(Clone, Debug, PartialEq)]
pub struct Language {
    pub kind: LanguageKind,
    pub version: Option<String>,
}

#[repr(u8)]
#[cfg_attr(
    feature = "golem",
    derive(golem_rust::FromValueAndType, golem_rust::IntoValue)
)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Encoding {
    Utf8,
    Base64,
    Hex,
}

#[cfg_attr(
    feature = "golem",
    derive(golem_rust::FromValueAndType, golem_rust::IntoValue)
)]
#[derive(Clone, Debug, PartialEq)]
pub struct File {
    pub name: String,
    pub content: Vec<u8>,
    pub encoding: Option<Encoding>,
}

#[cfg_attr(
    feature = "golem",
    derive(golem_rust::FromValueAndType, golem_rust::IntoValue)
)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Limits {
    pub time_ms: Option<u64>,
    pub memory_bytes: Option<u64>,
    pub file_size_bytes: Option<u64>,
    pub max_processes: Option<u32>,
}

#[cfg_attr(
    feature = "golem",
    derive(golem_rust::FromValueAndType, golem_rust::IntoValue)
)]
#[derive(Clone, Debug, PartialEq)]
pub struct StageResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub signal: Option<String>,
}

#[cfg_attr(
    feature = "golem",
    derive(golem_rust::FromValueAndType, golem_rust::IntoValue)
)]
#[derive(Clone, Debug, PartialEq)]
pub struct ExecResult {
    pub compile: Option<StageResult>,
    pub run: StageResult,
    pub time_ms: Option<u64>,
    pub memory_bytes: Option<u64>,
}

#[cfg_attr(
    feature = "golem",
    derive(golem_rust::FromValueAndType, golem_rust::IntoValue)
)]
#[derive(Clone, Debug, PartialEq)]
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

#[cfg_attr(
    feature = "golem",
    derive(golem_rust::FromValueAndType, golem_rust::IntoValue)
)]
#[derive(Clone, Debug, PartialEq)]
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
