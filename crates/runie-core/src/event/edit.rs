//! Edit approval event variants.

use std::fmt;
use strum::IntoStaticStr;

/// Events for the edit preview and approval workflow.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, IntoStaticStr)]
#[strum(serialize_all = "PascalCase")]
pub enum EditEvent {
    PendingEdit {
        path: String,
        original: String,
        proposed: String,
    },
    ApproveEdit,
    RejectEdit,
}

impl fmt::Display for EditEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EditEvent::PendingEdit { .. } => write!(f, "PendingEdit"),
            EditEvent::ApproveEdit => write!(f, "ApproveEdit"),
            EditEvent::RejectEdit => write!(f, "RejectEdit"),
        }
    }
}
