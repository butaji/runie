//! System commands using the new DSL

use crate::commands::{CommandCategory, CommandRegistry, CommandResult, DialogType};
use crate::dialog::{ItemAction, Panel, PanelStack};
use crate::model::AppState;

pub fn register(registry: &mut CommandRegistry) {
    // Dialog commands
    registry.register(
        crate::cmd!("settings")
            .desc("Open settings dialog")
            .category(CommandCategory::System)
            .dialog(DialogType::Settings),
    );

    registry.register(
        crate::cmd!("copy")
            .desc("Copy last response to clipboard")
            .category(CommandCategory::System)
            .handler(handle_copy),
    );

    registry.register(
        crate::cmd!("reload")
            .desc("Reload config, keybindings, themes")
            .category(CommandCategory::System)
            .handler(handle_reload),
    );

    registry.register(
        crate::cmd!("diagnostics")
            .desc("Show resource loading diagnostics")
            .category(CommandCategory::System)
            .handler(handle_diagnostics),
    );

    registry.register(
        crate::cmd!("skills")
            .desc("List loaded skills")
            .category(CommandCategory::System)
            .handler(handle_skills),
    );

    registry.register(
        crate::cmd!("skill")
            .desc("Show skill details")
            .category(CommandCategory::System)
            .handler(handle_skill),
    );

    registry.register(
        crate::cmd!("prompt")
            .desc("Switch prompt template")
            .category(CommandCategory::System)
            .form(
                "Switch Prompt",
                |f| f.field("Prompt name", "prompt-name", "name"),
                crate::Event::RunPromptCommand {
                    name: String::new(),
                },
            ),
    );

    registry.register(
        crate::cmd!("changelog")
            .desc("Show changelog")
            .category(CommandCategory::System)
            .msg("Changelog: not yet implemented"),
    );

    registry.register(
        crate::cmd!("hotkeys")
            .desc("Show all keyboard shortcuts")
            .category(CommandCategory::System)
            .msg("Keyboard shortcuts: not yet implemented"),
    );

    registry.register(
        crate::cmd!("theme")
            .desc("Switch theme or list available themes")
            .category(CommandCategory::System)
            .handler(handle_theme),
    );

    registry.register(
        crate::cmd!("approve")
            .desc("Apply pending file edits")
            .category(CommandCategory::System)
            .handler(handle_approve),
    );

    registry.register(
        crate::cmd!("reject")
            .desc("Cancel pending file edits")
            .category(CommandCategory::System)
            .handler(handle_reject),
    );

    // Login/logout dialog flows
    registry.register(
        crate::cmd!("login")
            .desc("Add a provider via guided dialog")
            .category(CommandCategory::System)
            .handler(handle_login),
    );

    registry.register(
        crate::cmd!("logout")
            .desc("Remove a configured provider")
            .category(CommandCategory::System)
            .handler(handle_logout),
    );
}

fn handle_copy(state: &mut AppState, _: &str) -> CommandResult {
    let text = state
        .session
        .messages
        .iter()
        .rev()
        .find(|m| m.role == crate::model::Role::Assistant)
        .map(|m| m.content.clone())
        .unwrap_or_default();
    if text.is_empty() {
        return CommandResult::Message("No assistant response to copy".into());
    }
    match write_clipboard(&text) {
        Ok(path) => CommandResult::Message(format!("Copied to {}", path.display())),
        Err(e) => CommandResult::Message(format!("Could not copy: {}", e)),
    }
}

/// Write `text` to the clipboard file. Respects `$RUNIE_CACHE_DIR` for
/// tests; defaults to `<data_dir>/runie/clipboard.md`.
fn write_clipboard(text: &str) -> std::io::Result<std::path::PathBuf> {
    use std::io::Write;
    let dir = if let Ok(p) = std::env::var("RUNIE_CACHE_DIR") {
        std::path::PathBuf::from(p)
    } else {
        dirs::data_dir()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "no data dir"))?
            .join("runie")
    };
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("clipboard.md");
    let mut f = std::fs::File::create(&path)?;
    f.write_all(text.as_bytes())?;
    Ok(path)
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
        use crate::dialog::dsl::form;
        let stack = form("skill", "Show Skill")
            .field("Name", "skill-name", "name")
            .on_submit(crate::Event::RunSkillCommand {
                name: String::new(),
            })
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
        CommandResult::Message(format!(
            "Theme '{}' not found. Use /theme to list. (fallback: runie)",
            name
        ))
    }
}

fn open_theme_selector(state: &mut AppState) {
    let mut panel = Panel::new("theme", "Choose Theme")
        .header("available themes")
        .keep_open(); // Keep dialog open for live theme preview
    for theme in crate::themes::BUILTIN_THEMES {
        panel = panel.item(
            *theme,
            ItemAction::Emit(crate::Event::SwitchTheme {
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
    CommandResult::Event(crate::Event::ApproveEdit)
}

fn handle_reject(_: &mut AppState, _: &str) -> CommandResult {
    CommandResult::Event(crate::Event::RejectEdit)
}

fn handle_login(_: &mut AppState, _: &str) -> CommandResult {
    CommandResult::Event(crate::Event::LoginFlowStart)
}

fn handle_logout(_: &mut AppState, _: &str) -> CommandResult {
    use crate::login_config::list_configured_providers;
    let configured = list_configured_providers();
    if configured.is_empty() {
        return CommandResult::Message("No providers configured. Use /login to add one.".into());
    }
    let mut panel = Panel::new("logout", "Logout").header("Select a provider to remove");
    for (name, _, _) in configured {
        let evt = crate::Event::RunLogoutCommand {
            provider: name.clone(),
        };
        panel = panel.item(&name, ItemAction::Emit(evt));
    }
    panel = panel.separator().item("Cancel", ItemAction::Close);
    CommandResult::OpenPanelStack(PanelStack::new(panel))
}

// ============================================================================
// Form-submit handlers (called from update/mod.rs with form values).
// ============================================================================

/// Switch the prompt template. Empty name lists the current prompt and
/// available templates.
pub(crate) fn run_prompt(state: &mut AppState, name: &str) {
    let name = name.trim();
    if name.is_empty() {
        let current = if state.current_prompt.is_empty() {
            "default"
        } else {
            &state.current_prompt
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
        state.current_prompt = name.to_string();
        state.add_system_msg(format!("Prompt switched to '{}'", name));
    } else {
        state.add_system_msg(format!("Prompt '{}' not found.", name));
    }
}
