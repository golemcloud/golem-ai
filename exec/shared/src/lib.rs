use std::collections::HashMap;
use std::time::{Duration, Instant};
use anyhow::Result;

/// WIT-compatible types
#[derive(Debug, Clone)]
pub struct StageResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub signal: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub compile: Option<StageResult>,
    pub run: StageResult,
    pub time_ms: Option<u64>,
    pub memory_bytes: Option<u64>,
}

/// Session management utilities
pub struct SessionManager {
    sessions: HashMap<String, SessionData>,
}

#[derive(Debug)]
pub struct SessionData {
    pub language: String,
    pub files: HashMap<String, Vec<u8>>,
    pub working_dir: String,
    pub created_at: Instant,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }

    pub fn create_session(&mut self, id: String, language: String) -> Result<()> {
        let session = SessionData {
            language,
            files: HashMap::new(),
            working_dir: "/".to_string(),
            created_at: Instant::now(),
        };
        self.sessions.insert(id, session);
        Ok(())
    }

    pub fn get_session(&self, id: &str) -> Option<&SessionData> {
        self.sessions.get(id)
    }

    pub fn get_session_mut(&mut self, id: &str) -> Option<&mut SessionData> {
        self.sessions.get_mut(id)
    }

    pub fn remove_session(&mut self, id: &str) -> Option<SessionData> {
        self.sessions.remove(id)
    }
}

/// Resource limiting utilities
pub struct ResourceLimiter {
    max_memory: Option<u64>,
    max_time: Option<Duration>,
}

impl ResourceLimiter {
    pub fn new(max_memory: Option<u64>, max_time_ms: Option<u64>) -> Self {
        Self {
            max_memory,
            max_time: max_time_ms.map(Duration::from_millis),
        }
    }

    pub fn check_timeout(&self, start: Instant) -> Result<()> {
        if let Some(max_time) = self.max_time {
            if start.elapsed() > max_time {
                return Err(anyhow::anyhow!("Execution timeout"));
            }
        }
        Ok(())
    }
}

/// Execution engine trait
pub trait ExecutionEngine {
    fn execute(&self, code: &str) -> Result<ExecutionResult>;
    fn execute_with_limits(&self, code: &str, limiter: &ResourceLimiter) -> Result<ExecutionResult>;
}