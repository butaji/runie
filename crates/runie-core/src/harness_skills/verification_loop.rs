use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::{HarnessSkill, TurnEndCtx, TurnEndResult};

/// Configuration for the verification loop skill.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct VerificationConfig {
    /// Command to run for verification (e.g., "cargo test", "npm test").
    #[serde(default)]
    pub command: Option<String>,
    /// Maximum number of fix attempts after verification failure.
    #[serde(default = "default_max_fix_passes")]
    pub max_fix_passes: usize,
    /// Whether verification is enabled.
    #[serde(default = "super::default_true")]
    pub enabled: bool,
    /// Timeout for the verification command in seconds.
    #[serde(default = "default_timeout_seconds")]
    pub timeout_seconds: u64,
}

fn default_max_fix_passes() -> usize {
    3
}

fn default_timeout_seconds() -> u64 {
    120
}

/// Verification loop skill: runs command after turn to verify results.
pub struct VerificationLoopSkill {
    config: VerificationConfig,
    fix_pass_count: std::sync::atomic::AtomicUsize,
}

impl VerificationLoopSkill {
    pub fn new(config: VerificationConfig) -> Self {
        Self {
            config,
            fix_pass_count: std::sync::atomic::AtomicUsize::new(0),
        }
    }
    pub(crate) fn needs_verification(message: &str) -> bool {
        message.contains("```")
            || message.contains("file")
            || message.contains("fn ")
            || message.contains("class")
            || message.contains("const ")
            || message.contains("let ")
    }

    async fn run_verification(command: &str, timeout: Duration) -> Option<std::process::Output> {
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return None;
        }
        let mut cmd = tokio::process::Command::new(parts[0]);
        cmd.args(&parts[1..]);
        tokio::time::timeout(timeout, cmd.output())
            .await
            .ok()
            .and_then(|res| res.ok())
    }
}

#[async_trait]
impl HarnessSkill for VerificationLoopSkill {
    fn name(&self) -> &str {
        "verification_loop"
    }
    async fn on_turn_end(&self, ctx: &TurnEndCtx) -> TurnEndResult {
        if !self.config.enabled {
            return TurnEndResult::Continue;
        }
        let command = match &self.config.command {
            Some(cmd) if !cmd.is_empty() => cmd,
            _ => return TurnEndResult::Continue,
        };
        if !Self::needs_verification(&ctx.assistant_message) {
            return TurnEndResult::Continue;
        }
        let passes = self
            .fix_pass_count
            .load(std::sync::atomic::Ordering::Relaxed);
        if passes >= self.config.max_fix_passes {
            return TurnEndResult::Continue;
        }
        let timeout = Duration::from_secs(self.config.timeout_seconds);
        let output = match Self::run_verification(command, timeout).await {
            Some(out) => out,
            None => return TurnEndResult::Continue,
        };
        if output.status.success() {
            self.fix_pass_count
                .store(0, std::sync::atomic::Ordering::Relaxed);
            TurnEndResult::Continue
        } else {
            self.fix_pass_count
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            TurnEndResult::RequestAnotherPass
        }
    }
}
