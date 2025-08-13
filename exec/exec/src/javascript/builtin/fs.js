import {
    read_file,
    read_file_with_encoding,
    write_file,
    write_file_with_encoding
} from '__golem_exec_js_builtin/fs_native';
import {Buffer} from 'node:buffer';

export function readFile(path, optionsOrCallback, callback) {
    if (typeof optionsOrCallback === 'function') {
        callback = optionsOrCallback;
        optionsOrCallback = {};
    }
    if (typeof optionsOrCallback === 'string') {
        optionsOrCallback = {encoding: optionsOrCallback};
    }
    if (optionsOrCallback.encoding && optionsOrCallback.encoding !== '') {
        const [contents, error] = read_file_with_encoding(path, optionsOrCallback.encoding);
        if (error === undefined) {
            callback(contents);
        } else {
            callback(undefined, error);
        }
    } else {
        const [contents, error] = read_file(path);
        if (error === undefined) {
            const buffer = Buffer.from(contents);
            callback(buffer);
        } else {
            callback(undefined, error);
        }
    }
}

export function readFileSync(path, options) {
    if (typeof options === 'string') {
        options = {encoding: options};
    }
    if (options && options.encoding && options.encoding !== '') {
        const [contents, error] = read_file_with_encoding(path, options.encoding);
        if (error === undefined) {
            return contents;
        } else {
            throw new Error(error);
        }
    } else {
        const [contents, error] = read_file(path);
        if (error === undefined) {
            return Buffer.from(contents);
        } else {
            throw new Error(error);
        }
    }
}

export function writeFile(path, data, optionsOrCallback, callback) {
    if (typeof optionsOrCallback === 'function') {
        callback = optionsOrCallback;
        optionsOrCallback = {};
    }
    if (typeof optionsOrCallback === 'string') {
        optionsOrCallback = {encoding: optionsOrCallback};
    }
    if (optionsOrCallback && optionsOrCallback.encoding && optionsOrCallback.encoding !== '') {
        const error = write_file_with_encoding(path, optionsOrCallback.encoding, data);
        callback(error);
    } else {
        if (typeof data === 'string') {
            const error = write_file_with_encoding(path, "utf8", data);
            callback(error);
        } else {
            const dataArray = new Uint8Array(data.buffer || data, data.byteOffset || 0, data.byteLength || data.length);
            const error = write_file(path, dataArray);
            callback(error);
        }
    }
}


export function writeFileSync(path, data, options) {
    if (typeof options === 'string') {
        options = {encoding: options};
    }
    if (options && options.encoding && options.encoding !== '') {
        const error = write_file_with_encoding(path, options.encoding, data);
        if (error !== undefined) {
            throw new Error(error);
        }
    } else {
        if (typeof data === 'string') {
            const error = write_file_with_encoding(path, "utf8", data);
            if (error !== undefined) {
                throw new Error(error);
            }
        } else {
            const dataArray = new Uint8Array(data.buffer || data, data.byteOffset || 0, data.byteLength || data.length);
            const error = write_file(path, dataArray);
            if (error !== undefined) {
                throw new Error(error);
            }
        }
    }
}