mod builtin;

use crate::durability::{EmptySnapshot, SessionSnapshot};
use crate::golem::exec::executor::{Error, ExecResult, File, Language, Limits};
use crate::golem::exec::types::{LanguageKind, StageResult};
use crate::{get_contents_as_string, stage_result_failure};
use futures::TryFutureExt;
use rquickjs::loader::{BuiltinLoader, BuiltinResolver};
use rquickjs::{async_with, AsyncContext, AsyncRuntime, CatchResultExt, Ctx, Module, Object};
use std::cell::RefCell;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use wstd::future::FutureExt;
use wstd::runtime::block_on;
use wstd::time::{Duration, Instant};

fn js_engine_error(err: rquickjs::Error) -> Error {
    Error::Internal(err.to_string())
}

static TEMP_DIR_COUNTER: AtomicU32 = AtomicU32::new(0);

pub struct JavascriptComponent;

impl JavascriptComponent {
    pub fn run(
        lang: Language,
        snippet: String,
        files: Vec<File>,
        stdin: Option<String>,
        args: Vec<String>,
        env: Vec<(String, String)>,
        constraints: Option<Limits>,
    ) -> Result<ExecResult, Error> {
        let session = JavaScriptSession::new(lang, files);
        session.run(snippet, args, stdin, env, constraints)
    }
}

fn ensure_language_is_supported(lang: &Language) -> Result<(), Error> {
    if lang.kind != LanguageKind::Javascript {
        Err(Error::UnsupportedLanguage)
    } else {
        Ok(())
    }
}

fn set_globals(
    ctx: Ctx<'_>,
    stdin: Option<String>,
    args: Vec<String>,
    env: Vec<(String, String)>,
    cwd: String,
    file_size_limit: Option<u64>,
) -> Result<(), rquickjs::Error> {
    ctx.globals()
        .set("__golem_exec_js_stdin", stdin.unwrap_or_default())?;
    ctx.globals().set("__golem_exec_js_args", args)?;

    let env_obj = Object::new(ctx.clone())?;
    for (key, value) in env {
        env_obj.set(key, value)?;
    }

    ctx.globals().set("__golem_exec_js_env", env_obj)?;
    ctx.globals().set("__golem_exec_js_cwd", cwd)?;
    ctx.globals()
        .set("__golem_exec_js_file_size_limit", file_size_limit)?;

    Ok(())
}

fn run_snippet(
    ctx: Ctx<'_>,
    main_name: String,
    main_content: String,
    data_root: &Path,
) -> Result<(), Error> {
    builtin::fs::init_fs(ctx.clone(), data_root)?;
    builtin::console::init_capturing(ctx.clone())?;
    let module = Module::evaluate(ctx.clone(), main_name, main_content)
        .catch(&ctx)
        .map_err(|err| Error::RuntimeFailed(stage_result_failure(err.to_string())))?;
    module
        .finish::<()>()
        .catch(&ctx)
        .map_err(|err| Error::RuntimeFailed(stage_result_failure(err.to_string())))?;
    Ok(())
}

struct JavaScriptSessionState {
    rt: AsyncRuntime,
    ctx: AsyncContext,
    cwd: String,
}

pub struct JavaScriptSession {
    lang: Language,
    modules: Vec<File>,
    data_root: PathBuf,
    state: RefCell<Option<JavaScriptSessionState>>,
}

impl JavaScriptSession {
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
        let data_root = Path::new("tmp")
            .join("js")
            .join("data")
            .join(TEMP_DIR_COUNTER.fetch_add(1, Ordering::Relaxed).to_string());
        Self {
            lang,
            modules,
            data_root,
            state: RefCell::new(None),
        }
    }

    pub fn run(
        &self,
        snippet: String,
        args: Vec<String>,
        stdin: Option<String>,
        env: Vec<(String, String)>,
        constraints: Option<Limits>,
    ) -> Result<ExecResult, Error> {
        ensure_language_is_supported(&self.lang)?;
        self.ensure_initialized()?;

        block_on(async { self.run_async(snippet, args, stdin, env, constraints).await })
    }

    fn ensure_initialized(&self) -> Result<(), Error> {
        let state = self.state.borrow_mut().take();
        match state {
            None => {
                let state = block_on(async { self.initialize().await })?;
                *self.state.borrow_mut() = Some(state);
            }
            Some(state) => {
                *self.state.borrow_mut() = Some(state);
            }
        }
        Ok(())
    }

    async fn initialize(&self) -> Result<JavaScriptSessionState, Error> {
        let rt = AsyncRuntime::new().map_err(js_engine_error)?;
        let ctx = AsyncContext::full(&rt).await.map_err(js_engine_error)?;

        let mut resolver = BuiltinResolver::default();

        let mut builtin_loader = BuiltinLoader::default();

        for file in &self.modules {
            let name = file.name.clone();
            let contents = get_contents_as_string(file).ok_or_else(|| {
                Error::CompilationFailed(stage_result_failure(format!(
                    "File {name} has invalid content encoding"
                )))
            })?;

            resolver = resolver.with_module(&name);
            builtin_loader = builtin_loader.with_module(&name, contents);
        }

        resolver = builtin::add_module_resolvers(resolver);
        let loader = (builtin_loader, builtin::module_loader());

        rt.set_loader(resolver, loader).await;

        Ok(JavaScriptSessionState {
            rt,
            ctx,
            cwd: "/".to_string(),
        })
    }

    #[allow(clippy::await_holding_refcell_ref)]
    async fn run_async(
        &self,
        snippet: String,
        args: Vec<String>,
        stdin: Option<String>,
        env: Vec<(String, String)>,
        constraints: Option<Limits>,
    ) -> Result<ExecResult, Error> {
        let maybe_state = self.state.borrow();
        let state = maybe_state.as_ref().unwrap();
        let start = Instant::now();

        if let Some(constraints) = constraints {
            if let Some(memory_bytes) = constraints.memory_bytes {
                state.rt.set_memory_limit(memory_bytes as usize).await;
            }
        }

        let abort_state = async_with!(state.ctx => |ctx| {
           set_globals(ctx.clone(), stdin, args, env, state.cwd.clone(), constraints.and_then(|c| c.file_size_bytes) ).map_err(js_engine_error)?;
            builtin::timeout::init_abort(ctx)
        })
        .await?;
        state.rt.idle().await;

        let wiring = builtin::wire_builtins();

        let main_name = "main";
        let main_content = format!("{wiring}\n{snippet}");

        let future = async {
            async_with!(state.ctx => |ctx| {
                run_snippet(ctx, main_name.to_string(), main_content, &self.data_root)
            })
            .await?;
            state.rt.idle().await;
            async_with!(state.ctx => |ctx| {
                    builtin::console::get_captured_output(ctx).map(|(stdout, stderr)| (stdout.join("\n"), stderr.join("\n")))
                })
                .await
        };
        let result = if let Some(timeout_ms) = constraints.and_then(|c| c.time_ms) {
            future
                .timeout(Duration::from_millis(timeout_ms))
                .map_err(|err| match err.kind() {
                    ErrorKind::TimedOut => Error::Timeout,
                    _ => Error::RuntimeFailed(stage_result_failure(format!("{err}"))),
                })
                .await
                .unwrap_or_else(Err)
        } else {
            future.await
        };

        for (_, abort_handle) in abort_state.abort_handles.borrow_mut().drain() {
            abort_handle.abort();
        }
        state.rt.idle().await;

        let (stdout, stderr) = result?;
        let memory_usage = state.rt.memory_usage().await;

        Ok(ExecResult {
            compile: None,
            run: StageResult {
                stdout,
                stderr,
                exit_code: Some(0),
                signal: None,
            },
            time_ms: Some(start.elapsed().as_millis() as u64),
            memory_bytes: Some(memory_usage.memory_used_size as u64),
        })
    }
}

impl SessionSnapshot<JavaScriptSession> for JavaScriptSession {
    type Snapshot = EmptySnapshot;

    fn supports_snapshot(_session: &JavaScriptSession) -> bool {
        false
    }

    fn take_snapshot(_session: &JavaScriptSession) -> Self::Snapshot {
        EmptySnapshot {}
    }

    fn restore_snapshot(_session: &JavaScriptSession, _snapshot: Self::Snapshot) {}
}

impl Drop for JavaScriptSession {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.data_root);
    }
}
