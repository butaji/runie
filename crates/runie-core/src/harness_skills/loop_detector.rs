use serde::{Deserialize, Serialize};

use super::HarnessSkill;

/// Configuration for the loop detector skill.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoopDetectorConfig {
    /// Maximum repeats before triggering loop detection.
    #[serde(default = "default_max_repeats")]
    pub max_repeats: usize,
    /// Whether detection is enabled.
    #[serde(default = "super::default_true")]
    pub enabled: bool,
}

impl Default for LoopDetectorConfig {
    fn default() -> Self {
        Self {
            max_repeats: 3,
            enabled: true,
        }
    }
}

fn default_max_repeats() -> usize {
    3
}

/// Loop detector skill.
pub struct LoopDetectorSkill {
    config: LoopDetectorConfig,
    recent_calls: parking_lot::Mutex<Vec<(String, String, bool)>>,
}

impl LoopDetectorSkill {
    pub fn new(config: LoopDetectorConfig) -> Self {
        Self {
            config,
            recent_calls: parking_lot::Mutex::new(Vec::new()),
        }
    }

    /// Record a tool call outcome.
    pub fn record_call(&self, tool_name: &str, input: &serde_json::Value, success: bool) {
        let target = input
            .get("path")
            .or_else(|| input.get("command"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_owned();
        let entry = (tool_name.to_owned(), target, success);
        // parking_lot MutexGuard doesn't panic on poison, use directly.
        let mut calls = self.recent_calls.lock();
        calls.push(entry);
        if calls.len() > 100 {
            calls.drain(0..50);
        }
    }

    /// Reset state at turn start.
    pub fn reset(&self) {
        self.recent_calls.lock().clear();
    }

    /// Check for loop. Returns message if detected.
    pub fn check_loop(&self) -> Option<String> {
        if !self.config.enabled {
            return None;
        }
        // parking_lot MutexGuard doesn't panic on poison.
        let calls = self.recent_calls.lock();
        let mut counts = std::collections::HashMap::new();
        for (tool, target, success) in calls.iter().rev() {
            if *success {
                break;
            }
            let key = format!("{}/{}", tool, target);
            *counts.entry(key).or_insert(0) += 1;
        }
        for (pattern, count) in counts {
            if count >= self.config.max_repeats {
                return Some(format!(
                    "Loop detected ({}x): {}. Try a different approach.",
                    count, pattern
                ));
            }
        }
        None
    }
}

impl HarnessSkill for LoopDetectorSkill {
    fn name(&self) -> &str {
        "loop_detector"
    }
}
