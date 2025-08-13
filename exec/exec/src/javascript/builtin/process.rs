pub const PROCESS_JS: &str = include_str!("process.js");

pub const WIRE_JS: &str = r#"
    import * as process from 'node:process';
    import * as __golem_exec_js_readline from 'node:readline';

    process.__update();
    globalThis.process = process;
"#;
