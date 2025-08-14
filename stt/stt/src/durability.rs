#[cfg(feature = "durability")]
pub mod durable_impl {
    use golem_rust::bindings::golem::durability::durability::DurableFunctionType;
    use golem_rust::durability::Durability;
    use golem_rust::value_and_type::IntoValue;
    use crate::golem::stt::{transcription, types};

    pub fn persist_transcribe<V: IntoValue + core::fmt::Debug>(
        component_id: &'static str,
        input: V,
        result: Result<transcription::TranscriptionResult, types::SttError>,
    ) -> Result<transcription::TranscriptionResult, types::SttError> {
        type OkT = transcription::TranscriptionResult;
        type ErrT = types::SttError;
        let durability: Durability<OkT, ErrT> = Durability::new(component_id, "transcribe", DurableFunctionType::WriteRemote);
        if durability.is_live() {
            match result {
                Ok(ok) => Ok(durability.persist_infallible(input, ok)),
                Err(e) => Err(e),
            }
        } else {
            Ok(durability.replay_infallible())
        }
    }
}

#[cfg(not(feature = "durability"))]
pub mod durable_impl {
    use golem_rust::value_and_type::IntoValue;
    use crate::golem::stt::{transcription, types};
    pub fn persist_transcribe<V: IntoValue + core::fmt::Debug>(
        _component_id: &'static str,
        _input: V,
        result: Result<transcription::TranscriptionResult, types::SttError>,
    ) -> Result<transcription::TranscriptionResult, types::SttError> { result }
}

pub mod retry {
    use std::time::Duration;
    pub fn with_retries<F, T, E>(mut f: F, max_retries: u32, base_delay_ms: u64) -> Result<T, E>
    where
        F: FnMut(u32) -> Result<T, E>,
    {
        let mut attempt = 0u32;
        loop {
            match f(attempt) {
                Ok(v) => return Ok(v),
                Err(_e) if attempt + 1 < max_retries => {
                    let delay = backoff_delay_ms(attempt, base_delay_ms, base_delay_ms.saturating_mul(25), 100);
                    std::thread::sleep(Duration::from_millis(delay));
                    attempt += 1;
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
    }

    pub fn backoff_delay_ms(attempt: u32, base_ms: u64, max_ms: u64, jitter_ms: u64) -> u64 {
        let exp = 1u64 << attempt.min(20);
        let raw = base_ms.saturating_mul(exp);
        let capped = raw.min(max_ms);
        let jitter = (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().subsec_nanos() as u64) % (jitter_ms.max(1));
        capped.saturating_add(jitter)
    }

    #[cfg(feature = "durability")]
    pub fn with_retries_notify<F, T, E, Notify>(
        mut f: F,
        max_retries: u32,
        base_delay_ms: u64,
        mut notify: Notify,
    ) -> Result<T, E>
    where
        F: FnMut(u32) -> Result<T, E>,
        Notify: FnMut(u32, u64),
    {
        let mut attempt = 0u32;
        loop {
            match f(attempt) {
                Ok(v) => return Ok(v),
                Err(_e) if attempt + 1 < max_retries => {
                    let delay = backoff_delay_ms(attempt, base_delay_ms, base_delay_ms.saturating_mul(25), 100);
                    notify(attempt, delay);
                    std::thread::sleep(Duration::from_millis(delay));
                    attempt += 1;
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
    }
}

pub mod saga {
    #[cfg(feature = "durability")]
    use golem_rust::bindings::golem::durability::durability::DurableFunctionType;
    #[cfg(feature = "durability")]
    use golem_rust::durability::Durability;
    use golem_rust::{FromValueAndType, IntoValue};

    /// A lightweight checkpoint for saga-like observability of STT flows.
    ///
    /// Common `state` values used by providers:
    /// - "started": processing has begun
    /// - "uploaded": media asset uploaded (provider-specific)
    /// - "completed": transcription finished successfully
    /// A lightweight checkpoint for saga-like observability of STT flows.
    ///
    /// Fields are intentionally generic to support different providers.
    #[derive(Clone, Debug, FromValueAndType, IntoValue)]
    pub struct SttCheckpoint {
        pub provider: String,
        pub state: String,
        pub job_id: Option<String>,
        pub media_uri: Option<String>,
        pub audio_sha256: Option<String>,
        pub retry_count: u32,
        pub backoff_ms: u64,
        pub last_ts_ms: u64,
    }

    // Common state constants for provider saga checkpoints
    pub const STATE_STARTED: &str = "started";
    pub const STATE_UPLOADED: &str = "uploaded";
    pub const STATE_COMPLETED: &str = "completed";
    pub const STATE_RETRYING: &str = "retrying";
    pub const STATE_FAILED: &str = "failed";

    #[cfg(feature = "durability")]
    #[derive(Clone, Debug, FromValueAndType, IntoValue)]
    struct NoInput;

    #[cfg(feature = "durability")]
    #[derive(Debug, FromValueAndType, IntoValue)]
    struct UnusedError;

    #[cfg(feature = "durability")]
    pub struct Saga<'a, OkT, ErrT> {
        durability: Durability<SttCheckpoint, UnusedError>,
        _p: core::marker::PhantomData<(&'a OkT, &'a ErrT)>,
    }

    #[cfg(not(feature = "durability"))]
    pub struct Saga<'a, OkT, ErrT> { _p: core::marker::PhantomData<(&'a OkT, &'a ErrT)> }

    #[cfg(not(feature = "durability"))]
    impl<'a, OkT: Clone, ErrT: Clone> Saga<'a, OkT, ErrT> {
        pub fn new(_component_id: &'a str, _fn_name: &'a str, _kind: impl core::fmt::Debug) -> Self { Self { _p: core::marker::PhantomData } }
        pub fn persist_checkpoint<C>(&self, _ckpt: C) {}
    }

    #[cfg(feature = "durability")]
    impl<'a, OkT: Clone, ErrT: Clone> Saga<'a, OkT, ErrT> {
        pub fn new(component_id: &'static str, fn_name: &'static str, _kind: impl core::fmt::Debug) -> Self {
            // Use a stable, static function name derived from the provided function name
            // while satisfying the 'static requirements of the durability API.
            fn saga_fn_name_from(fn_name: &'static str) -> &'static str {
                match fn_name {
                    "transcribe" => "transcribe_saga",
                    // Fallback to transcribe_saga for any unknowns to keep a static name
                    _ => "transcribe_saga",
                }
            }
            let saga_fn: &'static str = saga_fn_name_from(fn_name);
            let durability: Durability<SttCheckpoint, UnusedError> =
                Durability::new(component_id, saga_fn, DurableFunctionType::WriteRemote);
            Self { durability, _p: core::marker::PhantomData }
        }

        fn saga_enabled() -> bool {
            match std::env::var("GOLEM_STT_SAGA_ENABLED") {
                Ok(val) => {
                    let v = val.to_ascii_lowercase();
                    !(v == "0" || v == "false" || v == "off")
                }
                Err(_) => true,
            }
        }

        pub fn persist_checkpoint(&self, ckpt: SttCheckpoint) {
            if !Self::saga_enabled() {
                log::debug!("saga disabled; skipping checkpoint: provider={} state={}", ckpt.provider, ckpt.state);
                return;
            }
            if self.durability.is_live() {
                let mut ckpt_mut = ckpt;
                if ckpt_mut.last_ts_ms == 0 {
                    let now_ms = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_millis() as u64)
                        .unwrap_or(0);
                    ckpt_mut.last_ts_ms = now_ms;
                }
                log::info!(
                    "saga checkpoint persisted: provider={} state={} job_id={:?} retry_count={} backoff_ms={} ts_ms={}",
                    ckpt_mut.provider,
                    ckpt_mut.state,
                    ckpt_mut.job_id,
                    ckpt_mut.retry_count,
                    ckpt_mut.backoff_ms,
                    ckpt_mut.last_ts_ms
                );
                let _ = self.durability.persist_infallible(NoInput, ckpt_mut);
            }
        }

        pub fn persist_outcome(
            &self,
            provider: &str,
            result: &Result<crate::golem::stt::transcription::TranscriptionResult, crate::golem::stt::types::SttError>,
            retry_count: u32,
        ) {
            match result {
                Ok(_) => self.persist_checkpoint(SttCheckpoint {
                    provider: provider.to_string(),
                    state: STATE_COMPLETED.to_string(),
                    job_id: None,
                    media_uri: None,
                    audio_sha256: None,
                    retry_count,
                    backoff_ms: 0,
                    last_ts_ms: 0,
                }),
                Err(_) => self.persist_checkpoint(SttCheckpoint {
                    provider: provider.to_string(),
                    state: STATE_FAILED.to_string(),
                    job_id: None,
                    media_uri: None,
                    audio_sha256: None,
                    retry_count,
                    backoff_ms: 0,
                    last_ts_ms: 0,
                }),
            }
        }
    }
}