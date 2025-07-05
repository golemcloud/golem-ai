// golem-exec/exec-javascript/src/lib.rs
#![allow(warnings)]

mod bindings;

use std::cell::RefCell;
use std::fs;
use std::rc::Rc;

use bindings::exports::golem::exec::executor::Guest as ExecutorGuest;
use bindings::exports::golem::exec::session::GuestSession as SessionGuest;
use bindings::exports::golem::exec::session::Guest as GuestSession;
use bindings::golem::exec::input_stream::InputStream;
use bindings::golem::exec::output_stream::{ExecEvent, OutputStream};
use bindings::golem::exec::types::{Error, ExecResult, File, Language, LanguageKind, Limits, StageResult};
use rquickjs::{Context, Runtime};

fn stage_error(msg: &str) -> StageResult {
    StageResult {
        stdout: "".into(),
        stderr: msg.to_string(),
        exit_code: Some(1),
        signal: None,
    }
}

fn run_quickjs(code: &str) -> Result<StageResult, Error> {
    let rt = Runtime::new().map_err(|e| Error::Internal(e.to_string()))?;
    let ctx = Context::full(&rt).map_err(|e| Error::Internal(e.to_string()))?;
    let _guard = ctx.with(|ctx| {
        match ctx.eval::<String, _>(code) {
            Ok(output) => Ok(StageResult {
                stdout: output,
                stderr: "".into(),
                exit_code: Some(0),
                signal: None,
            }),
            Err(e) => Ok(StageResult {
                stdout: "".into(),
                stderr: e.to_string(),
                exit_code: Some(1),
                signal: None,
            }),
        }
    });

    Ok(_guard.map_err(|e:anyhow::Error| Error::Internal(e.to_string()))?)
}

pub struct ExecutorImpl;

impl bindings::exports::golem::exec::session::Guest for ExecutorImpl {
    type Session = JavascriptSession;
}

impl ExecutorGuest for ExecutorImpl {

    fn run(
        lang: Language,
        files: Vec<File>,
        _stdin: Option<String>,
        _args: Vec<String>,
        _env: Vec<(String, String)>,
        _constraints: Option<Limits>,
    ) -> Result<ExecResult, Error> {
        if lang.kind != LanguageKind::Javascript {
            return Err(Error::UnsupportedLanguage);
        }

        let file = files
            .iter()
            .find(|f| f.name == "main.js")
            .ok_or_else(|| Error::CompilationFailed(stage_error("main.js not found")))?;

        let stage = run_quickjs(std::str::from_utf8(&file.content).unwrap())?;

        Ok(ExecResult {
            compile: None,
            run: stage,
            time_ms: None,
            memory_bytes: None,
        })
    }

    fn run_streaming(
        lang: Language,
        files: Vec<File>,
        _stdin: Option<&InputStream>,
        _args: Vec<String>,
        _env: Vec<(String, String)>,
        _constraints: Option<Limits>,
        output: &OutputStream,
    ) -> Result<(), Error> {
        if lang.kind != LanguageKind::Javascript {
            let _ = output.push(&ExecEvent::Failed(Error::UnsupportedLanguage));
            return Ok(());
        }

        let file = files
            .iter()
            .find(|f| f.name == "main.js")
            .ok_or_else(|| {
                let err = Error::CompilationFailed(stage_error("main.js not found"));
                let _ = output.push(&ExecEvent::Failed(err.clone()));
                err
            })?;

        let stage = run_quickjs(std::str::from_utf8(&file.content).unwrap())?;

        let _ = output.push(&ExecEvent::Finished(ExecResult {
            compile: None,
            run: stage,
            time_ms: None,
            memory_bytes: None,
        }));
        let _ = output.close();

        Ok(())
    }
}

pub struct JavascriptSession {
    lang: Language,
    files: Rc<RefCell<Vec<File>>>,
    working_dir: Option<String>,
}




impl SessionGuest for JavascriptSession {
    fn new(lang: Language) -> Self {
        Self {
            lang,
            files: Rc::new(RefCell::new(vec![])),
            working_dir: None,
        }
    }

    fn upload(&self, file: File) -> Result<(), Error> {
        self.files.borrow_mut().push(file);
        Ok(())
    }

    fn run(
        &self,
        entrypoint: String,
        _args: Vec<String>,
        _stdin: Option<String>,
        _env: Vec<(String, String)>,
        _constraints: Option<Limits>,
    ) -> Result<ExecResult, Error> {
        let binding = self.files.borrow();
        let content = binding
            .iter()
            .find(|f| f.name == entrypoint)
            .ok_or_else(|| Error::CompilationFailed(stage_error("entrypoint not found")))?;

        let stage = run_quickjs(std::str::from_utf8(&content.content).unwrap())?;

        Ok(ExecResult {
            compile: None,
            run: stage,
            time_ms: None,
            memory_bytes: None,
        })
    }

    fn run_streaming(
        &self,
        entrypoint: String,
        _args: Vec<String>,
        _stdin: Option<&InputStream>,
        _env: Vec<(String, String)>,
        _constraints: Option<Limits>,
        output: &OutputStream,
    ) -> Result<(), Error> {
        let binding = self.files.borrow();
        let content = binding
            .iter()
            .find(|f| f.name == entrypoint)
            .ok_or_else(|| {
                let err = Error::CompilationFailed(stage_error("entrypoint not found"));
                let _ = output.push(&ExecEvent::Failed(err.clone()));
                err
            })?;

        let stage = run_quickjs(std::str::from_utf8(&content.content).unwrap())?;
        let _ = output.push(&ExecEvent::Finished(ExecResult {
            compile: None,
            run: stage,
            time_ms: None,
            memory_bytes: None,
        }));
        let _ = output.close();

        Ok(())
    }

    fn download(&self, _path: String) -> Result<Vec<u8>, Error> {
        Err(Error::Internal("download not supported".into()))
    }

    fn list_files(&self, _dir: String) -> Result<Vec<String>, Error> {
        Ok(self
            .files
            .borrow()
            .iter()
            .map(|f| f.name.clone())
            .collect())
    }

    fn set_working_dir(&self, path: String) -> Result<(), Error> {
        let mut wd = self.working_dir.clone();
        wd = Some(path);
        Ok(())
    }

    fn close(&self) {}
}

bindings::export!(ExecutorImpl with_types_in bindings);