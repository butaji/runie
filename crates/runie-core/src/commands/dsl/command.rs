//! Unified command representation — single `Command` struct with `Action` enum.
//!
//! ## Design
//!
//! - `Command` is the canonical runtime representation stored in the registry.
//! - `Action` is the enum describing what the command does.
//! - YAML deserialization uses `declarative::types::CommandKind` which converts to `Action`.
//!
//! This replaces the previous dual-representation pattern:
//! - Old: `CommandSpec` (static) + `CommandDef` (runtime) + `declarative::types::CommandDef` (YAML)
//! - New: Single `Command` struct.

use crate::dialog::dsl::FormPanel;
use crate::dialog::PanelStack as CoreStack;
use crate::model::AppState;

use super::{CommandCategory, CommandFlow, CommandResult};

/// Handler for form submissions.
pub type FormHandler = fn(&mut AppState, &str) -> CommandResult;

/// What a command does — replaces both `dsl::spec::CommandKind` and `declarative::types::CommandKind`.
#[derive(Clone)]
pub enum Action {
    /// Execute a custom handler function.
    Handler(fn(&mut AppState, &str) -> CommandResult),
    /// Form dialog with a handler: opens a form, executes handler on submit.
    Form {
        title: &'static str,
        fields: &'static [(&'static str, &'static str, &'static str)],
        handler: FormHandler,
    },
    /// Show a static message.
    Msg(&'static str),
    /// Open a panel stack (for complex dialogs).
    Panel {
        builder: std::sync::Arc<dyn Fn(&mut AppState, &str) -> CoreStack + Send + Sync>,
    },
}

impl Action {
    /// Convert to `CommandFlow`.
    pub fn to_flow(&self) -> CommandFlow {
        match self {
            Action::Handler(f) => CommandFlow::Handler(*f),
            Action::Form {
                title,
                fields,
                handler,
            } => {
                let title = *title;
                let fields = *fields;
                let handler = *handler;
                CommandFlow::PanelStack(std::sync::Arc::new(move |state, args| {
                    build_form_stack(state, title, fields, handler, args)
                }))
            }
            Action::Msg(m) => CommandFlow::Message((*m).to_string()),
            Action::Panel { builder } => CommandFlow::PanelStack(builder.clone()),
        }
    }
}

/// The canonical command representation — replaces `CommandSpec`, `CommandDef`,
/// and the duplicate `declarative::types::CommandDef`.
#[derive(Clone)]
pub struct Command {
    pub name: String,
    pub desc: String,
    pub aliases: Vec<String>,
    pub category: CommandCategory,
    pub action: Action,
    /// Handler for form submissions (called by `dispatch_form_to_registry`).
    pub form_handler: Option<FormHandler>,
    /// Whether this command opens a sub-dialog (current dialog pushed to back stack).
    pub is_sub: bool,
}

impl Command {
    /// Create a new command with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            desc: String::new(),
            aliases: Vec::new(),
            category: CommandCategory::System,
            action: Action::Msg(""),
            form_handler: None,
            is_sub: false,
        }
    }

    /// Set the description.
    pub fn desc(mut self, desc: impl Into<String>) -> Self {
        self.desc = desc.into();
        self
    }

    /// Add an alias.
    pub fn alias(mut self, alias: impl Into<String>) -> Self {
        self.aliases.push(alias.into());
        self
    }

    /// Set multiple aliases.
    pub fn aliases(mut self, aliases: &[&str]) -> Self {
        for s in aliases {
            self.aliases.push((*s).to_string());
        }
        self
    }

    /// Set the category.
    pub fn category(mut self, cat: CommandCategory) -> Self {
        self.category = cat;
        self
    }

    /// Set the action.
    pub fn action(mut self, action: Action) -> Self {
        self.action = action;
        self
    }

    /// Shortcut: set a message action.
    pub fn msg(self, msg: impl Into<String>) -> Self {
        self.action(Action::Msg(Box::leak(msg.into().into_boxed_str())))
    }

    /// Shortcut: set a handler action.
    pub fn handler(self, f: fn(&mut AppState, &str) -> CommandResult) -> Self {
        self.action(Action::Handler(f))
    }

    /// Shortcut: set a form action with handler.
    pub fn form_with_handler<Build>(
        mut self,
        title: &'static str,
        form_builder: Build,
        handler: FormHandler,
    ) -> Self
    where
        Build: FnOnce(FormPanel) -> FormPanel + Send + Sync + 'static,
    {
        let id = self.name.clone();
        let template =
            form_builder(crate::dialog::dsl::form(id, title).cmd_name(self.name.clone()));
        let builder = std::sync::Arc::new(move |_state: &mut AppState, args: &str| {
            build_form_stack_from_template(template.clone(), args)
        });
        self.action = Action::Panel { builder };
        self.form_handler = Some(handler);
        self
    }

    /// Mark this as a sub-dialog command.
    pub fn sub(mut self) -> Self {
        self.is_sub = true;
        self
    }

    /// Execute this command's action.
    pub fn exec(&self, state: &mut AppState, name: &str, args: &str) -> CommandResult {
        let flow = self.action.to_flow();
        if self.is_sub {
            let sub_flow = CommandFlow::Sub(Box::new(flow));
            sub_flow.exec(state, name, args)
        } else {
            flow.exec(state, name, args)
        }
    }

    /// Get the flow for this command.
    pub fn flow(&self) -> CommandFlow {
        let flow = self.action.to_flow();
        if self.is_sub {
            CommandFlow::Sub(Box::new(flow))
        } else {
            flow
        }
    }
}

/// Shorthand for creating commands — equivalent to `Command::new(name)`.
pub fn cmd(name: &'static str) -> Command {
    Command::new(name)
}

// ── Form Stack Builders ────────────────────────────────────────────────────────

fn build_form_stack(
    _state: &mut AppState,
    title: &'static str,
    fields: &'static [(&'static str, &'static str, &'static str)],
    _handler: FormHandler,
    args: &str,
) -> CoreStack {
    let args_list: Vec<&str> = args.split_whitespace().collect();
    let mut panel = crate::dialog::Panel::new("form", title).form();
    for (arg_idx, (label, placeholder, key)) in fields.iter().enumerate() {
        let val = if arg_idx < args_list.len() {
            args_list[arg_idx].to_owned()
        } else {
            String::new()
        };
        panel = panel.form_field_value(*label, *placeholder, *key, val);
    }
    CoreStack::new(panel)
}

fn build_form_stack_from_template(template: FormPanel, args: &str) -> CoreStack {
    let args_list: Vec<&str> = args.split_whitespace().collect();
    let built = template.build();
    let mut panel = crate::dialog::Panel::new(built.id, built.title).form();
    panel.cmd_name = built.cmd_name;
    panel.field_keys = built.field_keys;
    let mut arg_idx = 0;
    for item in built.items {
        match item {
            crate::dialog::PanelItem::FormField {
                label,
                placeholder,
                key,
                value,
                ..
            } => {
                let val = if arg_idx < args_list.len() {
                    args_list[arg_idx].to_owned()
                } else {
                    value
                };
                panel = panel.form_field_value(label, placeholder, key, val);
                arg_idx += 1;
            }
            crate::dialog::PanelItem::FormSubmit => panel = panel.form_submit(),
            _ => {}
        }
    }
    CoreStack::new(panel)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // Layer 1: command_has_single_representation
    #[test]
    fn command_builder_creates_message_command() {
        let cmd = Command::new("hello")
            .desc("A test")
            .alias("h")
            .category(CommandCategory::System)
            .msg("Hello!");
        assert_eq!(cmd.name, "hello");
        assert!(matches!(cmd.action, Action::Msg(_)));
        assert_eq!(cmd.aliases, vec!["h"]);
    }

    // Layer 1: cmd_shorthand_works
    #[test]
    fn cmd_function_creates_command() {
        let cmd = super::cmd("hello").msg("Hello!");
        assert_eq!(cmd.name, "hello");
        assert!(matches!(cmd.action, Action::Msg(_)));
    }

    // Layer 1: command_executes_handler
    #[test]
    fn command_exec_executes_handler() {
        let cmd = Command::new("greet")
            .handler(|_state, args| CommandResult::Message(format!("Hello, {}!", args)));
        let mut state = AppState::default();
        let result = cmd.exec(&mut state, "greet", "world");
        assert!(matches!(result, CommandResult::Message(msg) if msg.contains("world")));
    }

    // Layer 1: command_with_sub_executes_sub_flow
    #[test]
    fn command_sub_wraps_flow() {
        let cmd = Command::new("custom")
            .sub()
            .handler(|_: &mut AppState, _: &str| CommandResult::None);
        let flow = cmd.flow();
        assert!(matches!(flow, CommandFlow::Sub(_)));
    }

    // Layer 1: command_form_has_handler
    #[test]
    fn command_form_provides_handler() {
        let cmd = Command::new("save")
            .form_with_handler(
                "Save",
                |f| f.field("Name", "session", "name"),
                |_, _| CommandResult::None,
            );
        assert!(cmd.form_handler.is_some());
    }

    // Layer 1: command_builder_chain
    #[test]
    fn command_builder_chain_works() {
        let cmd = Command::new("test")
            .desc("Test command")
            .alias("t")
            .aliases(&["tt", "ttt"])
            .category(CommandCategory::System)
            .msg("Test message");
        assert_eq!(cmd.name, "test");
        assert_eq!(cmd.desc, "Test command");
        assert_eq!(cmd.aliases, vec!["t", "tt", "ttt"]);
    }

    // Layer 1: action_to_flow
    #[test]
    fn action_converts_to_flow() {
        let action = Action::Msg("test");
        let flow = action.to_flow();
        assert!(matches!(flow, CommandFlow::Message(_)));
    }

    // Layer 2: slash_command_executes_handler
    #[test]
    fn slash_command_executes_handler_via_exec() {
        let cmd = Command::new("greet")
            .handler(|_state, args| CommandResult::Message(format!("Hello, {}!", args)));
        let mut state = AppState::default();
        let result = cmd.exec(&mut state, "greet", "world");
        assert!(matches!(result, CommandResult::Message(msg) if msg.contains("world")));
    }

    // Layer 1: sub_is_noop_for_empty_action
    #[test]
    fn sub_is_noop_for_msg() {
        let cmd = Command::new("nothing").msg("nothing");
        let flow = cmd.flow();
        assert!(matches!(flow, CommandFlow::Message(_)));
    }
}
