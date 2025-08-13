import * as timeoutNative from '__golem_exec_js_builtin/timeout_native'

export function setTimeout(callback, time, ...args) {
    return timeoutNative.schedule(callback, time, false, args);
}

export function setInterval(callback, time, ...args) {
    return timeoutNative.schedule(callback, time, true, args);
}

export function setImmediate(callback, ...args) {
    return setTimeout(callback, 0, ...args);
}

export function clearTimeout(id) {
    return timeoutNative.clear_schedule(id);
}