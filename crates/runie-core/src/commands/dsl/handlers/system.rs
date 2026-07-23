//! System commands.

use crate::commands::dsl::handlers::NamedHandler;
use crate::commands::{CommandResult, DialogType};
use crate::dialog::{ItemAction, Panel, PanelStack};
use crate::model::AppState;

pub fn register_handlers(registry: &mut crate::commands::dsl::handlers::registry::HandlerRegistry) {
    registry.register("settings", NamedHandler::Handler(handle_settings));
    registry.register("copy", NamedHandler::Handler(handle_copy));
    registry.register("reload", NamedHandler::Handler(handle_reload));
    registry.register("diagnostics", NamedHandler::Handler(handle_diagnostics));
    registry.register("skills", NamedHandler::Handler(handle_skills));
    registry.register(
        "skill",
        NamedHandler::FormWithHandler { title: "Show Skill", fields: &[("Name", "skill-name", "name")], handler: run_skill },
    );
    registry.register("prompt", NamedHandler::Handler(handle_prompt));
    registry.register("hotkeys", NamedHandler::Handler(handle_hotkeys));
    registry.register("theme", NamedHandler::Handler(handle_theme));
    registry.register("approve", NamedHandler::Handler(handle_approve));
    registry.register("reject", NamedHandler::Handler(handle_reject));
    registry.register("provider", NamedHandler::Handler(handle_providers));
    registry.register("mcp-servers", NamedHandler::Handler(handle_mcp_servers));
    registry.register("skills-dialog", NamedHandler::Handler(handle_skills_dialog));
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
        return CommandResult::Message(crate::ui_strings::system::NOTHING_TO_COPY.into());
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
    use crate::ui_strings::system as s;
    if state.skills().is_empty() {
        return CommandResult::Warning(s::NO_SKILLS.into());
    }
    let lines: Vec<_> = std::iter::once(s::LOADED_SKILLS.into())
        .chain(
            state
                .skills()
                .iter()
                .map(|sk| format!("  {}", sk.summary())),
        )
        .collect();
    CommandResult::Message(lines.join("\n"))
}

/// Handler for `/skill <name>` — shows skill info.
pub fn run_skill(state: &mut AppState, args: &str) -> CommandResult {
    use crate::ui_strings::system as s;
    let name = args.trim();
    match state.skills().iter().find(|sk| sk.name == name) {
        Some(skill) => CommandResult::Message(s::skill_info(
            &skill.name,
            Some(&skill.description),
            Some(&skill.context),
        )),
        None => CommandResult::Message(s::skill_not_found(name)),
    }
}

pub fn handle_theme(_state: &mut AppState, args: &str) -> CommandResult {
    let name = args.trim();
    if name.is_empty() {
        return CommandResult::OpenDialog(DialogType::ThemeSelector);
    }
    CommandResult::Event(crate::Event::SwitchTheme { name: name.to_owned() })
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
    CommandResult::Event(crate::Event::RunPromptCommand { name: args.trim().to_owned() })
}

/// Open the MCP servers management dialog.
pub fn handle_mcp_servers(_: &mut AppState, _: &str) -> CommandResult {
    CommandResult::OpenDialog(DialogType::McpServers)
}

/// Open the skills management dialog.
pub fn handle_skills_dialog(_: &mut AppState, _: &str) -> CommandResult {
    CommandResult::OpenDialog(DialogType::Skills)
}
