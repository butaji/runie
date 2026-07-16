//! Agentic loop detection — identifies when an agent repeatedly calls the same
//! sequence of tools, indicating a potential infinite loop or ineffective strategy.
//!
//! The detector works by fingerprinting tool call sequences and tracking how
//! many times each fingerprint has been seen at similar depths in the conversation.

use sha2::{Digest, Sha256};

use crate::proto::message::tool_call::ToolCall;

/// Configuration for agentic loop detection.
#[derive(Debug, Clone, Default)]
pub struct AgenticLoopConfig {
    /// Maximum number of times the same tool fingerprint can appear at similar
    /// depth before considering it an agentic loop. Default is 3.
    pub max_loops: u32,
}

impl AgenticLoopConfig {
    /// Create a new config with the given max loop count.
    pub fn new(max_loops: u32) -> Self {
        Self { max_loops }
    }

    /// Default configuration (max_loops: 3).
    pub fn default_config() -> Self {
        Self::default()
    }
}

/// Generates a fingerprint hash from a list of tool calls.
///
/// The fingerprint is a SHA-256 hash of the sorted tool names, making it
/// independent of tool call ordering. Tool names are concatenated with commas
/// and hashed together.
///
/// # Arguments
/// * `tool_calls` - A slice of tool calls to fingerprint
///
/// # Returns
/// A hexadecimal string representation of the SHA-256 hash
///
/// # Example
/// ```
/// use runie_core::proto::message::tool_call::ToolCall;
/// use runie_core::agentic_loop::fingerprint_tools;
///
/// let tools = vec![
///     ToolCall::new("1", "bash", serde_json::json!({})),
///     ToolCall::new("2", "read_file", serde_json::json!({})),
/// ];
/// let fp = fingerprint_tools(&tools);
/// // Same tools in different order produce same fingerprint
/// let tools2 = vec![
///     ToolCall::new("3", "read_file", serde_json::json!({})),
///     ToolCall::new("4", "bash", serde_json::json!({})),
/// ];
/// assert_eq!(fp, fingerprint_tools(&tools2));
/// ```
pub fn fingerprint_tools(tool_calls: &[ToolCall]) -> String {
    let mut names: Vec<&str> = tool_calls.iter().map(|tc| tc.name.as_str()).collect();
    names.sort_unstable();
    let combined = names.join(",");
    let hash = Sha256::digest(combined.as_bytes());
    hex::encode(hash)
}

/// Checks if a fingerprint pattern is safe based on history.
///
/// This function analyzes the fingerprint history to determine if the current
/// tool pattern has exceeded the maximum allowed loop count at the given depth.
///
/// # Arguments
/// * `fingerprints` - Slice of historical fingerprints to check against
/// * `new_fp` - The new fingerprint being added
/// * `depth` - The current turn/loop depth in the conversation
/// * `max_loops` - Maximum allowed occurrences before flagging as unsafe
///
/// # Returns
/// `true` if the pattern is safe (within loop limits), `false` if an agentic
/// loop has been detected
pub fn check_agentic_loop_safety(
    fingerprints: &[String],
    new_fp: &str,
    _depth: u32,
    max_loops: u32,
) -> bool {
    if fingerprints.is_empty() && new_fp.is_empty() {
        return true;
    }
    if max_loops == 0 {
        return true;
    }

    // Count occurrences of this fingerprint in history plus the new one
    let history_count = fingerprints.iter().filter(|fp| *fp == new_fp).count();
    let total_count = history_count + 1; // Include the new fingerprint

    // If total count exceeds max_loops, it's a loop
    total_count as u32 <= max_loops
}

/// State tracker for agentic loop fingerprints.
///
/// Stores the history of tool call fingerprints to detect repeated patterns
/// over time.
#[derive(Debug, Clone, Default)]
pub struct AgenticLoopTracker {
    fingerprints: Vec<String>,
    depths: Vec<u32>,
    max_fingerprints: usize,
}

/// Type alias for backwards compatibility.
#[deprecated(since = "0.1.0", note = "Use AgenticLoopTracker instead")]
pub type AgenticLoopState = AgenticLoopTracker;

impl AgenticLoopTracker {
    /// Create a new state tracker with default capacity (100 fingerprints).
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new state tracker with custom capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            fingerprints: Vec::with_capacity(capacity),
            depths: Vec::with_capacity(capacity),
            max_fingerprints: capacity,
        }
    }

    /// Add a fingerprint to the history.
    pub fn push(&mut self, fingerprint: String) {
        self.push_with_depth(fingerprint, 0);
    }

    /// Add a fingerprint with its depth to the history.
    pub fn push_with_depth(&mut self, fingerprint: String, depth: u32) {
        if self.fingerprints.len() >= self.max_fingerprints {
            // Remove oldest entries (FIFO)
            let drain_count = (self.fingerprints.len() + 1) / 2;
            self.fingerprints.drain(0..drain_count);
            self.depths.drain(0..drain_count);
        }
        self.fingerprints.push(fingerprint);
        self.depths.push(depth);
    }

    /// Get all stored fingerprints.
    pub fn fingerprints(&self) -> &[String] {
        &self.fingerprints
    }

    /// Get all stored depths.
    pub fn depths(&self) -> &[u32] {
        &self.depths
    }

    /// Check if the current state indicates an agentic loop.
    ///
    /// Returns `true` if safe, `false` if an agentic loop is detected.
    pub fn is_safe(&self, depth: u32, max_loops: u32) -> bool {
        if let Some(last_fp) = self.fingerprints.last() {
            check_agentic_loop_safety(&self.fingerprints, last_fp, depth, max_loops)
        } else {
            true
        }
    }

    /// Clear the fingerprint history.
    pub fn reset(&mut self) {
        self.fingerprints.clear();
        self.depths.clear();
    }

    /// Add a tool call fingerprint and check for safety in one operation.
    ///
    /// Returns `true` if safe after adding, `false` if an agentic loop is detected.
    pub fn push_and_check(&mut self, fingerprint: String, depth: u32, max_loops: u32) -> bool {
        let safe = check_agentic_loop_safety(&self.fingerprints, &fingerprint, depth, max_loops);
        self.push_with_depth(fingerprint, depth);
        safe
    }

    /// Get the number of fingerprints currently stored.
    pub fn len(&self) -> usize {
        self.fingerprints.len()
    }

    /// Check if the state is empty.
    pub fn is_empty(&self) -> bool {
        self.fingerprints.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proto::message::tool_call::ToolCall;

    fn make_tool(name: &str) -> ToolCall {
        ToolCall::new("1", name, serde_json::json!({}))
    }

    fn make_tools(names: &[&str]) -> Vec<ToolCall> {
        names
            .iter()
            .enumerate()
            .map(|(i, n)| ToolCall::new(format!("id-{}", i), n, serde_json::json!({})))
            .collect()
    }

    #[test]
    fn test_fingerprint_tools_empty() {
        let tools: Vec<ToolCall> = vec![];
        let fp = fingerprint_tools(&tools);
        // Empty input should produce a known hash
        assert!(!fp.is_empty());
        assert_eq!(fp.len(), 64); // SHA-256 hex is 64 chars
    }

    #[test]
    fn test_fingerprint_tools_order_independent() {
        let tools1 = make_tools(&["bash", "read_file", "grep"]);
        let tools2 = make_tools(&["read_file", "grep", "bash"]);
        let tools3 = make_tools(&["grep", "bash", "read_file"]);

        let fp1 = fingerprint_tools(&tools1);
        let fp2 = fingerprint_tools(&tools2);
        let fp3 = fingerprint_tools(&tools3);

        assert_eq!(fp1, fp2);
        assert_eq!(fp2, fp3);
    }

    #[test]
    fn test_fingerprint_tools_different_tools_different_hash() {
        let tools1 = make_tools(&["bash", "read_file"]);
        let tools2 = make_tools(&["bash", "write_file"]);

        let fp1 = fingerprint_tools(&tools1);
        let fp2 = fingerprint_tools(&tools2);

        assert_ne!(fp1, fp2);
    }

    #[test]
    fn test_fingerprint_tools_single_tool() {
        let tools = make_tools(&["bash"]);
        let fp = fingerprint_tools(&tools);
        assert!(!fp.is_empty());
        assert_eq!(fp.len(), 64);
    }

    #[test]
    fn test_fingerprint_tools_duplicate_tools() {
        let tools = make_tools(&["bash", "bash", "read_file"]);
        let fp = fingerprint_tools(&tools);
        assert!(!fp.is_empty());

        // Same tools, different order should still match
        let tools2 = make_tools(&["read_file", "bash", "bash"]);
        let fp2 = fingerprint_tools(&tools2);
        assert_eq!(fp, fp2);
    }

    #[test]
    fn test_check_agentic_loop_safety_empty() {
        let fingerprints: Vec<String> = vec![];
        assert!(check_agentic_loop_safety(&fingerprints, "fp1", 1, 3));
    }

    #[test]
    fn test_check_agentic_loop_safety_max_zero() {
        let fingerprints = vec!["abc".to_string(), "def".to_string()];
        assert!(check_agentic_loop_safety(&fingerprints, "fp1", 1, 0));
    }

    #[test]
    fn test_check_agentic_loop_safety_no_repetition() {
        let fingerprints = vec!["fp1".to_string(), "fp2".to_string(), "fp3".to_string()];
        assert!(check_agentic_loop_safety(&fingerprints, "fp4", 1, 3));
    }

    #[test]
    fn test_check_agentic_loop_safety_single_repetition() {
        let fingerprints = vec![
            "fp1".to_string(),
            "fp2".to_string(),
            "fp1".to_string(), // repetition, but only 2 times
        ];
        // new_fp is fp1 which already has 2 occurrences, total would be 3 with max_loops=3
        assert!(check_agentic_loop_safety(&fingerprints, "fp1", 1, 3));
        // new_fp is fp2 which has 1 occurrence, total would be 2
        assert!(check_agentic_loop_safety(&fingerprints, "fp2", 1, 3));
    }

    #[test]
    fn test_check_agentic_loop_safety_detects_loop() {
        let fingerprints = vec![
            "fp1".to_string(),
            "fp2".to_string(),
            "fp1".to_string(),
            "fp1".to_string(), // 3rd repetition at position 4
        ];
        // fp1 appears 3 times in history, new occurrence would be 4th, exceeds max_loops=3
        assert!(!check_agentic_loop_safety(&fingerprints, "fp1", 1, 3));

        let fingerprints_with_more = vec![
            "fp1".to_string(),
            "fp2".to_string(),
            "fp1".to_string(),
            "fp1".to_string(),
            "fp1".to_string(), // 4th repetition
        ];
        // fp1 appears 4 times in history, new occurrence would be 5th
        assert!(!check_agentic_loop_safety(
            &fingerprints_with_more,
            "fp1",
            1,
            3
        ));
    }

    #[test]
    fn test_agentic_loop_tracker_new() {
        let tracker = AgenticLoopTracker::new();
        assert!(tracker.is_empty());
        assert_eq!(tracker.len(), 0);
    }

    #[test]
    fn test_agentic_loop_tracker_push() {
        let mut tracker = AgenticLoopTracker::new();
        tracker.push("fp1".to_string());
        assert_eq!(tracker.len(), 1);
        assert!(!tracker.is_empty());
    }

    #[test]
    fn test_agentic_loop_tracker_fingerprints() {
        let mut tracker = AgenticLoopTracker::new();
        tracker.push("fp1".to_string());
        tracker.push("fp2".to_string());

        let fps = tracker.fingerprints();
        assert_eq!(fps.len(), 2);
        assert_eq!(fps[0], "fp1");
        assert_eq!(fps[1], "fp2");
    }

    #[test]
    fn test_agentic_loop_tracker_depths() {
        let mut tracker = AgenticLoopTracker::new();
        tracker.push_with_depth("fp1".to_string(), 1);
        tracker.push_with_depth("fp2".to_string(), 2);

        let depths = tracker.depths();
        assert_eq!(depths.len(), 2);
        assert_eq!(depths[0], 1);
        assert_eq!(depths[1], 2);
    }

    #[test]
    fn test_agentic_loop_tracker_reset() {
        let mut tracker = AgenticLoopTracker::new();
        tracker.push("fp1".to_string());
        tracker.push("fp2".to_string());
        tracker.reset();
        assert!(tracker.is_empty());
    }

    #[test]
    fn test_agentic_loop_tracker_capacity() {
        let mut tracker = AgenticLoopTracker::with_capacity(5);
        for i in 0..10 {
            tracker.push(format!("fp{}", i));
        }
        // Should have been trimmed
        assert!(tracker.len() <= 5);
    }

    #[test]
    fn test_agentic_loop_tracker_is_safe() {
        let mut tracker = AgenticLoopTracker::new();
        tracker.push("fp1".to_string());
        assert!(tracker.is_safe(1, 3));

        tracker.push("fp1".to_string());
        assert!(tracker.is_safe(1, 3));

        tracker.push("fp1".to_string());
        tracker.push("fp1".to_string()); // 4th
        assert!(!tracker.is_safe(1, 3));
    }

    #[test]
    fn test_agentic_loop_tracker_push_and_check() {
        let mut tracker = AgenticLoopTracker::new();
        let config = AgenticLoopConfig::new(3);

        // First 3 pushes should be safe
        assert!(tracker.push_and_check("fp1".to_string(), 1, config.max_loops));
        assert!(tracker.push_and_check("fp1".to_string(), 1, config.max_loops));
        assert!(tracker.push_and_check("fp1".to_string(), 1, config.max_loops));

        // 4th should fail
        assert!(!tracker.push_and_check("fp1".to_string(), 1, config.max_loops));
    }

    #[test]
    fn test_agentic_loop_config_default() {
        let config = AgenticLoopConfig::default();
        assert_eq!(config.max_loops, 3);
    }

    #[test]
    fn test_agentic_loop_config_new() {
        let config = AgenticLoopConfig::new(5);
        assert_eq!(config.max_loops, 5);
    }

    #[test]
    fn test_fingerprint_with_real_tool_structures() {
        // Test with more realistic tool call structures
        let tools = vec![
            ToolCall::with_json_args("tc1", "bash", r#"{"command": "ls -la"}"#),
            ToolCall::with_json_args("tc2", "read_file", r#"{"path": "file.txt"}"#),
        ];
        let fp = fingerprint_tools(&tools);
        assert_eq!(fp.len(), 64);

        // Same tools, different args - should have same fingerprint
        let tools2 = vec![
            ToolCall::with_json_args("tc3", "bash", r#"{"command": "pwd"}"#),
            ToolCall::with_json_args("tc4", "read_file", r#"{"path": "other.txt"}"#),
        ];
        let fp2 = fingerprint_tools(&tools2);
        assert_eq!(fp, fp2);
    }

    #[test]
    fn test_detecting_read_loop_pattern() {
        // Simulate an agent repeatedly reading the same files
        let mut tracker = AgenticLoopTracker::new();
        let config = AgenticLoopConfig::new(3);

        // Simulate a read loop: bash -> read_file -> bash -> read_file
        let bash_fp = fingerprint_tools(&make_tools(&["bash"]));
        let read_fp = fingerprint_tools(&make_tools(&["read_file"]));
        let bash_read_fp = fingerprint_tools(&make_tools(&["bash", "read_file"]));

        // First iteration - safe
        assert!(tracker.push_and_check(bash_read_fp.clone(), 1, config.max_loops));

        // Second iteration - safe (only 1 occurrence so far)
        assert!(tracker.push_and_check(bash_read_fp.clone(), 2, config.max_loops));

        // Third iteration - still safe (3 occurrences)
        assert!(tracker.push_and_check(bash_read_fp.clone(), 3, config.max_loops));

        // Fourth iteration - UNSAFE (4 occurrences exceeds max_loops=3)
        assert!(!tracker.push_and_check(bash_read_fp.clone(), 4, config.max_loops));
    }

    #[test]
    fn test_different_patterns_are_safe() {
        let mut tracker = AgenticLoopTracker::new();
        let config = AgenticLoopConfig::new(2);

        // Different patterns should not trigger loop detection
        let fp1 = fingerprint_tools(&make_tools(&["bash"]));
        let fp2 = fingerprint_tools(&make_tools(&["read_file"]));
        let fp3 = fingerprint_tools(&make_tools(&["write_file"]));

        assert!(tracker.push_and_check(fp1.clone(), 1, config.max_loops));
        assert!(tracker.push_and_check(fp2.clone(), 2, config.max_loops));
        assert!(tracker.push_and_check(fp3.clone(), 3, config.max_loops));
        assert!(tracker.push_and_check(fp1.clone(), 4, config.max_loops));
        assert!(tracker.push_and_check(fp2.clone(), 5, config.max_loops));

        // All should be safe since they're different patterns
        assert!(tracker.is_safe(6, config.max_loops));
    }
}
