use crate::commands::{CommandCategory, CommandDef, CommandHandler, CommandRegistry, CommandResult, Dialog};
use crate::model::AppState;

pub fn register(registry: &mut CommandRegistry) {
    registry.register(cmd("copy", "Copy last response to clipboard", &[], CommandCategory::System, handle_copy));
    registry.register(cmd("settings", "Open settings dialog", &[], CommandCategory::System, handle_settings));
    registry.register(cmd("reload", "Reload config, keybindings, themes", &[], CommandCategory::System, handle_reload));
    registry.register(cmd("changelog", "Show changelog", &[], CommandCategory::System, handle_changelog));
    registry.register(cmd("hotkeys", "Show all keyboard shortcuts", &[], CommandCategory::System, handle_hotkeys));
    registry.register(cmd("theme", "Switch theme or list available themes", &[], CommandCategory::System, handle_theme));
    registry.register(cmd("approve", "Apply pending file edits", &[], CommandCategory::System, handle_approve));
    registry.register(cmd("reject", "Cancel pending file edits", &[], CommandCategory::System, handle_reject));
    registry.register(cmd("reload", "Reload config, keybindings, and themes", &[], CommandCategory::System, handle_reload));
    registry.register(cmd("diagnostics", "Show resource loading diagnostics", &[], CommandCategory::System, handle_diagnostics));
    registry.register(cmd("skills", "List loaded skills", &[], CommandCategory::System, handle_skills));
    registry.register(cmd("skill", "Invoke a skill by name", &[], CommandCategory::System, handle_skill));
    registry.register(cmd("prompt", "Switch prompt template", &[], CommandCategory::System, handle_prompt));
    registry.register(cmd("login", "Store API key for a provider", &[], CommandCategory::System, handle_login));
    registry.register(cmd("logout", "Remove stored token for a provider", &[], CommandCategory::System, handle_logout));
}

fn cmd(name: &str, desc: &str, aliases: &[&str], category: CommandCategory, handler: CommandHandler) -> CommandDef {
    CommandDef {
        name: name.into(),
        description: desc.into(),
        aliases: aliases.iter().map(|s| s.to_string()).collect(),
        category,
        handler,
        completer: None,
    }
}

fn handle_copy(state: &mut AppState, _args: &str) -> CommandResult {
    let text = state.session.messages.iter().rev()
        .find(|m| m.role == crate::model::Role::Assistant)
        .map(|m| m.content.clone())
        .unwrap_or_default();
    if text.is_empty() {
        return CommandResult::Message("No assistant response to copy".into());
    }
    CommandResult::Message("Copied to clipboard".into())
}

fn handle_settings(_state: &mut AppState, _args: &str) -> CommandResult {
    CommandResult::OpenDialog(Dialog::Settings)
}

fn handle_reload(state: &mut AppState, _args: &str) -> CommandResult {
    state.config.keybindings = crate::keybindings::load_keybindings(&None);
    CommandResult::Event(crate::Event::ReloadAll)
}

fn handle_diagnostics(_state: &mut AppState, _args: &str) -> CommandResult {
    CommandResult::Event(crate::Event::ShowDiagnostics)
}

fn handle_skills(state: &mut AppState, _args: &str) -> CommandResult {
    if state.skills.is_empty() {
        return CommandResult::Message("No skills loaded.".into());
    }
    let mut lines = vec!["Loaded skills:".to_string()];
    for skill in &state.skills {
        lines.push(format!("  {}", skill.summary()));
    }
    CommandResult::Message(lines.join("\n"))
}

fn handle_skill(state: &mut AppState, args: &str) -> CommandResult {
    let name = args.trim();
    if name.is_empty() {
        return CommandResult::Message("Usage: /skill <name>".into());
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
        None => CommandResult::Message(format!("Skill '{}' not found. Use /skills to list loaded skills.", name)),
    }
}

fn handle_prompt(state: &mut AppState, args: &str) -> CommandResult {
    let name = args.trim();
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
        return CommandResult::Message(lines.join("\n"));
    }
    if state.prompts.iter().any(|p| p.name == name) {
        state.current_prompt = name.to_string();
        CommandResult::Message(format!("Prompt switched to '{}'", name))
    } else {
        CommandResult::Message(format!("Prompt '{}' not found.", name))
    }
}

fn handle_login(_state: &mut AppState, args: &str) -> CommandResult {
    let parts: Vec<&str> = args.trim().splitn(2, ' ').collect();
    if parts.len() < 2 || parts[0].is_empty() || parts[1].is_empty() {
        return CommandResult::Message("Usage: /login <provider> <token>".into());
    }
    let provider = parts[0];
    let token = parts[1];
    let mut storage = crate::auth::AuthStorage::load();
    storage.set(provider, token, None);
    match storage.save() {
        Ok(()) => CommandResult::Message(format!("Logged in to '{}'.", provider)),
        Err(e) => CommandResult::Message(format!("Could not save token: {}", e)),
    }
}

fn handle_logout(_state: &mut AppState, args: &str) -> CommandResult {
    let provider = args.trim();
    if provider.is_empty() {
        return CommandResult::Message("Usage: /logout <provider>".into());
    }
    let mut storage = crate::auth::AuthStorage::load();
    storage.remove(provider);
    match storage.save() {
        Ok(()) => CommandResult::Message(format!("Logged out from '{}'.", provider)),
        Err(e) => CommandResult::Message(format!("Could not remove token: {}", e)),
    }
}

fn handle_changelog(_state: &mut AppState, _args: &str) -> CommandResult {
    CommandResult::Message("Changelog: not yet implemented".into())
}

fn handle_hotkeys(_state: &mut AppState, _args: &str) -> CommandResult {
    CommandResult::Message("Keyboard shortcuts: not yet implemented".into())
}

fn handle_theme(state: &mut AppState, args: &str) -> CommandResult {
    let name = args.trim();
    if name.is_empty() {
        return CommandResult::Message(format!(
            "Current theme: {}\n\nAvailable themes:\n{}",
            state.config.theme_name,
            builtin_themes().join(", ")
        ));
    }
    state.config.theme_name = name.to_string();
    if builtin_themes().contains(&name) {
        CommandResult::Message(format!("Theme switched to '{}'", name))
    } else {
        CommandResult::Message(format!("Theme '{}' not found. Using fallback 'silkcircuit-neon'. Use /theme to list available themes.", name))
    }
}

fn handle_approve(_state: &mut AppState, _args: &str) -> CommandResult {
    CommandResult::Event(crate::Event::ApproveEdit)
}

fn handle_reject(_state: &mut AppState, _args: &str) -> CommandResult {
    CommandResult::Event(crate::Event::RejectEdit)
}

fn builtin_themes() -> &'static [&'static str] {
    &[
        "silkcircuit-neon", "silkcircuit-glow", "silkcircuit-soft", "silkcircuit-vibrant", "silkcircuit-dawn",
        "catppuccin-mocha", "catppuccin-macchiato", "catppuccin-frappe", "catppuccin-latte",
        "dracula", "nord", "gruvbox-dark", "gruvbox-light", "tokyo-night", "tokyo-night-storm", "tokyo-night-moon",
        "rose-pine", "rose-pine-moon", "rose-pine-dawn", "kanagawa-wave", "kanagawa-dragon", "kanagawa-lotus",
        "everforest-dark", "everforest-light", "ayu-dark", "ayu-light", "ayu-mirage",
        "one-dark", "one-light", "github-dark-dimmed", "github-light", "night-owl", "light-owl",
        "monokai-pro", "palenight", "solarized-dark", "solarized-light", "flexoki-dark", "flexoki-light",
    ]
}
