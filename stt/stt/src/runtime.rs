use std::time::Duration;

use wstd::task;

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
        task::sleep(duration.into()).await;
    }
}
