# Delete dead declarative command registration

## Status

`done`

## Context

`crates/runie-core/src/declarative/register.rs` leaked command names to `'static` and registered intent builders in a global `RwLock<Option<HashMap<&'static str, EventBuilder>>>`. The exported functions were unused outside their own tests; `DeclarativeLoader` was exported but never constructed.

## Goal

Delete `declarative/register.rs`. Route any remaining declarative command loading through the existing `CommandRegistry` (which is already YAML-capable).

## Acceptance Criteria

- [x] Delete `crates/runie-core/src/declarative/register.rs`. — **Done**; file deleted. `declarative/` now only contains `loader.rs`, `mod.rs`, `tests.rs`, `types.rs`.
- [x] Remove exports and call sites. — **Done**; no remaining imports of `register.rs`.
- [x] `cargo check --workspace` passes. — **Done**; verified 2026-07-01.
- [x] No production behavior changes. — **Done**; declarative commands are loaded via `loader.rs` + `CommandRegistry`.

## Design Impact

No change to TUI element design or composition. Only internal command registration changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** Command registry still produces the same events.
- **Layer 3 — Rendering:** `TestBackend` command palette unchanged.
- **Layer 4 — E2E:** Headless CLI slash commands work.
- **Live tmux validation:** Slash commands and command palette behave as before.

## Completion Validation

- [x] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [x] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
