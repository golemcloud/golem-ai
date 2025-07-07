use crate::error::stream;
use crate::types::*;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[cfg(target_arch = "wasm32")]
use crate::exports::golem::exec::exec_stream;

#[derive(Debug, Clone)]
pub struct ExecStream {
    inner: Arc<Mutex<ExecStreamInner>>,
    stdin_sender: Option<Arc<Mutex<std::sync::mpsc::Sender<Vec<u8>>>>>,
}

#[derive(Debug)]
struct ExecStreamInner {
    events: VecDeque<ExecEvent>,
    finished: bool,
    error: Option<Error>,
    stdin_buffer: VecDeque<Vec<u8>>,
    supports_bidirectional: bool,
}

impl Default for ExecStream {
    fn default() -> Self {
        Self::new()
    }
}

impl ExecStream {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(ExecStreamInner {
                events: VecDeque::new(),
                finished: false,
                error: None,
                stdin_buffer: VecDeque::new(),
                supports_bidirectional: false,
            })),
            stdin_sender: None,
        }
    }

    pub fn new_bidirectional() -> (Self, std::sync::mpsc::Receiver<Vec<u8>>) {
        let (sender, receiver) = std::sync::mpsc::channel();
        let stream = Self {
            inner: Arc::new(Mutex::new(ExecStreamInner {
                events: VecDeque::new(),
                finished: false,
                error: None,
                stdin_buffer: VecDeque::new(),
                supports_bidirectional: true,
            })),
            stdin_sender: Some(Arc::new(Mutex::new(sender))),
        };
        (stream, receiver)
    }

    pub fn push_event(&self, event: ExecEvent) -> crate::error::ExecResult<()> {
        let mut inner = self.inner.lock().map_err(|_| stream::mutex_lock_failed())?;

        match &event {
            ExecEvent::Finished(_) => {
                inner.finished = true;
            }
            ExecEvent::Failed(error) => {
                inner.error = Some(error.clone());
                inner.finished = true;
            }
            _ => {}
        }

        inner.events.push_back(event);
        Ok(())
    }

    pub fn push_stdout(&self, data: Vec<u8>) -> crate::error::ExecResult<()> {
        if !data.is_empty() {
            self.push_event(ExecEvent::StdoutChunk(data))
        } else {
            Ok(())
        }
    }

    pub fn push_stderr(&self, data: Vec<u8>) -> crate::error::ExecResult<()> {
        if !data.is_empty() {
            self.push_event(ExecEvent::StderrChunk(data))
        } else {
            Ok(())
        }
    }

    pub fn push_finished(&self, result: ExecResult) -> crate::error::ExecResult<()> {
        self.push_event(ExecEvent::Finished(result))
    }

    pub fn push_failed(&self, error: Error) -> crate::error::ExecResult<()> {
        self.push_event(ExecEvent::Failed(error))
    }

    pub fn get_next(&self) -> crate::error::ExecResult<Option<ExecEvent>> {
        let mut inner = self.inner.lock().map_err(|_| stream::mutex_lock_failed())?;
        Ok(inner.events.pop_front())
    }

    pub fn blocking_get_next(
        &self,
        timeout: Duration,
    ) -> crate::error::ExecResult<Option<ExecEvent>> {
        let start = std::time::Instant::now();

        loop {
            if let Some(event) = self.get_next()? {
                return Ok(Some(event));
            }

            let inner = self.inner.lock().map_err(|_| stream::mutex_lock_failed())?;
            if inner.finished {
                return Ok(None);
            }
            drop(inner);

            if start.elapsed() >= timeout {
                return Ok(None);
            }

            std::thread::sleep(Duration::from_millis(10));
        }
    }

    pub fn is_finished(&self) -> crate::error::ExecResult<bool> {
        let inner = self.inner.lock().map_err(|_| stream::mutex_lock_failed())?;
        Ok(inner.finished)
    }

    pub fn has_error(&self) -> crate::error::ExecResult<bool> {
        let inner = self.inner.lock().map_err(|_| stream::mutex_lock_failed())?;
        Ok(inner.error.is_some())
    }

    pub fn get_error(&self) -> crate::error::ExecResult<Option<Error>> {
        let inner = self.inner.lock().map_err(|_| stream::mutex_lock_failed())?;
        Ok(inner.error.clone())
    }

    pub fn remaining_events(&self) -> crate::error::ExecResult<usize> {
        let inner = self.inner.lock().map_err(|_| stream::mutex_lock_failed())?;
        Ok(inner.events.len())
    }

    pub fn drain_events(&self) -> crate::error::ExecResult<Vec<ExecEvent>> {
        let mut inner = self.inner.lock().map_err(|_| stream::mutex_lock_failed())?;
        Ok(inner.events.drain(..).collect())
    }

    pub fn send_stdin(&self, data: Vec<u8>) -> crate::error::ExecResult<()> {
        if let Some(ref sender) = self.stdin_sender {
            let sender = sender.lock().map_err(|_| stream::mutex_lock_failed())?;
            sender
                .send(data)
                .map_err(|_| Error::Internal("Failed to send data to stdin".to_string()))?;
            Ok(())
        } else {
            Err(Error::Internal(
                "Bidirectional I/O not supported for this stream".to_string(),
            ))
        }
    }

    pub fn supports_bidirectional(&self) -> crate::error::ExecResult<bool> {
        let inner = self.inner.lock().map_err(|_| stream::mutex_lock_failed())?;
        Ok(inner.supports_bidirectional)
    }

    pub fn buffer_stdin(&self, data: Vec<u8>) -> crate::error::ExecResult<()> {
        let mut inner = self.inner.lock().map_err(|_| stream::mutex_lock_failed())?;
        inner.stdin_buffer.push_back(data);
        Ok(())
    }

    pub fn get_stdin_buffer(&self) -> crate::error::ExecResult<Option<Vec<u8>>> {
        let mut inner = self.inner.lock().map_err(|_| stream::mutex_lock_failed())?;
        Ok(inner.stdin_buffer.pop_front())
    }
}

pub struct ExecStreamBuilder {
    buffer_size: Option<usize>,
    timeout: Option<Duration>,
}

impl Default for ExecStreamBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ExecStreamBuilder {
    pub fn new() -> Self {
        Self {
            buffer_size: None,
            timeout: None,
        }
    }

    pub fn with_buffer_size(mut self, size: usize) -> Self {
        self.buffer_size = Some(size);
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn build(self) -> ExecStream {
        ExecStream::new()
    }
}

pub mod utils {
    use super::*;
    use std::io::{BufRead, BufReader};
    use std::process::Child;
    use std::thread;

    pub fn stream_from_process(mut process: Child, stream: ExecStream) -> Result<(), Error> {
        let stdout = process
            .stdout
            .take()
            .ok_or_else(stream::stdout_capture_failed)?;
        let stderr = process
            .stderr
            .take()
            .ok_or_else(stream::stderr_capture_failed)?;

        let stream_stdout = stream.clone();
        let stream_stderr = stream.clone();
        let stream_result = stream.clone();

        let stdout_handle = thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                match line {
                    Ok(line) => {
                        let mut data = line.into_bytes();
                        data.push(b'\n');
                        let _ = stream_stdout.push_stdout(data);
                    }
                    Err(_) => break,
                }
            }
        });

        let stderr_handle = thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                match line {
                    Ok(line) => {
                        let mut data = line.into_bytes();
                        data.push(b'\n');
                        let _ = stream_stderr.push_stderr(data);
                    }
                    Err(_) => break,
                }
            }
        });

        thread::spawn(move || {
            match process.wait() {
                Ok(status) => {
                    let exit_code = status.code();
                    let stage_result = StageResult {
                        stdout: String::new(),
                        stderr: String::new(),
                        exit_code,
                        signal: None,
                    };

                    let result = ExecResult {
                        compile: None,
                        run: stage_result,
                        time_ms: None,
                        memory_bytes: None,
                    };

                    let _ = stream_result.push_finished(result); // Ignore errors in background thread
                }
                Err(e) => {
                    let _ = stream_result.push_failed(stream::process_error(e.to_string()));
                    // Ignore errors in background thread
                }
            }
        });

        let _ = stdout_handle.join();
        let _ = stderr_handle.join();

        Ok(())
    }

    pub fn collect_stream_result(
        stream: &ExecStream,
        timeout: Duration,
    ) -> Result<ExecResult, Error> {
        let start = std::time::Instant::now();
        let mut stdout_chunks = Vec::new();
        let mut stderr_chunks = Vec::new();

        loop {
            if start.elapsed() >= timeout {
                return Err(Error::Timeout);
            }

            match stream.get_next()? {
                Some(ExecEvent::StdoutChunk(data)) => {
                    stdout_chunks.extend_from_slice(&data);
                }
                Some(ExecEvent::StderrChunk(data)) => {
                    stderr_chunks.extend_from_slice(&data);
                }
                Some(ExecEvent::Finished(mut result)) => {
                    if !stdout_chunks.is_empty() {
                        result.run.stdout = String::from_utf8_lossy(&stdout_chunks).to_string();
                    }
                    if !stderr_chunks.is_empty() {
                        result.run.stderr = String::from_utf8_lossy(&stderr_chunks).to_string();
                    }
                    return Ok(result);
                }
                Some(ExecEvent::Failed(error)) => {
                    return Err(error);
                }
                None => {
                    if stream.is_finished()? {
                        return Err(stream::stream_finished_without_result());
                    }
                    thread::sleep(Duration::from_millis(10));
                }
            }
        }
    }

    pub fn create_test_stream(events: Vec<ExecEvent>) -> crate::error::ExecResult<ExecStream> {
        let stream = ExecStream::new();
        for event in events {
            stream.push_event(event)?;
        }
        Ok(stream)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exec_stream_basic() {
        let stream = ExecStream::new();

        assert!(stream.get_next().unwrap().is_none());
        assert!(!stream.is_finished().unwrap());

        stream.push_stdout(b"Hello".to_vec()).unwrap();
        stream.push_stderr(b"Error".to_vec()).unwrap();

        match stream.get_next().unwrap() {
            Some(ExecEvent::StdoutChunk(data)) => {
                assert_eq!(data, b"Hello");
            }
            _ => panic!("Expected stdout chunk"),
        }

        match stream.get_next().unwrap() {
            Some(ExecEvent::StderrChunk(data)) => {
                assert_eq!(data, b"Error");
            }
            _ => panic!("Expected stderr chunk"),
        }

        let result = ExecResult {
            compile: None,
            run: StageResult {
                stdout: "Hello".to_string(),
                stderr: "Error".to_string(),
                exit_code: Some(0),
                signal: None,
            },
            time_ms: Some(100),
            memory_bytes: None,
        };

        stream.push_finished(result.clone()).unwrap();
        assert!(stream.is_finished().unwrap());

        match stream.get_next().unwrap() {
            Some(ExecEvent::Finished(r)) => {
                assert_eq!(r.run.exit_code, Some(0));
            }
            _ => panic!("Expected finished event"),
        }
    }

    #[test]
    fn test_stream_builder() {
        let stream = ExecStreamBuilder::new()
            .with_buffer_size(1024)
            .with_timeout(Duration::from_secs(30))
            .build();

        assert!(!stream.is_finished().unwrap());
        assert!(!stream.has_error().unwrap());
    }

    #[test]
    fn test_stream_error_handling() {
        let stream = ExecStream::new();

        let error = Error::Timeout;
        stream.push_failed(error.clone()).unwrap();

        assert!(stream.is_finished().unwrap());
        assert!(stream.has_error().unwrap());

        match stream.get_next().unwrap() {
            Some(ExecEvent::Failed(e)) => {
                assert!(matches!(e, Error::Timeout));
            }
            _ => panic!("Expected failed event"),
        }
    }
}

#[cfg(target_arch = "wasm32")]
impl From<ExecStream> for exec_stream::ExecStream {
    fn from(stream: ExecStream) -> Self {
        use std::sync::{Arc, Mutex};

        let events = {
            let inner = stream.inner.lock().unwrap();
            inner.events.clone()
        };
        let stream_resource = Arc::new(Mutex::new(StreamResource {
            events,
            position: std::cell::Cell::new(0),
            is_closed: false,
        }));

        let handle = STREAM_REGISTRY.with(|registry| {
            let mut registry = registry.borrow_mut();
            let handle = registry.next_handle;
            registry.next_handle += 1;
            registry
                .streams
                .insert(handle, Arc::clone(&stream_resource));
            handle
        });

        unsafe { exec_stream::ExecStream::from_handle(handle) }
    }
}

thread_local! {
    static STREAM_REGISTRY: std::cell::RefCell<StreamRegistry> = std::cell::RefCell::new(StreamRegistry::new());
}

#[allow(dead_code)]
struct StreamRegistry {
    streams: std::collections::HashMap<u32, Arc<Mutex<StreamResource>>>,
    next_handle: u32,
}

impl StreamRegistry {
    #[allow(dead_code)]
    fn new() -> Self {
        Self {
            streams: std::collections::HashMap::new(),
            next_handle: 2, // Start from 2 to avoid 0 and 1 which might be special
        }
    }

    #[allow(dead_code)]
    fn get_stream(&self, handle: u32) -> Option<Arc<Mutex<StreamResource>>> {
        self.streams.get(&handle).cloned()
    }

    #[allow(dead_code)]
    fn remove_stream(&mut self, handle: u32) -> bool {
        self.streams.remove(&handle).is_some()
    }
}

#[allow(dead_code)]
pub struct StreamResource {
    events: VecDeque<ExecEvent>,
    position: std::cell::Cell<usize>,
    is_closed: bool,
}

#[cfg(target_arch = "wasm32")]
impl exec_stream::GuestExecStream for StreamResource {
    fn get_next(&self) -> Option<ExecEvent> {
        if self.is_closed {
            return None;
        }

        let current_position = self.position.get();
        if current_position < self.events.len() {
            let event = self.events[current_position].clone();
            self.position.set(current_position + 1);
            Some(event.into())
        } else {
            None
        }
    }

    fn blocking_get_next(&self) -> Option<ExecEvent> {
        // For now, just delegate to get_next since we don't have async support in this context
        self.get_next()
    }
}

#[cfg(target_arch = "wasm32")]
impl exec_stream::Guest for crate::ExecComponent {
    type ExecStream = StreamResource;
}
