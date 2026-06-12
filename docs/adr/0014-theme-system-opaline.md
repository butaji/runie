# Theme System via Opaline

Runie's theme system is implemented with the [opaline](https://github.com/hyperb1iss/opaline) crate — a token-based theme engine with 39 builtin themes, custom TOML theme support, and deep ratatui integration.

## Decision

- Use `opaline` for all color/styling instead of the hardcoded `Colors` struct
- Keep `theme_name: String` in `runie-core` (`AppState` + `Session`); the actual `Theme` object lives only in `runie-tui`
- Register app-specific styles under the `runie.*` namespace so custom themes can override them
- Support builtin themes (`opaline::load_by_name`), custom themes (`~/.runie/themes/*.toml`), and config-driven startup theme

## Rationale

- **39 builtin themes** — immediate visual variety without user effort
- **Token contract** — 26 semantic tokens + 13 styles are guaranteed in every theme, giving us stable APIs
- **Ratatui adapter** — `theme.style("name")` → `ratatui::style::Style` with no glue code
- **Custom themes** — users drop TOML files; no compile-time theme registration needed
- **Small footprint** — opaline has no heavy dependencies beyond `serde` and `ratatui` types

## App-Specific Token Registration

Opaline's contract covers generic UI tokens. Runie registers additional default styles so every theme works out of the box:

```
runie.user              → bold accent.primary
runie.agent             → text.primary
runie.thought           → text.dim
runie.tool.header       → text.muted
runie.tool.output       → text.primary
runie.tool.running      → text.dim
runie.status.active     → success
runie.status.idle       → text.dim
runie.border            → border.unfocused
runie.border.flash      → warning
runie.code.block        → code.function on bg.code
runie.input.cursor      → bg.base on text.primary
runie.popup.selected    → accent.secondary on bg.highlight, bold
runie.popup.unselected  → text.secondary
runie.popup.border      → border.focused
runie.turn.complete     → text.dim
runie.empty             → text.dim
```

Custom theme TOML can override any of these via `[styles.runie.user]` etc.

## Boundary

| Crate | Responsibility |
|-------|---------------|
| `runie-core` | Holds `theme_name: String`; emits `SwitchTheme` event; persists in `Session` |
| `runie-tui` | Loads `opaline::Theme` by name; maps tokens to `style_*()` functions; renders |
| `runie-term` | Receives `theme_name` in `Snapshot`; no theme logic |

## Events

- `SwitchTheme { name: String }` — emitted by `/theme` command or config watcher
- `AppState` updates `theme_name` and adds a system message indicating the change

## Custom Theme Path

```
~/.runie/themes/
  my-theme.toml
  dracula-custom.toml
```

Resolution order: builtin name → custom file → fallback default.
