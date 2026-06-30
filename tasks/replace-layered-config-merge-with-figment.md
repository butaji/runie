# Replace custom layered config merge with Figment

## Status

`todo`

## Context

`crates/runie-core/src/config/layers.rs` implements a hand-rolled recursive `toml::Value` merge plus manual env overrides for `RUNIE_PROVIDER`, `RUNIE_MODEL`, `RUNIE_THEME`. The same env-fallback logic is duplicated in `CredentialResolver`. This is error-prone and already partially overlaps with provider-level credential resolution.

## Goal

Adopt the `figment` crate for layered config: defaults → global TOML → project TOML → environment. Delete the custom merge code.

## Acceptance Criteria

- [ ] Add `figment` to workspace dependencies.
- [ ] Reproduce current precedence with `Figment::from(Serialized::defaults(...)).merge(Toml::file(global)).merge(Toml::file(project)).merge(Env::prefixed("RUNIE_"))`.
- [ ] Support `RUNIE_PROVIDER`, `RUNIE_MODEL`, `RUNIE_THEME` through Figment env provider.
- [ ] Remove `crates/runie-core/src/config/layers.rs` or reduce it to thin path discovery.
- [ ] Keep JSON schema export (`schemars`) working.

## Tests

- **Layer 1 — State/Logic:** Unit tests with Figment `Jail` asserting precedence of default / global TOML / project TOML / env.
- **Layer 1:** Empty config falls back to defaults; invalid env produces a human-readable error.
- **Layer 2 — Event Handling:** `ConfigActor` emits `ConfigLoaded` with values from the correct layer.
- **Layer 3 — Rendering:** `TestBackend` snapshot of the status bar reflects the effective provider/model/theme.
- **Layer 4 — E2E:** Headless CLI reads the effective provider from env when config file is absent.
- **Live tmux validation:** Start the TUI in a project directory with a local `.runie/config.toml`; status bar shows the project-level provider/model, overriding the global value.
