//! Centralized constants for runie-agent.
//!
//! All magic numbers and tunable values are defined here to make
//! adjustments trivial and ensure consistent values across call sites.

/// Default timeout for permission requests in seconds.
pub const DEFAULT_PERMISSION_TIMEOUT_SECS: u64 = 60;

/// Default maximum number of tool rounds per agent turn.
/// Conservative limit for CLI and TUI use.
pub const DEFAULT_MAX_TOOL_ROUNDS: usize = 5;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permission_timeout_is_reasonable() {
        assert!(DEFAULT_PERMISSION_TIMEOUT_SECS > 0);
        assert!(DEFAULT_PERMISSION_TIMEOUT_SECS <= 600); // Max 10 minutes
    }

    #[test]
    fn max_tool_rounds_is_reasonable() {
        assert!(DEFAULT_MAX_TOOL_ROUNDS > 0);
        assert!(DEFAULT_MAX_TOOL_ROUNDS <= 100);
    }
}
