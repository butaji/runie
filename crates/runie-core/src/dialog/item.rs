//! Panel item types and activation actions.

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
        /// Cursor position in bytes within `value`. Forms support inline
        /// editing: arrow keys move the cursor and text is inserted/deleted
        /// at the cursor position.
        cursor_pos: usize,
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
            crate::Event::Quit => "Quit".into(),
            crate::Event::Submit => "Submit".into(),
            crate::Event::RunSaveCommand { .. } => "Save".into(),
            crate::Event::RunLoadCommand { .. } => "Load".into(),
            crate::Event::RunDeleteCommand { .. } => "Delete".into(),
            crate::Event::RunExportCommand { .. } => "Export".into(),
            crate::Event::RunImportCommand { .. } => "Import".into(),
            crate::Event::RunLoginCommand { .. } => "Login".into(),
            crate::Event::RunLogoutCommand { .. } => "Logout".into(),
            crate::Event::RunSkillCommand { .. } => "Run Skill".into(),
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
