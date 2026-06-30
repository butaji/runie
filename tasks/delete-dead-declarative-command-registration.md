# Delete dead declarative command registration

## Status

`todo`

## Context

`crates/runie-core/src/declarative/register.rs` leaks command names to `'static` and registers intent builders in a global `RwLock<Option<HashMap<&'static str, EventBuilder>>>`. The exported functions are unused outside their own tests; `DeclarativeLoader` is exported but never constructed.

## Goal

Delete `declarative/register.rs`. Route any remaining declarative command loading through the existing `CommandRegistry` (which is already YAML-capable).

## Acceptance Criteria

- [ ] Delete `crates/runie-core/src/declarative/register.rs`.
- [ ] Remove exports and call sites.
- [ ] `cargo check --workspace` passes.
- [ ] No production behavior changes.

## Design Impact

No change to TUI element design or composition. Only internal command registration changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** Command registry still produces the same events.
- **Layer 3 — Rendering:** `TestBackend` command palette unchanged.
- **Layer 4 — E2E:** Headless CLI slash commands work.
- **Live tmux validation:** Slash commands and command palette behave as before.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
