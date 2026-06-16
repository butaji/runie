//! Panel item types and activation actions.

use crate::event::{ControlEvent, InputEvent, CommandEvent};
use crate::Event;

/// A single row inside a panel.
///
/// The DSL is intentionally minimal: every navigable item carries an
/// `action: ItemAction` that is emitted on activation. Toggle items
/// render as checkboxes (`[ ]` / `[✓]`) in both list and form views.
/// Form fields are editable inline. There is no separate "checkbox"
/// variant — `Toggle` *is* the checkbox.
#[derive(Debug, Clone, PartialEq)]
pub enum PanelItem {
    Action {
        label: String,
        action: ItemAction,
    },
    /// Command palette entry with separate name and description.
    /// `label` is the combined "name description" string used for fuzzy
    /// filtering; `name` and `desc` are used by the renderer for styling.
    Command {
        name: String,
        desc: String,
        label: String,
        action: ItemAction,
    },
    Toggle {
        label: String,
        value: bool,
        action: ItemAction,
    },
    Select {
        label: String,
        current: String,
        options: Vec<String>,
        key: String,
    },
    FormField {
        label: String,
        value: String,
        placeholder: String,
        key: String,
    },
    FormSubmit,
    Header(String),
    Separator,
}

impl PanelItem {
    pub fn label(&self) -> Option<&str> {
        match self {
            PanelItem::Action { label, .. } => Some(label),
            PanelItem::Command { label, .. } => Some(label),
            PanelItem::Toggle { label, .. } => Some(label),
            PanelItem::Select { label, .. } => Some(label),
            PanelItem::FormField { label, .. } => Some(label),
            PanelItem::FormSubmit => Some("Submit"),
            PanelItem::Header(text) => Some(text),
            PanelItem::Separator => None,
        }
    }

    pub fn is_navigable(&self) -> bool {
        matches!(
            self,
            PanelItem::Action { .. }
                | PanelItem::Command { .. }
                | PanelItem::Toggle { .. }
                | PanelItem::Select { .. }
                | PanelItem::FormField { .. }
                | PanelItem::FormSubmit
        )
    }
}

/// What happens when a navigable panel item is activated.
#[derive(Debug, Clone, PartialEq)]
pub enum ItemAction {
    /// Navigate to another panel by id.
    Push(String),
    /// Go back to the previous panel.
    Pop,
    /// Close the entire dialog.
    Close,
    /// Emit an event to the main app.
    Emit(Event),
    /// Toggle a boolean setting by key.
    Toggle(String),
    /// Cycle a multi-choice setting by key.
    Cycle(String),
}

impl ItemAction {
    /// Default label for this action (used by `item_action()` builder method).
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
            ControlEvent::Quit => "Quit".into(),
            InputEvent::Submit => "Submit".into(),
            CommandEvent::RunSaveCommand { .. } => "Save".into(),
            CommandEvent::RunLoadCommand { .. } => "Load".into(),
            CommandEvent::RunDeleteCommand { .. } => "Delete".into(),
            CommandEvent::RunExportCommand { .. } => "Export".into(),
            CommandEvent::RunImportCommand { .. } => "Import".into(),
            CommandEvent::RunLoginCommand { .. } => "Login".into(),
            CommandEvent::RunLogoutCommand { .. } => "Logout".into(),
            CommandEvent::RunSkillCommand { .. } => "Run Skill".into(),
            _ => "Action".into(),
        }
    }
}

/// Parse an accelerator from a label like "_Submit" → Some('S').
/// The underscore is removed from display text.
pub fn parse_accel(label: &str) -> Option<char> {
    let mut chars = label.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '_' {
            return chars.next();
        }
    }
    None
}

/// Strip accelerator underscores from a label for display.
pub fn strip_accel(label: &str) -> String {
    label.replace('_', "")
}
