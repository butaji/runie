//! Typed messages for `TrustActor`.

use std::path::PathBuf;

use crate::actors::GenericActorHandle;
use crate::trust::TrustDecision;

/// Messages accepted by `TrustActor`.
#[derive(Debug, Clone)]
pub enum TrustMsg {
    /// Load trust decisions from an external source (e.g., from SessionActor on startup).
    LoadTrust {
        decisions: std::collections::HashMap<PathBuf, TrustDecision>,
    },
    /// Set a trust decision for a project path.
    SetTrust {
        path: PathBuf,
        decision: TrustDecision,
    },
    /// Initialize the read-only flag from trust decisions (called after LoadTrust).
    InitReadOnly {
        path: PathBuf,
    },
}

/// Handle for sending messages to `TrustActor`.
pub type TrustActorHandle = GenericActorHandle<TrustMsg>;
