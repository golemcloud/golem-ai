#[allow(static_mut_refs)]
mod bindings;

use crate::bindings::exports::test::exec_js_exports::test_exec_js_api::*;
use crate::bindings::golem::exec::executor::run;
use crate::bindings::golem::exec::types::{
    Encoding, Error, File, Language, LanguageKind, Limits, RunOptions,
};
use crate::bindings::test::helper_client::test_helper_client::TestHelperApi;
use golem_rust::{atomically, generate_idempotency_key};
use indoc::indoc;

struct Component;

impl Guest for Component {
    fn test01() -> bool {
        let restart = Restart::new();

        let result = run(
            &Language {
                kind: LanguageKind::Javascript,
                version: None,
            },
            &[],
            indoc! { r#"
                const x = 40 + 2;
                const name = "world";
                console.log(`Hello, ${name}!`, x);
            "# },
            &empty_run_options(),
        );

        restart.here();

        match result {
            Ok(result) => {
                println!("Result: {:?}", result);
                result.run.stdout == "Hello, world! 42" && result.run.exit_code == Some(0)
            }
            Err(err) => {
                println!("Error: {}", err);
                false
            }
        }
    }

    fn test02() -> bool {
        let restart = Restart::new();

        let result = run(
            &Language {
                kind: LanguageKind::Javascript,
                version: None,
            },
            &[],
            indoc! { r#"
                import { createInterface } from "node:readline";

                const rl = createInterface({
                    input: process.stdin,
                    output: process.stdout
                });

                let sum = 0;

                rl.on('line', (line) => {
                    const number = parseFloat(line);
                    if (!isNaN(number)) {
                        sum += number;
                    }
                });

                rl.on('close', () => {
                    console.log(`Total Sum: ${sum}`);
                });
            "# },
            &RunOptions {
                stdin: Some("1\n2\n3\n".to_string()),
                ..empty_run_options()
            },
        );

        restart.here();

        match result {
            Ok(result) => {
                println!("Result: {:?}", result);
                result.run.stdout == "Total Sum: 6" && result.run.exit_code == Some(0)
            }
            Err(err) => {
                println!("Error: {}", err);
                false
            }
        }
    }

    fn test03() -> bool {
        let restart = Restart::new();

        let result = run(
            &Language {
                kind: LanguageKind::Javascript,
                version: None,
            },
            &[],
            indoc! { r#"
                import { createInterface } from "node:readline";

                const rl = createInterface({
                    input: process.stdin,
                    output: process.stdout
                });

                let sum = 0;

                async function calculateSum() {
                    for await (const line of rl) {
                        const number = parseFloat(line);
                        if (!isNaN(number)) {
                            sum += number;
                        }
                    }
                    console.log(`Total Sum: ${sum}`);
                }

                await calculateSum();
            "# },
            &RunOptions {
                stdin: Some("1\n2\n3\n".to_string()),
                ..empty_run_options()
            },
        );

        restart.here();

        match result {
            Ok(result) => {
                println!("Result: {:?}", result);
                result.run.stdout == "Total Sum: 6" && result.run.exit_code == Some(0)
            }
            Err(err) => {
                println!("Error: {}", err);
                false
            }
        }
    }

    fn test04() -> bool {
        let restart = Restart::new();

        let result = run(
            &Language {
                kind: LanguageKind::Javascript,
                version: None,
            },
            &[],
            indoc! { r#"
                import { argv } from "node:process";
                console.log(...argv);
            "#},
            &RunOptions {
                args: Some(vec!["arg1".to_string(), "arg2".to_string()]),
                ..empty_run_options()
            },
        );

        restart.here();

        match result {
            Ok(result) => {
                println!("Result: {:?}", result);
                result.run.stdout == "arg1 arg2" && result.run.exit_code == Some(0)
            }
            Err(err) => {
                println!("Error: {}", err);
                false
            }
        }
    }

    fn test05() -> bool {
        let restart = Restart::new();

        let result = run(
            &Language {
                kind: LanguageKind::Javascript,
                version: None,
            },
            &[],
            indoc! { r#"
                import { env } from "node:process";
                console.log(env.INPUT);
            "# },
            &RunOptions {
                env: Some(vec![("INPUT".to_string(), "test_value".to_string())]),
                ..empty_run_options()
            },
        );

        restart.here();

        match result {
            Ok(result) => {
                println!("Result: {:?}", result);
                result.run.stdout == "test_value" && result.run.exit_code == Some(0)
            }
            Err(err) => {
                println!("Error: {}", err);
                false
            }
        }
    }

    fn test06() -> bool {
        let restart = Restart::new();
        let result = run(
            &Language {
                kind: LanguageKind::Javascript,
                version: None,
            },
            &[File {
                name: "test/module.js".to_string(),
                content: indoc! { r#"
                    export const x = 40 + 2;
                    export const name = "world";
                "# }
                .as_bytes()
                .to_vec(),
                encoding: Some(Encoding::Utf8),
            }],
            indoc! { r#"
                import { x, name } from "test/module.js";
                console.log(`Hello, ${name}!`, x);
            "# },
            &empty_run_options(),
        );

        restart.here();

        match result {
            Ok(result) => {
                println!("Result: {:?}", result);
                result.run.stdout == "Hello, world! 42" && result.run.exit_code == Some(0)
            }
            Err(err) => {
                println!("Error: {}", err);
                false
            }
        }
    }

    fn test07() -> bool {
        let restart = Restart::new();

        let session = bindings::golem::exec::executor::Session::new(
            &Language {
                kind: LanguageKind::Javascript,
                version: None,
            },
            &[File {
                name: "test/module.js".to_string(),
                content: indoc! { r#"
                    export const x = 40 + 2;
                    export const name = "world";
                "# }
                .as_bytes()
                .to_vec(),
                encoding: Some(Encoding::Utf8),
            }],
        );

        let r1 = session
            .run(
                indoc! { r#"
                    import { x, name } from "test/module.js";
                    console.log(`Hello, ${name}!`, x);
                "# },
                &empty_run_options(),
            )
            .map_or_else(
                |err| {
                    println!("Error: {}", err);
                    false
                },
                |result| {
                    println!("Result: {:?}", result);
                    result.run.stdout == "Hello, world! 42" && result.run.exit_code == Some(0)
                },
            );

        let r2 = session
            .run(
                indoc! { r#"
                    import { argv } from "node:process";
                    console.log(...argv);
                "# },
                &RunOptions {
                    args: Some(vec!["arg1".to_string(), "arg2".to_string()]),
                    ..empty_run_options()
                },
            )
            .map_or_else(
                |err| {
                    println!("Error: {}", err);
                    false
                },
                |result| {
                    println!("Result: {:?}", result);
                    result.run.stdout == "arg1 arg2" && result.run.exit_code == Some(0)
                },
            );

        restart.here();

        let r3 = session
            .run(
                indoc! { r#"
                    import { argv } from "node:process";
                    console.log(...argv);
                "# },
                &RunOptions {
                    args: Some(vec!["arg3".to_string()]),
                    ..empty_run_options()
                },
            )
            .map_or_else(
                |err| {
                    println!("Error: {}", err);
                    false
                },
                |result| {
                    println!("Result: {:?}", result);
                    result.run.stdout == "arg3" && result.run.exit_code == Some(0)
                },
            );

        const READLINE_SNIPPET: &str = indoc! { r#"
            import { createInterface } from "node:readline";

            const rl = createInterface({
                input: process.stdin,
                output: process.stdout
            });

            let sum = 0;

            async function calculateSum() {
                for await (const line of rl) {
                    const number = parseFloat(line);
                    if (!isNaN(number)) {
                        sum += number;
                    }
                }
                console.log(`Total Sum: ${sum}`);
            }

            await calculateSum();
        "# };

        let r4 = session
            .run(
                READLINE_SNIPPET,
                &RunOptions {
                    stdin: Some("1\n2\n3\n".to_string()),
                    ..empty_run_options()
                },
            )
            .map_or_else(
                |err| {
                    println!("Error: {}", err);
                    false
                },
                |result| {
                    println!("Result: {:?}", result);
                    result.run.stdout == "Total Sum: 6" && result.run.exit_code == Some(0)
                },
            );
        let r5 = session
            .run(
                READLINE_SNIPPET,
                &RunOptions {
                    stdin: Some("4\n100\n".to_string()),
                    ..empty_run_options()
                },
            )
            .map_or_else(
                |err| {
                    println!("Error: {}", err);
                    false
                },
                |result| {
                    println!("Result: {:?}", result);
                    result.run.stdout == "Total Sum: 104" && result.run.exit_code == Some(0)
                },
            );

        r1 && r2 && r3 && r4 && r5
    }

    fn test08() -> bool {
        let restart = Restart::new();

        let session = bindings::golem::exec::executor::Session::new(
            &Language {
                kind: LanguageKind::Javascript,
                version: None,
            },
            &[],
        );

        let r1 = session
            .upload(&File {
                name: "test/input.txt".to_string(),
                content: "Hello, Golem!".as_bytes().to_vec(),
                encoding: Some(Encoding::Utf8),
            })
            .map_or_else(
                |err| {
                    println!("Error uploading file: {}", err);
                    false
                },
                |_| true,
            );

        let r2 = session
            .run(
                indoc! { r#"
                    import { readFileSync, writeFileSync } from "node:fs";
                    const content = readFileSync("test/input.txt", "utf8");
                    console.log(content);
                    writeFileSync("test/output.txt", content + " - Processed by Golem");
                "# },
                &empty_run_options(),
            )
            .map_or_else(
                |err| {
                    println!("Error running script: {}", err);
                    false
                },
                |result| {
                    println!("Result: {:?}", result);
                    result.run.stdout == "Hello, Golem!" && result.run.exit_code == Some(0)
                },
            );

        restart.here();

        let r3 = session.download("test/output.txt").map_or_else(
            |err| {
                println!("Error downloading file: {}", err);
                false
            },
            |file| {
                let content = String::from_utf8(file).unwrap_or_default();
                println!("Downloaded file content: {}", content);
                content == "Hello, Golem! - Processed by Golem"
            },
        );

        r1 & &r2 & &r3
    }

    fn test09() -> bool {
        let restart = Restart::new();

        let session = bindings::golem::exec::executor::Session::new(
            &Language {
                kind: LanguageKind::Javascript,
                version: None,
            },
            &[],
        );

        let r1 = session
            .upload(&File {
                name: "test/input.txt".to_string(),
                content: "Hello, Golem!".as_bytes().to_vec(),
                encoding: Some(Encoding::Utf8),
            })
            .map_or_else(
                |err| {
                    println!("Error uploading file: {}", err);
                    false
                },
                |_| true,
            );

        let r2 = session
            .run(
                indoc! { r#"
                        import { readFile, writeFile } from "node:fs";
                        readFile("test/input.txt", "utf8", (content, error) => {
                            if (error) {
                                console.error("Error reading file:", error);
                                return;
                            }
                            console.log(content);
                            writeFile("test/output.txt", content + " - Processed by Golem", (error) => {
                                if (error) {
                                    console.error("Error writing file:", error);
                                    return;
                                }
                            });
                        });
                    "# },
                &empty_run_options(),
            )
            .map_or_else(
                |err| {
                    println!("Error running script: {}", err);
                    false
                },
                |result| {
                    println!("Result: {:?}", result);
                    result.run.stdout == "Hello, Golem!" && result.run.exit_code == Some(0)
                },
            );

        restart.here();

        let r3 = session.download("test/output.txt").map_or_else(
            |err| {
                println!("Error downloading file: {}", err);
                false
            },
            |file| {
                let content = String::from_utf8(file).unwrap_or_default();
                println!("Downloaded file content: {}", content);
                content == "Hello, Golem! - Processed by Golem"
            },
        );

        let r4 = session.set_working_dir("test").map_or_else(
            |err| {
                println!("Error setting working directory: {}", err);
                false
            },
            |_| true,
        );

        let r5 = session
            .run(
                indoc! { r#"
                    import { readFile, writeFile } from "node:fs";
                    import { cwd } from "node:process";

                    console.log("Current working directory:", cwd());
                    readFile("input.txt", "utf8", (content, error) => {
                        if (error) {
                            console.error("Error reading file:", error);
                            return;
                        }
                        console.log(content);
                        writeFile("/test/output2.txt", content + " - Processed by Golem", (error) => {
                            if (error) {
                                console.error("Error writing file:", error);
                                return;
                            }
                        });
                    });
                "# },
                &empty_run_options(),
            )
            .map_or_else(
                |err| {
                    println!("Error running script: {}", err);
                    false
                },
                |result| {
                    println!("Result: {:?}", result);
                    result.run.stdout == "Current working directory: test\nHello, Golem!" && result.run.exit_code == Some(0)
                },
            );

        let r6 = session.download("test/output2.txt").map_or_else(
            |err| {
                println!("Error downloading file: {}", err);
                false
            },
            |file| {
                let content = String::from_utf8(file).unwrap_or_default();
                println!("Downloaded file content: {}", content);
                content == "Hello, Golem! - Processed by Golem"
            },
        );

        r1 && r2 && r3 && r4 && r5 && r6
    }

    fn test10() -> bool {
        match run(
            &Language {
                kind: LanguageKind::Javascript,
                version: None,
            },
            &[],
            indoc! { r#"
                let x = 0;
                setInterval(() => {
                    x += 1;
                    console.log(x);
                }, 250);
            "# },
            &RunOptions {
                limits: Some(Limits {
                    time_ms: Some(1000),
                    memory_bytes: None,
                    file_size_bytes: None,
                    max_processes: None,
                }),
                ..empty_run_options()
            },
        ) {
            Ok(result) => {
                println!("Result: {:?}", result);
                false
            }
            Err(err) => {
                println!("Error: {}", err);
                matches!(err, Error::Timeout)
            }
        }
    }

    fn test11() -> bool {
        let session = bindings::golem::exec::executor::Session::new(
            &Language {
                kind: LanguageKind::Javascript,
                version: None,
            },
            &[],
        );

        let r1 = session
            .run(
                indoc! { r#"
                    import { writeFileSync } from "node:fs";
                    const content = new Array(1024).fill(0);
                    writeFileSync("output.bin", content);
                "# },
                &empty_run_options(),
            )
            .map_or_else(
                |err| {
                    println!("Error running script: {}", err);
                    false
                },
                |result| {
                    println!("Result: {:?}", result);
                    result.run.exit_code == Some(0)
                },
            );

        let r2 = session
            .run(
                indoc! { r#"
                    import { writeFileSync } from "node:fs";
                    const content = new Array(1024).fill(0);
                    writeFileSync("output2.bin", content);
                    "#
                },
                &RunOptions {
                    limits: Some(Limits {
                        time_ms: None,
                        memory_bytes: None,
                        file_size_bytes: Some(512),
                        max_processes: None,
                    }),
                    ..empty_run_options()
                },
            )
            .map_or_else(
                |err| {
                    println!("Error running script: {}", err);
                    true
                },
                |_result| false,
            );

        let r3 = session.list_files("").map_or_else(
            |err| {
                println!("Error listing files: {}", err);
                false
            },
            |files| {
                println!("List of files: {files:?}");
                files == vec!["output.bin".to_string()]
            },
        );

        r1 && r2 && r3
    }
}

struct Restart {
    name: String,
}

impl Restart {
    pub fn new() -> Self {
        let name = std::env::var("GOLEM_WORKER_NAME").unwrap();
        let key = generate_idempotency_key();
        Self {
            name: format!("{name}-{key}"),
        }
    }

    pub fn here(&self) {
        atomically(|| {
            let client = TestHelperApi::new(&self.name);
            let answer = client.blocking_inc_and_get();
            if answer == 1 {
                panic!("Simulating crash")
            }
        });
    }
}

fn empty_run_options() -> RunOptions {
    RunOptions {
        stdin: None,
        args: None,
        env: None,
        limits: None,
    }
}

bindings::export!(Component with_types_in bindings);
