//! Centralized constants for runie-agent.
//!
//! All magic numbers and tunable values are defined here to make
//! adjustments trivial and ensure consistent values across call sites.

/// Default timeout for permission requests in seconds.
pub const DEFAULT_PERMISSION_TIMEOUT_SECS: u64 = 60;

/// Default maximum number of tool rounds per agent turn.
/// Conservative limit for CLI and TUI use.
pub const DEFAULT_MAX_TOOL_ROUNDS: usize = 5;

// Compile-time assertions for invariants
const _: () = assert!(DEFAULT_PERMISSION_TIMEOUT_SECS > 0, "timeout must be positive");
const _: () = assert!(
    DEFAULT_PERMISSION_TIMEOUT_SECS <= 600,
    "max timeout is 10 minutes"
);
const _: () = assert!(DEFAULT_MAX_TOOL_ROUNDS > 0, "tool rounds must be positive");
const _: () = assert!(
    DEFAULT_MAX_TOOL_ROUNDS <= 100,
    "max tool rounds is 100"
);
