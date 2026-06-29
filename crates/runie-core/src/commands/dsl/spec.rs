//! Unified command specification — single representation for all slash commands.
//!
//! `CommandSpec` is the static struct used in command tables (no heap allocation).
//! `CommandDef` is the runtime-owned version used by the registry.
//! `build_cmd()` converts a `CommandSpec` to a `CommandDef`.

#[cfg(test)]
use std::collections::HashMap;

use crate::dialog::dsl::FormPanel;
use crate::dialog::PanelStack as CoreStack;
use crate::model::AppState;

use super::{CommandCategory, CommandFlow, CommandResult};

/// Handler for form submissions.
pub type FormHandler = fn(&mut AppState, &str) -> CommandResult;

/// What a registered command does — mirrors the variants used in command tables.
#[derive(Clone)]
pub enum CommandKind {
    /// Custom handler function.
    Handler(fn(&mut AppState, &str) -> CommandResult),
    /// Form dialog: args pre-fill fields, `.on_submit` runs when submitted.
    Form {
        title: &'static str,
        fields: &'static [(&'static str, &'static str, &'static str)],
        submit: fn(&std::collections::HashMap<String, String>) -> crate::Event,
    },
    /// Form dialog with a separate submission handler (open from palette, execute on submit).
    FormWithHandler {
        title: &'static str,
        fields: &'static [(&'static str, &'static str, &'static str)],
        handler: FormHandler,
    },
    /// Show a static message.
    Msg(&'static str),
}

/// A declarative command row — used in static command tables.
/// All string data is borrowed (no heap allocation in static context).
#[derive(Clone)]
pub struct CommandSpec {
    pub name: &'static str,
    pub desc: &'static str,
    pub aliases: &'static [&'static str],
    pub category: CommandCategory,
    pub sub: bool,
    pub kind: CommandKind,
}

// ── CommandDef — runtime-owned version ─────────────────────────────────────────

use crate::dialog::dsl::form as make_form;

/// A single command definition — runtime-owned version stored in the registry.
#[derive(Clone)]
pub struct CommandDef {
    pub name: String,
    pub desc: String,
    pub aliases: Vec<String>,
    pub category: CommandCategory,
    pub flow: CommandFlow,
    /// Handler for form submissions (called by `dispatch_form_to_registry`).
    pub form_handler: Option<FormHandler>,
    /// Whether this command opens a sub-dialog (current dialog pushed to back stack).
    pub is_sub: bool,
}

impl CommandDef {
    pub fn new(name: &'static str) -> Self {
        Self {
            name: name.into(),
            desc: String::new(),
            aliases: Vec::new(),
            category: CommandCategory::System,
            flow: CommandFlow::None,
            form_handler: None,
            is_sub: false,
        }
    }

    pub fn desc(mut self, desc: &'static str) -> Self {
        self.desc = desc.into();
        self
    }

    pub fn alias(mut self, alias: &'static str) -> Self {
        self.aliases.push(alias.into());
        self
    }

    pub fn aliases(mut self, aliases: &'static [&'static str]) -> Self {
        self.aliases.extend(aliases.iter().map(|s| s.to_string()));
        self
    }

    pub fn category(mut self, cat: CommandCategory) -> Self {
        self.category = cat;
        self
    }

    pub fn msg(self, msg: &'static str) -> Self {
        self.with_flow(CommandFlow::Message(msg)).apply_sub()
    }

    pub fn handler(self, f: fn(&mut AppState, &str) -> CommandResult) -> Self {
        self.with_flow(CommandFlow::Handler(f)).apply_sub()
    }

    pub fn panel<F>(self, f: F) -> Self
    where
        F: Fn(&mut AppState, &str) -> CoreStack + Send + Sync + 'static,
    {
        self.with_flow(CommandFlow::PanelStack(std::sync::Arc::new(f)))
            .apply_sub()
    }

    pub fn sub(mut self) -> Self {
        self.is_sub = true;
        self
    }

    pub fn form<F>(self, title: &'static str, build: F) -> Self
    where
        F: FnOnce(FormPanel) -> FormPanel + Send + Sync + 'static,
    {
        let id = self.name.clone();
        let template = build(make_form(id, title));
        self.panel(move |_state, args| build_form_stack_from_template(template.clone(), args))
    }

    pub fn form_with_handler<Build>(self, title: &'static str, form_builder: Build, handler: FormHandler) -> Self
    where
        Build: FnOnce(FormPanel) -> FormPanel + Send + Sync + 'static,
    {
        let id = self.name.clone();
        let template = form_builder(crate::dialog::dsl::form(id, title));
        self.panel(move |_state, args| build_form_stack_from_template(template.clone(), args))
            .with_form_handler(handler)
    }

    pub fn with_form_handler(mut self, handler: FormHandler) -> Self {
        self.form_handler = Some(handler);
        self
    }

    fn with_flow(mut self, flow: CommandFlow) -> Self {
        self.flow = flow;
        self
    }

    fn apply_sub(mut self) -> Self {
        if self.is_sub && !matches!(self.flow, CommandFlow::None) {
            let inner = std::mem::replace(&mut self.flow, CommandFlow::None);
            self.flow = CommandFlow::Sub(Box::new(inner));
        }
        self
    }

    /// Execute this command's flow.
    pub fn exec(&self, state: &mut AppState, name: &str, args: &str) -> CommandResult {
        self.flow.exec(state, name, args)
    }
}

/// Build a `CommandDef` from a `CommandSpec`.
pub fn build_cmd(spec: &CommandSpec) -> CommandDef {
    let mut cmd = CommandDef::new(spec.name)
        .desc(spec.desc)
        .aliases(spec.aliases)
        .category(spec.category);
    if spec.sub {
        cmd = cmd.sub();
    }
    match &spec.kind {
        CommandKind::Handler(f) => cmd.handler(*f),
        CommandKind::Form { title, fields, submit } => {
            let fields = *fields;
            let submit = *submit;
            let name = spec.name;
            cmd.form(title, move |f| add_fields(f, fields).on_submit(submit).cmd_name(name))
        }
        CommandKind::FormWithHandler { title, fields, handler } => {
            let fields = *fields;
            let handler = *handler;
            let name = spec.name;
            cmd.form_with_handler(
                title,
                move |f| add_fields(f, fields).cmd_name(name),
                handler,
            )
        }
        CommandKind::Msg(m) => cmd.msg(m),
    }
}

fn add_fields(
    mut builder: FormPanel,
    fields: &[(&'static str, &'static str, &'static str)],
) -> FormPanel {
    for (label, placeholder, key) in fields {
        builder = builder.field(*label, *placeholder, *key);
    }
    builder
}

fn build_form_stack_from_template(template: FormPanel, args: &str) -> CoreStack {
    let args_list: Vec<&str> = args.split_whitespace().collect();
    let built = template.build();
    let mut panel = crate::dialog::Panel::new(built.id, built.title).form();
    panel.submit_factory = built.submit_factory;
    panel.cmd_name = built.cmd_name;
    panel.field_keys = built.field_keys;
    let mut arg_idx = 0;
    for item in built.items {
        match item {
            crate::dialog::PanelItem::FormField { label, placeholder, key, value, .. } => {
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

/// Register every command from a spec table.
pub fn register_commands(registry: &mut crate::commands::CommandRegistry, commands: &[CommandSpec]) {
    for spec in commands {
        registry.register(build_cmd(spec));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Layer 1: command_registry_has_single_representation
    #[test]
    fn command_spec_and_def_are_distinct_types() {
        let spec = CommandSpec {
            name: "test",
            desc: "A test",
            aliases: &[],
            category: CommandCategory::System,
            sub: false,
            kind: CommandKind::Msg("hello"),
        };
        let def = build_cmd(&spec);
        assert_eq!(def.name, "test");
        assert!(matches!(def.flow, CommandFlow::Message(_)));
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
        registry.register(build_cmd(&spec));
        assert!(registry.get("check").is_some());
    }

    // Layer 1: cmd_macro_removed
    #[test]
    fn cmd_function_works() {
        let def = super::super::cmd("hello").msg("Hello!");
        assert_eq!(def.name, "hello");
        assert!(matches!(def.flow, CommandFlow::Message(_)));
    }

    #[test]
    fn def_builder_chain() {
        let def = CommandDef::new("test")
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
        let def = CommandDef::new("nothing").sub();
        assert!(matches!(def.flow, CommandFlow::None));
    }

    #[test]
    fn sub_wraps_handler() {
        let def = CommandDef::new("custom")
            .sub()
            .handler(|_: &mut AppState, _: &str| CommandResult::None);
        assert!(matches!(def.flow, CommandFlow::Sub(_)));
    }

    fn save_submit(values: &HashMap<String, String>) -> crate::Event {
        crate::Event::RunSaveCommand { name: crate::dialog::dsl::get_field(values, "name") }
    }

    #[test]
    fn build_cmd_form_builds_panel_stack() {
        let spec = CommandSpec {
            name: "save",
            desc: "Save session",
            aliases: &[],
            category: CommandCategory::Session,
            sub: false,
            kind: CommandKind::Form {
                title: "Save",
                fields: &[("Name", "session", "name")],
                submit: save_submit,
            },
        };
        let def = build_cmd(&spec);
        assert!(matches!(def.flow, CommandFlow::PanelStack(_)));
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
            kind: CommandKind::Handler(|state, args| {
                CommandResult::Message(format!("Hello, {}!", args))
            }),
        };
        let def = build_cmd(&spec);
        let mut state = AppState::default();
        let result = def.exec(&mut state, "greet", "world");
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
        let def = build_cmd(&spec);
        assert!(def.form_handler.is_some());
    }
}
