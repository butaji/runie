//! Safety envelope for agent operations
//! Enforces cost caps, protected paths, and test gates

use std::path::{Path, PathBuf};

/// Safety configuration
#[derive(Debug, Clone)]
pub struct SafetyConfig {
    pub max_cost_per_task: f64,
    pub max_cost_per_session: f64,
    pub protected_paths: Vec<String>,
    pub required_tests: bool,
    pub max_retries: u32,
}

impl Default for SafetyConfig {
    fn default() -> Self {
        Self {
            max_cost_per_task: 5.00,
            max_cost_per_session: 50.00,
            protected_paths: vec![
                ".env".to_string(),
                "secrets".to_string(),
                ".ssh".to_string(),
                "*.pem".to_string(),
                "*.key".to_string(),
            ],
            required_tests: true,
            max_retries: 3,
        }
    }
}

/// Safety violation types
#[derive(Debug, Clone)]
pub enum SafetyViolation {
    CostExceeded { limit: f64, actual: f64 },
    ProtectedPath { path: String },
    TestFailed { output: String },
    MaxRetriesExceeded { retries: u32 },
}

/// Safety envelope that enforces rules
pub struct SafetyEnvelope {
    config: SafetyConfig,
    session_spent: f64,
    task_spent: f64,
}

impl SafetyEnvelope {
    pub fn new(config: SafetyConfig) -> Self {
        Self {
            config,
            session_spent: 0.0,
            task_spent: 0.0,
        }
    }

    /// Check if a path is protected
    pub fn is_protected(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        self.config.protected_paths.iter().any(|p| {
            if p.ends_with('*') {
                let prefix = &p[..p.len() - 1];
                path_str.contains(prefix)
            } else {
                path_str.contains(p)
            }
        })
    }

    /// Check if an operation would exceed cost limits
    pub fn check_cost(&self, additional_cost: f64) -> Result<(), SafetyViolation> {
        let task_total = self.task_spent + additional_cost;
        if task_total > self.config.max_cost_per_task {
            return Err(SafetyViolation::CostExceeded {
                limit: self.config.max_cost_per_task,
                actual: task_total,
            });
        }

        let session_total = self.session_spent + additional_cost;
        if session_total > self.config.max_cost_per_session {
            return Err(SafetyViolation::CostExceeded {
                limit: self.config.max_cost_per_session,
                actual: session_total,
            });
        }

        Ok(())
    }

    /// Track spending
    pub fn track_spend(&mut self, amount: f64) {
        self.task_spent += amount;
        self.session_spent += amount;
    }

    /// Reset task spending (call at task start)
    pub fn reset_task(&mut self) {
        self.task_spent = 0.0;
    }

    /// Get current safety status
    pub fn status(&self) -> SafetyStatus {
        let cost_percentage = if self.config.max_cost_per_session > 0.0 {
            (self.session_spent / self.config.max_cost_per_session) * 100.0
        } else {
            0.0
        };

        SafetyStatus {
            session_spent: self.session_spent,
            session_budget: self.config.max_cost_per_session,
            task_spent: self.task_spent,
            task_budget: self.config.max_cost_per_task,
            cost_percentage,
            is_safe: cost_percentage < 80.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SafetyStatus {
    pub session_spent: f64,
    pub session_budget: f64,
    pub task_spent: f64,
    pub task_budget: f64,
    pub cost_percentage: f64,
    pub is_safe: bool,
}

/// File change for safety analysis
#[derive(Debug, Clone)]
pub struct FileChange {
    pub path: PathBuf,
    pub deletions: usize,
    pub additions: usize,
}

impl FileChange {
    pub fn new(path: PathBuf, deletions: usize, additions: usize) -> Self {
        Self { path, deletions, additions }
    }

    pub fn is_high_risk(&self) -> bool {
        self.deletions > 100
    }
}
