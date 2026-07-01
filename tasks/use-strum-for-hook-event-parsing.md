# Use strum for hook event parsing

## Status

`todo`

## Context

`crates/runie-core/src/hooks.rs::parse_event_name` is a manual match over snake/camel-case strings.

## Goal

Replace with `strum::EnumString` and `HookEvent::from_str`.

## Acceptance Criteria
- [ ] Derive `EnumString` on `HookEvent` with aliases.
- [ ] Delete `parse_event_name`.
- [ ] Unknown names still return `None`.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, or documentation changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for alias parsing.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Hook tests pass.
- **Live tmux validation:** N/A.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
