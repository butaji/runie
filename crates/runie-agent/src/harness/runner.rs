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
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde::Deserialize;

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
    id: String,
    name: String,
    description: String,
    setup: TaskSetup,
    #[serde(rename = "expected")]
    expected: HashMap<String, bool>,
    grader: String,
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
    use std::fs;

    let start = Instant::now();
    let task_dir = task_dir(task_id);

    // Load task definition
    let task_json = match fs::read_to_string(task_dir.join("task.json")) {
        Ok(t) => t,
        Err(e) => {
            return TaskResult {
                task_id: task_id.to_string(),
                status: TaskStatus::Error,
                elapsed_ms: start.elapsed().as_millis() as u64,
                checks_passed: 0,
                checks_total: 0,
                detail: format!("Failed to read task.json: {}", e),
            };
        }
    };

    let task_def: TaskDef = match serde_json::from_str(&task_json) {
        Ok(t) => t,
        Err(e) => {
            return TaskResult {
                task_id: task_id.to_string(),
                status: TaskStatus::Error,
                elapsed_ms: start.elapsed().as_millis() as u64,
                checks_passed: 0,
                checks_total: 0,
                detail: format!("Failed to parse task.json: {}", e),
            };
        }
    };

    // Create temp workspace using std::env::temp_dir()
    let sandbox_base = std::env::temp_dir().join("runie-harness").join(task_id);
    let workspace_path = sandbox_base.join("workspace");
    if let Err(e) = fs::create_dir_all(&workspace_path) {
        return TaskResult {
            task_id: task_id.to_string(),
            status: TaskStatus::Error,
            elapsed_ms: start.elapsed().as_millis() as u64,
            checks_passed: 0,
            checks_total: 0,
            detail: format!("Failed to create temp workspace: {}", e),
        };
    }

    // Copy setup files into workspace
    for (rel_path, content) in &task_def.setup.files {
        let dest = workspace_path.join(rel_path);
        if let Some(parent) = dest.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Err(e) = fs::write(&dest, content) {
            return TaskResult {
                task_id: task_id.to_string(),
                status: TaskStatus::Error,
                elapsed_ms: start.elapsed().as_millis() as u64,
                checks_passed: 0,
                checks_total: 0,
                detail: format!("Failed to write setup file {}: {}", rel_path, e),
            };
        }
    }

    if config.verbose {
        eprintln!(
            "[harness] {} — sandbox: {}",
            task_id,
            workspace_path.display()
        );
    }

    // NOTE: The actual agent execution step is a placeholder.
    // In a full harness this would spawn the agent process with
    // the task description and workspace path.
    // For now we run the grader directly against the setup state,
    // which tests the harness infrastructure itself.

    // Run grader
    let grader_result =
        run_grader(&task_dir, &workspace_path, &task_def.grader, config);

    // Clean up sandbox
    let _ = fs::remove_dir_all(&sandbox_base);

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

/// Run the Python grader script and parse its output.
fn run_grader(
    task_dir: &Path,
    workspace_path: &Path,
    grader_script: &str,
    config: &HarnessConfig,
) -> (TaskStatus, usize, usize, String) {
    let grader_path = task_dir.join(grader_script);
    if !grader_path.exists() {
        return (
            TaskStatus::Error,
            0,
            0,
            format!("grader not found: {}", grader_path.display()),
        );
    }

    // Chdir into the workspace so grader sees the right files
    let output = Command::new(&config.python)
        .current_dir(workspace_path)
        .arg(grader_path.as_path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    let output = match output {
        Ok(o) => o,
        Err(e) => {
            return (
                TaskStatus::Error,
                0,
                0,
                format!("failed to spawn python: {}", e),
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

    let task_ids = ["alloc_error", "readme_maker", "param_struct"];

    let mut task_results = Vec::new();

    for task_id in task_ids {
        let result = run_harness_task(task_id, config).await;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_harness_runs_all_tasks() {
        let config = HarnessConfig {
            verbose: true,
            ..Default::default()
        };
        let result = run_all_tasks(&config).await;
        eprintln!("Pass rate: {:.0}%", result.pass_rate() * 100.0);
        eprintln!("{}", result.to_csv());
        assert!(!result.task_results.is_empty());
    }
}
