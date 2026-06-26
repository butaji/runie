//! System commands.

use crate::commands::{CommandCategory, CommandRegistry, CommandResult, DialogType};
use crate::dialog::{ItemAction, Panel, PanelStack};
use crate::model::AppState;

use super::spec::{CommandKind, CommandSpec};

static SYSTEM_COMMANDS: &[CommandSpec] = &[
    CommandSpec {
        name: "settings",
        desc: "Open settings dialog",
        aliases: &[],
        category: CommandCategory::System,
        sub: true,
        kind: CommandKind::Handler(handle_settings),
    },
    CommandSpec {
        name: "copy",
        desc: "Copy last response to clipboard",
        aliases: &[],
        category: CommandCategory::System,
        sub: false,
        kind: CommandKind::Handler(handle_copy),
    },
    CommandSpec {
        name: "reload",
        desc: "Reload config, keybindings, themes",
        aliases: &[],
        category: CommandCategory::System,
        sub: false,
        kind: CommandKind::Handler(handle_reload),
    },
    CommandSpec {
        name: "diagnostics",
        desc: "Show resource loading diagnostics",
        aliases: &[],
        category: CommandCategory::System,
        sub: false,
        kind: CommandKind::Handler(handle_diagnostics),
    },
    CommandSpec {
        name: "skills",
        desc: "List loaded skills",
        aliases: &[],
        category: CommandCategory::System,
        sub: false,
        kind: CommandKind::Handler(handle_skills),
    },
    CommandSpec {
        name: "skill",
        desc: "Show skill details",
        aliases: &[],
        category: CommandCategory::System,
        sub: false,
        kind: CommandKind::Handler(handle_skill),
    },
    CommandSpec {
        name: "prompt",
        desc: "Switch prompt template",
        aliases: &[],
        category: CommandCategory::System,
        sub: true,
        kind: CommandKind::Handler(handle_prompt),
    },
    CommandSpec {
        name: "hotkeys",
        desc: "Show keyboard shortcuts",
        aliases: &["keys", "shortcuts"],
        category: CommandCategory::System,
        sub: true,
        kind: CommandKind::Handler(handle_hotkeys),
    },
    CommandSpec {
        name: "theme",
        desc: "Switch theme or list available themes",
        aliases: &[],
        category: CommandCategory::System,
        sub: true,
        kind: CommandKind::Handler(handle_theme),
    },
    CommandSpec {
        name: "approve",
        desc: "Apply pending file edits",
        aliases: &[],
        category: CommandCategory::System,
        sub: false,
        kind: CommandKind::Handler(handle_approve),
    },
    CommandSpec {
        name: "reject",
        desc: "Cancel pending file edits",
        aliases: &[],
        category: CommandCategory::System,
        sub: false,
        kind: CommandKind::Handler(handle_reject),
    },
    CommandSpec {
        name: "provider",
        desc: "Manage providers: add, disconnect, choose models",
        aliases: &["providers"],
        category: CommandCategory::System,
        sub: true,
        kind: CommandKind::Handler(handle_providers),
    },
];

pub fn register(registry: &mut CommandRegistry) {
    super::spec::register_commands(registry, SYSTEM_COMMANDS);
}

fn handle_copy(state: &mut AppState, _: &str) -> CommandResult {
    let text = state
        .session
        .messages
        .iter()
        .rev()
        .find(|m| m.role == crate::model::Role::Assistant)
        .map(|m| m.content())
        .unwrap_or_default();
    if text.is_empty() {
        return CommandResult::Message("No assistant response to copy".into());
    }
    CommandResult::Event(crate::Event::CopyToClipboard(text))
}

fn handle_reload(_state: &mut AppState, _: &str) -> CommandResult {
    // Emit ReloadAll event which will be handled by ConfigActor and other actors
    CommandResult::Event(crate::Event::ReloadAll)
}

fn handle_settings(_state: &mut AppState, _: &str) -> CommandResult {
    // Emit intent event; owning actor handles opening the dialog
    CommandResult::Event(crate::Event::ToggleSettingsDialog)
}

fn handle_diagnostics(_: &mut AppState, _: &str) -> CommandResult {
    CommandResult::Event(crate::Event::ShowDiagnostics)
}

fn handle_skills(state: &mut AppState, _: &str) -> CommandResult {
    if state.skills().is_empty() {
        return CommandResult::Warning("No skills loaded.".into());
    }
    let lines: Vec<_> = std::iter::once("Loaded skills:".into())
        .chain(state.skills().iter().map(|s| format!("  {}", s.summary())))
        .collect();
    CommandResult::Message(lines.join("\n"))
}

fn handle_skill(state: &mut AppState, args: &str) -> CommandResult {
    let name = args.trim();
    if name.is_empty() {
        use crate::dialog::dsl::form;
        let stack = form("skill", "Show Skill")
            .field("Name", "skill-name", "name")
            .on_submit(|values| crate::Event::RunSkillCommand {
                name: crate::dialog::dsl::get_field(values, "name"),
            })
            .into_stack();
        return CommandResult::OpenPanelStack(Box::new(stack));
    }
    match state.skills().iter().find(|s| s.name == name) {
        Some(skill) => {
            let mut lines = vec![format!("Skill: {}", skill.name)];
            if !skill.description.is_empty() {
                lines.push(format!("Description: {}", skill.description));
            }
            if !skill.context.is_empty() {
                lines.push(format!("Context: {}", skill.context));
            }
            CommandResult::Message(lines.join("\n"))
        }
        None => CommandResult::Message(format!("Skill '{}' not found. Use /skills.", name)),
    }
}

fn handle_theme(_state: &mut AppState, args: &str) -> CommandResult {
    let name = args.trim();
    if name.is_empty() {
        return CommandResult::OpenDialog(DialogType::ThemeSelector);
    }
    // Emit SwitchTheme event; handler will validate and show error if invalid
    CommandResult::Event(crate::Event::SwitchTheme { name: name.to_owned() })
}

fn handle_approve(_: &mut AppState, _: &str) -> CommandResult {
    CommandResult::Event(crate::Event::ApproveEdit)
}

fn handle_reject(_: &mut AppState, _: &str) -> CommandResult {
    CommandResult::Event(crate::Event::RejectEdit)
}

fn handle_providers(_: &mut AppState, _args: &str) -> CommandResult {
    CommandResult::Event(crate::Event::ProvidersDialog)
}

fn handle_hotkeys(state: &mut AppState, _: &str) -> CommandResult {
    let mut panel = Panel::new("hotkeys", " Keyboard Shortcuts ");

    let mut bindings: Vec<(String, String)> = state
        .config
        .keybindings
        .iter()
        .map(|(combo, name)| (combo.clone(), name.clone()))
        .collect();
    bindings.sort_by(|a, b| a.0.cmp(&b.0));

    if bindings.is_empty() {
        panel = panel.header("No keybindings configured.");
    } else {
        panel = panel.header(format!("{} bindings", bindings.len()));
        for (combo, name) in bindings {
            panel = panel.item(format!("{}  →  {}", combo, name), ItemAction::Close);
        }
    }
    CommandResult::OpenPanelStack(Box::new(PanelStack::new(panel)))
}

// ── Prompt command handler ─────────────────────────────────────────────────────

/// Handler for the `/prompt` command.
/// Emits `RunPromptCommand` intent for state mutation.
fn handle_prompt(_state: &mut AppState, args: &str) -> CommandResult {
    CommandResult::Event(crate::Event::RunPromptCommand {
        name: args.trim().to_owned(),
    })
}
