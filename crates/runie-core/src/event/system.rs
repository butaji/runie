//! System event variants (notifications, diagnostics, transient messages).

use std::fmt;
use strum::IntoStaticStr;

/// System-level notifications and transient messages shown to the user.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, IntoStaticStr)]
#[strum(serialize_all = "PascalCase")]
pub enum SystemEvent {
    SystemMessage { content: String },
    TransientMessage { content: String, level: crate::event::TransientLevel },
    TransientError { content: String },
    ClearTransient,
    ShowDiagnostics,
    ToggleReadOnly,
    TrustProject,
    UntrustProject,
    OpenAgentsManager,
}

impl SystemEvent {
    /// Canonical name for bindable events. Returns `None` for parameterized variants.
    pub fn variant_name(&self) -> Option<&'static str> {
        match self {
            SystemEvent::SystemMessage { .. } => None,
            SystemEvent::TransientMessage { .. } => None,
            SystemEvent::TransientError { .. } => None,
            SystemEvent::ClearTransient => Some("ClearTransient"),
            SystemEvent::ShowDiagnostics => Some("ShowDiagnostics"),
            SystemEvent::ToggleReadOnly => Some("ToggleReadOnly"),
            SystemEvent::TrustProject => Some("TrustProject"),
            SystemEvent::UntrustProject => Some("UntrustProject"),
            SystemEvent::OpenAgentsManager => Some("OpenAgentsManager"),
        }
    }
}

impl fmt::Display for SystemEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SystemEvent::SystemMessage { .. } => write!(f, "SystemMessage"),
            SystemEvent::TransientMessage { .. } => write!(f, "TransientMessage"),
            SystemEvent::TransientError { .. } => write!(f, "TransientError"),
            SystemEvent::ClearTransient => write!(f, "ClearTransient"),
            SystemEvent::ShowDiagnostics => write!(f, "ShowDiagnostics"),
            SystemEvent::ToggleReadOnly => write!(f, "ToggleReadOnly"),
            SystemEvent::TrustProject => write!(f, "TrustProject"),
            SystemEvent::UntrustProject => write!(f, "UntrustProject"),
            SystemEvent::OpenAgentsManager => write!(f, "OpenAgentsManager"),
        }
    }
}
