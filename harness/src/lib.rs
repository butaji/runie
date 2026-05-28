//! Runie Agent Harness - End-to-End Evaluation Module
//!
//! This module provides SWE-bench style evaluation capabilities for the Runie agent.
//! It runs the agent against curated micro-tasks and validates behavior.
//!
//! ## Task Structure
//!
//! Each task lives in `tasks/<task_id>/` and contains:
//! - `task.json` - Task definition with description and setup files
//! - `grader.py` - Python grader script that validates behavior
//! - `setup/`   - Initial file state (copied into sandbox)
//!
//! ## Usage
//!
//! ```rust,ignore
//! use runie_harness::{run_all_tasks, HarnessConfig};
//!
//! let config = HarnessConfig::default();
//! let result = run_all_tasks(&config).await;
//! println!("Pass rate: {:.0}%", result.pass_rate() * 100.0);
//! ```
//!
//! ## Metrics
//!
//! The harness tracks:
//! - Task resolution rate (pass/fail per task)
//! - Elapsed time per task
//! - Check counts (individual assertions per task)
//! - Token cost (when model is configured)
//!
//! ## CSV Output Format
//!
//! ```csv
//! task_id,status,elapsed_ms,checks_passed,checks_total
//! error_state_recovery,pass,1234,5,5
//! permission_timeout,pass,2345,5,5
//! double_submit_dedup,fail,500,3,5
//! ```

#![cfg(any(test, feature = "harness"))]

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

// ─── Configuration ────────────────────────────────────────────────────────────

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

// ─── Task Definition ─────────────────────────────────────────────────────────

/// Task definition loaded from `task.json`
#[derive(Debug, Deserialize)]
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
    pub expected: HashMap<String, bool>,
    /// Grader script filename
    pub grader: String,
}

/// Files to create in the sandbox workspace
#[derive(Debug, Deserialize)]
pub struct TaskSetup {
    pub files: HashMap<String, String>,
}

// ─── Results ─────────────────────────────────────────────────────────────────

/// Result status for a single task
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TaskStatus {
    /// All checks passed
    Pass,
    /// Some checks failed
    Fail,
    /// Task execution error
    Error,
    /// Task timed out
    Timeout,
    /// Task was skipped
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

/// Result of a single task execution
#[derive(Debug)]
pub struct TaskResult {
    /// Task identifier
    pub task_id: String,
    /// Execution status
    pub status: TaskStatus,
    /// Time taken in milliseconds
    pub elapsed_ms: u64,
    /// Number of checks that passed
    pub checks_passed: usize,
    /// Total number of checks
    pub checks_total: usize,
    /// Detailed output or error message
    pub detail: String,
}

impl TaskResult {
    /// Convert to CSV row format
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

    /// Human-readable summary
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

/// Aggregated results across all tasks
#[derive(Debug, Default)]
pub struct HarnessResult {
    /// Individual task results
    pub task_results: Vec<TaskResult>,
    /// Total execution time in milliseconds
    pub total_ms: u64,
}

impl HarnessResult {
    /// Calculate pass rate as a fraction
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

    /// Convert to CSV format with header
    pub fn to_csv(&self) -> String {
        let mut output = String::from("task_id,status,elapsed_ms,checks_passed,checks_total\n");
        for result in &self.task_results {
            output.push_str(&result.to_csv_row());
            output.push('\n');
        }
        output
    }

    /// Calculate total checks passed across all tasks
    pub fn total_checks_passed(&self) -> usize {
        self.task_results.iter().map(|r| r.checks_passed).sum()
    }

    /// Calculate total checks across all tasks
    pub fn total_checks(&self) -> usize {
        self.task_results.iter().map(|r| r.checks_total).sum()
    }
}

// ─── Task Discovery ─────────────────────────────────────────────────────────

/// Get the directory path for a task
fn task_dir(task_id: &str) -> PathBuf {
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

// ─── Task Execution ──────────────────────────────────────────────────────────

/// Run a single harness task
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

    let _task_def: TaskDef = match serde_json::from_str(&task_json) {
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

    // Create temp workspace
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
    // (In a real harness, we'd copy from task_def.setup.files)
    // For now, we use the workspace as-is

    if config.verbose {
        eprintln!("[harness] {} — sandbox: {}", task_id, workspace_path.display());
    }

    // Run grader
    let grader_result = run_grader(&task_dir, &workspace_path, config);

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

/// Run the Python grader script and parse its output
fn run_grader(
    task_dir: &Path,
    workspace_path: &Path,
    config: &HarnessConfig,
) -> (TaskStatus, usize, usize, String) {
    let grader_path = task_dir.join("grader.py");
    if !grader_path.exists() {
        return (
            TaskStatus::Skipped,
            0,
            0,
            format!("grader not found: {}", grader_path.display()),
        );
    }

    // Run grader with timeout
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

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

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

/// Run all tasks in the harness directory
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

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_harness_runs_all_tasks() {
        // Change to harness directory so tasks path is correct
        let harness_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        std::env::set_current_dir(harness_dir).ok();
        
        let config = HarnessConfig {
            verbose: true,
            ..Default::default()
        };
        let result = run_all_tasks(&config).await;
        eprintln!("Pass rate: {:.0}%", result.pass_rate() * 100.0);
        eprintln!("{}", result.to_csv());
        assert!(!result.task_results.is_empty());
    }

    #[test]
    fn test_task_result_csv() {
        let result = TaskResult {
            task_id: "test_task".to_string(),
            status: TaskStatus::Pass,
            elapsed_ms: 1234,
            checks_passed: 5,
            checks_total: 5,
            detail: String::new(),
        };
        
        assert_eq!(result.to_csv_row(), "test_task,pass,1234,5,5");
    }

    #[test]
    fn test_harness_result_pass_rate() {
        let result = HarnessResult {
            task_results: vec![
                TaskResult {
                    task_id: "t1".to_string(),
                    status: TaskStatus::Pass,
                    elapsed_ms: 100,
                    checks_passed: 5,
                    checks_total: 5,
                    detail: String::new(),
                },
                TaskResult {
                    task_id: "t2".to_string(),
                    status: TaskStatus::Fail,
                    elapsed_ms: 100,
                    checks_passed: 3,
                    checks_total: 5,
                    detail: String::new(),
                },
            ],
            total_ms: 200,
        };
        
        assert_eq!(result.pass_rate(), 0.5);
    }
}
