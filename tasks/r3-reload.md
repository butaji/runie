# Reload (/reload)

**Status**: done
**Milestone**: R3
**Category**: Configuration

## Description

Hot reload all configuration: keybindings, themes, skills, prompts.

## Architecture

```rust
fn cmd_reload(_args: &str) -> Option<Event> {
    Some(Event::ReloadAll)
}

fn update_reload(state: &mut AppState) -> String {
    // Reload keybindings
    state.keybindings = load_keybindings(&None);
    // Reload theme
    state.theme = RunieTheme::load(&state.theme_name);
    // Reload config
    state.config = Config::load();
    // Skills, prompts would go here
    "Reloaded keybindings, theme, and config".to_string()
}
```

## Acceptance Criteria

- [x] `/reload` reloads keybindings from file
- [x] Reloads theme from file
- [x] Reloads config.toml
- [x] No restart required
- [x] Shows confirmation message

## Tests

### Layer 2
- [x] `reload_emits_event` — /reload triggers ReloadAll
- [x] `reload_updates_keybindings` — new bindings take effect
