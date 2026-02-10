use golem_ai_exec::model::*;
use golem_ai_exec::{DurableExecution, ExecutionProvider, ExecutionSession};
use golem_rust::{agent_definition, agent_implementation, generate_idempotency_key, mark_atomic_operation};
use indoc::indoc;

type Provider = DurableExecution;
type Session = <Provider as ExecutionProvider>::Session;

#[agent_definition]
pub trait TestHelper {
    fn new(name: String) -> Self;
    fn inc_and_get(&mut self) -> u64;
}

struct TestHelperImpl {
    _name: String,
    total: u64,
}

#[agent_implementation]
impl TestHelper for TestHelperImpl {
    fn new(name: String) -> Self {
        Self {
            _name: name,
            total: 0,
        }
    }

    fn inc_and_get(&mut self) -> u64 {
        self.total += 1;
        self.total
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

    pub async fn here(&self) {
        let _guard = mark_atomic_operation();
        let mut client = TestHelperClient::get(self.name.clone());
        let answer = client.inc_and_get().await;
        if answer == 1 {
            panic!("Simulating crash")
        }
    }
}

#[agent_definition]
pub trait ExecJsTest {
    fn new(name: String) -> Self;
    async fn test01(&self) -> bool;
    async fn test02(&self) -> bool;
    async fn test03(&self) -> bool;
    async fn test04(&self) -> bool;
    async fn test05(&self) -> bool;
    fn test06(&self) -> bool;
    async fn test07(&self) -> bool;
    async fn test08(&self) -> bool;
    async fn test09(&self) -> bool;
    fn test10(&self) -> bool;
    fn test11(&self) -> bool;
}

struct ExecJsTestImpl {
    _name: String,
}

#[agent_implementation]
impl ExecJsTest for ExecJsTestImpl {
    fn new(name: String) -> Self {
        Self { _name: name }
    }

    async fn test01(&self) -> bool {
        let restart = Restart::new();

        let result = Provider::run(
            Language {
                kind: LanguageKind::Javascript,
                version: None,
            },
            vec![],
            indoc! { r#"
                const x = 40 + 2;
                const name = "world";
                console.log(`Hello, ${name}!`, x);
            "# }
            .to_string(),
            empty_run_options(),
        );

        restart.here().await;

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

    async fn test02(&self) -> bool {
        let restart = Restart::new();

        let result = Provider::run(
            Language {
                kind: LanguageKind::Javascript,
                version: None,
            },
            vec![],
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
            "# }
            .to_string(),
            RunOptions {
                stdin: Some("1\n2\n3\n".to_string()),
                ..empty_run_options()
            },
        );

        restart.here().await;

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

    async fn test03(&self) -> bool {
        let restart = Restart::new();

        let result = Provider::run(
            Language {
                kind: LanguageKind::Javascript,
                version: None,
            },
            vec![],
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
            "# }
            .to_string(),
            RunOptions {
                stdin: Some("1\n2\n3\n".to_string()),
                ..empty_run_options()
            },
        );

        restart.here().await;

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

    async fn test04(&self) -> bool {
        let restart = Restart::new();

        let result = Provider::run(
            Language {
                kind: LanguageKind::Javascript,
                version: None,
            },
            vec![],
            indoc! { r#"
                import { argv } from "node:process";
                console.log(...argv);
            "#}
            .to_string(),
            RunOptions {
                args: Some(vec!["arg1".to_string(), "arg2".to_string()]),
                ..empty_run_options()
            },
        );

        restart.here().await;

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

    async fn test05(&self) -> bool {
        let restart = Restart::new();

        let result = Provider::run(
            Language {
                kind: LanguageKind::Javascript,
                version: None,
            },
            vec![],
            indoc! { r#"
                import { env } from "node:process";
                console.log(env.INPUT);
            "# }
            .to_string(),
            RunOptions {
                env: Some(vec![("INPUT".to_string(), "test_value".to_string())]),
                ..empty_run_options()
            },
        );

        restart.here().await;

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

    fn test06(&self) -> bool {
        let result = Provider::run(
            Language {
                kind: LanguageKind::Javascript,
                version: None,
            },
            vec![File {
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
            "# }
            .to_string(),
            empty_run_options(),
        );

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

    async fn test07(&self) -> bool {
        let restart = Restart::new();

        let session = Session::new(
            Language {
                kind: LanguageKind::Javascript,
                version: None,
            },
            vec![File {
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
                "# }
                .to_string(),
                empty_run_options(),
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
                "# }
                .to_string(),
                RunOptions {
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

        restart.here().await;

        let r3 = session
            .run(
                indoc! { r#"
                    import { argv } from "node:process";
                    console.log(...argv);
                "# }
                .to_string(),
                RunOptions {
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
                READLINE_SNIPPET.to_string(),
                RunOptions {
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
                READLINE_SNIPPET.to_string(),
                RunOptions {
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

    async fn test08(&self) -> bool {
        let restart = Restart::new();

        let session = Session::new(
            Language {
                kind: LanguageKind::Javascript,
                version: None,
            },
            vec![],
        );

        let r1 = session
            .upload(File {
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
                "# }
                .to_string(),
                empty_run_options(),
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

        restart.here().await;

        let r3 = session
            .download("test/output.txt".to_string())
            .map_or_else(
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

        r1 && r2 && r3
    }

    async fn test09(&self) -> bool {
        let restart = Restart::new();

        let session = Session::new(
            Language {
                kind: LanguageKind::Javascript,
                version: None,
            },
            vec![],
        );

        let r1 = session
            .upload(File {
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
                    "# }
                .to_string(),
                empty_run_options(),
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

        restart.here().await;

        let r3 = session
            .download("test/output.txt".to_string())
            .map_or_else(
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

        let r4 = session
            .set_working_dir("test".to_string())
            .map_or_else(
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
                "# }
                .to_string(),
                empty_run_options(),
            )
            .map_or_else(
                |err| {
                    println!("Error running script: {}", err);
                    false
                },
                |result| {
                    println!("Result: {:?}", result);
                    result.run.stdout == "Current working directory: test\nHello, Golem!"
                        && result.run.exit_code == Some(0)
                },
            );

        let r6 = session
            .download("test/output2.txt".to_string())
            .map_or_else(
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

    fn test10(&self) -> bool {
        match Provider::run(
            Language {
                kind: LanguageKind::Javascript,
                version: None,
            },
            vec![],
            indoc! { r#"
                let x = 0;
                setInterval(() => {
                    x += 1;
                    console.log(x);
                }, 250);
            "# }
            .to_string(),
            RunOptions {
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

    fn test11(&self) -> bool {
        let session = Session::new(
            Language {
                kind: LanguageKind::Javascript,
                version: None,
            },
            vec![],
        );

        let r1 = session
            .run(
                indoc! { r#"
                    import { writeFileSync } from "node:fs";
                    const content = new Array(1024).fill(0);
                    writeFileSync("output.bin", content);
                "# }
                .to_string(),
                empty_run_options(),
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
                }
                .to_string(),
                RunOptions {
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

        let r3 = session
            .list_files("".to_string())
            .map_or_else(
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

fn empty_run_options() -> RunOptions {
    RunOptions {
        stdin: None,
        args: None,
        env: None,
        limits: None,
    }
}
