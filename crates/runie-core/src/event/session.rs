//! Session event variants (fork, clone, tree navigation).

use std::fmt;
use strum::IntoStaticStr;

/// Events that manipulate the session tree.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, IntoStaticStr)]
#[strum(serialize_all = "PascalCase")]
pub enum SessionEvent {
    ForkSession { message_index: usize },
    CloneSession,
    ToggleSessionTree,
    SessionTreeFilterCycle,
    SessionTreeSelect { id: String },
}

impl fmt::Display for SessionEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SessionEvent::ForkSession { .. } => write!(f, "ForkSession"),
            SessionEvent::CloneSession => write!(f, "CloneSession"),
            SessionEvent::ToggleSessionTree => write!(f, "ToggleSessionTree"),
            SessionEvent::SessionTreeFilterCycle => write!(f, "SessionTreeFilterCycle"),
            SessionEvent::SessionTreeSelect { .. } => write!(f, "SessionTreeSelect"),
        }
    }
}
