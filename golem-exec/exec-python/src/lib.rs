
#[allow(warnings)]
mod bindings;

use anyhow::Result;
use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use tempfile::tempdir;
use std::cell::RefCell;

use bindings::golem::exec::input_stream::InputStream;
use bindings::golem::exec::output_stream::{ExecEvent, OutputStream};
use bindings::golem::exec::types::{Error, ExecResult, File, Language, LanguageKind, Limits, StageResult};
use bindings::exports::golem::exec::executor::Guest as ExecutorGuest;
use bindings::exports::golem::exec::session::GuestSession as SessionGuest;

fn stage_error(msg: &str) -> StageResult {
    StageResult {
        stdout: "".to_string(),
        stderr: msg.to_string(),
        exit_code: Some(1),
        signal: None,
    }
}

pub struct ExecutorImpl;
impl bindings::exports::golem::exec::session::Guest for ExecutorImpl {
    type Session = PythonSession;
}

impl ExecutorGuest for ExecutorImpl {
    fn run(
        lang: Language,
        files: Vec<File>,
        stdin: Option<String>,
        args: Vec<String>,
        env: Vec<(String, String)>,
        constraints: Option<Limits>,
    ) -> Result<ExecResult, Error> {
        let session = PythonSession::new(lang.clone());
        for file in files {
            session.upload(file)?;
        }
        session.run("main.py".to_string(), args, stdin, env, constraints)
    }

    fn run_streaming(
        lang: Language,
        files: Vec<File>,
        stdin: Option<&InputStream>,
        args: Vec<String>,
        env: Vec<(String, String)>,
        constraints: Option<Limits>,
        output: &OutputStream,
    ) -> Result<(), Error> {
        let session = PythonSession::new(lang.clone());
        for file in files {
            session.upload(file)?;
        }
        session.run_streaming("main.py".to_string(), args, stdin, env, constraints, output)
    }
}


pub struct PythonSession {
    lang: Language,
    files: RefCell<Vec<File>>,
    working_dir: RefCell<Option<String>>,
}

impl SessionGuest for PythonSession {
    fn new(lang: Language) -> Self {
        PythonSession {
            lang,
            files: RefCell::new(Vec::new()),
            working_dir: RefCell::new(None),
        }
    }

    fn upload(&self, file: File) -> Result<(), Error> {
        self.files.borrow_mut().push(file);
        Ok(())
    }

    fn run(
        &self,
        entrypoint: String,
        args: Vec<String>,
        _stdin: Option<String>,
        env: Vec<(String, String)>,
        constraints: Option<Limits>,
    ) -> Result<ExecResult, Error> {
        let tmp_dir = tempdir().map_err(|e| Error::Internal(e.to_string()))?;
        for file in self.files.borrow().iter() {
            let path = tmp_dir.path().join(&file.name);
            fs::write(&path, &file.content).map_err(|e| Error::Internal(e.to_string()))?;
        }

        let entry_path = tmp_dir.path().join(&entrypoint);
        if !entry_path.exists() {
            return Err(Error::CompilationFailed(stage_error("entrypoint not found")));
        }

        let python_path = env::var("EXEC_PYTHON_WASI_PATH").unwrap_or_else(|_| "python3".into());
        let output = Command::new(python_path)
            .arg(entry_path)
            .args(&args)
            .envs(env)
            .output()
            .map_err(|e| Error::Internal(e.to_string()))?;

        let stdout = String::from_utf8_lossy(&output.stdout).into();
        let stderr = String::from_utf8_lossy(&output.stderr).into();
        let exit_code = output.status.code();

        Ok(ExecResult {
            compile: None,
            run: StageResult {
                stdout,
                stderr,
                exit_code,
                signal: None,
            },
            time_ms: constraints.and_then(|c| c.time_ms),
            memory_bytes: None,
        })
    }

    fn run_streaming(
        &self,
        entrypoint: String,
        args: Vec<String>,
        stdin: Option<&InputStream>,
        env: Vec<(String, String)>,
        constraints: Option<Limits>,
        output: &OutputStream,
    ) -> Result<(), Error> {
        if self.lang.kind != LanguageKind::Python {
            let _ = output.push(&ExecEvent::Failed(Error::UnsupportedLanguage));
            return Ok(());
        }

        let tmp_dir = tempdir().map_err(|e| Error::Internal(e.to_string()))?;
        for file in self.files.borrow().iter() {
            let path = tmp_dir.path().join(&file.name);
            fs::write(&path, &file.content).map_err(|e| Error::Internal(e.to_string()))?;
        }

        let entry_path = tmp_dir.path().join(&entrypoint);
        if !entry_path.exists() {
            let _ = output.push(&ExecEvent::Failed(Error::CompilationFailed(stage_error("entrypoint not found"))));
            return Ok(());
        }

        let python_path = env::var("EXEC_PYTHON_WASI_PATH").unwrap_or_else(|_| "python3".into());
        let mut command = Command::new(python_path);
        command.arg(entry_path).args(&args).envs(env);
        command.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());

        let mut child = command.spawn().map_err(|e| Error::Internal(e.to_string()))?;

        if let Some(stream) = stdin {
            if let Some(mut stdin_pipe) = child.stdin.take() {
                while let Some(chunk) = stream.get_next() {
                    stdin_pipe.write_all(&chunk).map_err(|e| {
                        let _ = output.push(&ExecEvent::Failed(Error::RuntimeFailed(stage_error(&e.to_string()))));
                        Error::Internal(e.to_string())
                    })?;
                }
            }
        }

        let mut stdout_reader = BufReader::new(child.stdout.take().unwrap());
        let mut stderr_reader = BufReader::new(child.stderr.take().unwrap());

        let mut buf = Vec::new();
        while stdout_reader.read_until(b'\n', &mut buf).unwrap_or(0) > 0 {
            let _ = output.push(&ExecEvent::StdoutChunk(buf.clone()));
            buf.clear();
        }

        while stderr_reader.read_until(b'\n', &mut buf).unwrap_or(0) > 0 {
            let _ = output.push(&ExecEvent::StderrChunk(buf.clone()));
            buf.clear();
        }

        let status = child.wait().map_err(|e| Error::Internal(e.to_string()))?;

        let _ = output.push(&ExecEvent::Finished(ExecResult {
            compile: None,
            run: StageResult {
                stdout: "".into(),
                stderr: "".into(),
                exit_code: status.code(),
                signal: None,
            },
            time_ms: constraints.and_then(|c| c.time_ms),
            memory_bytes: None,
        }));
        let _ = output.close();

        Ok(())
    }

    fn download(&self, path: String) -> Result<Vec<u8>, Error> {
        fs::read(&path).map_err(|e| Error::Internal(e.to_string()))
    }

    fn list_files(&self, dir: String) -> Result<Vec<String>, Error> {
        let entries = fs::read_dir(&dir).map_err(|e| Error::Internal(e.to_string()))?;
        let mut result = vec![];
        for entry in entries {
            if let Ok(entry) = entry {
                if let Ok(name) = entry.file_name().into_string() {
                    result.push(name);
                }
            }
        }
        Ok(result)
    }

    fn set_working_dir(&self, path: String) -> Result<(), Error> {
        *self.working_dir.borrow_mut() = Some(path);
        Ok(())
    }

    fn close(&self) {}
}

bindings::export!(ExecutorImpl with_types_in bindings);