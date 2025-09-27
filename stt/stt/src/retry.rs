use crate::runtime::AsyncRuntime;
use std::time::Duration;

#[derive(Clone)]
pub struct RetryConfig {
    pub max_attempts: usize,
    pub min_delay: Duration,
    pub max_delay: Duration,
}

impl RetryConfig {
    pub fn new() -> Self {
        Self {
            max_attempts: 3,
            min_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
        }
    }

    pub fn with_max_attempts(mut self, max_attempts: usize) -> Self {
        self.max_attempts = max_attempts;
        self
    }

    pub fn with_min_delay(mut self, min_delay: Duration) -> Self {
        self.min_delay = min_delay;
        self
    }

    pub fn with_max_delay(mut self, max_delay: Duration) -> Self {
        self.max_delay = max_delay;
        self
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Retry<R: AsyncRuntime> {
    config: RetryConfig,
    runtime: R,
}

impl<R: AsyncRuntime> Retry<R> {
    pub fn new(config: RetryConfig, runtime: R) -> Self {
        Self { config, runtime }
    }

    pub async fn retry_when<F, Fut, T, E, P>(
        &self,
        should_retry: P,
        mut operation: F,
    ) -> Result<T, E>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
        P: Fn(&Result<T, E>) -> bool,
    {
        let mut attempts = 0;

        loop {
            let result = operation().await;

            if attempts < self.config.max_attempts - 1 && should_retry(&result) && result.is_err() {
                attempts += 1;
                let delay = std::cmp::min(
                    self.config.min_delay * 2_u32.pow(attempts as u32),
                    self.config.max_delay,
                );
                self.runtime.sleep(delay).await;
                continue;
            }

            return result;
        }
    }
}
