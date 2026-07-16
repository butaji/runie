//! Unified terminal backend trait (from Grok Build)

use async_trait::async_trait;
use std::path::PathBuf;
use std::pin::Pin;
use std::time::{Duration, Instant};

/// Terminal execution request
#[derive(Debug, Clone)]
pub struct TerminalRunRequest {
    /// The command to execute
    pub command: String,
    /// Working directory
    pub cwd: PathBuf,
    /// Timeout in milliseconds
    pub timeout_ms: Option<u64>,
    /// Notification handle for streaming output
    pub notification_handle: ToolNotificationHandle,
    /// Unique identifier for this tool call
    pub tool_call_id: String,
    /// Path to write output file
    pub output_file: PathBuf,
    /// Run in background
    pub background: bool,
    /// Environment variables (extends current env)
    pub env: Vec<(String, String)>,
    /// User ID for permission tracking
    pub user_id: Option<String>,
    /// Session ID for tracking
    pub session_id: String,
}

impl TerminalRunRequest {
    /// Create a new request
    pub fn new(
        command: String,
        cwd: PathBuf,
        tool_call_id: String,
        output_file: PathBuf,
        session_id: String,
    ) -> Self {
        Self {
            command,
            cwd,
            timeout_ms: None,
            notification_handle: ToolNotificationHandle::default(),
            tool_call_id,
            output_file,
            background: false,
            env: Vec::new(),
            user_id: None,
            session_id,
        }
    }

    /// Set timeout
    pub fn with_timeout(mut self, ms: u64) -> Self {
        self.timeout_ms = Some(ms);
        self
    }

    /// Set as background task
    pub fn background(mut self) -> Self {
        self.background = true;
        self
    }

    /// Add environment variable
    pub fn with_env(mut self, key: &str, value: &str) -> Self {
        self.env.push((key.to_string(), value.to_string()));
        self
    }
}

/// Terminal execution result
#[derive(Debug, Clone)]
pub struct TerminalRunResult {
    /// Exit code
    pub exit_code: i32,
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Signal that killed the process (if any)
    pub signal: Option<i32>,
}

impl TerminalRunResult {
    /// Check if the command succeeded
    pub fn is_success(&self) -> bool {
        self.exit_code == 0 && self.signal.is_none()
    }

    /// Get combined output
    pub fn combined(&self) -> String {
        if self.stderr.is_empty() {
            self.stdout.clone()
        } else {
            format!("{}\n{}", self.stdout, self.stderr)
        }
    }

    /// Create from process output
    pub fn from_process(
        output: std::process::Output,
        start: Instant,
    ) -> Self {
        let duration_ms = start.elapsed().as_millis() as u64;
        let signal = output.status.signal();

        Self {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            duration_ms,
            signal,
        }
    }
}

/// Handle to a background task
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackgroundHandle {
    /// Unique task identifier
    pub task_id: String,
    /// When the task was started
    pub start_time: Instant,
}

impl BackgroundHandle {
    /// Create a new handle
    pub fn new(task_id: String) -> Self {
        Self {
            task_id,
            start_time: Instant::now(),
        }
    }
}

/// Snapshot of a running task
#[derive(Debug, Clone)]
pub struct TaskSnapshot {
    /// Task identifier
    pub task_id: String,
    /// Command being run
    pub command: String,
    /// Current status
    pub status: TaskStatus,
    /// When the task started
    pub start_time: Instant,
    /// Current working directory
    pub cwd: PathBuf,
}

impl TaskSnapshot {
    /// Check if task is still running
    pub fn is_running(&self) -> bool {
        matches!(self.status, TaskStatus::Running)
    }
}

/// Task status
#[derive(Debug, Clone)]
pub enum TaskStatus {
    /// Task is running
    Running,
    /// Task completed with exit code
    Completed(i32),
    /// Task failed with error message
    Failed(String),
    /// Task was killed
    Killed,
    /// Task timed out
    TimedOut,
}

impl TaskStatus {
    /// Check if task is finished
    pub fn is_finished(&self) -> bool {
        !matches!(self, TaskStatus::Running)
    }

    /// Get exit code if completed
    pub fn exit_code(&self) -> Option<i32> {
        match self {
            TaskStatus::Completed(code) => Some(*code),
            _ => None,
        }
    }
}

/// Outcome of killing a task
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KillOutcome {
    /// Task was killed successfully
    Killed,
    /// Task had already finished
    AlreadyFinished,
    /// Task was not found
    NotFound,
}

impl KillOutcome {
    pub fn is_success(&self) -> bool {
        matches!(self, KillOutcome::Killed)
    }
}

/// Notification handle for streaming output
#[derive(Debug, Clone, Default)]
pub struct ToolNotificationHandle {
    /// Channel sender for output notifications
    channel: Option<std::sync::mpsc::Sender<OutputEvent>>,
}

impl ToolNotificationHandle {
    /// Create a no-op handle
    pub fn noop() -> Self {
        Self { channel: None }
    }

    /// Create a handle with a channel
    pub fn with_channel(sender: std::sync::mpsc::Sender<OutputEvent>) -> Self {
        Self { channel: Some(sender) }
    }

    /// Send an output event
    pub fn send(&self, event: OutputEvent) -> bool {
        match &self.channel {
            Some(sender) => sender.send(event).is_ok(),
            None => false,
        }
    }

    /// Check if this is a noop handle
    pub fn is_noop(&self) -> bool {
        self.channel.is_none()
    }
}

/// Output event for streaming
#[derive(Debug, Clone)]
pub enum OutputEvent {
    /// Stdout data
    Stdout(String),
    /// Stderr data
    Stderr(String),
    /// Exit event
    Exit { code: i32, signal: Option<i32> },
    /// Error event
    Error(String),
}

impl OutputEvent {
    /// Get the event type name
    pub fn event_type(&self) -> &'static str {
        match self {
            OutputEvent::Stdout(_) => "stdout",
            OutputEvent::Stderr(_) => "stderr",
            OutputEvent::Exit { .. } => "exit",
            OutputEvent::Error(_) => "error",
        }
    }

    /// Get the content as string
    pub fn content(&self) -> &str {
        match self {
            OutputEvent::Stdout(s) => s,
            OutputEvent::Stderr(s) => s,
            OutputEvent::Exit { .. } => "",
            OutputEvent::Error(s) => s,
        }
    }
}

/// Unified trait for terminal execution
#[async_trait]
pub trait TerminalBackend: Send + Sync {
    /// Run a command and wait for completion
    async fn run(&self, request: TerminalRunRequest) -> anyhow::Result<TerminalRunResult>;

    /// Run a command in the background
    async fn run_background(
        &self,
        request: TerminalRunRequest,
    ) -> anyhow::Result<BackgroundHandle>;

    /// Get the current state of a background task
    async fn get_task(&self, task_id: &str) -> Option<TaskSnapshot>;

    /// Kill a running task
    async fn kill_task(&self, task_id: &str) -> KillOutcome;

    /// Kill all background tasks owned by a session
    async fn kill_all_background_tasks_by_owner(&self, owner_session_id: &str);

    /// Reparent notifications to a new owner
    async fn reparent_notifications(
        &self,
        old_owner: &str,
        new_owner: &str,
        handle: ToolNotificationHandle,
    );

    /// List all running tasks
    async fn list_tasks(&self) -> Vec<TaskSnapshot>;

    /// Wait for a task to complete
    async fn wait_for_task(
        &self,
        task_id: &str,
        timeout: Duration,
    ) -> anyhow::Result<TaskStatus>;
}

/// Default implementation using std::process
#[derive(Debug, Clone, Default)]
pub struct DefaultTerminalBackend;

impl DefaultTerminalBackend {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl TerminalBackend for DefaultTerminalBackend {
    async fn run(&self, request: TerminalRunRequest) -> anyhow::Result<TerminalRunResult> {
        let start = Instant::now();

        // Build the command
        let mut cmd = if cfg!(target_os = "windows") {
            let mut c = std::process::Command::new("cmd");
            c.args(["/C", &request.command]);
            c
        } else {
            let mut c = std::process::Command::new("sh");
            c.args(["-c", &request.command]);
            c
        };

        cmd.current_dir(&request.cwd);

        // Add environment
        for (key, value) in &request.env {
            cmd.env(key, value);
        }

        // Set timeout if specified
        if let Some(timeout_ms) = request.timeout_ms {
            cmd.arg(format!("timeout {} ", timeout_ms / 1000));
        }

        let output = tokio::task::spawn_blocking(move || {
            cmd.output()
        })
        .await??;

        Ok(TerminalRunResult::from_process(output, start))
    }

    async fn run_background(
        &self,
        request: TerminalRunRequest,
    ) -> anyhow::Result<BackgroundHandle> {
        let task_id = uuid::Uuid::new_v4().to_string();

        // Spawn the process
        let mut cmd = if cfg!(target_os = "windows") {
            let mut c = std::process::Command::new("cmd");
            c.args(["/C", &request.command]);
            c
        } else {
            let mut c = std::process::Command::new("sh");
            c.args(["-c", &request.command]);
            c
        };

        cmd.current_dir(&request.cwd);
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        for (key, value) in &request.env {
            cmd.env(key, value);
        }

        let _child = cmd.spawn()?;

        // In a real implementation, we would track the child process
        Ok(BackgroundHandle::new(task_id))
    }

    async fn get_task(&self, _task_id: &str) -> Option<TaskSnapshot> {
        None
    }

    async fn kill_task(&self, _task_id: &str) -> KillOutcome {
        KillOutcome::NotFound
    }

    async fn kill_all_background_tasks_by_owner(&self, _owner_session_id: &str) {
        // No-op for default implementation
    }

    async fn reparent_notifications(
        &self,
        _old_owner: &str,
        _new_owner: &str,
        _handle: ToolNotificationHandle,
    ) {
        // No-op for default implementation
    }

    async fn list_tasks(&self) -> Vec<TaskSnapshot> {
        Vec::new()
    }

    async fn wait_for_task(
        &self,
        _task_id: &str,
        _timeout: Duration,
    ) -> anyhow::Result<TaskStatus> {
        Ok(TaskStatus::Completed(0))
    }
}

/// Builder for terminal requests
pub struct TerminalRequestBuilder {
    command: String,
    cwd: PathBuf,
    timeout_ms: Option<u64>,
    tool_call_id: String,
    output_file: PathBuf,
    background: bool,
    env: Vec<(String, String)>,
    session_id: String,
}

impl TerminalRequestBuilder {
    /// Create a new builder
    pub fn new(
        command: String,
        tool_call_id: String,
        session_id: String,
    ) -> Self {
        Self {
            command,
            cwd: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            timeout_ms: None,
            tool_call_id,
            output_file: PathBuf::from("/dev/null"),
            background: false,
            env: Vec::new(),
            session_id,
        }
    }

    /// Set working directory
    pub fn cwd(mut self, path: impl Into<PathBuf>) -> Self {
        self.cwd = path.into();
        self
    }

    /// Set timeout
    pub fn timeout(mut self, ms: u64) -> Self {
        self.timeout_ms = Some(ms);
        self
    }

    /// Set output file
    pub fn output_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.output_file = path.into();
        self
    }

    /// Set background mode
    pub fn background(mut self) -> Self {
        self.background = true;
        self
    }

    /// Add environment variable
    pub fn env(mut self, key: &str, value: &str) -> Self {
        self.env.push((key.to_string(), value.to_string()));
        self
    }

    /// Build the request
    pub fn build(self) -> TerminalRunRequest {
        TerminalRunRequest {
            command: self.command,
            cwd: self.cwd,
            timeout_ms: self.timeout_ms,
            notification_handle: ToolNotificationHandle::default(),
            tool_call_id: self.tool_call_id,
            output_file: self.output_file,
            background: self.background,
            env: self.env,
            user_id: None,
            session_id: self.session_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_run_request() {
        let req = TerminalRunRequest::new(
            "echo hello".to_string(),
            PathBuf::from("/tmp"),
            "test-1".to_string(),
            PathBuf::from("/tmp/out.txt"),
            "session-1".to_string(),
        );

        assert_eq!(req.command, "echo hello");
        assert!(!req.background);
    }

    #[test]
    fn test_terminal_run_result() {
        let result = TerminalRunResult {
            exit_code: 0,
            stdout: "hello".to_string(),
            stderr: "".to_string(),
            duration_ms: 100,
            signal: None,
        };

        assert!(result.is_success());
        assert_eq!(result.combined(), "hello");
    }

    #[test]
    fn test_task_status() {
        assert!(!TaskStatus::Running.is_finished());
        assert!(TaskStatus::Completed(0).is_finished());
        assert!(TaskStatus::Failed("error".to_string()).is_finished());

        assert_eq!(TaskStatus::Completed(42).exit_code(), Some(42));
        assert_eq!(TaskStatus::Running.exit_code(), None);
    }

    #[test]
    fn test_kill_outcome() {
        assert!(KillOutcome::Killed.is_success());
        assert!(!KillOutcome::AlreadyFinished.is_success());
        assert!(!KillOutcome::NotFound.is_success());
    }

    #[test]
    fn test_output_event() {
        let stdout = OutputEvent::Stdout("hello".to_string());
        assert_eq!(stdout.event_type(), "stdout");
        assert_eq!(stdout.content(), "hello");

        let exit = OutputEvent::Exit { code: 0, signal: None };
        assert_eq!(exit.event_type(), "exit");
    }

    #[test]
    fn test_background_handle() {
        let handle = BackgroundHandle::new("task-123".to_string());
        assert_eq!(handle.task_id, "task-123");
    }

    #[test]
    fn test_request_builder() {
        let req = TerminalRequestBuilder::new(
            "ls".to_string(),
            "call-1".to_string(),
            "session-1".to_string(),
        )
        .cwd("/home")
        .timeout(5000)
        .background()
        .env("FOO", "bar")
        .build();

        assert_eq!(req.command, "ls");
        assert_eq!(req.cwd, PathBuf::from("/home"));
        assert_eq!(req.timeout_ms, Some(5000));
        assert!(req.background);
        assert_eq!(req.env, vec![("FOO".to_string(), "bar".to_string())]);
    }
}
