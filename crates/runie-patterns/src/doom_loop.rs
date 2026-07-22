//! Doom Loop Detection — detect repeating tool call patterns.
//!
//! A doom loop occurs when an agent repeatedly calls the same tool (e.g., read_file)
//! without making progress. This module tracks consecutive identical tool calls and
//! signals when the pattern exceeds a configurable threshold.
//!
//! # Usage
//!
//! ```rust
//! use runie_patterns::doom_loop::{DoomLoopDetector, DoomLoopSignal};
//!
//! let mut detector = DoomLoopDetector::new(5); // threshold of 5
//!
//! // First 4 calls don't trigger (streak: 1, 2, 3, 4)
//! detector.check("read_file");
//! detector.check("read_file");
//! detector.check("read_file");
//! detector.check("read_file");
//!
//! // After 5 consecutive same-tool calls, signal is returned
//! let signal = detector.check("read_file");
//! assert!(signal.is_some());
//! ```

use std::collections::VecDeque;

/// A detected doom loop signal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoomLoopSignal {
    /// The name of the repeating tool.
    pub tool_name: String,
    /// How many consecutive times this tool was called.
    pub repetition_count: usize,
    /// Human-readable message describing the loop.
    pub message: String,
}

/// Default threshold for doom loop detection (5 consecutive same-tool calls).
pub const DEFAULT_DOOM_LOOP_THRESHOLD: usize = 5;

/// Tracks consecutive tool calls to detect doom loops.
#[derive(Debug, Clone)]
pub struct DoomLoopDetector {
    /// Maximum consecutive identical tool calls before signaling.
    threshold: usize,
    /// History of recent tool calls (tool names only).
    history: VecDeque<String>,
    /// Count of consecutive calls to the current tool.
    current_streak: usize,
    /// The tool name currently in the streak.
    current_tool: Option<String>,
}

impl DoomLoopDetector {
    /// Create a new detector with the given threshold.
    ///
    /// The threshold is the number of consecutive identical tool calls
    /// that triggers a doom loop signal.
    pub fn new(threshold: usize) -> Self {
        Self {
            threshold: threshold.max(1), // Ensure at least 1
            history: VecDeque::with_capacity(threshold * 2),
            current_streak: 0,
            current_tool: None,
        }
    }

    /// Create a detector with the default threshold (5).
    pub fn with_default_threshold() -> Self {
        Self::new(DEFAULT_DOOM_LOOP_THRESHOLD)
    }

    /// Check if a tool call triggers a doom loop signal.
    ///
    /// Returns `Some(DoomLoopSignal)` if the tool has been called consecutively
    /// `threshold` or more times. Returns `None` otherwise.
    pub fn check(&mut self, tool_name: &str) -> Option<DoomLoopSignal> {
        let tool_name = tool_name.to_string();
        
        // If this is the same as the current streak tool, increment
        if let Some(ref current) = self.current_tool {
            if current == &tool_name {
                self.current_streak += 1;
            } else {
                // Reset streak for new tool
                self.current_tool = Some(tool_name.clone());
                self.current_streak = 1;
            }
        } else {
            // First call
            self.current_tool = Some(tool_name.clone());
            self.current_streak = 1;
        }

        // Add to history
        if self.history.len() >= self.history.capacity().max(1) {
            self.history.pop_front();
        }
        self.history.push_back(tool_name.clone());

        // Check if we exceeded the threshold
        if self.current_streak >= self.threshold {
            Some(DoomLoopSignal {
                tool_name,
                repetition_count: self.current_streak,
                message: format!(
                    "Detected a repeating pattern. The tool '{}' has been called {} times in a row.",
                    self.current_tool.as_ref().unwrap(),
                    self.current_streak
                ),
            })
        } else {
            None
        }
    }

    /// Reset the detector state.
    pub fn reset(&mut self) {
        self.history.clear();
        self.current_streak = 0;
        self.current_tool = None;
    }

    /// Get the current streak count.
    pub fn streak(&self) -> usize {
        self.current_streak
    }

    /// Get the configured threshold.
    pub fn threshold(&self) -> usize {
        self.threshold
    }

    /// Check if we're currently in a doom loop (streak >= threshold).
    pub fn is_looping(&self) -> bool {
        self.current_streak >= self.threshold
    }

    /// Get recent tool call history.
    pub fn history(&self) -> Vec<&str> {
        self.history.iter().map(|s| s.as_str()).collect()
    }
}

impl Default for DoomLoopDetector {
    fn default() -> Self {
        Self::with_default_threshold()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detector_triggers_at_threshold() {
        let mut detector = DoomLoopDetector::new(3);
        
        assert!(detector.check("read_file").is_none());
        assert!(detector.check("read_file").is_none());
        
        let signal = detector.check("read_file");
        assert!(signal.is_some());
        assert_eq!(signal.as_ref().unwrap().repetition_count, 3);
    }

    #[test]
    fn detector_resets_on_tool_change() {
        let mut detector = DoomLoopDetector::new(3);
        
        detector.check("read_file");
        detector.check("read_file");
        // Switching tools resets streak, so need 3 bash calls now
        assert!(detector.check("bash").is_none()); // streak = 1
        assert!(detector.check("bash").is_none()); // streak = 2
        let signal = detector.check("bash");       // streak = 3, should signal
        assert!(signal.is_some());
    }

    #[test]
    fn default_threshold_is_five() {
        let detector = DoomLoopDetector::with_default_threshold();
        assert_eq!(detector.threshold, DEFAULT_DOOM_LOOP_THRESHOLD);
    }

    #[test]
    fn reset_clears_state() {
        let mut detector = DoomLoopDetector::new(2);
        detector.check("read_file");
        detector.check("read_file"); // Should have signaled
        detector.reset();
        
        assert_eq!(detector.streak(), 0);
        // After reset, is_looping should be false since streak is 0
        assert!(!detector.is_looping());
        // After reset, check should be none again
        assert!(detector.check("read_file").is_none());
    }

    #[test]
    fn threshold_minimum_is_one() {
        let detector = DoomLoopDetector::new(0);
        assert_eq!(detector.threshold, 1);
    }

    #[test]
    fn signal_contains_correct_info() {
        let mut detector = DoomLoopDetector::new(4);
        for _ in 0..4 {
            detector.check("grep");
        }
        
        let signal = detector.check("grep").unwrap();
        assert_eq!(signal.tool_name, "grep");
        assert_eq!(signal.repetition_count, 5);
        assert!(signal.message.contains("grep"));
    }
}
