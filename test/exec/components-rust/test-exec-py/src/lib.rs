#[allow(static_mut_refs)]
mod bindings;

use crate::bindings::exports::test::exec_py_exports::test_exec_py_api::*;
use crate::bindings::golem::exec::executor::run;
use crate::bindings::golem::exec::types::{Encoding, File, Language, LanguageKind};
use indoc::indoc;

struct Component;

impl Guest for Component {
    fn test1() -> bool {
        match run(
            &Language {
                kind: LanguageKind::Python,
                version: None,
            },
            indoc!(
                r#"
            x = 40 + 2;
            name = "world"
            print(f'Hello, {name}!', x)
            "#
            ),
            &[],
            None,
            &[],
            &[],
            None,
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

    fn test2() -> bool {
        match run(
            &Language {
                kind: LanguageKind::Python,
                version: None,
            },
            indoc!(
                r#"
            import sys
            x = 40 + 2;
            name = sys.stdin.readline()
            print(f'Hello, {name}!', x)
            "#
            ),
            &[],
            Some("world"),
            &[],
            &[],
            None,
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

    fn test3() -> bool {
        match run(
            &Language {
                kind: LanguageKind::Python,
                version: None,
            },
            indoc!(
                r#"
            import sys
            print(sys.argv)
            "#
            ),
            &[],
            None,
            &["arg1".to_string(), "arg2".to_string()],
            &[],
            None,
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

    fn test4() -> bool {
        match run(
            &Language {
                kind: LanguageKind::Python,
                version: None,
            },
            indoc!(
                r#"
            import os
            print(os.environ.get('TEST_ENV_VAR', 'default_value'))
            "#
            ),
            &[],
            None,
            &[],
            &[("TEST_ENV_VAR".to_string(), "test_value".to_string())],
            None,
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

    fn test5() -> bool {
        match run(
            &Language {
                kind: LanguageKind::Python,
                version: None,
            },
            indoc!(
                r#"
                import mytest.mymodule as t
                print(f'Hello, {t.name}!', t.x)
                "#
            ),
            &[
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
            None,
            &[],
            &[],
            None,
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

    fn test6() -> bool {
        let session = bindings::golem::exec::executor::Session::new(
            &Language {
                kind: LanguageKind::Python,
                version: None,
            },
            &[
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
        );

        let r1 = session
            .run(
                indoc!(
                    r#"
                import mytest.mymodule as t
                print(f'Hello, {t.name}!', t.x)
                "#
                ),
                &[],
                None,
                &[],
                None,
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
                indoc!(
                    r#"
                    import sys
                    print(sys.argv)
                "#
                ),
                &["arg1".to_string(), "arg2".to_string()],
                None,
                &[],
                None,
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
                indoc!(
                    r#"
                    import sys
                    print(sys.argv)
                "#
                ),
                &["arg3".to_string()],
                None,
                &[],
                None,
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

        const READLINE_SNIPPET: &str = indoc!(
            r#"
            import sys

            total_sum = 0

            for line in sys.stdin:
                try:
                    number = float(line.strip())
                    total_sum += number
                except ValueError:
                    continue

            print(f'Total Sum: {total_sum}')
            "#
        );

        let r4 = session
            .run(READLINE_SNIPPET, &[], Some("1\n2\n3\n"), &[], None)
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
            .run(READLINE_SNIPPET, &[], Some("4\n100\n"), &[], None)
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

    fn test7() -> bool {
        let session = bindings::golem::exec::executor::Session::new(
            &Language {
                kind: LanguageKind::Python,
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
                indoc!(
                    r#"
                with open('test/input.txt', 'r') as f:
                    content = f.read()
                print(content)
                with open('test/output.txt', 'w') as f:
                    f.write(content + ' - Processed by Golem')
                "#
                ),
                &[],
                None,
                &[],
                None,
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
                indoc!(
                    r#"
                with open('input.txt', 'r') as f:
                    content = f.read()
                print(os.getcwd())
                print(content)
                with open('output2.txt', 'w') as f:
                    f.write(content + ' - Processed by Golem')
                "#
                ),
                &[],
                None,
                &[],
                None,
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
}

bindings::export!(Component with_types_in bindings);
