# Remove AuthToken custom serde wrappers

## Status

`wontfix`

## Context

`crates/runie-core/src/auth/storage.rs:10-24` hand-implements `serialize_secret`/`deserialize_secret` around `secrecy::SecretString`.

## Why Not Applicable

The `secrecy` crate's `serde` feature requires implementing the `SerializableSecret` marker trait on the inner type to enable automatic `Serialize`/`Deserialize` derives. Since `String` is in the standard library, we cannot implement `SerializableSecret` for it from external code.

The current custom serde wrappers are the correct approach for this use case.

## Acceptance Criteria
- [x] Custom wrappers kept (correct approach for this case).
- [x] `SecretString` used directly in struct.
- [x] Debug redaction verified and tests pass.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, or async runtime changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for serialization and Debug redaction.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Auth tests pass.
- **Live tmux testing session (required):** `/login` still works.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [x] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [x] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [x] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
