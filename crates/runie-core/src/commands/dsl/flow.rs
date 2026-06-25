//! Command Flow Types
//!
//! ## Borrow pattern
//! `open_dialog.take()` is used in `exec_sub` to temporarily move the dialog out of
//! `AppState` before executing a sub-dialog. This is a legitimate borrow-conflict
//! workaround: pushing to the back stack requires mutable access to `AppState`.

use crate::dialog::PanelStack;
use crate::model::AppState;
use crate::Event;

/// Type alias for a function that produces a panel stack at runtime.
pub type PanelStackFn = std::sync::Arc<dyn Fn(&mut AppState, &str) -> PanelStack + Send + Sync>;

/// What happens when a command is invoked.
#[derive(Clone)]
pub enum CommandFlow {
    /// No action (handled internally)
    None,
    /// Show a static message
    Message(&'static str),
    /// Execute a handler function
    Handler(fn(&mut AppState, &str) -> CommandResult),
    /// Open a panel stack produced at runtime
    PanelStack(PanelStackFn),
    /// Sub-dialog: push the current dialog (e.g. the command palette
    /// = Main Menu) onto the global back stack before executing the
    /// inner flow. Esc returns to the previous dialog. Only at the
    /// absolute root does Esc close the bar. Android-like.
    Sub(Box<CommandFlow>),
}

impl CommandFlow {
    /// Execute this flow
    pub fn exec(&self, state: &mut AppState, _cmd_name: &str, args: &str) -> CommandResult {
        match self {
            Self::None => CommandResult::None,
            Self::Message(msg) => CommandResult::Message((*msg).into()),
            Self::Handler(f) => f(state, args),
            Self::PanelStack(f) => CommandResult::OpenPanelStack(Box::new(f(state, args))),
            Self::Sub(inner) => Self::exec_sub(inner, state, _cmd_name, args),
        }
    }

    fn exec_sub(
        inner: &CommandFlow,
        state: &mut AppState,
        cmd_name: &str,
        args: &str,
    ) -> CommandResult {
        if let Some(current) = state.open_dialog_mut().take() {
            state.push_dialog_to_back_stack(current);
        }
        inner.exec(state, cmd_name, args)
    }
}

/// Command result
#[derive(Debug, Clone, PartialEq)]
pub enum CommandResult {
    Message(String),
    Warning(String),
    Event(Event),
    OpenDialog(DialogType),
    OpenPanelStack(Box<PanelStack>),
    None,
}

impl CommandResult {
    /// Convert to Option<String> for convenience
    pub fn ok(self) -> Option<String> {
        match self {
            Self::Message(s) | Self::Warning(s) => Some(s),
            _ => None,
        }
    }

    /// True if this result has an action
    pub fn has_action(&self) -> bool {
        !matches!(self, Self::None)
    }

    /// Map message content
    pub fn map_message<F>(self, f: F) -> Self
    where
        F: FnOnce(String) -> String,
    {
        match self {
            Self::Message(msg) => Self::Message(f(msg)),
            Self::Warning(msg) => Self::Warning(f(msg)),
            other => other,
        }
    }
}

/// Dialog types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialogType {
    CommandPalette,
    ModelSelector,
    Settings,
    ScopedModels,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_flow_exec() {
        let flow = CommandFlow::Message("test");
        let mut state = AppState::default();
        let result = flow.exec(&mut state, "test", "");
        assert_eq!(result, CommandResult::Message("test".into()));
    }

    #[test]
    fn test_command_result_ok() {
        let result = CommandResult::Message("hello".into());
        assert_eq!(result.ok(), Some("hello".into()));
        let none = CommandResult::None;
        assert_eq!(none.ok(), None);
    }

    #[test]
    fn test_dialog_type_equality() {
        assert_eq!(DialogType::Settings, DialogType::Settings);
        assert_ne!(DialogType::Settings, DialogType::CommandPalette);
    }
}
