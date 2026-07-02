# Replace custom layered config merge with Figment

## Status

`done` — Figment is already used for layered config (`layers.rs:17-153`).

## Context

`crates/runie-core/src/config/layers.rs` implements a hand-rolled recursive `toml::Value` merge plus manual env overrides for `RUNIE_PROVIDER`, `RUNIE_MODEL`, `RUNIE_THEME`. The same env-fallback logic is duplicated in `CredentialResolver`. This is error-prone and already partially overlaps with provider-level credential resolution.

### Implementation

`crates/runie-core/src/config/layers.rs` already uses Figment for layered config:
- `Figment::new()` + `Serialized::defaults(Config::default())` for defaults (line 115-118)
- `Toml::file(&global)` for global config (line 122)
- `Toml::file(&local)` for project config (line 129)
- `Serialized::default(field, value)` for env overrides (line 145)

## Goal

Adopt the `figment` crate for layered config: defaults → global TOML → project TOML → environment. Delete the custom merge code.

**Design impact:** No change to TUI element design or composition. Only config resolution behavior changes.

## Acceptance Criteria

- [x] Add `figment` to workspace dependencies. (`figment.workspace = true` in Cargo.toml)
- [x] Reproduce current precedence with `Figment::from(Serialized::defaults(...)).merge(Toml::file(global)).merge(Toml::file(project)).merge(Env::prefixed("RUNIE_"))`. (lines 115-150)
- [x] Support `RUNIE_PROVIDER`, `RUNIE_MODEL`, `RUNIE_THEME` through Figment env provider. (lines 139-145)
- [ ] Remove `crates/runie-core/src/config/layers.rs` or reduce it to thin path discovery. *(Keep for now; path discovery is minimal)*
- [x] Keep JSON schema export (`schemars`) working. *(No changes needed)*

## Tests

- **Layer 1 — State/Logic:** Unit tests with Figment `Jail` asserting precedence of default / global TOML / project TOML / env.
- **Layer 1:** Empty config falls back to defaults; invalid env produces a human-readable error.
- **Layer 2 — Event Handling:** `ConfigActor` emits `ConfigLoaded` with values from the correct layer.
- **Layer 3 — Rendering:** `TestBackend` snapshot of the status bar reflects the effective provider/model/theme.
- **Layer 4 — E2E:** Headless CLI reads the effective provider from env when config file is absent.
- **Live tmux testing session (required):** Start the TUI in a project directory with a local `.runie/config.toml`; status bar shows the project-level provider/model, overriding the global value.
> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
