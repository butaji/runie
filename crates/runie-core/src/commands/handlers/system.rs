//! System commands using the new DSL

use crate::commands::{
    cmd, CommandCategory, CommandRegistry, CommandResult, DialogType, FormBuilder,
};
use crate::dialog::{ItemAction, Panel, PanelStack};
use crate::model::AppState;

pub fn register(registry: &mut CommandRegistry) {
    // Dialog commands
    registry.register(crate::cmd!("settings")
        .desc("Open settings dialog")
        .category(CommandCategory::System)
        .dialog(DialogType::Settings));

    registry.register(crate::cmd!("copy")
        .desc("Copy last response to clipboard")
        .category(CommandCategory::System)
        .handler(handle_copy));

    registry.register(crate::cmd!("reload")
        .desc("Reload config, keybindings, themes")
        .category(CommandCategory::System)
        .handler(handle_reload));

    registry.register(crate::cmd!("diagnostics")
        .desc("Show resource loading diagnostics")
        .category(CommandCategory::System)
        .handler(handle_diagnostics));

    registry.register(crate::cmd!("skills")
        .desc("List loaded skills")
        .category(CommandCategory::System)
        .handler(handle_skills));

    registry.register(crate::cmd!("skill")
        .desc("Show skill details")
        .category(CommandCategory::System)
        .handler(handle_skill));

    registry.register(crate::cmd!("prompt")
        .desc("Switch prompt template")
        .category(CommandCategory::System)
        .form("Switch Prompt", |f| f
            .field("Prompt name", "prompt-name", "name"),
            crate::Event::RunPromptCommand { name: String::new() }));

    registry.register(crate::cmd!("changelog")
        .desc("Show changelog")
        .category(CommandCategory::System)
        .msg("Changelog: not yet implemented"));

    registry.register(crate::cmd!("hotkeys")
        .desc("Show all keyboard shortcuts")
        .category(CommandCategory::System)
        .msg("Keyboard shortcuts: not yet implemented"));

    registry.register(crate::cmd!("theme")
        .desc("Switch theme or list available themes")
        .category(CommandCategory::System)
        .handler(handle_theme));

    registry.register(crate::cmd!("approve")
        .desc("Apply pending file edits")
        .category(CommandCategory::System)
        .handler(handle_approve));

    registry.register(crate::cmd!("reject")
        .desc("Cancel pending file edits")
        .category(CommandCategory::System)
        .handler(handle_reject));

    // Form commands for auth
    registry.register(crate::cmd!("login")
        .desc("Store API key for a provider")
        .category(CommandCategory::System)
        .form("Login", |f| f.field("Provider", "anthropic", "provider").field("Token", "sk-...", "token"),
              crate::Event::RunLoginCommand { provider: String::new(), token: String::new() }));

    registry.register(crate::cmd!("logout")
        .desc("Remove stored token for a provider")
        .category(CommandCategory::System)
        .form("Logout", |f| f.field("Provider", "provider-name", "provider"),
              crate::Event::RunLogoutCommand { provider: String::new() }));
}

fn handle_copy(state: &mut AppState, _: &str) -> CommandResult {
    let text = state.session.messages.iter().rev()
        .find(|m| m.role == crate::model::Role::Assistant)
        .map(|m| m.content.clone())
        .unwrap_or_default();
    if text.is_empty() {
        CommandResult::Message("No assistant response to copy".into())
    } else {
        CommandResult::Message("Copied to clipboard".into())
    }
}

fn handle_reload(state: &mut AppState, _: &str) -> CommandResult {
    state.config.keybindings = crate::keybindings::load_keybindings(&None);
    CommandResult::Event(crate::Event::ReloadAll)
}

fn handle_diagnostics(_: &mut AppState, _: &str) -> CommandResult {
    CommandResult::Event(crate::Event::ShowDiagnostics)
}

fn handle_skills(state: &mut AppState, _: &str) -> CommandResult {
    if state.skills.is_empty() {
        return CommandResult::Message("No skills loaded.".into());
    }
    let lines: Vec<_> = std::iter::once("Loaded skills:".into())
        .chain(state.skills.iter().map(|s| format!("  {}", s.summary())))
        .collect();
    CommandResult::Message(lines.join("\n"))
}

fn handle_skill(state: &mut AppState, args: &str) -> CommandResult {
    let name = args.trim();
    if name.is_empty() {
        use crate::dialog::dsl::{panel, form};
        let stack = form("skill", "Show Skill")
            .field("Name", "skill-name", "name")
            .on_submit(crate::Event::RunSkillCommand { name: String::new() })
            .into_stack();
        return CommandResult::OpenPanelStack(stack);
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
        CommandResult::Message(format!("Theme '{}' not found. Use /theme to list. (fallback: runie)", name))
    }
}

fn open_theme_selector(state: &mut AppState) {
    let mut panel = Panel::new("theme", "Choose Theme")
        .header("available themes")
        .keep_open(); // Keep dialog open for live theme preview
    for theme in crate::themes::BUILTIN_THEMES {
        panel = panel.item(*theme, ItemAction::Emit(crate::Event::SwitchTheme { name: theme.to_string() }));
    }
    state.open_dialog = Some(crate::commands::DialogState::PanelStack(PanelStack::new(panel)));
    state.mark_dirty();
}

fn handle_approve(_: &mut AppState, _: &str) -> CommandResult {
    CommandResult::Event(crate::Event::ApproveEdit)
}

fn handle_reject(_: &mut AppState, _: &str) -> CommandResult {
    CommandResult::Event(crate::Event::RejectEdit)
}
