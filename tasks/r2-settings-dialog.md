# Settings Dialog

**Status**: done
**Milestone**: R2
**Category**: Configuration / TUI Rendering

## Description

Interactive settings menu opened via `/settings` or the command palette. Shows current values and lets users toggle/edit without manually editing `config.toml`.

## Architecture

```rust
pub enum DialogState {
    // ... other variants ...
    Settings {
        category: SettingsCategory,
        selected: usize,
    },
}

pub enum SettingsCategory {
    Models,
    Appearance,
    Behavior,
    Safety,
}

pub struct SettingItem {
    pub key: String,
    pub label: String,
    pub value: SettingValue,
    pub description: String,
}

pub enum SettingValue {
    Bool(bool),
    String(String),
    Enum { current: String, options: Vec<String> },
}
```

### Settings

| Setting | Type | Category |
|---------|------|----------|
| Provider | Enum | Models |
| Model | Enum | Models |
| Theme | Enum | Appearance |
| Thinking Level | Enum | Behavior |
| Read-Only | Bool | Safety |
| Steering Mode | Enum | Behavior |
| Follow-Up Mode | Enum | Behavior |

## Acceptance Criteria

- [ ] `/settings` opens settings dialog
- [ ] Shows settings grouped by category
- [ ] Arrow keys navigate
- [ ] Enter toggles bools / opens edit for enums
- [ ] Changes applied immediately
- [ ] Esc closes dialog

## Files

| File | Description |
|------|-------------|
| `crates/runie-core/src/model.rs` | `DialogState::Settings` |
| `crates/runie-core/src/update/dialog.rs` | Settings update logic |
| `crates/runie-tui/src/ui.rs` | `render_settings_dialog()` |

## Tests

### Layer 3
- [ ] `settings_renders_categories` — TestBackend shows groups
- [ ] `toggle_updates_state` — Enter on bool flips value
