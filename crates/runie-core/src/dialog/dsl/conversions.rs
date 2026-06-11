//! Extension traits for From conversions

use super::{ItemAction, PanelItem};
use crate::Event;

/// Extension trait for String -> ItemAction
pub trait FromStringExt {
    fn into_action(self) -> ItemAction;
}

impl FromStringExt for String {
    fn into_action(self) -> ItemAction {
        ItemAction::Push(self)
    }
}

impl FromStringExt for &str {
    fn into_action(self) -> ItemAction {
        ItemAction::Push(self.into())
    }
}

/// Extension trait for Event -> ItemAction
pub trait FromEventExt {
    fn into_action(self) -> ItemAction;
}

impl FromEventExt for Event {
    fn into_action(self) -> ItemAction {
        ItemAction::Emit(self)
    }
}

impl From<Event> for ItemAction {
    fn from(e: Event) -> Self {
        Self::Emit(e)
    }
}

impl ItemAction {
    /// Default label for this action
    pub fn default_label(&self) -> String {
        match self {
            Self::Push(id) => format!("Go to {}", id),
            Self::Pop => "Back".into(),
            Self::Close => "Close".into(),
            Self::Emit(e) => e.default_label(),
            Self::Toggle(_) => "Toggle".into(),
            Self::Cycle(_) => "Change".into(),
        }
    }
}

impl Event {
    pub(crate) fn default_label(&self) -> String {
        match self {
            Event::Quit => "Quit".into(),
            Event::Submit => "Submit".into(),
            Event::RunSaveCommand { .. } => "Save".into(),
            Event::RunLoadCommand { .. } => "Load".into(),
            Event::RunDeleteCommand { .. } => "Delete".into(),
            Event::RunExportCommand { .. } => "Export".into(),
            Event::RunImportCommand { .. } => "Import".into(),
            Event::RunLoginCommand { .. } => "Login".into(),
            Event::RunLogoutCommand { .. } => "Logout".into(),
            Event::RunSkillCommand { .. } => "Run Skill".into(),
            _ => "Action".into(),
        }
    }
}
