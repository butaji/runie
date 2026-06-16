//! Declarative command specification.
//!
//! Every slash command is defined here as a `CommandSpec` table entry.
//! The spec is used to build a `CommandDef` via `build_cmd()`.

use super::{CommandCategory, CommandDef, CommandResult};
use crate::dialog::PanelStack;
use crate::model::AppState;
use crate::Event;

/// Factory that produces the submit event from collected form values.
pub type FormSubmitFn = fn(&std::collections::HashMap<String, String>) -> Event;

/// What a registered command does.
#[derive(Clone)]
pub enum CommandKind {
    /// Custom handler function.
    Handler(fn(&mut AppState, &str) -> CommandResult),
    /// Form dialog with text fields.
    Form {
        title: &'static str,
        fields: &'static [(&'static str, &'static str, &'static str)],
        submit: FormSubmitFn,
    },
    /// Open a named dialog.
    Dialog(super::DialogType),
    /// Open a panel stack produced at runtime.
    Panel(fn(&mut AppState, &str) -> PanelStack),
    /// Show a static message.
    Msg(&'static str),
}

/// Declarative row for a slash command.
pub struct CommandSpec {
    pub name: &'static str,
    pub desc: &'static str,
    pub aliases: &'static [&'static str],
    pub category: CommandCategory,
    pub sub: bool,
    pub kind: CommandKind,
}

/// Build a `CommandDef` from a declarative spec.
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
        CommandKind::Form {
            title,
            fields,
            submit,
        } => {
            let fields = *fields;
            let submit = *submit;
            cmd.form(title, move |f| add_fields(f, fields).on_submit(submit))
        }
        CommandKind::Dialog(d) => cmd.dialog(d.clone()),
        CommandKind::Panel(f) => cmd.panel(*f),
        CommandKind::Msg(m) => cmd.msg(m),
    }
}

fn add_fields(
    mut builder: crate::dialog::dsl::FormPanel,
    fields: &[(&'static str, &'static str, &'static str)],
) -> crate::dialog::dsl::FormPanel {
    for (label, placeholder, key) in fields {
        builder = builder.field(*label, *placeholder, *key);
    }
    builder
}

/// Register every command in a static table.
pub fn register_commands(
    registry: &mut crate::commands::CommandRegistry,
    commands: &[CommandSpec],
) {
    for spec in commands {
        registry.register(build_cmd(spec));
    }
}
