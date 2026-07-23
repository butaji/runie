//! Legacy command types — kept only for test compatibility.
//!
//! `Command` from `command.rs` is the canonical type.

use crate::dialog::dsl::FormPanel;
use crate::model::AppState;

use super::{CommandCategory, CommandResult};
pub use super::command::{Action, Command, FormHandler};

// Re-exported for yaml.rs

pub type CommandDef = Command;

// Legacy kind enum — used by yaml.rs handler registry lookup.
#[derive(Clone)]
pub enum CommandKind {
    Handler(fn(&mut AppState, &str) -> CommandResult),
    FormWithHandler { title: &'static str, fields: &'static [(&'static str, &'static str, &'static str)], handler: FormHandler },
    Msg(&'static str),
}

impl CommandKind {
    pub fn to_action(&self) -> Action {
        match self {
            CommandKind::Handler(f) => Action::Handler(*f),
            CommandKind::FormWithHandler { title, fields, handler } => Action::Form { title, fields, handler: *handler },
            CommandKind::Msg(m) => Action::Msg(m),
        }
    }
}

#[derive(Clone)]
pub struct CommandSpec {
    pub name: &'static str,
    pub desc: &'static str,
    pub aliases: &'static [&'static str],
    pub category: CommandCategory,
    pub sub: bool,
    pub kind: CommandKind,
}

impl Command {
    /// Convert from legacy `CommandSpec` — used in tests.
    pub fn from_spec(spec: &CommandSpec) -> Self {
        let mut cmd = Command::new(spec.name).desc(spec.desc).aliases(spec.aliases).category(spec.category);
        if spec.sub { cmd = cmd.sub(); }
        if let CommandKind::FormWithHandler { title, fields, handler } = &spec.kind {
            return cmd.form_with_handler(title, |f| add_fields(f, fields), *handler);
        }
        let action = spec.kind.to_action();
        if let Action::Form { handler, .. } = &action { cmd.form_handler = Some(*handler); }
        cmd.action(action)
    }
}

fn add_fields(mut builder: FormPanel, fields: &[(&'static str, &'static str, &'static str)]) -> FormPanel {
    for (label, placeholder, key) in fields { builder = builder.field(*label, *placeholder, *key); }
    builder
}

#[cfg(test)]
mod tests {
    use super::super::CommandFlow;
    use super::*;

    #[test]
    fn spec_can_be_converted_to_command() {
        let spec = CommandSpec { name: "test", desc: "A test", aliases: &[], category: CommandCategory::System, sub: false, kind: CommandKind::Msg("hello") };
        let cmd = Command::from_spec(&spec);
        assert_eq!(cmd.name, "test");
        assert!(matches!(cmd.action, Action::Msg(_)));
    }

    #[test]
    fn spec_can_be_registered() {
        let mut registry = crate::commands::CommandRegistry::new();
        let spec = CommandSpec { name: "check", desc: "Check", aliases: &[], category: CommandCategory::System, sub: false, kind: CommandKind::Handler(|_, _| CommandResult::None) };
        registry.register(Command::from_spec(&spec));
        assert!(registry.get("check").is_some());
    }

    #[test]
    fn cmd_macro_works() {
        let def = super::super::cmd("hello").msg("Hello!");
        assert_eq!(def.name, "hello");
        assert!(matches!(def.action, Action::Msg(_)));
    }

    #[test]
    fn def_builder_chain() {
        let def = Command::new("test").desc("Test").aliases(&["t", "tt"]).category(CommandCategory::System).msg("Test message");
        assert_eq!(def.name, "test");
        assert_eq!(def.aliases, vec!["t", "tt"]);
    }

    #[test]
    fn sub_wraps_flow() {
        let def = Command::new("custom").sub().handler(|_: &mut AppState, _: &str| CommandResult::None);
        assert!(matches!(def.flow(), CommandFlow::Sub(_)));
    }

    #[test]
    fn spec_form_builds_panel_stack() {
        let spec = CommandSpec { name: "save", desc: "Save", aliases: &[], category: CommandCategory::Session, sub: false, kind: CommandKind::FormWithHandler { title: "Save", fields: &[("Name", "session", "name")], handler: |_, _| CommandResult::None } };
        let cmd = Command::from_spec(&spec);
        assert!(cmd.form_handler.is_some());
        assert!(matches!(cmd.flow(), CommandFlow::PanelStack(_)));
    }

    #[test]
    fn slash_command_executes_handler() {
        let spec = CommandSpec { name: "greet", desc: "Greet", aliases: &[], category: CommandCategory::System, sub: false, kind: CommandKind::Handler(|_state, args| CommandResult::Message(format!("Hello, {}!", args))) };
        let cmd = Command::from_spec(&spec);
        let mut state = AppState::default();
        let result = cmd.exec(&mut state, "greet", "world");
        assert!(matches!(result, CommandResult::Message(msg) if msg.contains("world")));
    }
}
