//! Harness results types and formatting.

use serde::{Deserialize, Serialize};

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

    pub(super) fn with_elapsed(self, start: std::time::Instant) -> TaskResult {
        TaskResult { elapsed_ms: start.elapsed().as_millis() as u64, ..self }
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
