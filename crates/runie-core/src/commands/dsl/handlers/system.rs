//! System commands.

use crate::commands::{CommandCategory, CommandRegistry, CommandResult};
use crate::dialog::{ItemAction, Panel, PanelStack};
use crate::model::AppState;

use super::spec::{CommandKind, CommandSpec};

fn prompt_submit(values: &std::collections::HashMap<String, String>) -> crate::Event {
    crate::event::CommandEvent::RunPromptCommand {
        name: crate::dialog::dsl::get_field(values, "name"),
    }
}

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
        kind: CommandKind::Form {
            title: "Switch Prompt",
            fields: &[("Prompt name", "prompt-name", "name")],
            submit: prompt_submit,
        },
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
        .map(|m| m.content().clone())
        .unwrap_or_default();
    if text.is_empty() {
        return CommandResult::Message("No assistant response to copy".into());
    }
    CommandResult::Event(crate::event::DialogEvent::CopyToClipboard(text))
}

fn handle_reload(state: &mut AppState, _: &str) -> CommandResult {
    if let Some(ref tx) = state.config_tx {
        let tx = tx.clone();
        tokio::spawn(async move {
            let _ = tx.send(crate::actors::ConfigMsg::Reload).await;
        });
    }
    state.skills = crate::async_io::block_in_place_if_runtime(crate::skills::load_all);
    CommandResult::Message("Reloaded config, keybindings, theme, skills, and prompts.".into())
}

fn handle_settings(state: &mut AppState, _: &str) -> CommandResult {
    crate::update::dialog::open_settings_dialog(state);
    CommandResult::None
}

fn handle_diagnostics(_: &mut AppState, _: &str) -> CommandResult {
    CommandResult::Event(crate::event::SystemEvent::ShowDiagnostics)
}

fn handle_skills(state: &mut AppState, _: &str) -> CommandResult {
    if state.skills.is_empty() {
        return CommandResult::Warning("No skills loaded.".into());
    }
    let lines: Vec<_> = std::iter::once("Loaded skills:".into())
        .chain(state.skills.iter().map(|s| format!("  {}", s.summary())))
        .collect();
    CommandResult::Message(lines.join("\n"))
}

fn handle_skill(state: &mut AppState, args: &str) -> CommandResult {
    let name = args.trim();
    if name.is_empty() {
        use crate::dialog::dsl::form;
        let stack = form("skill", "Show Skill")
            .field("Name", "skill-name", "name")
            .on_submit(|values| crate::event::CommandEvent::RunSkillCommand {
                name: crate::dialog::dsl::get_field(values, "name"),
            })
            .into_stack();
        return CommandResult::OpenPanelStack(Box::new(stack));
    }
    match state.skills.iter().find(|s| s.name == name) {
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

fn handle_theme(state: &mut AppState, args: &str) -> CommandResult {
    let name = args.trim();
    if name.is_empty() {
        open_theme_selector(state);
        return CommandResult::None;
    }
    state.config.theme_name = name.to_string();
    if crate::themes::BUILTIN_THEMES.contains(&name) {
        CommandResult::Message(format!("Theme switched to '{}'", name))
    } else {
        CommandResult::Message(format!(
            "Theme '{}' not found. Use /theme to list. (fallback: runie)",
            name
        ))
    }
}

fn open_theme_selector(state: &mut AppState) {
    let mut panel = Panel::new("theme", "Choose Theme")
        .header("available themes")
        .keep_open();
    for theme in crate::themes::BUILTIN_THEMES {
        panel = panel.item(
            *theme,
            ItemAction::Emit(crate::event::ModelConfigEvent::SwitchTheme {
                name: theme.to_string(),
            }),
        );
    }
    state.open_dialog = Some(crate::commands::DialogState::PanelStack(PanelStack::new(
        panel,
    )));
    state.mark_dirty();
}

fn handle_approve(_: &mut AppState, _: &str) -> CommandResult {
    CommandResult::Event(crate::event::EditEvent::ApproveEdit)
}

fn handle_reject(_: &mut AppState, _: &str) -> CommandResult {
    CommandResult::Event(crate::event::EditEvent::RejectEdit)
}

fn handle_providers(_: &mut AppState, _args: &str) -> CommandResult {
    CommandResult::Event(crate::event::DialogEvent::ProvidersDialog)
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

// ── Form-submit handlers ──────────────────────────────────────────────────────

pub fn run_prompt(state: &mut AppState, name: &str) {
    let name = name.trim();
    if name.is_empty() {
        let current = if state.input.current_prompt.is_empty() {
            "default"
        } else {
            &state.input.current_prompt
        };
        let mut lines = vec![format!("Current prompt: {}", current)];
        if !state.prompts.is_empty() {
            lines.push("Available prompts:".into());
            for p in &state.prompts {
                lines.push(format!("  {}", p.summary()));
            }
        }
        state.add_system_msg(lines.join("\n"));
        return;
    }
    if state.prompts.iter().any(|p| p.name == name) {
        state.input.current_prompt = name.to_string();
        state.add_system_msg(format!("Prompt switched to '{}'", name));
    } else {
        state.add_system_msg(format!("Prompt '{}' not found.", name));
    }
}
