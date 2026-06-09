# Reload (/reload)

**Status**: todo
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

- [ ] `/reload` reloads keybindings from file
- [ ] Reloads theme from file
- [ ] Reloads config.toml
- [ ] No restart required
- [ ] Shows confirmation message

## Tests

### Layer 2
- [ ] `reload_emits_event` — /reload triggers ReloadAll
- [ ] `reload_updates_keybindings` — new bindings take effect
