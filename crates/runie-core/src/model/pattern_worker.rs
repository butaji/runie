//! Swarm pattern worker lifecycle rows (GROK.md §26).
//!
//! Rows are transient, turn-scoped UI state: the turn hook emits
//! `Event::PatternWorkerSpawned` / `Event::PatternWorkerFinished`, the
//! projections maintain this registry on `AgentState`, and the view
//! transform renders each row as an `Element::SubagentRow` feed post.
//! Rows are cleared when the next turn starts.

/// Lifecycle status of a swarm pattern worker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternWorkerStatus {
    Running,
    Completed,
    Failed,
}

/// Detail view state for an open subagent row overlay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubagentDetail {
    pub worker_id: String,
    pub scroll: usize,
}

/// One swarm worker's lifecycle row for the current turn.
#[derive(Debug, Clone)]
pub struct PatternWorkerRow {
    pub id: String,
    pub description: String,
    pub model: String,
    pub status: PatternWorkerStatus,
    /// Wall-clock start — drives the braille spinner while `Running`.
    pub started: std::time::Instant,
    pub duration_ms: Option<u64>,
    /// Live activity suffix shown while `Running` (e.g. "Waiting for response…").
    pub activity: String,
    /// Final worker output (capped); renders as the expandable post body.
    pub output: String,
}
