use crate::error::{fs, validation};
use crate::types::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use uuid::Uuid;

pub struct ExecutionEnvironment {
    pub working_dir: PathBuf,
    pub files: HashMap<String, Vec<u8>>,
    pub env_vars: HashMap<String, String>,
    pub stdin_content: Option<String>,
    pub args: Vec<String>,
}

impl ExecutionEnvironment {
    pub fn new(context: &ExecutionContext) -> crate::error::ExecResult<Self> {
        let working_dir = {
            #[cfg(target_arch = "wasm32")]
            {
                // In WASM, we can't create real directories, so use a virtual path
                PathBuf::from(format!("golem-exec-{}", Uuid::new_v4()))
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let temp_dir = std::env::temp_dir().join(format!("golem-exec-{}", Uuid::new_v4()));
                std::fs::create_dir_all(&temp_dir)
                    .map_err(|e| fs::dir_creation_failed(&temp_dir.to_string_lossy(), e))?;
                temp_dir
            }
        };

        let mut files = HashMap::new();
        for file in &context.files {
            let encoding = file.encoding.unwrap_or(Encoding::Utf8);
            let content_str = std::str::from_utf8(&file.content).map_err(|e| {
                validation::invalid_encoding(&file.name, "UTF-8", &format!("Invalid UTF-8: {e}"))
            })?;
            let content = crate::encoding::decode_content(content_str, encoding)?;
            files.insert(file.name.clone(), content);
        }

        let env_vars: HashMap<String, String> = context.env.iter().cloned().collect();

        Ok(Self {
            working_dir,
            files,
            env_vars,
            stdin_content: context.stdin.clone(),
            args: context.args.clone(),
        })
    }

    pub fn write_files(&self) -> crate::error::ExecResult<()> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            for (filename, content) in &self.files {
                let file_path = self.working_dir.join(filename);

                if let Some(parent) = file_path.parent() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| fs::dir_creation_failed(&parent.to_string_lossy(), e))?;
                }

                std::fs::write(&file_path, content)
                    .map_err(|e| fs::file_write_failed(filename, e))?
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            // In WASM, files are kept in memory only
            // No actual file system operations needed
        }

        Ok(())
    }

    pub fn cleanup(&self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Err(e) = std::fs::remove_dir_all(&self.working_dir) {
                log::warn!(
                    "Failed to cleanup working dir {:?}: {}",
                    &self.working_dir,
                    e
                );
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            // In WASM, no actual cleanup needed for virtual directories
        }
    }

    pub fn get_file_content(&self, filename: &str) -> crate::error::ExecResult<String> {
        let content = self
            .files
            .get(filename)
            .ok_or_else(|| validation::file_not_found(filename))?;

        String::from_utf8(content.clone()).map_err(|e| {
            validation::invalid_encoding(
                filename,
                "UTF-8",
                &format!("File {filename} is not valid UTF-8: {e}"),
            )
        })
    }

    pub fn has_file(&self, filename: &str) -> bool {
        self.files.contains_key(filename)
    }

    pub fn list_files(&self) -> Vec<String> {
        self.files.keys().cloned().collect()
    }
}

pub struct ProcessExecutor {
    pub timeout: Option<Duration>,
    pub memory_limit: Option<u64>,
    pub working_dir: PathBuf,
    pub env_vars: HashMap<String, String>,
}

impl ProcessExecutor {
    pub fn new(working_dir: PathBuf) -> Self {
        Self {
            timeout: None,
            memory_limit: None,
            working_dir,
            env_vars: HashMap::new(),
        }
    }

    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout = Some(Duration::from_millis(timeout_ms));
        self
    }

    pub fn with_memory_limit(mut self, memory_mb: u64) -> Self {
        self.memory_limit = Some(memory_mb * 1024 * 1024); // Convert to bytes
        self
    }

    pub fn with_env(mut self, key: String, value: String) -> Self {
        self.env_vars.insert(key, value);
        self
    }

    pub fn with_env_vars(mut self, vars: HashMap<String, String>) -> Self {
        self.env_vars.extend(vars);
        self
    }
}

pub struct OutputCapture {
    pub stdout: Vec<String>,
    pub stderr: Vec<String>,
    pub start_time: Instant,
}

impl Default for OutputCapture {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputCapture {
    pub fn new() -> Self {
        Self {
            stdout: Vec::new(),
            stderr: Vec::new(),
            start_time: Instant::now(),
        }
    }

    pub fn add_stdout(&mut self, line: String) {
        self.stdout.push(line);
    }

    pub fn add_stderr(&mut self, line: String) {
        self.stderr.push(line);
    }

    pub fn get_stdout(&self) -> String {
        self.stdout.join("\n")
    }

    pub fn get_stderr(&self) -> String {
        self.stderr.join("\n")
    }

    pub fn elapsed_ms(&self) -> u64 {
        self.start_time.elapsed().as_millis() as u64
    }

    pub fn to_stage_result(&self, exit_code: Option<i32>) -> StageResult {
        StageResult {
            stdout: self.get_stdout(),
            stderr: self.get_stderr(),
            exit_code,
            signal: None,
        }
    }
}

pub mod security {
    use super::*;

    pub fn validate_filename(filename: &str) -> crate::error::ExecResult<()> {
        if filename.contains("..") {
            return Err(validation::invalid_filename("Path traversal not allowed"));
        }

        if filename.starts_with('/') {
            return Err(validation::invalid_filename("Absolute paths not allowed"));
        }

        if filename.is_empty() {
            return Err(validation::invalid_filename("Empty filename not allowed"));
        }

        if filename.len() > 255 {
            return Err(validation::invalid_filename("Filename too long"));
        }

        Ok(())
    }

    pub fn sanitize_env_name(name: &str) -> String {
        name.chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect()
    }

    pub fn validate_env_value(value: &str) -> crate::error::ExecResult<()> {
        if value.contains('\0') {
            return Err(validation::invalid_limit(
                "Null bytes not allowed in environment values",
            ));
        }

        if value.len() > 4096 {
            return Err(validation::invalid_limit("Environment value too long"));
        }

        Ok(())
    }

    pub fn create_secure_temp_dir(prefix: &str) -> crate::error::ExecResult<PathBuf> {
        let temp_base = std::env::temp_dir();
        let dir_name = format!("{}-{}-{}", prefix, std::process::id(), Uuid::new_v4());
        let temp_dir = temp_base.join(dir_name);

        std::fs::create_dir_all(&temp_dir)
            .map_err(|e| fs::dir_creation_failed(&temp_dir.to_string_lossy(), e))?;

        // Set restrictive permissions (Unix only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&temp_dir)
                .map_err(|e| fs::metadata_failed(&temp_dir.to_string_lossy(), e))?
                .permissions();
            perms.set_mode(0o700); // Owner read/write/execute only
            std::fs::set_permissions(&temp_dir, perms)
                .map_err(|e| fs::permissions_failed(&temp_dir.to_string_lossy(), e))?;
        }

        Ok(temp_dir)
    }
}

pub mod monitoring {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Duration;

    #[derive(Debug, Clone)]
    pub struct ResourceUsage {
        pub cpu_percent: f64,
        pub memory_bytes: u64,
        pub elapsed_ms: u64,
    }

    pub struct ResourceMonitor {
        start_time: Instant,
        usage: Arc<Mutex<ResourceUsage>>,
        _monitor_thread: Option<thread::JoinHandle<()>>,
    }

    impl Default for ResourceMonitor {
        fn default() -> Self {
            Self::new()
        }
    }

    impl ResourceMonitor {
        pub fn new() -> Self {
            let usage = Arc::new(Mutex::new(ResourceUsage {
                cpu_percent: 0.0,
                memory_bytes: 0,
                elapsed_ms: 0,
            }));

            Self {
                start_time: Instant::now(),
                usage,
                _monitor_thread: None,
            }
        }

        pub fn start_monitoring(&mut self, pid: u32) {
            let usage = Arc::clone(&self.usage);
            let start_time = self.start_time;
            let should_stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
            let stop_flag = Arc::clone(&should_stop);

            let handle = thread::spawn(move || {
                let mut last_cpu_time = get_process_cpu_time(pid).unwrap_or(0);
                let mut last_check = Instant::now();

                while !stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
                    thread::sleep(Duration::from_millis(100));

                    if let Ok(mut usage) = usage.lock() {
                        usage.elapsed_ms = start_time.elapsed().as_millis() as u64;

                        if let Some(memory) = get_process_memory(pid) {
                            usage.memory_bytes = memory;
                        }

                        let now = Instant::now();
                        let time_delta = now.duration_since(last_check).as_millis() as f64;

                        if let Some(cpu_time) = get_process_cpu_time(pid) {
                            let cpu_delta = cpu_time.saturating_sub(last_cpu_time) as f64;
                            if time_delta > 0.0 {
                                usage.cpu_percent = (cpu_delta / time_delta) * 100.0;
                            }
                            last_cpu_time = cpu_time;
                        }

                        last_check = now;
                    }

                    if !process_exists(pid) {
                        break;
                    }
                }
            });

            self._monitor_thread = Some(handle);
        }

        pub fn get_usage(&self) -> ResourceUsage {
            if let Ok(usage) = self.usage.lock() {
                usage.clone()
            } else {
                ResourceUsage {
                    cpu_percent: 0.0,
                    memory_bytes: 0,
                    elapsed_ms: self.start_time.elapsed().as_millis() as u64,
                }
            }
        }
    }
}

#[cfg(target_os = "macos")]
fn get_process_memory(pid: u32) -> Option<u64> {
    use std::process::Command;

    let output = Command::new("ps")
        .args(["-o", "rss=", "-p", &pid.to_string()])
        .output()
        .ok()?;

    let rss_str = String::from_utf8(output.stdout).ok()?;
    let rss_kb: u64 = rss_str.trim().parse().ok()?;
    Some(rss_kb * 1024) // Convert KB to bytes
}

#[cfg(target_os = "linux")]
fn get_process_memory(pid: u32) -> Option<u64> {
    use std::fs;

    let status_path = format!("/proc/{}/status", pid);
    let content = fs::read_to_string(status_path).ok()?;

    for line in content.lines() {
        if line.starts_with("VmRSS:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let kb: u64 = parts[1].parse().ok()?;
                return Some(kb * 1024); // Convert KB to bytes
            }
        }
    }
    None
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn get_process_memory(_pid: u32) -> Option<u64> {
    None // Unsupported platform
}

#[cfg(target_os = "macos")]
fn get_process_cpu_time(pid: u32) -> Option<u64> {
    use std::process::Command;

    let output = Command::new("ps")
        .args(["-o", "time=", "-p", &pid.to_string()])
        .output()
        .ok()?;

    let time_str = String::from_utf8(output.stdout).ok()?;
    // Parse time format like "0:00.01" to milliseconds
    parse_time_to_ms(time_str.trim())
}

#[cfg(target_os = "linux")]
fn get_process_cpu_time(pid: u32) -> Option<u64> {
    use std::fs;

    let stat_path = format!("/proc/{}/stat", pid);
    let content = fs::read_to_string(stat_path).ok()?;
    let fields: Vec<&str> = content.split_whitespace().collect();

    if fields.len() >= 15 {
        let utime: u64 = fields[13].parse().ok()?;
        let stime: u64 = fields[14].parse().ok()?;
        // Convert clock ticks to milliseconds (assuming 100 Hz)
        Some((utime + stime) * 10)
    } else {
        None
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn get_process_cpu_time(_pid: u32) -> Option<u64> {
    None // Unsupported platform
}

#[allow(dead_code)]
fn parse_time_to_ms(time_str: &str) -> Option<u64> {
    let parts: Vec<&str> = time_str.split(':').collect();
    if parts.len() == 2 {
        let minutes: u64 = parts[0].parse().ok()?;
        let seconds_parts: Vec<&str> = parts[1].split('.').collect();
        if seconds_parts.len() == 2 {
            let seconds: u64 = seconds_parts[0].parse().ok()?;
            let centiseconds: u64 = seconds_parts[1].parse().ok()?;
            Some(minutes * 60000 + seconds * 1000 + centiseconds * 10)
        } else {
            None
        }
    } else {
        None
    }
}

fn process_exists(_pid: u32) -> bool {
    #[cfg(unix)]
    {
        use std::process::Command;
        Command::new("kill")
            .args(["-0", &_pid.to_string()])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    #[cfg(not(unix))]
    {
        // Fallback for non-Unix systems
        true
    }
}

pub mod utils {
    use super::*;

    pub fn create_test_context(lang: LanguageKind, code: &str, filename: &str) -> ExecutionContext {
        ExecutionContext {
            language: Language {
                kind: lang,
                version: None,
            },
            files: vec![File {
                name: filename.to_string(),
                content: code.as_bytes().to_vec(),
                encoding: Some(Encoding::Utf8),
            }],
            stdin: None,
            args: vec![],
            env: vec![],
            constraints: None,
            config: Config::default(),
        }
    }

    pub fn merge_results(compile: Option<StageResult>, run: StageResult) -> ExecResult {
        ExecResult {
            compile,
            run,
            time_ms: None,
            memory_bytes: None,
        }
    }

    pub fn is_successful(result: &ExecResult) -> bool {
        result.run.exit_code.unwrap_or(-1) == 0
    }

    pub fn extract_error_message(result: &ExecResult) -> Option<String> {
        if !is_successful(result) {
            if !result.run.stderr.is_empty() {
                Some(result.run.stderr.clone())
            } else if !result.run.stdout.is_empty() {
                Some(result.run.stdout.clone())
            } else {
                Some(format!(
                    "Process exited with code: {:?}",
                    result.run.exit_code
                ))
            }
        } else {
            None
        }
    }

    pub fn format_duration(ms: u64) -> String {
        if ms < 1000 {
            format!("{ms}ms")
        } else if ms < 60000 {
            format!("{:.1}s", ms as f64 / 1000.0)
        } else {
            let minutes = ms / 60000;
            let seconds = (ms % 60000) as f64 / 1000.0;
            format!("{minutes}m {seconds:.1}s")
        }
    }

    pub fn format_memory(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
        let mut size = bytes as f64;
        let mut unit_index = 0;

        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }

        if unit_index == 0 {
            format!("{} {}", bytes, UNITS[unit_index])
        } else {
            format!("{:.1} {}", size, UNITS[unit_index])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_environment() {
        let context =
            utils::create_test_context(LanguageKind::Javascript, "console.log('test');", "test.js");

        let env = ExecutionEnvironment::new(&context).unwrap();
        assert!(env.has_file("test.js"));
        assert_eq!(
            env.get_file_content("test.js").unwrap(),
            "console.log('test');"
        );

        env.cleanup();
    }

    #[test]
    fn test_output_capture() {
        let mut capture = OutputCapture::new();
        capture.add_stdout("line 1".to_string());
        capture.add_stdout("line 2".to_string());
        capture.add_stderr("error".to_string());

        assert_eq!(capture.get_stdout(), "line 1\nline 2");
        assert_eq!(capture.get_stderr(), "error");

        let stage_result = capture.to_stage_result(Some(0));
        assert_eq!(stage_result.exit_code, Some(0));
    }

    #[test]
    fn test_security_validation() {
        assert!(security::validate_filename("test.js").is_ok());
        assert!(security::validate_filename("../test.js").is_err());
        assert!(security::validate_filename("/etc/passwd").is_err());
        assert!(security::validate_filename("").is_err());

        assert_eq!(security::sanitize_env_name("TEST-VAR!"), "TESTVAR");

        assert!(security::validate_env_value("normal value").is_ok());
        assert!(security::validate_env_value("value\0with\0nulls").is_err());
    }

    #[test]
    fn test_utils() {
        let result = ExecResult {
            compile: None,
            run: StageResult {
                stdout: "output".to_string(),
                stderr: "".to_string(),
                exit_code: Some(0),
                signal: None,
            },
            time_ms: Some(1500),
            memory_bytes: Some(1024 * 1024),
        };

        assert!(utils::is_successful(&result));
        assert_eq!(utils::format_duration(1500), "1.5s");
        assert_eq!(utils::format_memory(1024 * 1024), "1.0 MB");
    }
}
