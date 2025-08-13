import * as consoleNative from '__golem_exec_js_builtin/console_native'

export function assert(condition, ...v) {
    if (!condition) {
        warn("Assertion failed:", ...v)
    }
}

export function clear() {
    // not supported
}

// TODO: count()
// TODO: countReset()
export function debug(...v) {
    consoleNative.debug(format(...v))
}

// TODO: dir()
// TODO: dirxml()

export function error(...v) {
    consoleNative.error(format(...v))
}

export function group(label) {
    if (label !== undefined) {
        log(label)
    }
}

export function groupCollapsed(label) {
    if (label !== undefined) {
        log(label)
    }
}

export function groupEnd() {
}

export function info(...v) {
    consoleNative.info(format(...v))
}

export function log(...v) {
    consoleNative.println(format(...v))
}

// TODO: table()
// TODO: time()
// TODO: timeEnd()
// TODO: timeLog()

export function trace(...v) {
    consoleNative.trace(format(...v))
}

export function warn(...v) {
    consoleNative.warn(format(...v))
}

function format(...v) {
    // TODO: support string substitutions: https://developer.mozilla.org/en-US/docs/Web/API/console#using_string_substitutions
    return v.join(" ")
}