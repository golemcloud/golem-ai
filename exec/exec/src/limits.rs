use crate::error::runtime;
use crate::types::*;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct ResourceMonitor {
    start_time: Instant,
    timeout: Option<Duration>,
    memory_limit: Option<u64>,
    max_processes: Option<u32>,
}

impl ResourceMonitor {
    pub fn new(limits: Option<Limits>) -> Self {
        let timeout = limits
            .as_ref()
            .and_then(|l| l.time_ms.map(Duration::from_millis));
        let memory_limit = limits.as_ref().and_then(|l| l.memory_bytes);
        let max_processes = limits.as_ref().and_then(|l| l.max_processes);

        Self {
            start_time: Instant::now(),
            timeout,
            memory_limit,
            max_processes,
        }
    }

    pub fn check_timeout(&self) -> crate::error::ExecResult<()> {
        if let Some(timeout) = self.timeout {
            if self.start_time.elapsed() > timeout {
                return Err(runtime::execution_timeout());
            }
        }
        Ok(())
    }

    pub fn remaining_time(&self) -> Option<Duration> {
        self.timeout.map(|timeout| {
            let elapsed = self.start_time.elapsed();
            if elapsed >= timeout {
                Duration::from_millis(0)
            } else {
                timeout - elapsed
            }
        })
    }

    pub fn check_memory(&self, current_usage: Option<u64>) -> crate::error::ExecResult<()> {
        if let (Some(limit), Some(usage)) = (self.memory_limit, current_usage) {
            if usage > limit {
                return Err(runtime::memory_limit_exceeded(usage, limit));
            }
        }
        Ok(())
    }

    pub fn check_process_count(&self, current_count: Option<u32>) -> crate::error::ExecResult<()> {
        if let (Some(limit), Some(count)) = (self.max_processes, current_count) {
            if count > limit {
                return Err(runtime::process_limit_exceeded(count, limit));
            }
        }
        Ok(())
    }

    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    pub fn elapsed_ms(&self) -> u64 {
        self.elapsed().as_millis() as u64
    }
}

pub struct TimeoutWrapper<T> {
    inner: T,
    monitor: ResourceMonitor,
}

impl<T> TimeoutWrapper<T> {
    pub fn new(inner: T, limits: Option<Limits>) -> Self {
        Self {
            inner,
            monitor: ResourceMonitor::new(limits),
        }
    }

    pub fn with_timeout<F, R>(&self, operation: F) -> crate::error::ExecResult<R>
    where
        F: FnOnce(&T) -> crate::error::ExecResult<R>,
    {
        self.monitor.check_timeout()?;
        operation(&self.inner)
    }

    pub fn monitor(&self) -> &ResourceMonitor {
        &self.monitor
    }
}

pub struct MemoryTracker {
    initial_usage: Option<u64>,
}

impl Default for MemoryTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryTracker {
    pub fn new() -> Self {
        Self {
            initial_usage: Self::get_current_memory_usage(),
        }
    }

    pub fn get_current_memory_usage() -> Option<u64> {
        #[cfg(target_os = "linux")]
        {
            Self::get_linux_memory_usage()
        }

        #[cfg(target_os = "macos")]
        {
            Self::get_macos_memory_usage()
        }

        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            None // Unsupported platform
        }
    }

    #[cfg(target_os = "linux")]
    fn get_linux_memory_usage() -> Option<u64> {
        use std::fs;

        let status = fs::read_to_string("/proc/self/status").ok()?;
        for line in status.lines() {
            if line.starts_with("VmRSS:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(kb) = parts[1].parse::<u64>() {
                        return Some(kb * 1024); // Convert KB to bytes
                    }
                }
            }
        }
        None
    }

    #[cfg(target_os = "macos")]
    fn get_macos_memory_usage() -> Option<u64> {
        None
    }

    pub fn get_usage_delta(&self) -> Option<u64> {
        let current = Self::get_current_memory_usage()?;
        let initial = self.initial_usage?;

        if current >= initial {
            Some(current - initial)
        } else {
            Some(0)
        }
    }
}

pub struct ProcessCounter;

impl ProcessCounter {
    pub fn get_current_process_count() -> Option<u32> {
        Some(1) // Just the current process
    }
}

pub mod utils {
    use super::*;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    pub fn run_with_timeout<F, R>(timeout: Duration, f: F) -> Result<R, Error>
    where
        F: FnOnce() -> Result<R, Error> + Send + 'static,
        R: Send + 'static,
    {
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            let result = f();
            let _ = tx.send(result);
        });

        match rx.recv_timeout(timeout) {
            Ok(result) => result,
            Err(mpsc::RecvTimeoutError::Timeout) => Err(Error::Timeout),
            Err(mpsc::RecvTimeoutError::Disconnected) => Err(runtime::thread_disconnected()),
        }
    }

    pub fn create_default_monitor() -> ResourceMonitor {
        let default_limits = Limits {
            time_ms: Some(5000), // 5 seconds default
            memory_bytes: None,  // No memory limit by default
            file_size_bytes: None,
            max_processes: Some(1), // Single process by default
        };
        ResourceMonitor::new(Some(default_limits))
    }

    pub fn validate_limits(limits: &Limits) -> Result<(), Error> {
        if let Some(time_ms) = limits.time_ms {
            if time_ms == 0 {
                return Err(runtime::invalid_timeout("Timeout cannot be zero"));
            }
            if time_ms > 300_000 {
                // 5 minutes max
                return Err(runtime::invalid_timeout(
                    "Timeout too large (max 5 minutes)",
                ));
            }
        }

        if let Some(memory_bytes) = limits.memory_bytes {
            if memory_bytes == 0 {
                return Err(runtime::invalid_memory_limit("Memory limit cannot be zero"));
            }
            if memory_bytes > 2_147_483_648 {
                // 2GB max
                return Err(runtime::invalid_memory_limit(
                    "Memory limit too large (max 2GB)",
                ));
            }
        }

        if let Some(file_size_bytes) = limits.file_size_bytes {
            if file_size_bytes == 0 {
                return Err(runtime::invalid_file_size_limit(
                    "File size limit cannot be zero",
                ));
            }
            if file_size_bytes > 104_857_600 {
                // 100MB max
                return Err(runtime::invalid_file_size_limit(
                    "File size limit too large (max 100MB)",
                ));
            }
        }

        if let Some(max_processes) = limits.max_processes {
            if max_processes == 0 {
                return Err(runtime::invalid_process_limit(
                    "Process limit cannot be zero",
                ));
            }
            if max_processes > 10 {
                // 10 processes max
                return Err(runtime::invalid_process_limit(
                    "Process limit too large (max 10)",
                ));
            }
        }

        Ok(())
    }

    pub fn get_timeout_duration(limits: &Option<Limits>, default_ms: u64) -> Duration {
        limits
            .as_ref()
            .and_then(|l| l.time_ms)
            .map(Duration::from_millis)
            .unwrap_or_else(|| Duration::from_millis(default_ms))
    }

    pub fn get_memory_limit_bytes(limits: &Option<Limits>, default_mb: u64) -> u64 {
        limits
            .as_ref()
            .and_then(|l| l.memory_bytes)
            .unwrap_or(default_mb * 1024 * 1024)
    }

    pub fn check_file_size(
        file_size: usize,
        limits: &Option<Limits>,
    ) -> crate::error::ExecResult<()> {
        if let Some(ref limits) = limits {
            if let Some(max_size) = limits.file_size_bytes {
                if file_size as u64 > max_size {
                    return Err(runtime::file_size_limit_exceeded(
                        "file",
                        file_size as u64,
                        max_size,
                    ));
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_resource_monitor_timeout() {
        let limits = Limits {
            time_ms: Some(100), // 100ms timeout
            memory_bytes: None,
            file_size_bytes: None,
            max_processes: None,
        };

        let monitor = ResourceMonitor::new(Some(limits));

        assert!(monitor.check_timeout().is_ok());

        thread::sleep(Duration::from_millis(150));

        assert!(monitor.check_timeout().is_err());
    }

    #[test]
    fn test_memory_tracker() {
        let tracker = MemoryTracker::new();
        let usage = tracker.get_usage_delta();
        if let Some(_usage) = usage {}
    }

    #[test]
    fn test_validate_limits() {
        let valid_limits = Limits {
            time_ms: Some(5000),
            memory_bytes: Some(1024 * 1024), // 1MB
            file_size_bytes: None,
            max_processes: Some(2),
        };
        assert!(utils::validate_limits(&valid_limits).is_ok());

        let invalid_limits = Limits {
            time_ms: Some(0), // Invalid: zero timeout
            memory_bytes: None,
            file_size_bytes: None,
            max_processes: None,
        };
        assert!(utils::validate_limits(&invalid_limits).is_err());
    }
}
