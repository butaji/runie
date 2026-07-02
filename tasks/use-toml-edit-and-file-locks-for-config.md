# Use toml_edit + fs2 file locks for config persistence

## Status

`partial` — `fs2` locks are implemented. `toml_edit` for comment preservation is not.

## Context

`crates/runie-core/src/config/config_impl.rs` and `crates/runie-core/src/provider/config.rs` load the whole TOML file into `toml::Value`, mutate it, and write it back with `toml::to_string_pretty`. This strips user comments and formatting. The in-process `RwLock` in `provider/config.rs` does not protect across separate CLI processes, so `runie login` and `runie mcp add` can race on `config.toml`.

### Implementation Status

- **fs2 locks**: ✅ Implemented in `config_impl.rs:334` and `provider/config.rs:43`
- **toml_edit**: ❌ Not implemented; `toml::to_string_pretty` still strips comments

## Goal

Make config edits comment-preserving and cross-process safe by using `toml_edit` for surgical edits and `fs2` advisory file locks.

**Design impact:** No change to TUI element design or composition. Only internal config persistence behavior changes.

## Acceptance Criteria

- [ ] Replace load-modify-save via `toml::Value` with `toml_edit::Document` edits for provider, MCP, theme, and auth sections.
- [ ] Preserve comments and key order in existing `config.toml` files after edits.
- [x] Use `fs2::FileExt::lock_exclusive` / `lock_shared` around reads and writes. (config_impl.rs:334, provider/config.rs:43)
- [x] Remove the process-level `RwLock<()>` in `provider/config.rs`. (replaced with fs2 locks)
- [x] No new `std::fs` writes on the async runtime thread. (uses `spawn_blocking` via `save_nonblocking`)

## Tests

- **Layer 1 — State/Logic:** Serialize a TOML document with comments, apply an edit, assert comments and formatting survive.
- **Layer 1:** Simulate concurrent writer processes using `fs2` locks and verify no corruption.
- **Layer 2 — Event Handling:** Send `ConfigMsg::SetProvider` and assert the emitted `ConfigLoaded` fact contains the edited value.
- **Layer 3 — Rendering (if TUI-visible):** `TestBackend` snapshot of `/settings` or `/inspect` after a config edit shows the new value.
- **Layer 4 — Provider Replay / E2E:** Run `runie login mock` and `runie mcp add` in parallel from two processes; both succeed and config remains valid TOML.
- **Live tmux testing session (required):** Start the TUI, run `/login mock`, `/model mock-model`, and `/mcp add ...`; inspect `~/.runie/config.toml` and confirm comments and formatting are preserved.
> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
