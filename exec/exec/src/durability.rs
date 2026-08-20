use std::marker::PhantomData;

/// Wraps an Exec implementation with custom durability
pub struct DurableExec<Impl> {
    phantom: PhantomData<Impl>,
}

#[cfg(feature = "golem")]
pub use snapshot::{EmptySnapshot, SessionSnapshot};

#[cfg(feature = "golem")]
mod snapshot {
    use golem_rust::{FromSchema, IntoSchema};
    use std::fmt::Debug;

    pub trait SessionSnapshot<Session> {
        type Snapshot: Debug + Clone + IntoSchema + FromSchema;

        fn supports_snapshot(session: &Session) -> bool;

        fn take_snapshot(session: &Session) -> Self::Snapshot;
        fn restore_snapshot(session: &Session, snapshot: Self::Snapshot);
    }

    #[derive(Debug, Clone, IntoSchema, FromSchema)]
    pub struct EmptySnapshot {}
}

/// When the durability feature flag is off, `DurableExec<Impl>` is a transparent wrapper that
/// forwards every call to the inner provider without any oplog persistence.
#[cfg(not(feature = "golem"))]
mod passthrough_impl {
    use crate::durability::DurableExec;
    use crate::model::{Error, ExecResult, File, Language, RunOptions};
    use crate::ExecutionProvider;
    use async_trait::async_trait;

    #[async_trait(?Send)]
    impl<Impl: ExecutionProvider + 'static> ExecutionProvider for DurableExec<Impl> {
        type Session = Impl::Session;

        async fn run(
            lang: Language,
            modules: Vec<File>,
            snippet: String,
            options: RunOptions,
        ) -> Result<ExecResult, Error> {
            Impl::run(lang, modules, snippet, options).await
        }
    }
}

#[cfg(feature = "golem")]
mod durable_impl {
    use crate::durability::{DurableExec, SessionSnapshot};
    use crate::model::{Error, ExecResult, File, Language, RunOptions};
    use crate::{ExecutionProvider, ExecutionSession};
    use async_trait::async_trait;
    use golem_rust::durability::{Durability, DurableFunctionType};
    use golem_rust::{FromSchema, IntoSchema};
    use std::fmt::{Debug, Display, Formatter};

    impl From<&Error> for Error {
        fn from(error: &Error) -> Self {
            error.clone()
        }
    }

    #[async_trait(?Send)]
    impl<Impl: ExecutionProvider + SessionSnapshot<Impl::Session> + 'static> ExecutionProvider
        for DurableExec<Impl>
    {
        type Session = DurableSession<Impl>;

        async fn run(
            lang: Language,
            modules: Vec<File>,
            snippet: String,
            options: RunOptions,
        ) -> Result<ExecResult, Error> {
            let input = RunInput {
                language: lang.clone(),
                modules: modules.iter().map(|f| f.name.clone()).collect(),
                snippet: snippet.clone(),
                options: options.clone(),
            };
            let durability = Durability::<ExecResult, Error>::new(
                "golem_ai_exec",
                "run",
                DurableFunctionType::WriteLocal,
                &input,
            );
            durability
                .run_async(|| Impl::run(lang, modules, snippet, options))
                .await
        }
    }

    pub struct DurableSession<Impl: ExecutionProvider> {
        inner: Impl::Session,
        lang: Language,
        module_names: Vec<String>,
    }

    #[async_trait(?Send)]
    impl<Impl: ExecutionProvider + SessionSnapshot<Impl::Session> + 'static> ExecutionSession
        for DurableSession<Impl>
    {
        fn new(lang: Language, modules: Vec<File>) -> Self {
            Self {
                lang: lang.clone(),
                module_names: modules.iter().map(|f| f.name.clone()).collect(),
                inner: Impl::Session::new(lang.clone(), modules),
            }
        }

        fn upload(&self, file: File) -> Result<(), Error> {
            self.inner.upload(file)
        }

        async fn run(&self, snippet: String, options: RunOptions) -> Result<ExecResult, Error> {
            let input = RunInput {
                language: self.lang.clone(),
                modules: self.module_names.clone(),
                snippet: snippet.clone(),
                options: options.clone(),
            };
            if Impl::supports_snapshot(&self.inner) {
                // We can take a snapshot of the session and restore it during replay without
                // actually running the snippet.
                let durability = Durability::<SessionRunResult<Impl::Snapshot>, UnusedError>::new(
                    "golem_ai_exec",
                    "session_run",
                    DurableFunctionType::WriteLocal,
                    &input,
                );
                let mut ran = false;
                let result = durability
                    .run_infallible_async(|| async {
                        ran = true;
                        let result = self.inner.run(snippet, options).await;
                        let snapshot = Impl::take_snapshot(&self.inner);
                        SessionRunResult {
                            result,
                            snapshot: Some(snapshot),
                        }
                    })
                    .await;
                if !ran {
                    if let Some(snapshot) = result.snapshot {
                        Impl::restore_snapshot(&self.inner, snapshot);
                    }
                }
                result.result
            } else {
                // We cannot take a snapshot of the session, so we have to run the actual snippet
                // in both live and replay modes.
                //
                // We still persist a custom oplog entry to increase oplog readability. Construct
                // it only after the run because `Durability::new` opens its replay subtree
                // immediately, which would otherwise consume the snippet's host calls.
                let result = self.inner.run(snippet, options).await;
                let result = SessionRunResult {
                    result,
                    snapshot: None,
                };
                let durability = Durability::<SessionRunResult<Impl::Snapshot>, UnusedError>::new(
                    "golem_ai_exec",
                    "session_run",
                    DurableFunctionType::WriteLocal,
                    &input,
                );
                let _: SessionRunResult<Impl::Snapshot> =
                    durability.run_infallible(|| result.clone());
                result.result
            }
        }

        fn download(&self, path: String) -> Result<Vec<u8>, Error> {
            self.inner.download(path)
        }

        fn list_files(&self, dir: String) -> Result<Vec<String>, Error> {
            self.inner.list_files(dir)
        }

        fn set_working_dir(&self, path: String) -> Result<(), Error> {
            self.inner.set_working_dir(path)
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }
    }

    #[derive(Debug, Clone, IntoSchema)]
    struct RunInput {
        language: Language,
        modules: Vec<String>,
        snippet: String,
        options: RunOptions,
    }

    #[derive(Debug, Clone, FromSchema, IntoSchema)]
    struct SessionRunResult<Snapshot: Debug + Clone> {
        result: Result<ExecResult, Error>,
        snapshot: Option<Snapshot>,
    }

    #[derive(Debug, FromSchema, IntoSchema)]
    struct UnusedError;

    impl Display for UnusedError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "UnusedError")
        }
    }
}
