# Round-trip session tree snapshot through durable events

## Status

`todo`

## Context

`crates/runie-core/src/event/durable.rs:367-370` added `DurableCoreEvent::TreeSnapshot`, but `try_from_event` never produces it and `Event::try_from(&DurableCoreEvent)` returns `Err(())` for it.

## Goal

Add an `Event` variant and conversions so session tree branching state actually persists and loads.

## Acceptance Criteria
- [ ] Add `Event::SessionTreeSnapshot` (or reuse `SessionChanged`).
- [ ] Implement both directions in `durable.rs`.
- [ ] Add round-trip test.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, async runtime, or documentation changes.

## Tests

- **Layer 1 — State/Logic:** Unit test for durable event round-trip.
- **Layer 2 — Event Handling:** Save/load emits tree snapshot fact.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Session fork/resume tests pass.
- **Live tmux validation:** Fork a session and resume it with tree intact.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
