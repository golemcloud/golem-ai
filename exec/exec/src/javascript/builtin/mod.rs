use std::fmt::Write;

mod base64_js;
mod buffer;
pub mod console;
mod eventemitter;
pub mod fs;
mod ieee754;
mod process;
mod readline;
pub mod timeout;

pub fn add_module_resolvers(
    resolver: rquickjs::loader::BuiltinResolver,
) -> rquickjs::loader::BuiltinResolver {
    resolver
        .with_module("__golem_exec_js_builtin/console_native")
        .with_module("__golem_exec_js_builtin/console")
        .with_module("__golem_exec_js_builtin/eventemitter")
        .with_module("__golem_exec_js_builtin/timeout_native")
        .with_module("__golem_exec_js_builtin/timeout")
        .with_module("node:readline")
        .with_module("node:process")
        .with_module("__golem_exec_js_builtin/fs_native")
        .with_module("node:fs")
        .with_module("node:buffer")
        .with_module("base64-js")
        .with_module("ieee754")
}

pub fn module_loader() -> (
    rquickjs::loader::ModuleLoader,
    rquickjs::loader::BuiltinLoader,
) {
    (
        rquickjs::loader::ModuleLoader::default()
            .with_module(
                "__golem_exec_js_builtin/console_native",
                console::js_native_module,
            )
            .with_module(
                "__golem_exec_js_builtin/timeout_native",
                timeout::js_native_module,
            )
            .with_module("__golem_exec_js_builtin/fs_native", fs::js_native_module),
        rquickjs::loader::BuiltinLoader::default()
            .with_module("__golem_exec_js_builtin/console", console::CONSOLE_JS)
            .with_module(
                "__golem_exec_js_builtin/eventemitter",
                eventemitter::EVENTEMITTER_JS,
            )
            .with_module("__golem_exec_js_builtin/timeout", timeout::TIMEOUT_JS)
            .with_module("node:readline", readline::READLINE_JS)
            .with_module("node:process", process::PROCESS_JS)
            .with_module("base64-js", base64_js::BASE64_JS)
            .with_module("ieee754", ieee754::IEEE754_JS)
            .with_module("node:buffer", buffer::BUFFER_JS)
            .with_module("node:fs", fs::FS_JS),
    )
}

pub fn wire_builtins() -> String {
    let mut result = String::new();
    writeln!(result, "{}", console::WIRE_JS).unwrap();
    writeln!(result, "{}", timeout::WIRE_JS).unwrap();
    writeln!(result, "{}", process::WIRE_JS).unwrap();
    result
}
