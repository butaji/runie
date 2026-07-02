//! Unified command specification — bridges legacy `CommandSpec` / `CommandDef` to the new `Command` type.
//!
//! ## Migration Notes
//!
//! This module provides backward compatibility during migration:
//! - `CommandKind` → use `Action` from `command.rs`
//! - `CommandDef` → use `Command` from `command.rs`
//! - `CommandSpec` → still used in some tests; prefer `Command::new()` builder
//!
//! The canonical command representation is now `Command` from `command.rs`.
//! This module will be removed once all consumers migrate to `Command`.

use crate::dialog::dsl::FormPanel;
use crate::model::AppState;

use super::{CommandCategory, CommandResult};

// Re-export the canonical types
pub use super::command::{Action, Command, FormHandler};

// ── Legacy CommandKind (for backward compatibility) ─────────────────────────────

/// What a registered command does — mirrors the variants used in command tables.
/// DEPRECATED: Use `Action` from `command.rs` instead.
#[derive(Clone)]
pub enum CommandKind {
    /// Custom handler function.
    Handler(fn(&mut AppState, &str) -> CommandResult),
    /// Form dialog: open from palette, execute on submit via command registry.
    FormWithHandler {
        title: &'static str,
        fields: &'static [(&'static str, &'static str, &'static str)],
        handler: FormHandler,
    },
    /// Show a static message.
    Msg(&'static str),
}

impl CommandKind {
    /// Convert to `Action`.
    pub fn to_action(&self) -> Action {
        match self {
            CommandKind::Handler(f) => Action::Handler(*f),
            CommandKind::FormWithHandler {
                title,
                fields,
                handler,
            } => Action::Form {
                title,
                fields,
                handler: *handler,
            },
            CommandKind::Msg(m) => Action::Msg(m),
        }
    }
}

// ── Legacy CommandSpec (for backward compatibility in tests) ────────────────────

/// A declarative command row — used in static command tables.
/// All string data is borrowed (no heap allocation in static context).
/// DEPRECATED: Prefer `Command::new()` builder for new code.
#[derive(Clone)]
pub struct CommandSpec {
    pub name: &'static str,
    pub desc: &'static str,
    pub aliases: &'static [&'static str],
    pub category: CommandCategory,
    pub sub: bool,
    pub kind: CommandKind,
}

// ── Legacy CommandDef (re-export of Command for backward compatibility) ─────────

/// A single command definition — runtime-owned version stored in the registry.
/// ALIAS: This is now `Command` from `command.rs`.
pub type CommandDef = Command;

// ── CommandDef Builder Methods ─────────────────────────────────────────────────

impl Command {
    /// Convert from legacy `CommandSpec`.
    pub fn from_spec(spec: &CommandSpec) -> Self {
        let mut cmd = Command::new(spec.name)
            .desc(spec.desc)
            .aliases(spec.aliases)
            .category(spec.category);
        if spec.sub {
            cmd = cmd.sub();
        }
        let action = spec.kind.to_action();
        // Extract form_handler for Form actions
        if let Action::Form { handler, .. } = &action {
            cmd.form_handler = Some(*handler);
        }
        cmd.action(action)
    }
}

// ── Build Functions ─────────────────────────────────────────────────────────────

/// Build a `CommandDef` from a `CommandSpec`.
/// DEPRECATED: Use `Command::from_spec()` instead.
pub fn build_cmd(spec: &CommandSpec) -> CommandDef {
    Command::from_spec(spec)
}

/// Build a `CommandDef` from a YAML definition and handler registry.
pub fn build_cmd_from_yaml(
    yaml: &crate::declarative::types::DeclarativeCommandYaml,
    handler_registry: &super::handlers::registry::HandlerRegistry,
) -> Option<Command> {
    use crate::declarative::types::CommandKind as YamlKind;

    let mut cmd = Command::new(yaml.name.clone())
        .desc(yaml.description.clone())
        .category(yaml.category);
    if !yaml.aliases.is_empty() {
        let aliases: Vec<&str> = yaml.aliases.iter().map(|s| s.as_str()).collect();
        cmd = cmd.aliases(&aliases);
    }

    if yaml.sub {
        cmd = cmd.sub();
    }

    // Look up handler from registry based on YAML kind.
    match &yaml.kind {
        YamlKind::Handler { handler } => {
            if let Some(kind) = handler_registry.to_command_kind(handler) {
                let action = kind.to_action();
                // Extract form_handler for Form actions
                if let Action::Form { handler, .. } = &action {
                    cmd.form_handler = Some(*handler);
                }
                cmd = cmd.action(action);
            }
        }
        YamlKind::FormWithHandler { title, fields, handler } => {
            if let Some(kind) = handler_registry.to_command_kind(handler) {
                let title_static: &'static str = Box::leak(title.clone().into_boxed_str());
                let fields_vec: Vec<(&'static str, &'static str, &'static str)> = fields
                    .iter()
                    .map(|f| {
                        let label: &'static str = Box::leak(f.label.clone().into_boxed_str());
                        let ph: &'static str = Box::leak(f.placeholder.clone().into_boxed_str());
                        let key: &'static str = Box::leak(f.key.clone().into_boxed_str());
                        (label, ph, key)
                    })
                    .collect();
                let fields_box: Box<[(&'static str, &'static str, &'static str)]> =
                    fields_vec.into();
                let fields_ptr: &'static [_] = Box::leak(fields_box);
                cmd = cmd.form_with_handler(
                    title_static,
                    move |f| add_fields(f, fields_ptr),
                    get_form_handler_from_kind(&kind),
                );
            }
        }
        YamlKind::Msg { message } => {
            let msg: &'static str = Box::leak(message.clone().into_boxed_str());
            cmd = cmd.action(Action::Msg(msg));
        }
    }

    Some(cmd)
}

/// Get form handler from CommandKind.
fn get_form_handler_from_kind(kind: &CommandKind) -> FormHandler {
    match kind {
        CommandKind::FormWithHandler { handler, .. } => *handler,
        CommandKind::Handler(f) => *f,
        CommandKind::Msg(_) => |_, _| CommandResult::None,
    }
}

/// Add fields to a form panel.
fn add_fields(
    mut builder: FormPanel,
    fields: &[(&'static str, &'static str, &'static str)],
) -> FormPanel {
    for (label, placeholder, key) in fields {
        builder = builder.field(*label, *placeholder, *key);
    }
    builder
}

// ── Register Commands ──────────────────────────────────────────────────────────

/// Register every command from a spec table.
pub fn register_commands(
    registry: &mut crate::commands::CommandRegistry,
    commands: &[CommandSpec],
) {
    for spec in commands {
        registry.register(Command::from_spec(spec));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::CommandFlow;

    // Layer 1: command_spec_and_def_are_distinct_types
    #[test]
    fn spec_can_be_converted_to_command() {
        let spec = CommandSpec {
            name: "test",
            desc: "A test",
            aliases: &[],
            category: CommandCategory::System,
            sub: false,
            kind: CommandKind::Msg("hello"),
        };
        let cmd = Command::from_spec(&spec);
        assert_eq!(cmd.name, "test");
        assert!(matches!(cmd.action, Action::Msg(_)));
    }

    // Layer 1: all_slash_commands_registered
    #[test]
    fn spec_can_be_registered() {
        let mut registry = crate::commands::CommandRegistry::new();
        let spec = CommandSpec {
            name: "check",
            desc: "Check",
            aliases: &[],
            category: CommandCategory::System,
            sub: false,
            kind: CommandKind::Handler(|_, _| CommandResult::None),
        };
        registry.register(Command::from_spec(&spec));
        assert!(registry.get("check").is_some());
    }

    // Layer 1: cmd_macro_removed
    #[test]
    fn cmd_function_works() {
        let def = super::super::cmd("hello").msg("Hello!");
        assert_eq!(def.name, "hello");
        assert!(matches!(def.action, Action::Msg(_)));
    }

    #[test]
    fn def_builder_chain() {
        let def = Command::new("test")
            .desc("Test command")
            .alias("t")
            .aliases(&["tt", "ttt"])
            .category(CommandCategory::System)
            .msg("Test message");
        assert_eq!(def.name, "test");
        assert_eq!(def.desc, "Test command");
        assert_eq!(def.aliases, vec!["t", "tt", "ttt"]);
    }

    #[test]
    fn sub_is_noop_for_empty_flow() {
        // When .sub() is called before .msg(), the flow is Sub(None)
        // because apply_sub only wraps non-None flows
        let def = Command::new("nothing").sub();
        assert!(def.is_sub);
        // Default action is Msg(""), but .sub() wrapping happens at flow() time
        assert!(matches!(def.flow(), CommandFlow::Sub(_)));
    }

    #[test]
    fn sub_wraps_handler() {
        let def = Command::new("custom")
            .sub()
            .handler(|_: &mut AppState, _: &str| CommandResult::None);
        assert!(matches!(def.flow(), CommandFlow::Sub(_)));
    }

    #[test]
    fn spec_form_builds_panel_stack() {
        let spec = CommandSpec {
            name: "save",
            desc: "Save session",
            aliases: &[],
            category: CommandCategory::Session,
            sub: false,
            kind: CommandKind::FormWithHandler {
                title: "Save",
                fields: &[("Name", "session", "name")],
                handler: |_, _| CommandResult::None,
            },
        };
        let cmd = Command::from_spec(&spec);
        assert!(cmd.form_handler.is_some());
        // Form action should convert to PanelStack flow
        let flow = cmd.flow();
        assert!(matches!(flow, CommandFlow::PanelStack(_)));
    }

    // Layer 2: slash_command_parses_typed_args
    #[test]
    fn slash_command_executes_handler() {
        let spec = CommandSpec {
            name: "greet",
            desc: "Greet",
            aliases: &[],
            category: CommandCategory::System,
            sub: false,
            kind: CommandKind::Handler(|_state, args| {
                CommandResult::Message(format!("Hello, {}!", args))
            }),
        };
        let cmd = Command::from_spec(&spec);
        let mut state = AppState::default();
        let result = cmd.exec(&mut state, "greet", "world");
        assert!(matches!(result, CommandResult::Message(msg) if msg.contains("world")));
    }

    // Layer 1: form_handler_is_accessible
    #[test]
    fn form_handler_available_for_form_with_handler() {
        let spec = CommandSpec {
            name: "save",
            desc: "Save",
            aliases: &[],
            category: CommandCategory::Session,
            sub: false,
            kind: CommandKind::FormWithHandler {
                title: "Save",
                fields: &[("Name", "session", "name")],
                handler: |_, _| CommandResult::None,
            },
        };
        let cmd = Command::from_spec(&spec);
        assert!(cmd.form_handler.is_some());
    }
}
