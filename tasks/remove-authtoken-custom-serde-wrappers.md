# Remove AuthToken custom serde wrappers

## Status

`todo`

## Context

`crates/runie-core/src/auth/storage.rs:10-24` hand-implements `serialize_secret`/`deserialize_secret` around `secrecy::SecretString`, but `secrecy` already provides serde support.

## Goal

Delete the custom wrappers and rely on the `secrecy` serde feature.

## Acceptance Criteria
- [ ] Delete wrapper functions.
- [ ] Use `SecretString` directly in struct.
- [ ] Verify redacted Debug and snapshot stability.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, or async runtime changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for serialization and Debug redaction.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Auth tests pass.
- **Live tmux validation:** `/login` still works.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
