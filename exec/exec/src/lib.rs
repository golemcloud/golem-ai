#[cfg(target_arch = "wasm32")]
use wit_bindgen::generate;

#[cfg(target_arch = "wasm32")]
generate!({
    world: "exec-library",
    path: "../wit",
});

#[cfg(target_arch = "wasm32")]
use crate::exports::golem::exec::exec_stream;
#[cfg(target_arch = "wasm32")]
use crate::exports::golem::exec::executor::Guest as ExecutorGuest;
#[cfg(target_arch = "wasm32")]
use crate::exports::golem::exec::session::{Guest as SessionGuest, GuestSession};

#[cfg(target_arch = "wasm32")]
pub use golem::exec::types::*;

#[cfg(not(target_arch = "wasm32"))]
pub use types::*;

pub mod config;
#[cfg(test)]
mod config_test;
pub mod encoding;
pub mod error;
pub mod executor;
pub mod limits;
pub mod runtime;
pub mod stream;
pub mod types;

pub use error::{
    convert, fs, messages, runtime as runtime_errors, stream as stream_errors, validation,
    ExecResult,
};

#[cfg(feature = "no-logging")]
struct NoOpLogger;

#[cfg(feature = "no-logging")]
impl log::Log for NoOpLogger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        false
    }

    fn log(&self, _record: &log::Record) {}

    fn flush(&self) {}
}

#[allow(dead_code)]
fn init_logging() {
    use std::sync::Once;
    static INIT: Once = Once::new();

    INIT.call_once(|| {
        #[cfg(feature = "no-logging")]
        {
            let _ =
                log::set_logger(&NoOpLogger).map(|()| log::set_max_level(log::LevelFilter::Off));
        }
        #[cfg(not(feature = "no-logging"))]
        {
            if std::env::var("GOLEM_EXEC_LOG").is_ok() {
                #[cfg(all(target_arch = "wasm32", feature = "wasi-logging"))]
                {
                    use std::str::FromStr;
                    // Try to install wasi_logger, but don't fail if WASI logging interface is not available
                    if wasi_logger::Logger::install().is_ok() {
                        let max_level: log::LevelFilter = log::LevelFilter::from_str(
                            &std::env::var("GOLEM_EXEC_LOG").unwrap_or_default(),
                        )
                        .unwrap_or(log::LevelFilter::Info);
                        log::set_max_level(max_level);
                    }
                }
                #[cfg(not(target_arch = "wasm32"))]
                {
                    env_logger::init();
                }
            }
        }
    });
}

#[cfg(target_arch = "wasm32")]
pub struct ExecComponent;

#[cfg(target_arch = "wasm32")]
impl ExecutorGuest for ExecComponent {
    fn run(
        lang: Language,
        files: Vec<File>,
        stdin: Option<String>,
        args: Vec<String>,
        env: Vec<(String, String)>,
        constraints: Option<Limits>,
    ) -> Result<types::ExecResult, Error> {
        init_logging();
        log::info!("Executing code: {:?}", lang);

        let result = executor::run(lang, files, stdin, args, env, constraints);
        match result {
            Ok(exec_result) => Ok(exec_result.into()),
            Err(error) => Err(error.into()),
        }
    }

    fn run_streaming(
        lang: Language,
        files: Vec<File>,
        stdin: Option<String>,
        args: Vec<String>,
        env: Vec<(String, String)>,
        constraints: Option<Limits>,
    ) -> Result<exec_stream::ExecStream, Error> {
        init_logging();
        log::info!("Executing code (streaming): {:?}", lang);

        let result = executor::run_streaming(lang, files, stdin, args, env, constraints);
        match result {
            Ok(stream) => Ok(stream.into()),
            Err(error) => Err(error.into()),
        }
    }
}

#[cfg(target_arch = "wasm32")]
export!(ExecComponent with_types_in crate);

#[cfg(target_arch = "wasm32")]
pub struct ExecSession {
    lang: Language,
    files: std::cell::RefCell<std::collections::HashMap<String, Vec<u8>>>,
    working_dir: std::cell::RefCell<String>,
    session_id: String,
    created_at: std::time::Instant,
    closed: std::cell::RefCell<bool>,
}

#[cfg(target_arch = "wasm32")]
impl GuestSession for ExecSession {
    fn new(lang: Language) -> Self {
        Self {
            lang,
            files: std::cell::RefCell::new(std::collections::HashMap::new()),
            working_dir: std::cell::RefCell::new("/tmp".to_string()),
            session_id: uuid::Uuid::new_v4().to_string(),
            created_at: std::time::Instant::now(),
            closed: std::cell::RefCell::new(false),
        }
    }

    fn upload(&self, file: File) -> ExecResult<()> {
        if *self.closed.borrow() {
            return Err(validation::session_closed());
        }

        if file.name.is_empty() {
            return Err(validation::empty_filename());
        }

        if file.content.is_empty() {
            return Err(validation::empty_file_content(&file.name));
        }

        let config = config::ExecGlobalConfig::from_env();
        let max_file_size = config.max_file_size_bytes;

        if file.content.len() > max_file_size {
            return Err(validation::file_size_exceeded(
                &file.name,
                file.content.len(),
                max_file_size,
            ));
        }

        encoding::validate_file_encoding(&file)?;

        self.files.borrow_mut().insert(file.name, file.content);
        log::info!("Uploaded file to session {}", self.session_id);
        Ok(())
    }

    fn run(
        &self,
        entrypoint: String,
        args: Vec<String>,
        stdin: Option<String>,
        env: Vec<(String, String)>,
        constraints: Option<Limits>,
    ) -> ExecResult<types::ExecResult> {
        let files = self.files.borrow();
        let content = files
            .get(&entrypoint)
            .ok_or_else(|| validation::file_not_found(&entrypoint))?;

        let file = File {
            name: entrypoint,
            content: content.clone(),
            encoding: None,
        };

        executor::run(self.lang.clone(), vec![file], stdin, args, env, constraints)
    }

    fn run_streaming(
        &self,
        entrypoint: String,
        args: Vec<String>,
        stdin: Option<String>,
        env: Vec<(String, String)>,
        constraints: Option<Limits>,
    ) -> ExecResult<exec_stream::ExecStream> {
        let files = self.files.borrow();
        let content = files
            .get(&entrypoint)
            .ok_or_else(|| validation::file_not_found(&entrypoint))?;

        let file = File {
            name: entrypoint,
            content: content.clone(),
            encoding: None,
        };

        let stream =
            executor::run_streaming(self.lang.clone(), vec![file], stdin, args, env, constraints)?;
        Ok(stream.into())
    }

    fn download(&self, path: String) -> ExecResult<Vec<u8>> {
        let files = self.files.borrow();
        files
            .get(&path)
            .cloned()
            .ok_or_else(|| validation::file_not_found(&path))
    }

    fn list_files(&self, dir: String) -> ExecResult<Vec<String>> {
        let files = self.files.borrow();

        if dir.is_empty() || dir == "/" {
            Ok(files.keys().cloned().collect())
        } else {
            let dir_prefix = if dir.ends_with('/') {
                dir
            } else {
                format!("{}/", dir)
            };
            let matching_files: Vec<String> = files
                .keys()
                .filter(|path| path.starts_with(&dir_prefix))
                .map(|path| {
                    let relative = &path[dir_prefix.len()..];
                    if let Some(slash_pos) = relative.find('/') {
                        format!("{}/", &relative[..slash_pos])
                    } else {
                        relative.to_string()
                    }
                })
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();

            Ok(matching_files)
        }
    }

    fn set_working_dir(&self, path: String) -> ExecResult<()> {
        if *self.closed.borrow() {
            return Err(validation::session_closed());
        }

        if path.is_empty() {
            return Err(validation::empty_path());
        }

        if path.starts_with("/") && !path.starts_with("/tmp") && !path.starts_with("/workspace") {
            return Err(validation::invalid_path(&path));
        }

        *self.working_dir.borrow_mut() = path.clone();
        log::info!(
            "Set working directory to '{}' for session {}",
            path,
            self.session_id
        );
        Ok(())
    }

    fn close(&self) -> () {
        if *self.closed.borrow() {
            log::warn!(
                "Attempted to close an already closed session {}",
                self.session_id
            );
            return;
        }

        self.files.borrow_mut().clear();

        *self.closed.borrow_mut() = true;

        log::info!(
            "Closed session {} (duration: {:?})",
            self.session_id,
            self.created_at.elapsed()
        );
    }
}

#[cfg(target_arch = "wasm32")]
impl SessionGuest for ExecComponent {
    type Session = ExecSession;
}
