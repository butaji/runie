use tokio::time::Duration;

/// Configuration for the one-shot planner.
#[derive(Debug, Clone)]
pub struct PlannerConfig {
    /// Maximum parse retries before giving up. Defaults to 2.
    pub max_retries: usize,
    /// Timeout for a single LLM call. Defaults to 60s.
    pub timeout: Duration,
}

impl Default for PlannerConfig {
    fn default() -> Self {
        Self {
            max_retries: 2,
            timeout: Duration::from_secs(60),
        }
    }
}
