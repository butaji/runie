//! Control event variants (quit, reset, abort, external editor, etc.).

use std::fmt;
use strum::IntoStaticStr;

/// Global control events that affect the application lifecycle.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, IntoStaticStr)]
#[strum(serialize_all = "PascalCase")]
pub enum ControlEvent {
    Quit,
    Reset,
    Abort,
    FollowUp,
    SpawnAgent { prompt: String },
    ToggleExpand,
    Dequeue,
    OpenExternalEditor,
    ExternalEditorDone { content: String },
    ShareSession,
    /// Suspend to background (Unix only).
    Suspend,
    /// Toggle vim mode.
    ToggleVimMode,
    /// Copy the last assistant response to clipboard.
    CopyLastResponse,
    /// Open session list dialog.
    OpenSessionList,
    /// Start a new session (closes welcome screen).
    NewSession,
    /// Resume a session from the list (closes welcome screen).
    ResumeSession,
    SelectSession { id: String },
    StarSession { id: String },
    RenameSession { id: String, name: String },
    DeleteSession { id: String },
}

impl ControlEvent {
    /// Canonical name for bindable events. Returns `None` for parameterized variants.
    pub fn variant_name(&self) -> Option<&'static str> {
        match self {
            ControlEvent::Quit => Some("Quit"),
            ControlEvent::Reset => Some("Reset"),
            ControlEvent::Abort => Some("Abort"),
            ControlEvent::FollowUp => Some("FollowUp"),
            ControlEvent::SpawnAgent { .. } => None,
            ControlEvent::ToggleExpand => Some("ToggleExpand"),
            ControlEvent::Dequeue => Some("Dequeue"),
            ControlEvent::OpenExternalEditor => Some("OpenExternalEditor"),
            ControlEvent::ExternalEditorDone { .. } => None,
            ControlEvent::ShareSession => Some("ShareSession"),
            ControlEvent::Suspend => Some("Suspend"),
            ControlEvent::ToggleVimMode => Some("ToggleVimMode"),
            ControlEvent::CopyLastResponse => Some("CopyLastResponse"),
            ControlEvent::OpenSessionList => Some("OpenSessionList"),
            ControlEvent::NewSession => Some("NewSession"),
            ControlEvent::ResumeSession => Some("ResumeSession"),
            ControlEvent::SelectSession { .. } => None,
            ControlEvent::StarSession { .. } => None,
            ControlEvent::RenameSession { .. } => None,
            ControlEvent::DeleteSession { .. } => None,
        }
    }
}

impl fmt::Display for ControlEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ControlEvent::Quit => write!(f, "Quit"),
            ControlEvent::Reset => write!(f, "Reset"),
            ControlEvent::Abort => write!(f, "Abort"),
            ControlEvent::FollowUp => write!(f, "FollowUp"),
            ControlEvent::SpawnAgent { .. } => write!(f, "SpawnAgent"),
            ControlEvent::ToggleExpand => write!(f, "ToggleExpand"),
            ControlEvent::Dequeue => write!(f, "Dequeue"),
            ControlEvent::OpenExternalEditor => write!(f, "OpenExternalEditor"),
            ControlEvent::ExternalEditorDone { .. } => write!(f, "ExternalEditorDone"),
            ControlEvent::ShareSession => write!(f, "ShareSession"),
            ControlEvent::Suspend => write!(f, "Suspend"),
            ControlEvent::ToggleVimMode => write!(f, "ToggleVimMode"),
            ControlEvent::CopyLastResponse => write!(f, "CopyLastResponse"),
            ControlEvent::OpenSessionList => write!(f, "OpenSessionList"),
            ControlEvent::NewSession => write!(f, "NewSession"),
            ControlEvent::ResumeSession => write!(f, "ResumeSession"),
            ControlEvent::SelectSession { .. } => write!(f, "SelectSession"),
            ControlEvent::StarSession { .. } => write!(f, "StarSession"),
            ControlEvent::RenameSession { .. } => write!(f, "RenameSession"),
            ControlEvent::DeleteSession { .. } => write!(f, "DeleteSession"),
        }
    }
}
