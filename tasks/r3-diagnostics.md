# Diagnostics

**Status**: todo
**Milestone**: R3
**Category**: Configuration

## Description

Resource loading diagnostics. Shows what config files, themes, and keybindings were loaded.

## Architecture

```rust
pub struct Diagnostics {
    pub config_loaded: bool,
    pub config_path: Option<PathBuf>,
    pub keybindings_loaded: bool,
    pub keybindings_path: Option<PathBuf>,
    pub theme_loaded: String,
    pub skills_loaded: Vec<String>,
    pub providers_configured: Vec<String>,
}

fn cmd_diagnostics(_args: &str) -> Option<Event> {
    Some(Event::ShowDiagnostics)
}
```

## Acceptance Criteria

- [ ] `/diagnostics` shows resource loading status
- [ ] Lists all loaded config files with paths
- [ ] Shows configured providers
- [ ] Shows loaded skills
- [ ] Shows theme and keybinding status
- [ ] Useful for debugging setup issues

## Tests

### Layer 1
- [ ] `diagnostics_shows_config_path` — config file path displayed
- [ ] `diagnostics_shows_providers` — provider list correct
