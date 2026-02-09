#[cfg(feature = "javascript")]
pub mod javascript;

#[cfg(feature = "python")]
pub mod python;

pub mod durability;
mod executor;
pub mod model;

use crate::model::{Encoding, Error, ExecResult, File, Language, RunOptions, StageResult};
use base64::Engine;

pub use executor::DurableExecution;

pub trait ExecutionProvider {
    type Session: ExecutionSession;

    fn run(
        lang: Language,
        modules: Vec<File>,
        snippet: String,
        options: RunOptions,
    ) -> Result<ExecResult, Error>;
}

pub trait ExecutionSession: 'static {
    fn new(lang: Language, modules: Vec<File>) -> Self
    where
        Self: Sized;

    fn upload(&self, file: File) -> Result<(), Error>;

    fn run(&self, snippet: String, options: RunOptions) -> Result<ExecResult, Error>;

    fn download(&self, path: String) -> Result<Vec<u8>, Error>;

    fn list_files(&self, dir: String) -> Result<Vec<String>, Error>;

    fn set_working_dir(&self, path: String) -> Result<(), Error>;

    fn as_any(&self) -> &dyn std::any::Any;

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

pub(crate) fn get_contents_as_string(file: &File) -> Option<String> {
    get_contents(file).and_then(|bytes| String::from_utf8(bytes).ok())
}

pub(crate) fn get_contents(file: &File) -> Option<Vec<u8>> {
    match file.encoding.unwrap_or(Encoding::Utf8) {
        Encoding::Base64 => base64::prelude::BASE64_STANDARD
            .decode(file.content.clone())
            .ok(),
        Encoding::Hex => hex::decode(&file.content).ok(),
        Encoding::Utf8 => Some(file.content.clone()),
    }
}

pub(crate) fn stage_result_failure(message: impl AsRef<str>) -> StageResult {
    StageResult {
        stdout: "".to_string(),
        stderr: message.as_ref().to_string(),
        exit_code: Some(1),
        signal: None,
    }
}

#[allow(dead_code)]
pub(crate) fn io_error(error: std::io::Error) -> Error {
    Error::Internal(format!("IO error: {error}"))
}
