//! Actor definitions for the Runie runtime.
//!
//! All actors are tokio tasks receiving typed messages via `mpsc`. No ractor dependency
//! for most actors.

pub mod constants;
pub use constants::{CONFIG_WATCHER_DEBOUNCE_MS, LEADER_CMD_CHANNEL_CAPACITY, SHUTDOWN_TIMEOUT_SECS};

pub mod config;
pub mod input;
pub mod io;
pub mod leader;
pub mod permission;
pub mod provider;
pub mod session;
pub mod turn;

/// Error when the actor drops before sending an RPC reply.
/// Moved from macro_defs.rs (task 35: spawn_actor! macro deleted as dead code).
#[derive(Debug)]
pub enum RpcError {
    ActorDropped,
}

impl std::fmt::Display for RpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RpcError::ActorDropped => write!(f, "actor dropped"),
        }
    }
}

impl std::error::Error for RpcError {}

mod persistence;
pub use persistence::PersistenceActor;

// ractor_adapter kept for runie-agent (still uses ractor internally).
// Other actors use pure mpsc and don't need this.
pub mod ractor_adapter;

// ── Re-exports ───────────────────────────────────────────────────────────────

pub use config::ConfigMsg;

// Backward-compat aliases (existing call sites keep compiling)
#[allow(unused_imports)]
pub use config::{RactorConfigActor, RactorConfigHandle};

// StopCell — no-op marker for mpsc actor shutdown (ractors use ractor::ActorCell)
#[derive(Clone, Default)]
pub struct StopCell;

impl StopCell {
    /// No-op shutdown signal for mpsc-based actors.
    #[allow(dead_code)]
    pub fn stop(&self, _reason: Option<String>) {}
}

/// Unified actor cell ref — wraps ractor::ActorCell (ractors) or StopCell (mpsc).
/// Used by SpawnedHandles and LeaderHandle so both actor types can coexist.
#[derive(Clone)]
pub enum ActorCellRef {
    /// A ractor actor cell.
    Ractor(ractor::ActorCell),
    /// An mpsc actor cell (no-op stop).
    Mpsc(StopCell),
}

impl ActorCellRef {
    pub fn stop(&self, reason: Option<String>) {
        match self {
            Self::Ractor(cell) => cell.stop(reason),
            Self::Mpsc(cell) => cell.stop(reason),
        }
    }
}

impl From<StopCell> for ActorCellRef {
    fn from(cell: StopCell) -> Self {
        Self::Mpsc(cell)
    }
}

impl From<ractor::ActorCell> for ActorCellRef {
    fn from(cell: ractor::ActorCell) -> Self {
        Self::Ractor(cell)
    }
}

// Back-compat alias (old code uses ActorCell)
#[allow(non_local_definitions)]
pub type ActorCell = ActorCellRef;

#[allow(unused_imports)]
pub use input::actor::InputActor;
#[allow(unused_imports)]
pub use input::{spawn_input_actor, InputActorBase, InputHandle, InputMsg, RactorInputHandle};
pub use io::{spawn_io_actor, IoActorHandle, IoMsg, RactorIoHandle};
pub use leader::{Leader, LeaderCommand, LeaderHandle, LeaderStatus};
#[allow(unused_imports)]
pub use permission::{PermissionMsg, RactorPermissionActor, RactorPermissionHandle};
#[allow(unused_imports)]
pub use provider::{BuiltProvider, ProviderFactory, ProviderMsg, RactorProviderActor, RactorProviderHandle};
pub use session::{spawn_session_actor, RactorSessionHandle, SessionHandle, SessionMsg};
#[allow(unused_imports)]
pub use turn::{RactorTurnActor, RactorTurnHandle, TurnMsg};
