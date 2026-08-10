//! Actor-owned session journal projection.
//!
//! This is the first persistence seam for Pi-compatible session behavior:
//! message entries are appended from the typed event bus, never by the TUI or
//! provider adapters. Storage backends can consume the immutable snapshot
//! later without becoming a second state owner.

use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

use tokio::sync::{mpsc, oneshot, watch};

use crate::events::EventBus;
use crate::task_owner::{mailbox_ack, spawn_actor_worker, spawn_owned_worker, TaskOwner};
use crate::types::{AgentEvent, AgentMessage, StopReason};

#[macro_use]
#[path = "session_config.rs"]
mod session_config;
pub use session_config::*;
include!("session_snapshot_core.rs");
include!("session_compaction.rs");
include!("session_lane_records.rs");
include!("session_tool_validation.rs");
include!("session_lane_validation.rs");
include!("session_snapshot_projection.rs");
include!("session_history.rs");
include!("session_json.rs");
include!("session_commands.rs");
#[path = "session_storage.rs"]
mod session_storage;
pub use session_storage::*;
include!("session_worker.rs");

pub struct SessionActor {
    tx: mpsc::Sender<Command>,
    snapshot: watch::Receiver<SessionSnapshot>,
    shared_snapshot: watch::Receiver<crate::SharedSnapshot<SessionSnapshot>>,
    _owner: Arc<TaskOwner>,
    _bus_owner: Option<Arc<TaskOwner>>,
}

impl Clone for SessionActor {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
            snapshot: self.snapshot.clone(),
            shared_snapshot: self.shared_snapshot.clone(),
            _owner: self._owner.clone(),
            _bus_owner: self._bus_owner.clone(),
        }
    }
}

#[derive(Clone)]
struct SessionSnapshotPublisher {
    snapshot_tx: watch::Sender<SessionSnapshot>,
    shared_tx: watch::Sender<crate::SharedSnapshot<SessionSnapshot>>,
}

impl SessionSnapshotPublisher {
    fn send(&self, state: SessionSnapshot) {
        crate::publish_shared_snapshot(&self.snapshot_tx, &self.shared_tx, state);
    }
}

type SessionEventReceiver = tokio::sync::broadcast::Receiver<AgentEvent>;
type SessionMailbox = mpsc::Sender<Command>;

include!("session_actor_impl.rs");
include!("session_shared.rs");
include!("session_navigation.rs");

fn reset_session_worker(
    state: &mut SessionSnapshot,
    next_id: &mut u64,
    tool_result_ids: &mut HashMap<String, String>,
    pending_tool_starts: &mut Vec<PendingToolStart>,
) {
    *state = SessionSnapshot::default();
    *next_id = 1;
    tool_result_ids.clear();
    pending_tool_starts.clear();
}

fn import_session_worker(
    state: &mut SessionSnapshot,
    imported: SessionSnapshot,
    next_id: &mut u64,
    tool_result_ids: &mut HashMap<String, String>,
    pending_tool_starts: &mut Vec<PendingToolStart>,
) {
    *next_id = imported
        .entries
        .iter()
        .filter_map(|entry| entry.id.strip_prefix("entry-"))
        .filter_map(|value| value.parse::<u64>().ok())
        .max()
        .unwrap_or(imported.sequence)
        .saturating_add(1);
    *state = imported;
    rebuild_tool_result_reservations(state, tool_result_ids);
    pending_tool_starts.clear();
}

impl Default for SessionActor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    include!("session_tests_00.rs");
    include!("session_tests_01.rs");
    include!("session_tests_02.rs");
    include!("session_tests_03.rs");
    include!("session_tests_04.inc");
    include!("session_tests_05.inc");
    include!("session_tests_06.rs");
}
