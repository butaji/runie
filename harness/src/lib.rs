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

pub mod runner;
pub mod results;

pub use results::{HarnessResult, TaskResult, TaskStatus};
pub use runner::{HarnessConfig, list_tasks, run_all_tasks, run_harness_task};

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn success_exit() -> std::process::ExitStatus {
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            std::process::ExitStatus::from_raw(0)
        }
        #[cfg(not(unix))]
        {
            std::process::Command::new("cmd").arg("/c").arg("exit 0").status().unwrap()
        }
    }

    fn fail_exit() -> std::process::ExitStatus {
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            std::process::ExitStatus::from_raw(1)
        }
        #[cfg(not(unix))]
        {
            std::process::Command::new("cmd").arg("/c").arg("exit 1").status().unwrap()
        }
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

    #[test]
    fn test_pass_rate_all_skipped() {
        let result = HarnessResult {
            task_results: vec![
                TaskResult {
                    task_id: "t1".to_string(),
                    status: TaskStatus::Skipped,
                    elapsed_ms: 0,
                    checks_passed: 0,
                    checks_total: 0,
                    detail: String::new(),
                },
                TaskResult {
                    task_id: "t2".to_string(),
                    status: TaskStatus::Skipped,
                    elapsed_ms: 0,
                    checks_passed: 0,
                    checks_total: 0,
                    detail: String::new(),
                },
            ],
            total_ms: 0,
        };
        
        assert_eq!(result.pass_rate(), 0.0, "All skipped should return 0.0 pass rate");
    }

    #[test]
    fn test_pass_rate_mixed_statuses() {
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
                TaskResult {
                    task_id: "t3".to_string(),
                    status: TaskStatus::Skipped,
                    elapsed_ms: 0,
                    checks_passed: 0,
                    checks_total: 0,
                    detail: String::new(),
                },
            ],
            total_ms: 200,
        };
        
        // Only Pass counts: 1 Pass out of 3 total = 0.333...
        assert_eq!(result.pass_rate(), 1.0 / 3.0, "Only Pass counts toward pass rate");
    }

    #[test]
    fn test_csv_output_format() {
        let result = HarnessResult {
            task_results: vec![
                TaskResult {
                    task_id: "test_task".to_string(),
                    status: TaskStatus::Pass,
                    elapsed_ms: 1234,
                    checks_passed: 5,
                    checks_total: 5,
                    detail: String::new(),
                },
            ],
            total_ms: 1234,
        };
        
        let csv = result.to_csv();
        let mut lines = csv.lines();
        
        // Verify header
        let header = lines.next().unwrap();
        assert_eq!(header, "task_id,status,elapsed_ms,checks_passed,checks_total");
        
        // Verify row
        let row = lines.next().unwrap();
        assert_eq!(row, "test_task,pass,1234,5,5");
        
        // No more lines
        assert!(lines.next().is_none());
    }

    #[test]
    fn test_harness_result_to_csv() {
        let result = HarnessResult {
            task_results: vec![
                TaskResult {
                    task_id: "task1".to_string(),
                    status: TaskStatus::Pass,
                    elapsed_ms: 100,
                    checks_passed: 3,
                    checks_total: 3,
                    detail: String::new(),
                },
                TaskResult {
                    task_id: "task2".to_string(),
                    status: TaskStatus::Fail,
                    elapsed_ms: 200,
                    checks_passed: 1,
                    checks_total: 5,
                    detail: String::new(),
                },
            ],
            total_ms: 300,
        };
        
        let csv = result.to_csv();
        let lines: Vec<&str> = csv.lines().collect();
        
        assert_eq!(lines.len(), 3); // header + 2 rows
        assert_eq!(lines[0], "task_id,status,elapsed_ms,checks_passed,checks_total");
        assert_eq!(lines[1], "task1,pass,100,3,3");
        assert_eq!(lines[2], "task2,fail,200,1,5");
    }

    #[test]
    fn parse_grader_counts_pass_lines() {
        use runner::parse_grader_output;
        let stdout = "PASS check1\nPASS check2\nFAIL check3\n";
        let (status, passed, total, detail) = parse_grader_output(stdout, "", Some(&success_exit()));
        assert_eq!(status, TaskStatus::Pass, "exit 0 -> Pass regardless of stdout content");
        assert_eq!(passed, 2, "two lines start with PASS");
        assert_eq!(total, 3);
        assert_eq!(detail, "PASS check1\nPASS check2\nFAIL check3");
    }

    #[test]
    fn parse_grader_zero_passes_when_no_pass_lines() {
        use runner::parse_grader_output;
        let stdout = "FAIL check1\nFAIL check2\n";
        let (status, passed, total, _) = parse_grader_output(stdout, "", Some(&fail_exit()));
        assert_eq!(status, TaskStatus::Fail);
        assert_eq!(passed, 0);
        assert_eq!(total, 2);
    }

    #[test]
    fn parse_grader_handles_empty_stdout() {
        use runner::parse_grader_output;
        let (status, passed, total, detail) = parse_grader_output("", "", Some(&success_exit()));
        assert_eq!(status, TaskStatus::Pass);
        assert_eq!(passed, 0);
        // empty stdout -> split('\n') yields a single empty line
        assert_eq!(total, 1);
        assert!(detail.starts_with("grader exited with status"));
    }

    #[test]
    fn parse_grader_handles_empty_stdout_and_fail_exit() {
        use runner::parse_grader_output;
        let (status, passed, total, detail) = parse_grader_output("", "", Some(&fail_exit()));
        assert_eq!(status, TaskStatus::Fail);
        assert_eq!(passed, 0);
        assert_eq!(total, 1);
        assert!(detail.starts_with("grader exited with status"));
    }

    #[test]
    fn parse_grader_trims_whitespace() {
        use runner::parse_grader_output;
        let stdout = "   \nPASS a\nPASS b\n   \n";
        let (_, passed, total, _) = parse_grader_output(stdout, "", Some(&success_exit()));
        // stdout.trim() strips surrounding whitespace, leaving "PASS a\nPASS b"
        // which is 2 lines, both PASS.
        assert_eq!(passed, 2);
        assert_eq!(total, 2);
    }
}
