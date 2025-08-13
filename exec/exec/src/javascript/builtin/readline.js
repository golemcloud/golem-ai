// Partially based on: https://github.com/franciscop/readline-polyfill/

import { EventEmitter } from "__golem_exec_js_builtin/eventemitter";
import * as consoleNative from '__golem_exec_js_builtin/console_native'

class Stdin {
    constructor(contents) {
        this.contents = contents || "";
    }

    readLine() {
        if (this.contents.length === 0) {
            return null; // No more input
        }

        const index = this.contents.indexOf("\n");
        if (index === -1) {
            const line = this.contents;
            this.contents = "";
            return line;
        } else {
            const line = this.contents.slice(0, index);
            this.contents = this.contents.slice(index + 1);
            return line;
        }
    }
}

class Stdout {
    write(data) {
        console.log(data);
    }
}

class Stderr {
    write(data) {
        consoleNative.eprintln(data);
    }
}

class InterfaceConstructor extends EventEmitter {
    constructor({ input, output, prompt }) {
        super();
        this.input = input;
        this.output = output;
        this.message = prompt;

        setImmediate(() =>
            this.readNext()
        );
    }

    prompt() {
        this.output.write(this.message);
    }

    readNext() {
        const line = this.input.readLine();
        if (line !== null) {
            this.emit("line", line);
            setImmediate(() => {
                this.readNext();
            });
        } else {
            this.emit("close");
        }
    }

    async *[Symbol.asyncIterator]() {
        let line = this.input.readLine();
        while (line !== null) {
            yield line;
            line = this.input.readLine();
        }
    }
}

const createInterface = (options) => new InterfaceConstructor(options);

const clearLine = (stream, dir, callback) => {
    // not supported
};

const clearScreenDown = (stream, callback) => {
    // not supported
};

const cursorTo = (stream, x, y = 0, callback) => {
    // not supported
};

const moveCursor = (stream, dx, dy, callback) => {
    // not supported
};

export { createInterface, clearLine, clearScreenDown, cursorTo, moveCursor, Stdin, Stdout, Stderr };
