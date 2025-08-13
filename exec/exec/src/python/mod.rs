use crate::durability::{EmptySnapshot, SessionSnapshot};
use crate::golem::exec::executor::{Error, ExecResult, File, Language, Limits};
use crate::golem::exec::types::{LanguageKind, StageResult};
use crate::{get_contents_as_string, io_error, stage_result_failure};
use indoc::indoc;
use rustpython::vm::builtins::{PyBaseException, PyBaseExceptionRef, PyStr, PyStrRef};
use rustpython::vm::{
    extend_class, py_class, Interpreter, PyObjectRef, PyRef, PyResult, Settings, VirtualMachine,
};
use rustpython::{vm, InterpreterConfig};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicU32;
use wstd::time::Instant;

static TEMP_DIR_COUNTER: AtomicU32 = AtomicU32::new(0);

fn py_exception_error(vm: &VirtualMachine, err: &PyBaseExceptionRef) -> Error {
    let mut output = String::new();
    match vm.write_exception(&mut output, err) {
        Ok(_) => Error::RuntimeFailed(StageResult {
            stdout: "".to_string(),
            stderr: output,
            exit_code: Some(1),
            signal: None,
        }),
        Err(err) => Error::Internal(format!("Failed to render Python exception: {err}")),
    }
}

pub struct PythonComponent;

impl PythonComponent {
    pub fn run(
        lang: Language,
        snippet: String,
        files: Vec<File>,
        stdin: Option<String>,
        args: Vec<String>,
        env: Vec<(String, String)>,
        constraints: Option<Limits>,
    ) -> Result<ExecResult, Error> {
        let session = PythonSession::new(lang, files);
        session.run(snippet, args, stdin, env, constraints)
    }
}

fn ensure_language_is_supported(lang: &Language) -> Result<(), Error> {
    if lang.kind != LanguageKind::Python {
        Err(Error::UnsupportedLanguage)
    } else {
        Ok(())
    }
}

pub fn make_stdout_object(
    vm: &VirtualMachine,
    write_f: impl Fn(&str, &VirtualMachine) -> PyResult<()> + 'static,
) -> PyObjectRef {
    let ctx = &vm.ctx;
    let cls = PyRef::leak(py_class!(
        ctx,
        "CapturingStdout",
        vm.ctx.types.object_type.to_owned(),
        {}
    ));
    let write_method = vm.new_method(
        "write",
        cls,
        move |_self: PyObjectRef, data: PyStrRef, vm: &VirtualMachine| -> PyResult<()> {
            write_f(data.as_str(), vm)
        },
    );
    let flush_method = vm.new_method("flush", cls, |_self: PyObjectRef| {});
    extend_class!(ctx, cls, {
        "write" => write_method,
        "flush" => flush_method,
    });
    ctx.new_base_object(cls.to_owned(), None)
}

struct PythonSessionState {
    interpreter: Interpreter,
    last_error: Option<PyBaseExceptionRef>,
    cwd: String,
}

pub struct PythonSession {
    lang: Language,
    modules: Vec<File>,
    data_root: PathBuf,
    module_root: PathBuf,
    state: RefCell<Option<PythonSessionState>>,
}

impl PythonSession {
    pub fn data_root(&self) -> &Path {
        &self.data_root
    }

    pub fn set_cwd(&self, path: String) -> Result<(), Error> {
        if let Some(state) = self.state.borrow_mut().as_mut() {
            state.cwd = path;
        }
        Ok(())
    }

    pub fn new(lang: Language, modules: Vec<File>) -> Self {
        let id = TEMP_DIR_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let module_root = PathBuf::from("/tmp")
            .join("py")
            .join("modules")
            .join(id.to_string());
        let data_root = PathBuf::from("/tmp")
            .join("py")
            .join("data")
            .join(id.to_string());
        Self {
            lang,
            modules,
            data_root,
            module_root,
            state: RefCell::new(None),
        }
    }

    pub fn run(
        &self,
        snippet: String,
        args: Vec<String>,
        stdin: Option<String>,
        env: Vec<(String, String)>,
        _constraints: Option<Limits>,
    ) -> Result<ExecResult, Error> {
        self.ensure_initialized()?;
        ensure_language_is_supported(&self.lang)?;

        let start = Instant::now();

        let maybe_state = self.state.borrow();
        let state = maybe_state.as_ref().unwrap();
        let mut result = None;

        let _vm_res: Result<(), PyRef<PyBaseException>> = state.interpreter.enter(|vm| {
            let code_obj = vm
                .compile(&snippet, vm::compiler::Mode::Exec, "<snippet>".to_string())
                .map_err(|err| vm.new_syntax_error(&err, Some(&snippet)))?;

            let scope = vm.new_scope_with_builtins();
            scope.globals.set_item(
                "__external_stdin",
                vm.new_pyobj(stdin.unwrap_or_default()),
                vm,
            )?;

            let env_pairs = env
                .iter()
                .map(|(k, v)| vm.new_pyobj((k, v)))
                .collect::<Vec<_>>();
            scope
                .globals
                .set_item("__env", vm.new_pyobj(env_pairs), vm)?;

            scope.globals.set_item(
                "__argv",
                vm.new_pyobj(args.iter().map(|s| vm.new_pyobj(s)).collect::<Vec<_>>()),
                vm,
            )?;

            scope.globals.set_item(
                "__module_root",
                vm.new_pyobj(self.module_root.to_string_lossy().to_string()),
                vm,
            )?;

            scope.globals.set_item(
                "__data_root",
                vm.new_pyobj(self.data_root.to_string_lossy().to_string()),
                vm,
            )?;

            scope
                .globals
                .set_item("__cwd", vm.new_pyobj(state.cwd.clone()), vm)?;

            let init_script = indoc!(
                r#"import io
                import os
                import sys
                import builtins

                __stdout = io.StringIO('')
                __stderr = io.StringIO('')
                __stdin = io.StringIO(__external_stdin)
                sys.stdout = __stdout
                sys.stderr = __stderr
                sys.stdin = __stdin

                sys.argv = __argv
                os.environ = dict(__env)

                class RestrictedFileSystem:
                    def __init__(self, base_directory):
                        self.base_directory = os.path.abspath(base_directory)
                        self._open = builtins.open
                        self._listdir = os.listdir
                        self._mkdir = os.mkdir
                        self._makedirs = os.makedirs
                        self._remove = os.remove
                        self._rmdir = os.rmdir
                        self._rename = os.rename

                    def open(self, path, *args, **kwargs):
                        path = self._to_abs_path(path)
                        return self._open(path, *args, **kwargs)

                    def getcwd(self):
                        return self._cwd

                    def listdir(self, path='.'):
                        path = self._to_abs_path(path)
                        return self._listdir(path)

                    def mkdir(self, path):
                        path = self._to_abs_path(path)
                        self._mkdir(path)

                    def makedirs(self, path):
                        path = self._to_abs_path(path)
                        self._makedirs(path)

                    def remove(self, path):
                        path = self._to_abs_path(path)
                        self._remove(path)

                    def rmdir(self, path):
                        path = self._to_abs_path(path)
                        self._rmdir(path)

                    def rename(self, src, dst):
                        src = self._to_abs_path(src)
                        dst = self._to_abs_path(dst)
                        self._rename(src, dst)

                    def set_cwd(self, path):
                        self._cwd = path

                    def _to_abs_path(self, path):
                        cwd = self._get_abs_cwd()
                        return os.path.join(cwd, path)

                    def _get_abs_cwd(self):
                        if self._cwd.startswith('/'):
                            path = os.path.join(self.base_directory, self._cwd[1:])
                        else:
                            path = os.path.join(self.base_directory, self._cwd)
                        if os.path.commonprefix([self.base_directory, path]) != self.base_directory:
                            raise OSError("Access denied: path is outside the data root")
                        return path
                if not globals().get('__fs_patched', False):
                    __restricted_fs = RestrictedFileSystem(__data_root)

                    builtins.open = __restricted_fs.open
                    os.getcwd = __restricted_fs.getcwd
                    os.listdir = __restricted_fs.listdir
                    os.mkdir = __restricted_fs.mkdir
                    os.makedirs = __restricted_fs.makedirs
                    os.remove = __restricted_fs.remove
                    os.rmdir = __restricted_fs.rmdir
                    os.rename = __restricted_fs.rename

                    __fs_patched = True

                __restricted_fs.set_cwd(__cwd)
                "#
            );
            match vm.run_code_string(scope.clone(), init_script, "<init>".to_string()) {
                Ok(_) => {}
                Err(err) => {
                    let err = py_exception_error(vm, &err);
                    result = Some(Err(err.clone()));
                    return Ok(());
                }
            }

            match vm.run_code_obj(code_obj, scope.clone()) {
                Ok(_) => {
                    let stdout = vm.sys_module.get_attr("stdout", vm)?;
                    let stderr = vm.sys_module.get_attr("stderr", vm)?;

                    let stdout_getvalue = stdout.get_attr("getvalue", vm)?;
                    let stderr_getvalue = stderr.get_attr("getvalue", vm)?;

                    let stdout =
                        unsafe { stdout_getvalue.call((), vm)?.downcast_unchecked::<PyStr>() };
                    let stderr =
                        unsafe { stderr_getvalue.call((), vm)?.downcast_unchecked::<PyStr>() };

                    let stdout = stdout.as_str();
                    let stderr = stderr.as_str();

                    result = Some(Ok(ExecResult {
                        compile: None,
                        run: StageResult {
                            stdout: stdout.to_string(),
                            stderr: stderr.to_string(),
                            exit_code: Some(0),
                            signal: None,
                        },
                        time_ms: Some(start.elapsed().as_millis() as u64),
                        memory_bytes: None,
                    }));
                }
                Err(err) => {
                    let err = py_exception_error(vm, &err);
                    result = Some(Err(err));
                }
            }

            Ok(())
        });

        result.unwrap()
    }

    fn ensure_initialized(&self) -> Result<(), Error> {
        let state = self.state.borrow_mut().take();
        match state {
            None => {
                let state = self.initialize()?;
                *self.state.borrow_mut() = Some(state);
            }
            Some(state) => {
                *self.state.borrow_mut() = Some(state);
            }
        }
        Ok(())
    }

    fn initialize(&self) -> Result<PythonSessionState, Error> {
        std::fs::create_dir_all(&self.module_root).map_err(io_error)?;

        let mut settings =
            Settings::default().with_path(self.module_root.to_string_lossy().to_string());
        settings.ignore_environment = true;

        let config = InterpreterConfig::new().settings(settings).init_stdlib();
        let interpreter = config.interpreter();

        for file in &self.modules {
            let name = &file.name;
            let path = self.module_root.join(name);
            if let Some(parent) = path.parent() {
                if let Err(err) = std::fs::create_dir_all(parent) {
                    return Err(io_error(err));
                }
            }
            if let Some(content) = get_contents_as_string(file) {
                if let Err(err) = std::fs::write(&path, content) {
                    return Err(io_error(err));
                }
            } else {
                return Err(Error::CompilationFailed(stage_result_failure(format!(
                    "Invalid file encoding for {}",
                    file.name
                ))));
            }
        }

        Ok(PythonSessionState {
            interpreter,
            last_error: None,
            cwd: "/".to_string(),
        })
    }
}

impl SessionSnapshot<PythonSession> for PythonSession {
    type Snapshot = EmptySnapshot;

    fn supports_snapshot(_session: &PythonSession) -> bool {
        false
    }

    fn take_snapshot(_session: &PythonSession) -> Self::Snapshot {
        EmptySnapshot {}
    }

    fn restore_snapshot(_session: &PythonSession, _snapshot: Self::Snapshot) {}
}

impl Drop for PythonSession {
    fn drop(&mut self) {
        if let Some(mut state) = self.state.borrow_mut().take() {
            state.interpreter.finalize(state.last_error.take());
        }

        let _ = std::fs::remove_dir_all(&self.data_root);
        let _ = std::fs::remove_dir_all(&self.module_root);
    }
}
