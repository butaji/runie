# Theme System with Opaline

**Status**: done
**Milestone**: R2
**Category**: TUI Rendering / Configuration

## Description

Replace runie's hardcoded `Colors` struct with [opaline](https://github.com/hyperb1iss/opaline), a token-based theme engine. This gives us 39 builtin themes, custom TOML themes, hot-swapping, and semantic color tokens — while keeping all existing style contracts intact.

## Why Opaline

- **39 builtin themes** — catppuccin, dracula, nord, gruvbox, tokyo-night, etc.
- **Token contract** — 26 semantic tokens + 13 styles guaranteed in every theme
- **Ratatui integration** — `theme.style("name")` returns `ratatui::style::Style` directly
- **Custom themes** — users drop TOML files into `~/.runie/themes/`
- **Runtime switching** — `/theme <name>` swaps without restart
- **Feature flags** — we only need `builtin-themes` + `ratatui` (both default)

## Acceptance Criteria

- [x] `opaline` added to `runie-tui/Cargo.toml`
- [x] `theme.rs` replaced: `RunieTheme` wraps `opaline::Theme` with runie-specific token registrations
- [x] All 25 `style_*()` functions query the theme (zero hardcoded colors)
- [x] Builtin themes loadable via `opaline::load_by_name()`
- [x] Custom themes loadable from `~/.runie/themes/*.toml` via `opaline::load_from_file()`
- [x] `theme` field in `config.toml` sets startup theme (default: `"silkcircuit-neon"`)
- [x] `/theme <name>` slash command lists available themes and switches
- [x] `theme_name` persisted in `Session` struct (save/load roundtrip)
- [x] Config watcher emits `SwitchTheme` on `theme` config change
- [x] Fallback: invalid theme name → `"silkcircuit-neon"` with system warning

## Architecture

### Token Mapping

Runie concepts map to opaline's token contract. App-specific styles are registered as defaults so any theme can override them.

| Runie Concept | Opaline Token / Style |
|---------------|----------------------|
| `bg` | `bg.base` |
| `fg` | `text.primary` |
| `fg_mid` | `text.secondary` |
| `fg_bright` | `text.primary` (brightened) |
| `accent` | `accent.primary` |
| `success` | `success` |
| `warning` | `warning` |
| `dim` | `text.dim` |
| `code` | `code.function` |
| `code_bg` | `bg.code` |
| `style_user` | `runie.user` → `{ fg = "accent.primary", bold = true }` |
| `style_agent` | `runie.agent` → `{ fg = "text.primary" }` |
| `style_thought` | `runie.thought` → `{ fg = "text.dim" }` |
| `style_tool_header` | `runie.tool.header` → `{ fg = "text.muted" }` |
| `style_tool_output` | `runie.tool.output` → `{ fg = "text.primary" }` |
| `style_status_active` | `runie.status.active` → `{ fg = "success" }` |
| `style_status_idle` | `runie.status.idle` → `{ fg = "text.dim" }` |
| `style_border` | `runie.border` → `{ fg = "border.unfocused" }` |
| `style_border_flash` | `runie.border.flash` → `{ fg = "warning" }` |
| `style_code_block` | `runie.code.block` → `{ fg = "code.function", bg = "bg.code" }` |
| `style_input_cursor` | `runie.input.cursor` → `{ fg = "bg.base", bg = "text.primary" }` |
| `style_popup_selected` | `runie.popup.selected` → `{ fg = "accent.secondary", bg = "bg.highlight", bold = true }` |
| `style_popup_unselected` | `runie.popup.unselected` → `{ fg = "text.secondary" }` |
| `style_popup_border` | `runie.popup.border` → `{ fg = "border.focused" }` |

### Files to Touch

| File | Change |
|------|--------|
| `crates/runie-tui/Cargo.toml` | Add `opaline = "0.4"` dependency |
| `crates/runie-tui/src/theme.rs` | Replace `Colors` with `RunieTheme` wrapping `opaline::Theme` |
| `crates/runie-core/src/event.rs` | Add `SwitchTheme { name: String }` |
| `crates/runie-core/src/model.rs` | Add `theme_name: String` to `AppState` |
| `crates/runie-core/src/snapshot.rs` | Add `theme_name: String` to `Snapshot` |
| `crates/runie-core/src/update/mod.rs` | Handle `SwitchTheme` event |
| `crates/runie-core/src/update/slash.rs` | Add `/theme` command |
| `crates/runie-core/src/config_reload.rs` | Parse `theme` field, emit `SwitchTheme` |
| `crates/runie-core/src/session.rs` | Add `theme_name` to `Session` struct |
| `crates/runie-core/src/update/agent.rs` | Pass `theme_name` into snapshot |

### Theme Resolution Order

```
1. User types `/theme nord`
2. Try opaline::load_by_name("nord")          → builtin
3. Try ~/.runie/themes/nord.toml              → custom file
4. Fallback to Theme::default() (silkcircuit-neon)
5. Register runie-specific default styles
6. Emit SwitchTheme { name } → update AppState.theme_name
7. Re-render with new colors
```

## Tests

### Layer 1 — State/Logic
- [x] `theme_loads_builtin_by_name` — `load_theme("dracula")` returns valid theme
- [x] `theme_loads_custom_from_file` — `load_theme("custom")` finds `~/.runie/themes/custom.toml`
- [x] `theme_fallback_on_invalid_name` — unknown name falls back to default
- [x] `theme_registers_runie_styles` — all `runie.*` styles are registered after load
- [x] `theme_style_returns_ratatui_style` — `theme.style("runie.user")` is `ratatui::style::Style`
- [x] `session_roundtrip_preserves_theme` — save/load keeps `theme_name`

### Layer 2 — Event Handling
- [x] `switch_theme_event_updates_state` — `Event::SwitchTheme` updates `AppState.theme_name`
- [x] `slash_theme_command_parses_name` — `/theme nord` sets `theme_name`
- [x] `slash_theme_no_args_lists_themes` — `/theme` without args shows available themes
- [x] `config_theme_field_emits_switch_theme` — config watcher detects theme change

### Layer 3 — Rendering
- [x] `theme_changes_border_color` — border style uses theme color
- [x] `theme_changes_user_message_color` — user glyph uses theme accent
- [x] `theme_changes_code_block_bg` — code block background uses `bg.code`

### Layer 4 — Smoke
- [x] All 758 workspace tests pass (no panics, no regressions)

## Custom Theme Template

Users can create `~/.runie/themes/my-theme.toml`:

```toml
[meta]
name = "My Custom Theme"
author = "user"
variant = "dark"
version = "1.0"

[palette]
bg = "#0c0c0c"
fg = "#8a8a8a"
accent = "#8b7cf4"
muted = "#4a4a4a"
highlight = "#1e1e28"
green = "#3ebd6a"
yellow = "#eab84a"

[tokens]
"text.primary" = "fg"
"text.secondary" = "fg"
"text.muted" = "muted"
"text.dim" = "muted"
"bg.base" = "bg"
"bg.code" = "highlight"
"bg.highlight" = "highlight"
"accent.primary" = "accent"
"border.focused" = "accent"
"border.unfocused" = "muted"
success = "green"
warning = "yellow"
"code.function" = "#b4b4c8"

[styles]
keyword = { fg = "accent.primary", bold = true }
runie.user = { fg = "accent.primary", bold = true }
runie.agent = { fg = "text.primary" }
runie.tool.header = { fg = "text.muted" }
runie.status.active = { fg = "success" }
runie.code.block = { fg = "code.function", bg = "bg.code" }
```

## Notes

- **No opaline in runie-core**: Theme is a TUI concern. `runie-core` only holds `theme_name: String`.
- **Snapshot carries theme_name**: The render actor in `runie-term` loads the theme by name on each frame. No `Theme` object crosses the core/tui boundary.
- **Backward compatibility**: If opaline is unavailable (feature gate?), fallback to current hardcoded colors. But since opaline is small and has no heavy deps, we add it unconditionally.
- **Performance**: `opaline::load_by_name()` returns a static reference for builtins (fast). Custom files are loaded once and cached.
