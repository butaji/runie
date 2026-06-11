//! Command Flow Types

use crate::dialog::{Panel as CorePanel, PanelStack as CoreStack, ItemAction};
use crate::model::AppState;
use crate::Event;

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
    /// Open a panel stack
    PanelStack(fn(&mut AppState, &str) -> CoreStack),
    /// Show a form dialog (always shows dialog, args pre-fill fields)
    Form {
        title: &'static str,
        fields: Vec<super::FormField>,
        submit: Event,
    },
    /// Execute a handler function
    Handler(fn(&mut AppState, &str) -> CommandResult),
    /// Chain multiple flows (tries each until one succeeds)
    Chain(Vec<CommandFlow>),
    /// Conditional flow based on state
    When(fn(&AppState) -> bool, Box<CommandFlow>),
    /// Message or fallback
    OrMessage(fn(&AppState, &str) -> CommandResult, &'static str),
}

impl CommandFlow {
    /// Execute this flow
    pub fn exec(self, state: &mut AppState, cmd_name: &str, args: &str) -> CommandResult {
        match self {
            Self::None => CommandResult::None,
            Self::Message(msg) => CommandResult::Message(msg.into()),
            Self::Dynamic(f) => CommandResult::Message(f(state, args)),
            Self::Dialog(d) => CommandResult::OpenDialog(d),
            Self::PanelStack(f) => CommandResult::OpenPanelStack(f(state, args)),
            Self::Form { title, fields, submit } => {
                let panel = build_form_panel(cmd_name, title, fields, args, submit);
                CommandResult::OpenPanelStack(CoreStack::new(panel))
            }
            Self::Handler(f) => f(state, args),
            Self::Chain(flows) => {
                for flow in flows {
                    let result = flow.clone().exec(state, cmd_name, args);
                    if !matches!(result, CommandResult::None) {
                        return result;
                    }
                }
                CommandResult::None
            }
            Self::When(predicate, flow) => {
                if predicate(state) {
                    flow.exec(state, cmd_name, args)
                } else {
                    CommandResult::None
                }
            }
            Self::OrMessage(handler, fallback) => {
                let result = handler(state, args);
                if matches!(result, CommandResult::None) {
                    CommandResult::Message(fallback.into())
                } else {
                    result
                }
            }
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

/// Command result
#[derive(Debug, Clone, PartialEq)]
pub enum CommandResult {
    Message(String),
    Event(Event),
    OpenDialog(DialogType),
    OpenPanelStack(CoreStack),
    None,
}

impl CommandResult {
    /// Convert to Option<String> for convenience
    pub fn ok(self) -> Option<String> {
        match self {
            Self::Message(s) => Some(s),
            _ => None,
        }
    }

    /// True if this result has an action
    pub fn has_action(&self) -> bool {
        !matches!(self, Self::None)
    }

    /// Map message content
    pub fn map_message<F>(self, f: F) -> Self
    where F: FnOnce(String) -> String {
        match self {
            Self::Message(msg) => Self::Message(f(msg)),
            other => other,
        }
    }
}

/// Build a form panel
fn build_form_panel(id: &str, title: &str, fields: Vec<super::FormField>, args: &str, submit: Event) -> CorePanel {
    let args_list: Vec<&str> = args.split_whitespace().collect();
    let mut panel = CorePanel::new(id, title).with_filter();

    for (i, field) in fields.into_iter().enumerate() {
        let value = args_list.get(i).map(|s| s.to_string())
            .or(field.prefill)
            .unwrap_or_default();

        panel = if value.is_empty() {
            panel.form_field(field.label, field.placeholder, field.key)
        } else {
            panel.form_field_value(field.label, field.placeholder, field.key, value)
        };
    }

    panel.form_submit()
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
        let flow = CommandFlow::Chain(vec![
            CommandFlow::None,
            CommandFlow::Message("fallback"),
        ]);
        let mut state = AppState::default();
        let result = flow.exec(&mut state, "test", "");
        assert!(matches!(result, CommandResult::Message(_)));
    }

    #[test]
    fn test_command_flow_when_true() {
        let flow = CommandFlow::When(
            |_| true,
            Box::new(CommandFlow::Message("shown")),
        );
        let mut state = AppState::default();
        let result = flow.exec(&mut state, "test", "");
        assert_eq!(result, CommandResult::Message("shown".into()));
    }

    #[test]
    fn test_command_flow_when_false() {
        let flow = CommandFlow::When(
            |_| false,
            Box::new(CommandFlow::Message("hidden")),
        );
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
