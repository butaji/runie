//! System commands.

use crate::commands::{CommandResult, DialogType};
use crate::dialog::{ItemAction, Panel, PanelStack};
use crate::model::AppState;

/// Register all system handlers with the handler registry (for YAML-based commands).
pub fn register_handlers(registry: &mut crate::commands::dsl::handlers::registry::HandlerRegistry) {
    use crate::register_handler;
    register_handler!(registry, "settings", Handler(handle_settings));
    register_handler!(registry, "copy", Handler(handle_copy));
    register_handler!(registry, "reload", Handler(handle_reload));
    register_handler!(registry, "diagnostics", Handler(handle_diagnostics));
    register_handler!(registry, "skills", Handler(handle_skills));
    register_handler!(registry, "skill", Handler(handle_skill));
    register_handler!(registry, "prompt", Handler(handle_prompt));
    register_handler!(registry, "hotkeys", Handler(handle_hotkeys));
    register_handler!(registry, "theme", Handler(handle_theme));
    register_handler!(registry, "approve", Handler(handle_approve));
    register_handler!(registry, "reject", Handler(handle_reject));
    register_handler!(registry, "provider", Handler(handle_providers));
}

pub fn handle_copy(state: &mut AppState, _: &str) -> CommandResult {
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

pub fn handle_reload(_: &mut AppState, _: &str) -> CommandResult {
    CommandResult::Event(crate::Event::ReloadAll)
}

pub fn handle_settings(_: &mut AppState, _: &str) -> CommandResult {
    CommandResult::Event(crate::Event::ToggleSettingsDialog)
}

pub fn handle_diagnostics(_: &mut AppState, _: &str) -> CommandResult {
    CommandResult::Event(crate::Event::ShowDiagnostics)
}

pub fn handle_skills(state: &mut AppState, _: &str) -> CommandResult {
    if state.skills().is_empty() {
        return CommandResult::Warning("No skills loaded.".into());
    }
    let lines: Vec<_> = std::iter::once("Loaded skills:".into())
        .chain(state.skills().iter().map(|s| format!("  {}", s.summary())))
        .collect();
    CommandResult::Message(lines.join("\n"))
}

pub fn handle_skill(state: &mut AppState, args: &str) -> CommandResult {
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

pub fn handle_theme(_state: &mut AppState, args: &str) -> CommandResult {
    let name = args.trim();
    if name.is_empty() {
        return CommandResult::OpenDialog(DialogType::ThemeSelector);
    }
    CommandResult::Event(crate::Event::SwitchTheme {
        name: name.to_owned(),
    })
}

pub fn handle_approve(_: &mut AppState, _: &str) -> CommandResult {
    CommandResult::Event(crate::Event::ApproveEdit)
}

pub fn handle_reject(_: &mut AppState, _: &str) -> CommandResult {
    CommandResult::Event(crate::Event::RejectEdit)
}

pub fn handle_providers(_: &mut AppState, _args: &str) -> CommandResult {
    CommandResult::Event(crate::Event::ProvidersDialog)
}

pub fn handle_hotkeys(state: &mut AppState, _: &str) -> CommandResult {
    let mut panel = Panel::new("hotkeys", " Keyboard Shortcuts ");

    let mut bindings: Vec<_> = state
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

pub fn handle_prompt(_state: &mut AppState, args: &str) -> CommandResult {
    CommandResult::Event(crate::Event::RunPromptCommand {
        name: args.trim().to_owned(),
    })
}
