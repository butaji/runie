//! Leader message types.

/// Commands to control the leader runtime.
#[derive(Debug, Clone)]
pub enum LeaderCommand {
    /// Request the current status of the leader and all actors.
    Status,
    /// Trigger graceful shutdown of all actors.
    Shutdown,
    /// Force abort all actors (for crash recovery).
    ForceAbort,
}

/// Status response from the leader.
#[derive(Debug, Clone)]
pub struct LeaderStatus {
    pub running: bool,
    pub actor_count: usize,
    pub bus_subscribers: usize,
}

impl Default for LeaderStatus {
    fn default() -> Self {
        Self {
            running: false,
            actor_count: 0,
            bus_subscribers: 0,
        }
    }
}
