# Theme System

**Status**: done
**Milestone**: R2
**Category**: TUI Rendering / Configuration

## Description

Replace hardcoded `Colors` with [opaline](https://github.com/hyperb1iss/opaline) — a token-based theme engine with 39 builtin themes and custom TOML support.

## Architecture

```rust
// runie-tui only — core holds theme_name string
pub struct RunieTheme {
    inner: opaline::Theme,
}

impl RunieTheme {
    pub fn load(name: &str) -> Self {
        let theme = opaline::load_by_name(name)
            .or_else(|| Self::load_custom(name))
            .unwrap_or_default();
        
        // Register runie-specific default styles
        theme.register_default_style("runie.user", 
            OpalineStyle::fg(theme.color("accent.primary")).bold());
        theme.register_default_style("runie.agent",
            OpalineStyle::fg(theme.color("text.primary")));
        // ... etc
        
        Self { inner: theme }
    }
    
    pub fn style(&self, name: &str) -> Style {
        self.inner.style(name).into()
    }
    
    pub fn color(&self, token: &str) -> Color {
        self.inner.color(token).into()
    }
}
```

### Token Mapping

| Runie | Opaline Token |
|-------|--------------|
| bg | bg.base |
| fg | text.primary |
| accent | accent.primary |
| success | success |
| warning | warning |
| dim | text.dim |
| code | code.function |
| code_bg | bg.code |

### Custom Themes

Users create `~/.runie/themes/my-theme.toml`:
```toml
[meta]
name = "My Theme"
variant = "dark"

[palette]
bg = "#0c0c0c"
fg = "#8a8a8a"
accent = "#8b7cf4"

[tokens]
"text.primary" = "fg"
"bg.base" = "bg"
"accent.primary" = "accent"

[styles]
runie.user = { fg = "accent.primary", bold = true }
```

## Acceptance Criteria

- [ ] `opaline = "0.4"` in `runie-tui/Cargo.toml`
- [ ] `RunieTheme` wraps `opaline::Theme`
- [ ] All 25 `style_*()` functions query theme
- [ ] Builtin themes via `load_by_name()`
- [ ] Custom themes from `~/.runie/themes/*.toml`
- [ ] `theme` field in `config.toml`
- [ ] `/theme <name>` slash command
- [ ] `theme_name` persisted in `Session`
- [ ] Config watcher emits `SwitchTheme`
- [ ] Invalid name falls back to default

## Files

| File | Description |
|------|-------------|
| `crates/runie-tui/Cargo.toml` | Add opaline dependency |
| `crates/runie-tui/src/theme.rs` | Replace Colors with RunieTheme |
| `crates/runie-core/src/event.rs` | `SwitchTheme` event |
| `crates/runie-core/src/model.rs` | `theme_name: String` |
| `crates/runie-core/src/session.rs` | Persist theme_name |
| `crates/runie-core/src/config_reload.rs` | Parse theme field |

## Tests

### Layer 1
- [ ] `theme_loads_builtin` — load_by_name returns valid theme
- [ ] `theme_loads_custom` — finds ~/.runie/themes/*.toml
- [ ] `theme_fallback` — invalid name → default
- [ ] `session_roundtrip` — save/load preserves theme_name

### Layer 2
- [ ] `switch_theme_event` — updates state
- [ ] `config_theme_change` — watcher detects change

### Layer 3
- [ ] `theme_changes_colors` — different border per theme
