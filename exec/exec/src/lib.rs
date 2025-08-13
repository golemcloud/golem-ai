#[cfg(feature = "javascript")]
pub mod javascript;

#[cfg(feature = "python")]
pub mod python;

mod component;
mod durability;

wit_bindgen::generate!({
    path: "../wit",
    world: "exec-library",
    generate_all,
    generate_unused_types: true,
    additional_derives: [PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue],
    pub_export_macro: true,
});

pub use crate::exports::golem;
use crate::golem::exec::executor::Error;
use crate::golem::exec::types::{Encoding, File, StageResult};
pub use __export_exec_library_impl as export_exec;
use base64::Engine;

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
