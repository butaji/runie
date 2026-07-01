# Collapse `DurableCoreEvent` into the canonical `Event` enum

## Status

`todo`

## Description

`crates/runie-core/src/event/durable.rs` maintains a parallel `DurableCoreEvent` enum with ~300 lines of hand-written `TryFrom` conversions. A single canonical `Event` enum with `#[serde(skip)]` transient fields can replace both enums.

## Acceptance criteria

1. **Unit tests** — Every transient `Event` variant serializes to skip/`None`; every durable variant round-trips through JSON.
2. **E2E tests** — Replaying a session from durable events produces the same `AppState` as before.
3. **Live run tests** — Save and resume a session in tmux; persisted events restore the same UI state.

## Tests

### Unit tests
- Every transient `Event` variant serializes to `None`/`skip`.
- Every durable variant round-trips through JSON.

### E2E tests
- Replaying a session from durable events produces the same `AppState` as before.

### Live run tests
- Save a session in tmux, restart, and resume to the same point.
