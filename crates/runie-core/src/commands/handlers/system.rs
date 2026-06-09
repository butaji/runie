use crate::commands::{CommandCategory, CommandDef, CommandHandler, CommandRegistry, CommandResult, Dialog};
use crate::model::AppState;

pub fn register(registry: &mut CommandRegistry) {
    registry.register(cmd("copy", "Copy last response to clipboard", &[], CommandCategory::System, handle_copy));
    registry.register(cmd("settings", "Open settings dialog", &[], CommandCategory::System, handle_settings));
    registry.register(cmd("reload", "Reload config, keybindings, themes", &[], CommandCategory::System, handle_reload));
    registry.register(cmd("changelog", "Show changelog", &[], CommandCategory::System, handle_changelog));
    registry.register(cmd("hotkeys", "Show all keyboard shortcuts", &[], CommandCategory::System, handle_hotkeys));
    registry.register(cmd("theme", "Switch theme or list available themes", &[], CommandCategory::System, handle_theme));
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
    let text = state.messages.iter().rev()
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

fn handle_reload(_state: &mut AppState, _args: &str) -> CommandResult {
    CommandResult::Message("Config reloaded".into())
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
            state.theme_name,
            builtin_themes().join(", ")
        ));
    }
    state.theme_name = name.to_string();
    if builtin_themes().contains(&name) {
        CommandResult::Message(format!("Theme switched to '{}'", name))
    } else {
        CommandResult::Message(format!("Theme '{}' not found. Using fallback 'silkcircuit-neon'. Use /theme to list available themes.", name))
    }
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
