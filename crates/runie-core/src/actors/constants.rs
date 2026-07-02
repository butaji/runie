//! Actor-level constants — channel capacities, timeouts, and tuning values.
//!
//! These were previously scattered as unnamed literals across actor modules.
//! Centralizing them makes the values searchable and documents their intent.

/// Capacity of the leader → coordinator command channel.
/// Sized for burst of concurrent commands without significant backpressure.
pub const LEADER_CMD_CHANNEL_CAPACITY: usize = 32;

/// Shutdown timeout for actors and JoinHandle await in graceful shutdown paths.
pub const SHUTDOWN_TIMEOUT_SECS: u64 = 5;

/// Debounce interval for config file watcher.
pub const CONFIG_WATCHER_DEBOUNCE_MS: u64 = 300;
