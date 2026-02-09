use golem_rust::value_and_type::{FromValueAndType, IntoValue};
use golem_rust::{FromValueAndType, IntoValue};
use std::fmt::Debug;
use std::marker::PhantomData;

/// Wraps an Exec implementation with custom durability
pub struct DurableExec<Impl> {
    phantom: PhantomData<Impl>,
}

pub trait SessionSnapshot<Session> {
    type Snapshot: Debug + Clone + IntoValue + FromValueAndType;

    fn supports_snapshot(session: &Session) -> bool;

    fn take_snapshot(session: &Session) -> Self::Snapshot;
    fn restore_snapshot(session: &Session, snapshot: Self::Snapshot);
}

#[derive(Debug, Clone, IntoValue, FromValueAndType)]
pub struct EmptySnapshot {}

/// When the durability feature flag is off, wrapping with `DurableLLM` is just a passthrough
#[cfg(not(feature = "durability"))]
mod passthrough_impl {
    use crate::durability::DurableExec;
    use crate::golem::exec::executor::{Error, ExecResult, File, Guest, Language, RunOptions};

    impl<Impl: Guest> Guest for DurableExec<Impl> {
        type Session = Impl::Session;

        fn run(
            lang: Language,
            modules: Vec<File>,
            snippet: String,
            options: RunOptions,
        ) -> Result<ExecResult, Error> {
            Impl::run(lang, modules, snippet, options)
        }
    }
}

#[cfg(feature = "durability")]
mod durable_impl {
    use crate::durability::{DurableExec, SessionSnapshot};
    use crate::model::{Error, ExecResult, File, Language, RunOptions};
    use crate::{ExecutionProvider, ExecutionSession};
    use golem_rust::bindings::golem::durability::durability::DurableFunctionType;
    use golem_rust::durability::Durability;
    use golem_rust::value_and_type::{
        FromValueAndType, IntoValue, NodeBuilder, TypeNodeBuilder, WitValueExtractor,
    };
    use golem_rust::{with_persistence_level, FromValueAndType, IntoValue, PersistenceLevel};
    use std::fmt::{Debug, Display, Formatter};

    impl<Impl: ExecutionProvider + SessionSnapshot<Impl::Session> + 'static> ExecutionProvider
        for DurableExec<Impl>
    {
        type Session = DurableSession<Impl>;

        fn run(
            lang: Language,
            modules: Vec<File>,
            snippet: String,
            options: RunOptions,
        ) -> Result<ExecResult, Error> {
            let durability = Durability::<ExecResult, Error>::new(
                "golem_exec",
                "run",
                DurableFunctionType::WriteLocal,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::run(
                        lang.clone(),
                        modules.clone(),
                        snippet.clone(),
                        options.clone(),
                    )
                });
                durability.persist_serializable(
                    RunInput {
                        language: lang,
                        modules: modules.iter().map(|f| f.name.clone()).collect(),
                        snippet,
                        options,
                    },
                    result.clone(),
                );
                result
            } else {
                durability.replay_serializable()
            }
        }
    }

    pub struct DurableSession<Impl: ExecutionProvider> {
        inner: Impl::Session,
        lang: Language,
        module_names: Vec<String>,
    }

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

        fn run(&self, snippet: String, options: RunOptions) -> Result<ExecResult, Error> {
            let durability = Durability::<SessionRunResult<Impl::Snapshot>, UnusedError>::new(
                "golem_exec",
                "session_run",
                DurableFunctionType::WriteLocal,
            );
            let input = RunInput {
                language: self.lang.clone(),
                modules: self.module_names.clone(),
                snippet: snippet.clone(),
                options: options.clone(),
            };
            if Impl::supports_snapshot(&self.inner) {
                // We can take a snapshot of the session and restore it during replay without
                // actually running the snippet.
                if durability.is_live() {
                    let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                        self.inner.run(snippet, options)
                    });
                    let snapshot = Impl::take_snapshot(&self.inner);
                    let result = SessionRunResult {
                        result,
                        snapshot: Some(snapshot),
                    };
                    durability.persist_infallible(input, result.clone());
                    result.result
                } else {
                    let result: SessionRunResult<Impl::Snapshot> = durability.replay_infallible();
                    if let Some(snapshot) = result.snapshot {
                        Impl::restore_snapshot(&self.inner, snapshot);
                    }
                    result.result
                }
            } else {
                // We cannot take a snapshot of the session so we have to run the actual snippet
                // in both live and replay modes.
                //
                // We still persist a custom oplog entry to increase oplog readability
                let result = self.inner.run(snippet, options);
                let result = SessionRunResult {
                    result,
                    snapshot: None,
                };

                if durability.is_live() {
                    durability.persist_infallible(input, result.clone());
                } else {
                    let _: SessionRunResult<Impl::Snapshot> = durability.replay_infallible();
                }
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

    #[derive(Debug, IntoValue)]
    struct RunInput {
        language: Language,
        modules: Vec<String>,
        snippet: String,
        options: RunOptions,
    }

    #[derive(Debug, Clone)]
    struct SessionRunResult<Snapshot: Debug + Clone> {
        result: Result<ExecResult, Error>,
        snapshot: Option<Snapshot>,
    }

    impl<Snapshot: IntoValue + Debug + Clone> IntoValue for SessionRunResult<Snapshot> {
        fn add_to_builder<T: NodeBuilder>(self, builder: T) -> T::Result {
            let builder = builder.record();
            let builder = self.result.add_to_builder(builder.item());
            let builder = self.snapshot.add_to_builder(builder.item());
            builder.finish()
        }

        fn add_to_type_builder<T: TypeNodeBuilder>(builder: T) -> T::Result {
            let builder = builder.record(Some("SessionRunResult".to_string()), None);
            let builder = Result::<ExecResult, Error>::add_to_type_builder(builder.field("result"));
            let builder = Option::<Snapshot>::add_to_type_builder(builder.field("snapshot"));
            builder.finish()
        }
    }

    impl<Snapshot: FromValueAndType + Debug + Clone> FromValueAndType for SessionRunResult<Snapshot> {
        fn from_extractor<'a, 'b>(
            extractor: &'a impl WitValueExtractor<'a, 'b>,
        ) -> Result<Self, String> {
            Ok(SessionRunResult {
                result: Result::<ExecResult, Error>::from_extractor(
                    &extractor
                        .field(0)
                        .ok_or_else(|| "Missing result field".to_string())?,
                )?,
                snapshot: Option::<Snapshot>::from_extractor(
                    &extractor.field(1).ok_or("Missing snapshot field")?,
                )?,
            })
        }
    }

    #[derive(Debug, FromValueAndType, IntoValue)]
    struct UnusedError;

    impl Display for UnusedError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "UnusedError")
        }
    }
}
