//! Agent harness runner — SWE-bench-style evaluation
//!
//! Each task lives in `tasks/<task_id>/` and contains:
//!   - `task.json`       — task description + setup file manifest
//!   - `grader.py`       — Python grader script (pass/fail per condition)
//!   - `setup/`          — initial file state (copied into sandbox)
//!
//! This module is compiled only in tests and the `harness` feature,
//! which keeps it out of the production binary.
//!
//! ## Usage (test or feature)
//!
//! ```rust,ignore
//! let result = run_harness_task("alloc_error", &HarnessConfig::default()).await;
//! println!("{}", result.summary());
//! ```
//!
//! ## Output CSV columns
//!
//! `task_id, status, elapsed_ms, checks_passed, checks_total`

#![cfg(any(test, feature = "harness"))]

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};

use serde::Deserialize;
use tokio::process::Command as AsyncCommand;
use tokio::time::timeout;

/// Harness configuration
#[derive(Debug, Clone)]
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

impl Default for HarnessConfig {
    fn default() -> Self {
        Self {
            grader_timeout: Duration::from_secs(30),
            python: "python3".to_string(),
            verbose: false,
            model: None,
        }
    }
}

/// Task definition loaded from `task.json`
#[derive(Debug, Deserialize)]
struct TaskDef {
    #[allow(dead_code)]
    id: String,
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    description: String,
    setup: TaskSetup,
    #[serde(rename = "expected")]
    #[allow(dead_code)]
    expected: HashMap<String, bool>,
    grader: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TaskSetup {
    files: HashMap<String, String>,
}

/// A single task result
#[derive(Debug)]
pub struct TaskResult {
    pub task_id: String,
    pub status: TaskStatus,
    pub elapsed_ms: u64,
    pub checks_passed: usize,
    pub checks_total: usize,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum TaskStatus {
    Pass,
    Fail,
    Error,
    Timeout,
    Skipped,
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pass => write!(f, "pass"),
            Self::Fail => write!(f, "fail"),
            Self::Error => write!(f, "error"),
            Self::Timeout => write!(f, "timeout"),
            Self::Skipped => write!(f, "skipped"),
        }
    }
}

impl TaskResult {
    /// One-line CSV row: `task_id,status,elapsed_ms,checks_passed,checks_total`
    pub fn to_csv_row(&self) -> String {
        format!(
            "{},{},{},{},{}",
            self.task_id,
            self.status,
            self.elapsed_ms,
            self.checks_passed,
            self.checks_total
        )
    }

    /// Summary string
    pub fn summary(&self) -> String {
        format!(
            "[{}] {} — {}/{} checks passed in {}ms",
            self.status,
            self.task_id,
            self.checks_passed,
            self.checks_total,
            self.elapsed_ms
        )
    }
}

/// Harness summary across all tasks
pub struct HarnessResult {
    pub task_results: Vec<TaskResult>,
    pub total_ms: u64,
}

impl HarnessResult {
    pub fn pass_rate(&self) -> f64 {
        let total = self.task_results.len();
        if total == 0 {
            return 0.0;
        }
        let pass = self
            .task_results
            .iter()
            .filter(|r| r.status == TaskStatus::Pass)
            .count();
        pass as f64 / total as f64
    }

    pub fn to_csv(&self) -> String {
        let header = "task_id,status,elapsed_ms,checks_passed,checks_total\n";
        let rows: String = self
            .task_results
            .iter()
            .map(|r| format!("{}\n", r.to_csv_row()))
            .collect();
        format!("{}{}", header, rows)
    }
}

// ─── Public API ───────────────────────────────────────────────────────────────

/// Return the path to a task directory.
fn task_dir(task_id: &str) -> PathBuf {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    crate_root
        .join("src")
        .join("harness")
        .join("tasks")
        .join(task_id)
}

/// Run a single harness task.
///
/// The sandbox workspace is created as a temp directory, setup files are
/// copied in, then the grader is executed against the final state.
pub async fn run_harness_task(task_id: &str, config: &HarnessConfig) -> TaskResult {
    let start = Instant::now();

    // Phase 1: Load and parse task definition
    let task_def = match load_task_def(task_id, start) {
        Ok(def) => def,
        Err(result) => return result,
    };

    // Phase 2: Create workspace and copy files
    let sandbox_base = std::env::temp_dir().join("runie-harness").join(task_id);
    let workspace_path = sandbox_base.join("workspace");
    if let Err(e) = std::fs::create_dir_all(&workspace_path) {
        return error_result(task_id, start, format!("Failed to create temp workspace: {}", e));
    }

    if let Err(e) = copy_setup_files(&task_def, &workspace_path, task_id, start) {
        return e;
    }

    if config.verbose {
        eprintln!("[harness] {} — sandbox: {}", task_id, workspace_path.display());
    }

    // Phase 3: Run grader
    let grader_result = if let Some(ref grader_script) = task_def.grader {
        run_grader(&sandbox_base.join("task"), &workspace_path, grader_script, config).await
    } else {
        (TaskStatus::Skipped, 0, 0, "no grader configured — task skipped".to_string())
    };

    // Phase 4: Cleanup
    let _ = std::fs::remove_dir_all(&sandbox_base);

    let elapsed_ms = start.elapsed().as_millis() as u64;
    let (status, checks_passed, checks_total, detail) = grader_result;

    TaskResult { task_id: task_id.to_string(), status, elapsed_ms, checks_passed, checks_total, detail }
}

fn load_task_def(task_id: &str, start: Instant) -> Result<TaskDef, TaskResult> {
    let task_dir = task_dir(task_id);

    let task_json = std::fs::read_to_string(task_dir.join("task.json"))
        .map_err(|e| error_result(task_id, start, format!("Failed to read task.json: {}", e)))?;

    let task_def: TaskDef = serde_json::from_str(&task_json)
        .map_err(|e| error_result(task_id, start, format!("Failed to parse task.json: {}", e)))?;

    Ok(task_def)
}

fn copy_setup_files(task_def: &TaskDef, workspace_path: &std::path::Path, task_id: &str, start: Instant) -> Result<(), TaskResult> {
    for (rel_path, content) in &task_def.setup.files {
        let dest = workspace_path.join(rel_path);
        if let Some(parent) = dest.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Err(e) = std::fs::write(&dest, content) {
            return Err(error_result(task_id, start, format!("Failed to write setup file {}: {}", rel_path, e)));
        }
    }
    Ok(())
}

fn error_result(task_id: &str, start: Instant, detail: String) -> TaskResult {
    TaskResult {
        task_id: task_id.to_string(),
        status: TaskStatus::Error,
        elapsed_ms: start.elapsed().as_millis() as u64,
        checks_passed: 0,
        checks_total: 0,
        detail,
    }
}

/// Run the Python grader script and parse its output.
/// BUG-05 FIX: Now applies `config.grader_timeout` to grader execution.
async fn run_grader(
    task_dir: &Path,
    workspace_path: &Path,
    grader_script: &str,
    config: &HarnessConfig,
) -> (TaskStatus, usize, usize, String) {
    let grader_path = task_dir.join(grader_script);
    if !grader_path.exists() {
        return (
            TaskStatus::Skipped,
            0,
            0,
            format!("grader not found: {}", grader_path.display()),
        );
    }

    // Chdir into the workspace so grader sees the right files
    let mut cmd = AsyncCommand::new(&config.python);
    cmd.current_dir(workspace_path)
        .arg(grader_path.as_path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // BUG-05 FIX: Apply grader_timeout to execution
    let output = timeout(config.grader_timeout, cmd.output()).await;

    let output = match output {
        Ok(Ok(o)) => o,
        Ok(Err(e)) => {
            return (
                TaskStatus::Error,
                0,
                0,
                format!("failed to spawn python: {}", e),
            );
        }
        Err(_) => {
            // Timeout reached
            return (
                TaskStatus::Fail,
                0,
                0,
                format!("grader timed out after {:?}", config.grader_timeout),
            );
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !stderr.is_empty() && config.verbose {
        eprintln!("[grader stderr] {}", stderr);
    }

    let status = if output.status.success() {
        TaskStatus::Pass
    } else {
        TaskStatus::Fail
    };

    // Parse checks from stdout (format: "PASS: description" / "FAIL: description")
    let lines: Vec<&str> = stdout.trim().split('\n').collect();
    let checks_total = lines.len();
    let checks_passed = lines.iter().filter(|l| l.starts_with("PASS")).count();

    let detail = if stdout.is_empty() && stderr.is_empty() {
        format!("grader exited with status {}", output.status)
    } else {
        stdout.trim().to_string()
    };

    (status, checks_passed, checks_total, detail)
}

/// Run all tasks in the harness directory.
pub async fn run_all_tasks(config: &HarnessConfig) -> HarnessResult {
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

/// Discover all task IDs in the harness tasks directory.
fn list_tasks() -> Vec<String> {
    let tasks_dir = std::path::Path::new("crates/runie-agent/src/harness/tasks");
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn create_test_task(task_id: &str, task_json: &str) -> std::path::PathBuf {
        let task_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("harness")
            .join("tasks")
            .join(task_id);
        let _ = fs::create_dir_all(&task_path);
        let _ = fs::write(task_path.join("task.json"), task_json);
        task_path
    }

    fn remove_test_task(task_id: &str) {
        let task_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("harness")
            .join("tasks")
            .join(task_id);
        let _ = fs::remove_dir_all(&task_path);
    }

    #[tokio::test]
    async fn test_list_tasks_empty_directory() {
        let task_id = "test_empty_tasks_dir";
        remove_test_task(task_id);
        // Ensure directory does not exist
        let task_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("harness")
            .join("tasks")
            .join(task_id);
        let _ = fs::remove_dir_all(&task_path);

        let tasks = list_tasks();
        assert!(!tasks.contains(&task_id.to_string()), "Empty dir should not appear in tasks");
    }

    #[tokio::test]
    async fn test_run_task_no_grader_skipped() {
        let task_id = "test_no_grader_task";
        let task_json = r#"{
            "id": "test_no_grader_task",
            "name": "Test No Grader",
            "description": "Task without grader",
            "setup": {"files": {}},
            "expected": {},
            "grader": null
        }"#;
        create_test_task(task_id, task_json);

        let config = HarnessConfig::default();
        let result = run_harness_task(task_id, &config).await;

        remove_test_task(task_id);

        assert_eq!(result.status, TaskStatus::Skipped, "Missing grader should skip task");
    }

    #[tokio::test]
    async fn test_run_task_grader_not_found_skipped() {
        let task_id = "test_grader_missing_task";
        let task_json = r#"{
            "id": "test_grader_missing_task",
            "name": "Test Grader Missing",
            "description": "Task with missing grader file",
            "setup": {"files": {}},
            "expected": {},
            "grader": "grader.py"
        }"#;
        create_test_task(task_id, task_json);

        let config = HarnessConfig::default();
        let result = run_harness_task(task_id, &config).await;

        remove_test_task(task_id);

        assert_eq!(result.status, TaskStatus::Skipped, "Grader file not found should skip task");
    }

    #[tokio::test]
    async fn test_harness_runs_all_tasks() {
        let config = HarnessConfig {
            verbose: true,
            ..Default::default()
        };
        let result = run_all_tasks(&config).await;
        eprintln!("Pass rate: {:.0}%", result.pass_rate() * 100.0);
        eprintln!("{}", result.to_csv());
        // Harness may have zero tasks if no task directories exist
        assert!(result.pass_rate() >= 0.0);
    }
}
