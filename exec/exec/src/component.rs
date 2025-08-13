use crate::durability::{DurableExec, EmptySnapshot, SessionSnapshot};
use crate::golem::exec::executor::{
    Error, ExecResult, File, Guest, GuestSession, Language, Limits,
};
use crate::golem::exec::types::LanguageKind;
use crate::{get_contents, io_error, stage_result_failure};
use std::path::PathBuf;

struct Component;

impl Guest for Component {
    type Session = Session;

    fn run(
        lang: Language,
        snippet: String,
        modules: Vec<File>,
        stdin: Option<String>,
        args: Vec<String>,
        env: Vec<(String, String)>,
        constraints: Option<Limits>,
    ) -> Result<ExecResult, Error> {
        match &lang.kind {
            LanguageKind::Javascript => {
                #[cfg(feature = "javascript")]
                {
                    let session = crate::javascript::JavaScriptSession::new(lang, modules);
                    session.run(snippet, args, stdin, env, constraints)
                }
                #[cfg(not(feature = "javascript"))]
                {
                    Err(Error::UnsupportedLanguage)
                }
            }
            LanguageKind::Python => {
                #[cfg(feature = "python")]
                {
                    let session = crate::python::PythonSession::new(lang, modules);
                    session.run(snippet, args, stdin, env, constraints)
                }
                #[cfg(not(feature = "python"))]
                {
                    Err(Error::UnsupportedLanguage)
                }
            }
        }
    }
}

#[allow(dead_code)]
#[allow(clippy::large_enum_variant)]
enum Session {
    #[cfg(feature = "javascript")]
    Javascript(crate::javascript::JavaScriptSession),
    #[cfg(feature = "python")]
    Python(crate::python::PythonSession),
    Unsupported,
}

impl Session {
    fn data_root(&self) -> Result<PathBuf, Error> {
        match self {
            #[cfg(feature = "javascript")]
            Session::Javascript(session) => Ok(session.data_root().to_path_buf()),
            #[cfg(feature = "python")]
            Session::Python(session) => Ok(session.data_root().to_path_buf()),
            Session::Unsupported => Err(Error::UnsupportedLanguage),
        }
    }

    fn set_cwd(&self, path: String) -> Result<(), Error> {
        match self {
            #[cfg(feature = "javascript")]
            Session::Javascript(session) => session.set_cwd(path),
            #[cfg(feature = "python")]
            Session::Python(session) => session.set_cwd(path),
            Session::Unsupported => Err(Error::UnsupportedLanguage),
        }
    }
}

impl GuestSession for Session {
    fn new(lang: Language, modules: Vec<File>) -> Self {
        match &lang.kind {
            LanguageKind::Javascript => {
                #[cfg(feature = "javascript")]
                {
                    Session::Javascript(crate::javascript::JavaScriptSession::new(lang, modules))
                }
                #[cfg(not(feature = "javascript"))]
                {
                    Session::Unsupported
                }
            }
            LanguageKind::Python => {
                #[cfg(feature = "python")]
                {
                    Session::Python(crate::python::PythonSession::new(lang, modules))
                }
                #[cfg(not(feature = "python"))]
                {
                    Session::Unsupported
                }
            }
        }
    }

    fn upload(&self, file: File) -> Result<(), Error> {
        let path = self.data_root()?.join(&file.name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|err| Error::Internal(err.to_string()))?;
        }
        let contents = get_contents(&file).ok_or_else(|| {
            Error::CompilationFailed(stage_result_failure("Invalid file encoding"))
        })?;

        std::fs::write(&path, contents).map_err(|err| {
            Error::Internal(format!("Failed to write file {}: {}", file.name, err))
        })?;

        Ok(())
    }

    fn run(
        &self,
        snippet: String,
        args: Vec<String>,
        stdin: Option<String>,
        env: Vec<(String, String)>,
        constraints: Option<Limits>,
    ) -> Result<ExecResult, Error> {
        match self {
            #[cfg(feature = "javascript")]
            Session::Javascript(session) => session.run(snippet, args, stdin, env, constraints),
            #[cfg(feature = "python")]
            Session::Python(session) => session.run(snippet, args, stdin, env, constraints),
            Session::Unsupported => Err(Error::UnsupportedLanguage),
        }
    }

    fn download(&self, path: String) -> Result<Vec<u8>, Error> {
        let full_path = self.data_root()?.join(&path);
        if !full_path.exists() {
            return Err(Error::Internal(format!(
                "File {} does not exist",
                full_path.display()
            )));
        }
        std::fs::read(&full_path)
            .map_err(|err| Error::Internal(format!("Failed to read file {path}: {err}")))
    }

    fn list_files(&self, dir: String) -> Result<Vec<String>, Error> {
        let path = self.data_root()?.join(&dir);
        let mut result = Vec::new();
        for entry in std::fs::read_dir(path).map_err(io_error)? {
            let entry = entry.map_err(io_error)?;
            if entry.metadata().map_err(io_error)?.is_file() {
                result.push(entry.file_name().to_string_lossy().to_string());
            }
        }
        Ok(result)
    }

    fn set_working_dir(&self, path: String) -> Result<(), Error> {
        self.set_cwd(path)?;
        Ok(())
    }
}

impl SessionSnapshot<Session> for Component {
    type Snapshot = EmptySnapshot;

    fn supports_snapshot(session: &Session) -> bool {
        match session {
            #[cfg(feature = "javascript")]
            Session::Javascript(session) => {
                crate::javascript::JavaScriptSession::supports_snapshot(session)
            }
            #[cfg(feature = "python")]
            Session::Python(session) => crate::python::PythonSession::supports_snapshot(session),
            Session::Unsupported => false,
        }
    }

    fn take_snapshot(session: &Session) -> Self::Snapshot {
        match session {
            #[cfg(feature = "javascript")]
            Session::Javascript(session) => {
                crate::javascript::JavaScriptSession::take_snapshot(session)
            }
            #[cfg(feature = "python")]
            Session::Python(session) => crate::python::PythonSession::take_snapshot(session),
            Session::Unsupported => EmptySnapshot {},
        }
    }

    fn restore_snapshot(session: &Session, snapshot: Self::Snapshot) {
        match session {
            #[cfg(feature = "javascript")]
            Session::Javascript(session) => {
                crate::javascript::JavaScriptSession::restore_snapshot(session, snapshot)
            }
            #[cfg(feature = "python")]
            Session::Python(session) => {
                crate::python::PythonSession::restore_snapshot(session, snapshot)
            }
            Session::Unsupported => {}
        }
    }
}

type DurableComponent = DurableExec<Component>;

crate::export_exec!(DurableComponent with_types_in crate);
