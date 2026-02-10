use golem_ai_exec::model::*;
use golem_ai_exec::{DurableExecution, ExecutionProvider, ExecutionSession};
use golem_rust::{agent_definition, agent_implementation};
use indoc::indoc;

type Provider = DurableExecution;
type Session = <Provider as ExecutionProvider>::Session;

#[agent_definition]
pub trait ExecPyTest {
    fn new(name: String) -> Self;
    fn test1(&self) -> bool;
    fn test2(&self) -> bool;
    fn test3(&self) -> bool;
    fn test4(&self) -> bool;
    fn test5(&self) -> bool;
    fn test6(&self) -> bool;
    fn test7(&self) -> bool;
}

struct ExecPyTestImpl {
    _name: String,
}

#[agent_implementation]
impl ExecPyTest for ExecPyTestImpl {
    fn new(name: String) -> Self {
        Self { _name: name }
    }

    fn test1(&self) -> bool {
        match Provider::run(
            Language {
                kind: LanguageKind::Python,
                version: None,
            },
            vec![],
            indoc! {r#"
                x = 40 + 2;
                name = "world"
                print(f'Hello, {name}!', x)
            "#}
            .to_string(),
            empty_run_options(),
        ) {
            Ok(result) => {
                println!("Result: {:?}", result);
                result.run.stdout == "Hello, world! 42\n" && result.run.exit_code == Some(0)
            }
            Err(err) => {
                println!("Error: {}", err);
                false
            }
        }
    }

    fn test2(&self) -> bool {
        match Provider::run(
            Language {
                kind: LanguageKind::Python,
                version: None,
            },
            vec![],
            indoc! { r#"
                import sys
                x = 40 + 2;
                name = sys.stdin.readline()
                print(f'Hello, {name}!', x)
            "# }
            .to_string(),
            RunOptions {
                stdin: Some("world".to_string()),
                ..empty_run_options()
            },
        ) {
            Ok(result) => {
                println!("Result: {:?}", result);
                result.run.stdout == "Hello, world! 42\n" && result.run.exit_code == Some(0)
            }
            Err(err) => {
                println!("Error: {}", err);
                false
            }
        }
    }

    fn test3(&self) -> bool {
        match Provider::run(
            Language {
                kind: LanguageKind::Python,
                version: None,
            },
            vec![],
            indoc! { r#"
                import sys
                print(sys.argv)
            "# }
            .to_string(),
            RunOptions {
                args: Some(vec!["arg1".to_string(), "arg2".to_string()]),
                ..empty_run_options()
            },
        ) {
            Ok(result) => {
                println!("Result: {:?}", result);
                result.run.stdout == "['arg1', 'arg2']\n" && result.run.exit_code == Some(0)
            }
            Err(err) => {
                println!("Error: {}", err);
                false
            }
        }
    }

    fn test4(&self) -> bool {
        match Provider::run(
            Language {
                kind: LanguageKind::Python,
                version: None,
            },
            vec![],
            indoc! { r#"
                import os
                print(os.environ.get('TEST_ENV_VAR', 'default_value'))
            "# }
            .to_string(),
            RunOptions {
                env: Some(vec![("TEST_ENV_VAR".to_string(), "test_value".to_string())]),
                ..empty_run_options()
            },
        ) {
            Ok(result) => {
                println!("Result: {:?}", result);
                result.run.stdout == "test_value\n" && result.run.exit_code == Some(0)
            }
            Err(err) => {
                println!("Error: {}", err);
                false
            }
        }
    }

    fn test5(&self) -> bool {
        match Provider::run(
            Language {
                kind: LanguageKind::Python,
                version: None,
            },
            vec![
                File {
                    name: "mytest/__init__.py".to_string(),
                    content: b"".to_vec(),
                    encoding: None,
                },
                File {
                    name: "mytest/mymodule.py".to_string(),
                    content: indoc!(
                        r#"
                        x = 40 + 2
                        name = "world"
                        "#,
                    )
                    .as_bytes()
                    .to_vec(),
                    encoding: None,
                },
            ],
            indoc! { r#"
                import mytest.mymodule as t
                print(f'Hello, {t.name}!', t.x)
            "#}
            .to_string(),
            empty_run_options(),
        ) {
            Ok(result) => {
                println!("Result: {:?}", result);
                result.run.stdout == "Hello, world! 42\n" && result.run.exit_code == Some(0)
            }
            Err(err) => {
                println!("Error: {}", err);
                false
            }
        }
    }

    fn test6(&self) -> bool {
        let session = Session::new(
            Language {
                kind: LanguageKind::Python,
                version: None,
            },
            vec![
                File {
                    name: "mytest/__init__.py".to_string(),
                    content: b"".to_vec(),
                    encoding: None,
                },
                File {
                    name: "mytest/mymodule.py".to_string(),
                    content: indoc! { r#"
                        x = 40 + 2
                        name = "world"
                    "#}
                    .as_bytes()
                    .to_vec(),
                    encoding: None,
                },
            ],
        );

        let r1 = session
            .run(
                indoc! { r#"
                    import mytest.mymodule as t
                    print(f'Hello, {t.name}!', t.x)
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
                    result.run.stdout == "Hello, world! 42\n" && result.run.exit_code == Some(0)
                },
            );

        let r2 = session
            .run(
                indoc! { r#"
                    import sys
                    print(sys.argv)
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
                    result.run.stdout == "['arg1', 'arg2']\n" && result.run.exit_code == Some(0)
                },
            );

        let r3 = session
            .run(
                indoc! { r#"
                    import sys
                    print(sys.argv)
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
                    result.run.stdout == "['arg3']\n" && result.run.exit_code == Some(0)
                },
            );

        const READLINE_SNIPPET: &str = indoc! { r#"
            import sys

            total_sum = 0

            for line in sys.stdin:
                try:
                    number = float(line.strip())
                    total_sum += number
                except ValueError:
                    continue

            print(f'Total Sum: {total_sum}')
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
                    result.run.stdout == "Total Sum: 6.0\n" && result.run.exit_code == Some(0)
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
                    result.run.stdout == "Total Sum: 104.0\n" && result.run.exit_code == Some(0)
                },
            );

        r1 && r2 && r3 && r4 && r5
    }

    fn test7(&self) -> bool {
        let session = Session::new(
            Language {
                kind: LanguageKind::Python,
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
                    with open('test/input.txt', 'r') as f:
                        content = f.read()
                    print(content)
                    with open('test/output.txt', 'w') as f:
                        f.write(content + ' - Processed by Golem')
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
                    result.run.stdout == "Hello, Golem!\n" && result.run.exit_code == Some(0)
                },
            );

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
                    with open('input.txt', 'r') as f:
                        content = f.read()
                    print(os.getcwd())
                    print(content)
                    with open('output2.txt', 'w') as f:
                        f.write(content + ' - Processed by Golem')
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
                    result.run.stdout == "test\nHello, Golem!\n" && result.run.exit_code == Some(0)
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
}

fn empty_run_options() -> RunOptions {
    RunOptions {
        stdin: None,
        args: None,
        env: None,
        limits: None,
    }
}
