use crate::golem::exec::types::Error;
use rquickjs::{Ctx, JsLifetime};
use std::cell::RefCell;

// Native functions for the console implementation
#[rquickjs::module(rename_vars = "camelCase")]
pub mod native_module {
    use rquickjs::Ctx;

    #[rquickjs::function]
    pub fn println(line: String, ctx: Ctx<'_>) {
        super::write_to_stdout(ctx, line);
    }

    #[rquickjs::function]
    pub fn eprintln(line: String, ctx: Ctx<'_>) {
        super::write_to_stderr(ctx, line);
    }

    #[rquickjs::function]
    pub fn trace(line: String, ctx: Ctx<'_>) {
        super::write_to_stdout(ctx, format!("[TRACE] {line}"));
    }

    #[rquickjs::function]
    pub fn debug(line: String, ctx: Ctx<'_>) {
        super::write_to_stdout(ctx, format!("[DEBUG] {line}"));
    }

    #[rquickjs::function]
    pub fn info(line: String, ctx: Ctx<'_>) {
        super::write_to_stdout(ctx, format!("[INFO] {line}"));
    }

    #[rquickjs::function]
    pub fn warn(line: String, ctx: Ctx<'_>) {
        super::write_to_stdout(ctx, format!("[WARN] {line}"));
    }

    #[rquickjs::function]
    pub fn error(line: String, ctx: Ctx<'_>) {
        super::write_to_stdout(ctx, format!("[ERROR] {line}"));
    }
}

#[derive(Default, JsLifetime)]
struct CapturedOutput {
    pub stdout: RefCell<Vec<String>>,
    pub stderr: RefCell<Vec<String>>,
}

pub fn init_capturing(ctx: Ctx<'_>) -> Result<(), Error> {
    let captured_output = CapturedOutput::default();
    ctx.store_userdata(captured_output)
        .map_err(|err| Error::Internal(err.to_string()))?;
    Ok(())
}

pub fn get_captured_output(ctx: Ctx<'_>) -> Result<(Vec<String>, Vec<String>), Error> {
    if let Some(captured_output) = ctx.userdata::<CapturedOutput>() {
        Ok((
            captured_output.stdout.borrow().clone(),
            captured_output.stderr.borrow().clone(),
        ))
    } else {
        Err(Error::Internal(
            "Captured output not initialized".to_string(),
        ))
    }
}

fn write_to_stdout(ctx: Ctx<'_>, line: String) {
    let captured_output = ctx.userdata::<CapturedOutput>().unwrap();
    captured_output.stdout.borrow_mut().push(line);
}

fn write_to_stderr(ctx: Ctx<'_>, line: String) {
    let captured_output = ctx.userdata::<CapturedOutput>().unwrap();
    captured_output.stderr.borrow_mut().push(line);
}

// JS functions for the console implementation
pub const CONSOLE_JS: &str = include_str!("console.js");

// JS code wiring the console module into the global context
pub const WIRE_JS: &str = r#"
        import * as __golem_exec_js_console from '__golem_exec_js_builtin/console';
        globalThis.console = __golem_exec_js_console;
    "#;
