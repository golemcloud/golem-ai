import * as __golem_exec_js_readline from 'node:readline';

export let argv = [];
export let argv0 = "golem:exec";
export let env = {};
export let stdin = new __golem_exec_js_readline.Stdin("");
export let stdout = new __golem_exec_js_readline.Stdout();
export let stderr = new __golem_exec_js_readline.Stderr();

export const __update = () => {
    argv = globalThis.__golem_exec_js_args || [];
    env = globalThis.__golem_exec_js_env || {};
    stdin = new __golem_exec_js_readline.Stdin(globalThis.__golem_exec_js_stdin);
};

export function cwd() {
    return globalThis.__golem_exec_js_cwd;
}