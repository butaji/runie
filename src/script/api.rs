//! API exposed to JavaScript agents
//! This module defines the ctx object available in anvil.js

/// Task information passed to agent scripts
#[derive(Debug, Clone)]
pub struct TaskInfo {
    pub id: String,
    pub task_type: String,
    pub intent: String,
    pub estimated_tokens: usize,
    pub severity: String,
}

/// Session information
#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub cost: f64,
    pub budget: f64,
    pub elapsed: f64,
}

/// Context API available to agents
pub struct AnvilContext {
    pub task: TaskInfo,
    pub session: SessionInfo,
}

impl AnvilContext {
    pub fn new() -> Self {
        Self {
            task: TaskInfo {
                id: "default".to_string(),
                task_type: "general".to_string(),
                intent: "".to_string(),
                estimated_tokens: 0,
                severity: "normal".to_string(),
            },
            session: SessionInfo {
                cost: 0.0,
                budget: 100.0,
                elapsed: 0.0,
            },
        }
    }
}

impl Default for AnvilContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Git operations API
pub struct GitApi;

impl GitApi {
    pub fn commit(&self, message: &str) -> Result<String, String> {
        Ok(format!("Would commit: {}", message))
    }

    pub fn changed_files(&self) -> Vec<String> {
        vec![]
    }
}

/// UI operations API
pub struct UiApi;

impl UiApi {
    pub fn show_error(&self, title: &str, detail: &str) {
        eprintln!("[ERROR] {}: {}", title, detail);
    }

    pub fn show_info(&self, message: &str) {
        println!("[INFO] {}", message);
    }
}

/// Run command API
pub struct RunApi;

impl RunApi {
    pub fn run(&self, cmd: &str) -> Result<CommandResult, String> {
        Ok(CommandResult {
            exit_code: 0,
            stdout: format!("Would run: {}", cmd),
            stderr: String::new(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct CommandResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

/// Router API
pub struct RouterApi;

impl RouterApi {
    pub fn estimate_cost(&self, tokens: usize) -> f64 {
        (tokens as f64 / 1_000_000.0) * 3.0
    }
}

/// Safety API
pub struct SafetyApi;

impl SafetyApi {
    pub fn check_path(&self, path: &str) -> bool {
        let protected = [".env", "secrets", ".ssh", "key.pem"];
        protected.iter().any(|p| path.contains(p))
    }
}
