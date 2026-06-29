//! Registration of declarative commands and skills.

use crate::commands::dsl::{CommandFlow, CommandDef};
use crate::commands::CommandRegistry;
use crate::model::AppState;

use super::types::{CommandDef as DeclarativeCommandDef, SkillDef};

pub fn register_declarative_command(registry: &mut CommandRegistry, cmd: DeclarativeCommandDef) {
    let def = build_def(cmd);
    registry.register(def);
}

pub fn register_declarative_commands(
    registry: &mut CommandRegistry,
    commands: impl IntoIterator<Item = DeclarativeCommandDef>,
) {
    for cmd in commands {
        register_declarative_command(registry, cmd);
    }
}

pub fn register_declarative_skills(
    skills: impl IntoIterator<Item = SkillDef>,
) -> Vec<crate::skills::Skill> {
    skills.into_iter().map(convert_skill_def).collect()
}

// ── Command building ──────────────────────────────────────────────────────────

fn build_def(cmd: DeclarativeCommandDef) -> CommandDef {
    let intent: &'static str = Box::leak(cmd.intent.clone().into_boxed_str());

    let builder = get_event_builder(intent);
    register_intent(intent, builder);

    let is_sub = cmd.has_subcommands;

    let panel_fn = move |_state: &mut AppState, args: &str| {
        let evt = if let Some(build) = get_intent_builder(intent) {
            build(args)
        } else {
            return crate::dialog::PanelStack::new(
                crate::dialog::Panel::new("error", "Error").header("Handler not found"),
            );
        };
        crate::dialog::PanelStack::new(
            crate::dialog::Panel::new("done", "Done").header(format!("Emitted: {:?}", evt)),
        )
    };

    let mut def = CommandDef::new("<declarative>").category(cmd.category);
    def.name = cmd.name;
    def.desc = cmd.description;
    def.flow = CommandFlow::PanelStack(std::sync::Arc::new(panel_fn));
    if is_sub {
        def.is_sub = true;
    }
    def
}

// ── Intent registry ─────────────────────────────────────────────────────────

use std::collections::HashMap;
use std::sync::RwLock;

static INTENT_EVENTS: RwLock<Option<HashMap<&'static str, EventBuilder>>> = RwLock::new(None);

type EventBuilder = fn(args: &str) -> crate::Event;

fn register_intent(intent: &'static str, builder: EventBuilder) {
    let mut intents = INTENT_EVENTS.write().unwrap();
    if intents.is_none() {
        *intents = Some(HashMap::new());
    }
    intents.as_mut().unwrap().insert(intent, builder);
}

fn get_intent_builder(intent: &str) -> Option<EventBuilder> {
    let intents = INTENT_EVENTS.read().unwrap();
    intents.as_ref()?.get(intent).copied()
}

fn get_event_builder(intent: &str) -> EventBuilder {
    match intent {
        "SaveCommand" => |args| crate::Event::RunSaveCommand { name: args.to_owned() },
        "LoadCommand" => |args| crate::Event::RunLoadCommand { name: args.to_owned() },
        "DeleteCommand" => |args| crate::Event::RunDeleteCommand { name: args.to_owned() },
        "ExportCommand" => |args| crate::Event::RunExportCommand { path: args.to_owned() },
        "ImportCommand" => |args| crate::Event::RunImportCommand { path: args.to_owned() },
        "ForkCommand" => |args| crate::Event::RunForkCommand { message_index: args.to_owned() },
        "CompactCommand" => |args| {
            let (keep, focus) = parse_compact_args(args);
            crate::Event::RunCompactCommand { keep, focus }
        },
        "NameCommand" => |args| crate::Event::RunNameCommand { name: args.to_owned() },
        "PromptCommand" => |args| crate::Event::RunPromptCommand { name: args.to_owned() },
        "SkillCommand" => |args| crate::Event::RunSkillCommand { name: args.to_owned() },
        "ModelCommand" => |_args| crate::Event::RunPaletteCommand { name: "model".into(), args: String::new() },
        "ThinkingCommand" => |_args| crate::Event::RunPaletteCommand { name: "thinking".into(), args: String::new() },
        "SwitchTheme" => |args| crate::Event::SwitchTheme { name: args.to_owned() },
        "CopyToClipboard" => |args| crate::Event::CopyToClipboard(args.to_owned()),
        "ClearQueues" => |_args| crate::Event::ClearQueues,
        "ToggleSettingsDialog" => |_args| crate::Event::ToggleSettingsDialog,
        "ShowDiagnostics" => |_args| crate::Event::ShowDiagnostics,
        "ToggleSessionTree" => |_args| crate::Event::ToggleSessionTree,
        "ShareSession" => |_args| crate::Event::ShareSession,
        "ApproveEdit" => |_args| crate::Event::ApproveEdit,
        "RejectEdit" => |_args| crate::Event::RejectEdit,
        "ReloadAll" => |_args| crate::Event::ReloadAll,
        "ProvidersDialog" => |_args| crate::Event::ProvidersDialog,
        _ => |_args| crate::Event::ShowDiagnostics,
    }
}

fn parse_compact_args(args: &str) -> (String, String) {
    let parts: Vec<_> = args.split_whitespace().collect();
    match parts.len() {
        0 => ("2000".to_owned(), String::new()),
        1 => (parts[0].to_owned(), String::new()),
        _ => (parts[0].to_owned(), parts[1..].join(" ")),
    }
}

// ── Skill conversion ─────────────────────────────────────────────────────────

fn convert_skill_def(def: SkillDef) -> crate::skills::Skill {
    crate::skills::Skill {
        name: def.name,
        description: def.description,
        context: def.context.unwrap_or_default(),
        user_invocable: def.user_invocable,
        file_path: def.file_path,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_compact_args_parsing() {
        assert_eq!(parse_compact_args(""), ("2000".to_owned(), String::new()));
        assert_eq!(parse_compact_args("1000"), ("1000".to_owned(), String::new()));
        assert_eq!(parse_compact_args("1000 focus"), ("1000".to_owned(), "focus".to_owned()));
    }

    #[test]
    fn test_register_command_creates_def() {
        let temp_dir = tempdir().unwrap();
        let cmd_path = temp_dir.path().join("test.yaml");
        std::fs::write(
            &cmd_path,
            "name: test-cmd\ndescription: Test command\nintent: SaveCommand\ncategory: Session\n",
        )
        .unwrap();

        let cmd = crate::declarative::loader::parse_command_yaml(&cmd_path).unwrap();
        let mut registry = crate::commands::CommandRegistry::new();
        register_declarative_command(&mut registry, cmd);

        let found = registry.get("test-cmd");
        assert!(found.is_some());
        assert_eq!(found.unwrap().desc, "Test command");
    }

    #[test]
    fn test_event_builder_for_save_command() {
        let builder = get_event_builder("SaveCommand");
        let evt = builder("my-session");
        assert!(matches!(evt, crate::Event::RunSaveCommand { .. }));
    }

    #[test]
    fn test_event_builder_for_compact() {
        let builder = get_event_builder("CompactCommand");
        let evt = builder("2000 auth");
        if let crate::Event::RunCompactCommand { keep, focus } = evt {
            assert_eq!(keep, "2000");
            assert_eq!(focus, "auth");
        } else {
            panic!("Expected RunCompactCommand");
        }
    }
}
