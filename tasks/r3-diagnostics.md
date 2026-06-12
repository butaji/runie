# Diagnostics

**Status**: done
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

- [x] `/diagnostics` shows resource loading status
- [x] Lists all loaded config files with paths
- [x] Shows configured providers
- [x] Shows theme and keybinding status
- [x] Useful for debugging setup issues

## Tests

### Layer 1
- [x] `diagnostics_shows_config_path` — config file path displayed
- [x] `diagnostics_shows_providers` — provider list correct
