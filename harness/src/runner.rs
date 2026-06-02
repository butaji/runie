//! Harness task execution.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Harness configuration
#[derive(Debug, Clone, Default)]
pub struct HarnessConfig {
    /// Maximum time to wait for grader execution
    pub grader_timeout: Duration,
    /// Python interpreter to use for graders
    pub python: String,
    /// When true, print verbose task output
    pub verbose: bool,
    /// Model to use (informational only — harness is model-agnostic)
    pub model: Option<String>,
}

impl HarnessConfig {
    /// Create a new config with required fields
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the Python interpreter path
    pub fn python<P: Into<String>>(mut self, path: P) -> Self {
        self.python = path.into();
        self
    }

    /// Enable verbose output
    pub fn verbose(mut self) -> Self {
        self.verbose = true;
        self
    }

    /// Set the model name
    pub fn model<M: Into<String>>(mut self, name: M) -> Self {
        self.model = Some(name.into());
        self
    }
}

/// Task definition loaded from `task.json`
#[derive(Debug, serde::Deserialize)]
pub struct TaskDef {
    /// Unique task identifier
    pub id: String,
    /// Human-readable task name
    pub name: String,
    /// Detailed task description
    pub description: String,
    /// Initial file setup
    pub setup: TaskSetup,
    /// Expected outcomes (key = outcome name, value = expected to pass)
    pub expected: std::collections::HashMap<String, bool>,
    /// Grader script filename (optional)
    pub grader: Option<String>,
}

/// Files to create in the sandbox workspace
#[derive(Debug, serde::Deserialize)]
pub struct TaskSetup {
    pub files: std::collections::HashMap<String, String>,
}

/// Setup phase result
pub(super) struct TaskSandbox {
    pub(super) task_dir: PathBuf,
    pub(super) sandbox_base: PathBuf,
    pub(super) workspace_path: PathBuf,
}

/// Get the directory path for a task
pub(super) fn task_dir(task_id: &str) -> PathBuf {
    Path::new("tasks").join(task_id)
}

/// List all available task IDs
pub fn list_tasks() -> Vec<String> {
    let tasks_dir = Path::new("tasks");
    
    std::fs::read_dir(tasks_dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .filter_map(|e| e.file_name().into_string().ok())
                .collect()
        })
        .unwrap_or_default()
}

/// Run a single harness task
///
/// The sandbox workspace is created as a temp directory, setup files are
/// copied in, then the grader is executed against the final state.
pub async fn run_harness_task(task_id: &str, config: &HarnessConfig) -> super::TaskResult {
    let start = Instant::now();

    // Phase 1: Setup
    let setup = match task_setup(task_id) {
        Ok(s) => s,
        Err(result) => return result.with_elapsed(start),
    };

    // Phase 2: Execute
    let grader_result = run_grader(&setup.task_dir, &setup.workspace_path, config);

    // Phase 3: Cleanup
    let _ = std::fs::remove_dir_all(&setup.sandbox_base);

    let elapsed_ms = start.elapsed().as_millis() as u64;
    let (status, checks_passed, checks_total, detail) = grader_result;

    TaskResult {
        task_id: task_id.to_string(),
        status,
        elapsed_ms,
        checks_passed,
        checks_total,
        detail,
    }
}

/// Execute setup phase, returns setup info or error result
pub(super) fn task_setup(task_id: &str) -> Result<TaskSandbox, super::TaskResult> {
    use std::fs;

    let task_dir = task_dir(task_id);

    // Load task definition
    let task_json = fs::read_to_string(task_dir.join("task.json"))
        .map_err(|e| TaskResult {
            task_id: task_id.to_string(),
            status: super::TaskStatus::Error,
            elapsed_ms: 0,
            checks_passed: 0,
            checks_total: 0,
            detail: format!("Failed to read task.json: {}", e),
        })?;

    let _task_def: TaskDef = serde_json::from_str(&task_json)
        .map_err(|e| TaskResult {
            task_id: task_id.to_string(),
            status: super::TaskStatus::Error,
            elapsed_ms: 0,
            checks_passed: 0,
            checks_total: 0,
            detail: format!("Failed to parse task.json: {}", e),
        })?;

    // Create temp workspace
    let sandbox_base = std::env::temp_dir().join("runie-harness").join(task_id);
    let workspace_path = sandbox_base.join("workspace");
    fs::create_dir_all(&workspace_path).map_err(|e| TaskResult {
        task_id: task_id.to_string(),
        status: super::TaskStatus::Error,
        elapsed_ms: 0,
        checks_passed: 0,
        checks_total: 0,
        detail: format!("Failed to create temp workspace: {}", e),
    })?;

    Ok(TaskSandbox { task_dir, sandbox_base, workspace_path })
}

/// Run the Python grader script and parse its output
pub(super) fn run_grader(
    task_dir: &Path,
    workspace_path: &Path,
    config: &HarnessConfig,
) -> (super::TaskStatus, usize, usize, String) {
    let grader_path = task_dir.join("grader.py");
    if !grader_path.exists() {
        return (super::TaskStatus::Skipped, 0, 0, format!("grader not found: {}", grader_path.display()));
    }
    let output = spawn_grader(&grader_path, workspace_path, config);
    let (stdout, stderr, status) = match output {
        Ok((s, e, st)) => (s, e, st),
        Err(e) => return (super::TaskStatus::Error, 0, 0, format!("failed to spawn python: {}", e)),
    };
    if !stderr.is_empty() && config.verbose {
        eprintln!("[grader stderr] {}", stderr);
    }
    parse_grader_output(&stdout, &stderr, status.as_ref())
}

pub(super) fn spawn_grader(
    grader_path: &Path,
    workspace_path: &Path,
    config: &HarnessConfig,
) -> Result<(String, String, Option<std::process::ExitStatus>), std::io::Error> {
    let output = Command::new(&config.python)
        .current_dir(workspace_path)
        .arg(grader_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    Ok((stdout, stderr, Some(output.status)))
}

pub(super) fn parse_grader_output(
    stdout: &str,
    stderr: &str,
    exit_status: Option<&std::process::ExitStatus>,
) -> (super::TaskStatus, usize, usize, String) {
    use super::TaskStatus;
    let status = if exit_status.map_or(false, |s| s.success()) {
        TaskStatus::Pass
    } else {
        TaskStatus::Fail
    };
    let lines: Vec<&str> = stdout.trim().split('\n').collect();
    let checks_total = lines.len();
    let checks_passed = lines.iter().filter(|l| l.starts_with("PASS")).count();
    let detail = if stdout.is_empty() && stderr.is_empty() {
        format!("grader exited with status {:?}", exit_status)
    } else {
        stdout.trim().to_string()
    };
    (status, checks_passed, checks_total, detail)
}

/// Run all tasks in the harness directory
pub async fn run_all_tasks(config: &HarnessConfig) -> super::HarnessResult {
    use super::{HarnessResult, TaskResult};
    let start = Instant::now();

    let task_ids = list_tasks();
    let mut task_results = Vec::new();

    for task_id in task_ids {
        let result = run_harness_task(&task_id, config).await;
        if config.verbose {
            eprintln!("[harness] {}", result.summary());
        }
        task_results.push(result);
    }

    HarnessResult {
        task_results,
        total_ms: start.elapsed().as_millis() as u64,
    }
}
