use crate::golem::exec::types::Error;
use futures::future::AbortHandle;
use rquickjs::function::Args;
use rquickjs::{CatchResultExt, Ctx, JsLifetime, Persistent, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::AtomicUsize;

// Native functions for the timeout implementation (based on wasm-rquickjs)
#[rquickjs::module]
pub mod native_module {
    use crate::javascript::builtin::timeout::get_abort_state;
    use futures::future::abortable;
    use rquickjs::{Ctx, Persistent, Value};
    use std::sync::atomic::Ordering;

    #[rquickjs::function]
    pub fn schedule(
        ctx: Ctx<'_>,
        code_or_fn: Persistent<Value<'static>>,
        delay: Option<u32>,
        periodic: bool,
        args: Persistent<Vec<Value<'static>>>,
    ) -> usize {
        let delay = delay.unwrap_or(0);

        let (task, abort_handle) = abortable(super::scheduled_task(
            ctx.clone(),
            code_or_fn,
            delay,
            periodic,
            args,
        ));

        let state = get_abort_state(ctx.clone()).unwrap();
        let key = state.last_abort_id.fetch_add(1, Ordering::Relaxed);
        ctx.spawn(async move {
            let _ = task.await;
        });
        state.abort_handles.borrow_mut().insert(key, abort_handle);
        key
    }

    #[rquickjs::function]
    pub fn clear_schedule(ctx: Ctx<'_>, timeout_id: usize) {
        let state = get_abort_state(ctx).unwrap();
        let mut abort_handles = state.abort_handles.borrow_mut();
        let handle = abort_handles
            .remove(&timeout_id)
            .expect("No such timeout ID");
        handle.abort();
    }
}

#[derive(Default, JsLifetime)]
pub struct AbortState {
    pub abort_handles: RefCell<HashMap<usize, AbortHandle>>,
    pub last_abort_id: AtomicUsize,
}

pub fn init_abort(ctx: Ctx<'_>) -> Result<Rc<AbortState>, Error> {
    let state = Rc::new(AbortState::default());
    ctx.store_userdata(state.clone())
        .map_err(|err| Error::Internal(err.to_string()))?;
    Ok(state)
}

pub fn get_abort_state(ctx: Ctx<'_>) -> Result<Rc<AbortState>, Error> {
    if let Some(abort_state) = ctx.userdata::<Rc<AbortState>>() {
        Ok(abort_state.clone())
    } else {
        Err(Error::Internal("Abort is not initialized".to_string()))
    }
}

// JS functions for the console implementation
pub const TIMEOUT_JS: &str = include_str!("timeout.js");

// JS code wiring the console module into the global context
pub const WIRE_JS: &str = r#"
        import * as __golem_exec_js_timeout from '__golem_exec_js_builtin/timeout';
        globalThis.setTimeout = __golem_exec_js_timeout.setTimeout;
        globalThis.setImmediate = __golem_exec_js_timeout.setImmediate;
        globalThis.setInterval = __golem_exec_js_timeout.setInterval;
        globalThis.clearTimeout = __golem_exec_js_timeout.clearTimeout;
        globalThis.clearInterval = __golem_exec_js_timeout.clearTimeout;
    "#;

async fn scheduled_task(
    ctx: Ctx<'_>,
    code_or_fn: Persistent<Value<'static>>,
    delay: u32,
    periodic: bool,
    args: Persistent<Vec<Value<'static>>>,
) {
    if delay == 0 {
        run_scheduled_task(ctx.clone(), code_or_fn.clone(), args.clone())
            .catch(&ctx)
            .expect("Failed to run scheduled task");
    } else {
        let duration = wstd::time::Duration::from_millis(delay as u64);

        loop {
            wstd::task::sleep(duration).await;

            run_scheduled_task(ctx.clone(), code_or_fn.clone(), args.clone())
                .catch(&ctx)
                .expect("Failed to run scheduled task");

            if !periodic {
                break;
            }
        }
    }
}

fn run_scheduled_task(
    ctx: Ctx,
    code_or_fn: Persistent<Value<'static>>,
    args: Persistent<Vec<Value<'static>>>,
) -> rquickjs::Result<()> {
    let restored_code_or_fn = code_or_fn.restore(&ctx)?;
    let restored_args = args.restore(&ctx)?;

    if let Some(func) = restored_code_or_fn.as_function() {
        let mut args = Args::new(ctx, restored_args.len());
        args.push_args(&restored_args)?;
        func.call_arg(args)
    } else if let Some(code) = restored_code_or_fn.as_string() {
        if !restored_args.is_empty() {
            panic!("Passing arguments to scheduled code snippets is not supported");
        }
        ctx.eval(code.to_string()?)
    } else {
        panic!("Unsupported value passed to setTimeout or setInterval: {restored_code_or_fn:?}");
    }
}
