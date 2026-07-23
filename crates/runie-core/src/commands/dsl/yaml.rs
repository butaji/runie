//! Build `Command` from declarative YAML definitions.

use super::{Command, FormHandler};
use crate::declarative::types::{CommandKind as YamlKind, DeclarativeCommandYaml};
use crate::dialog::dsl::FormPanel;

/// Build a `Command` from a YAML definition and handler registry.
pub fn build_cmd_from_yaml(
    yaml: &DeclarativeCommandYaml,
    handler_registry: &super::handlers::registry::HandlerRegistry,
) -> Option<Command> {
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

    match &yaml.kind {
        YamlKind::Handler { handler } => {
            if let Some(kind) = handler_registry.to_command_kind(handler) {
                if let super::spec::CommandKind::FormWithHandler { title, fields, handler: form_handler } = &kind {
                    cmd = cmd.form_with_handler(title, |f| add_fields(f, fields), *form_handler);
                } else {
                    let action = kind.to_action();
                    if let super::spec::Action::Form { handler, .. } = &action {
                        cmd.form_handler = Some(*handler);
                    }
                    cmd = cmd.action(action);
                }
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
                let fields_box: Box<[_]> = fields_vec.into();
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
            cmd = cmd.action(super::spec::Action::Msg(msg));
        }
    }
    Some(cmd)
}

fn get_form_handler_from_kind(kind: &super::spec::CommandKind) -> FormHandler {
    match kind {
        super::spec::CommandKind::FormWithHandler { handler, .. } => *handler,
        super::spec::CommandKind::Handler(f) => *f,
        super::spec::CommandKind::Msg(_) => |_, _| super::CommandResult::None,
    }
}

fn add_fields(mut builder: FormPanel, fields: &[(&'static str, &'static str, &'static str)]) -> FormPanel {
    for (label, placeholder, key) in fields {
        builder = builder.field(*label, *placeholder, *key);
    }
    builder
}
