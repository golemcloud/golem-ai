use std::time::Duration;

#[allow(async_fn_in_trait)]
pub trait AsyncRuntime {
    async fn sleep(&self, duration: Duration);
}

pub struct WasiAsyncRuntime {}

impl WasiAsyncRuntime {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for WasiAsyncRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl AsyncRuntime for WasiAsyncRuntime {
    async fn sleep(&self, duration: Duration) {
        let duration_nanos = u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX);
        wasip3::clocks::monotonic_clock::wait_for(duration_nanos).await;
    }
}
