# Use tagged enum for declarative command kind

## Status

`done`

## Context

`crates/runie-core/src/declarative/types.rs:43-107` uses `kind_type: String` plus many `Option` fields; `to_kind()` does string dispatch and `unwrap_or_default()`.

## Goal

Replace with `#[serde(tag = "type")]` enum `CommandKind { Handler { handler: String }, Msg { message: String }, Form { ... }, FormWithHandler { ... } }`.

## Acceptance Criteria

- [x] Define tagged enum for command kind.
- [x] Preserve YAML format with aliases if needed.
- [x] Remove string dispatch and `Option` fields.
- [x] Tests pass.

## Design Impact

No change to TUI element design or composition. Only declarative command parsing changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for YAML deserialization.
- **Layer 2 — Event Handling:** Declarative commands emit the same events.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Headless CLI loads declarative commands.
- **Live tmux validation:** Custom declarative command works.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
