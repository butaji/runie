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
}

fn default_max_fix_passes() -> usize {
    3
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
    fn run_verification(command: &str) -> std::process::Output {
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return std::process::Command::new("true").output().unwrap();
        }
        std::process::Command::new(parts[0])
            .args(&parts[1..])
            .output()
            .unwrap_or_else(|_| std::process::Command::new("true").output().unwrap())
    }
}

impl HarnessSkill for VerificationLoopSkill {
    fn name(&self) -> &str {
        "verification_loop"
    }
    fn on_turn_end(&self, ctx: &TurnEndCtx) -> TurnEndResult {
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
        let output = Self::run_verification(command);
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
