# Consolidate config modules into one config/ dir

**Status**: done
**Milestone**: R4
**Category**: Configuration
**Priority**: P2

**Depends on**: delete-config-reload-shim, consolidate-login-flow-handlers, split-runie-core-into-domain-and-io-crates
**Blocks**: none

## Description

`runie-core` has seven config-related locations scattered across the src root and dirs:

| Location | LOC | Role |
|----------|-----|------|
| `config.rs` | 467 | Main `Config` struct, `ModelProvider`, `ModelsSection`, `TruncationSection` |
| `config/` (dir) | `layers.rs`, `schema.rs`, `tests/` | Layered config + schemars schema |
| `config_migrate.rs` | — | One-time config migration |
| `config_reload/` (dir) | `mod.rs`, `types.rs` | Reload shim (slated for deletion by `delete-config-reload-shim`) |
| `login_config.rs` | 141 | Login-time config helpers (slated for deletion by `remove-login-config-test-shim`) |
| `login_config/` (dir) | `tests.rs` | Tests for the above |
| `login_flow/` (dir) | ~1,100 | TUI login wizard (moved to TUI by `gate-or-move-single-consumer-core-modules`) |

After `delete-config-reload-shim` removes `config_reload/`, `remove-login-config-test-shim` removes `login_config.rs`, and `gate-or-move-single-consumer-core-modules` moves `login_flow/` to the TUI, what remains is `config.rs` + `config/` (dir) + `config_migrate.rs`. Consolidate these into a single `config/` dir:

```
config/
  mod.rs        (was config.rs — Config struct + sections)
  layers.rs     (was config/layers.rs)
  schema.rs     (was config/schema.rs — if schemars kept; else deleted by reconsider-schemars-jsonschema)
  migrate.rs    (was config_migrate.rs)
  tests/        (was config/tests/)
```

This mirrors `group-session-modules-into-dir` (which consolidates the six `session_*.rs` files into `session/`). After both land, the `runie-core`/`runie-domain` src root drops two more loose files.

## Acceptance Criteria

- [ ] `config.rs` → `config/mod.rs` (content unchanged except path fixes).
- [ ] `config_migrate.rs` → `config/migrate.rs` (content unchanged).
- [ ] `config/layers.rs`, `config/schema.rs`, `config/tests/` stay in place (already under `config/`).
- [ ] No `config*.rs` loose file remains at `crates/runie-core/src/` (or `crates/runie-domain/src/`) root.
- [ ] All `use crate::config_migrate::` and `use runie_core::config_migrate::` callers updated to `use crate::config::migrate::` / `runie_domain::config::migrate::`.
- [ ] `cargo test --workspace` succeeds.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `config_load_round_trips_after_move` — `Config::load`/`save` (or `ConfigStore`) round-trip tests pass from the new `config/mod.rs` location.
- [ ] `config_migrate_still_runs_after_move` — migration logic tests pass from `config/migrate.rs`.

### Layer 2 — Event Handling
- N/A — file relocation.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [ ] `smoke_config_paths_resolve_after_consolidation` — `cargo check --workspace` green; `rg "crate::config_migrate|crate::config\b" crates/` returns the same caller set (paths updated).

## Files touched

- `crates/runie-core/src/config.rs` → `config/mod.rs` (or `crates/runie-domain/src/config/mod.rs` after the crate split)
- `crates/runie-core/src/config_migrate.rs` → `config/migrate.rs`
- Callers of `config_migrate` (grep-driven)
- `crates/runie-core/src/lib.rs` (remove `pub mod config_migrate;`, keep `pub mod config;`)

## Notes

Use `git mv` to preserve history; keep this commit mechanical. Depends on `delete-config-reload-shim` (removes `config_reload/`), `consolidate-login-flow-handlers` (absorbs login_flow handlers), and `split-runie-core-into-domain-and-io-crates` (establishes whether the dir is `runie-core` or `runie-domain`). The `config/schema.rs` file may be deleted by `reconsider-schemars-jsonschema` — order this task after that decision. Mirror the convention of `group-session-modules-into-dir`.
