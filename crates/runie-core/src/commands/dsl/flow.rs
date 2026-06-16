//! Command Flow Types

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
    /// Show a dynamic message computed at runtime
    Dynamic(fn(&AppState, &str) -> String),
    /// Open a named dialog
    Dialog(DialogType),
    /// Open a panel stack produced at runtime
    PanelStack(PanelStackFn),
    /// Execute a handler function
    Handler(fn(&mut AppState, &str) -> CommandResult),
    /// Chain multiple flows (tries each until one succeeds)
    Chain(Vec<CommandFlow>),
    /// Conditional flow based on state
    When(fn(&AppState) -> bool, Box<CommandFlow>),
    /// Message or fallback
    OrMessage(fn(&AppState, &str) -> CommandResult, &'static str),
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
            Self::Dynamic(f) => CommandResult::Message(f(state, args)),
            Self::Dialog(d) => CommandResult::OpenDialog(d.clone()),
            Self::PanelStack(f) => CommandResult::OpenPanelStack(Box::new(f(state, args))),
            Self::Handler(f) => f(state, args),
            Self::Chain(flows) => Self::exec_chain(flows, state, _cmd_name, args),
            Self::When(predicate, flow) => {
                Self::exec_when(*predicate, flow, state, _cmd_name, args)
            }
            Self::OrMessage(handler, fallback) => {
                Self::exec_or_message(*handler, fallback, state, args)
            }
            Self::Sub(inner) => Self::exec_sub(inner, state, _cmd_name, args),
        }
    }

    fn exec_chain(
        flows: &[CommandFlow],
        state: &mut AppState,
        cmd_name: &str,
        args: &str,
    ) -> CommandResult {
        for flow in flows {
            let result = flow.exec(state, cmd_name, args);
            if !matches!(result, CommandResult::None) {
                return result;
            }
        }
        CommandResult::None
    }

    fn exec_when(
        predicate: fn(&AppState) -> bool,
        flow: &CommandFlow,
        state: &mut AppState,
        cmd_name: &str,
        args: &str,
    ) -> CommandResult {
        if predicate(state) {
            flow.exec(state, cmd_name, args)
        } else {
            CommandResult::None
        }
    }

    fn exec_or_message(
        handler: fn(&AppState, &str) -> CommandResult,
        fallback: &'static str,
        state: &AppState,
        args: &str,
    ) -> CommandResult {
        let result = handler(state, args);
        if matches!(result, CommandResult::None) {
            CommandResult::Message(fallback.into())
        } else {
            result
        }
    }

    fn exec_sub(
        inner: &CommandFlow,
        state: &mut AppState,
        cmd_name: &str,
        args: &str,
    ) -> CommandResult {
        if let Some(current) = state.open_dialog.take() {
            state.push_dialog_to_back_stack(current);
        }
        inner.exec(state, cmd_name, args)
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

/// Build a form panel for the /spawn command when called without arguments.
/// Collects the prompt via a single text field.
fn spawn_submit(values: &std::collections::HashMap<String, String>) -> crate::Event {
    crate::event::ControlEvent::SpawnAgent {
        prompt: values.get("prompt").cloned().unwrap_or_default(),
    }
}

pub fn build_spawn_form_panel() -> PanelStack {
    use crate::dialog::dsl::form;
    form("spawn", "Spawn Subagent")
        .field("Prompt", "Describe the task for the subagent", "prompt")
        .on_submit(spawn_submit)
        .into_stack()
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
    fn test_command_flow_chain() {
        let flow = CommandFlow::Chain(vec![CommandFlow::None, CommandFlow::Message("fallback")]);
        let mut state = AppState::default();
        let result = flow.exec(&mut state, "test", "");
        assert!(matches!(result, CommandResult::Message(_)));
    }

    #[test]
    fn test_command_flow_when_true() {
        let flow = CommandFlow::When(|_| true, Box::new(CommandFlow::Message("shown")));
        let mut state = AppState::default();
        let result = flow.exec(&mut state, "test", "");
        assert_eq!(result, CommandResult::Message("shown".into()));
    }

    #[test]
    fn test_command_flow_when_false() {
        let flow = CommandFlow::When(|_| false, Box::new(CommandFlow::Message("hidden")));
        let mut state = AppState::default();
        let result = flow.exec(&mut state, "test", "");
        assert_eq!(result, CommandResult::None);
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
