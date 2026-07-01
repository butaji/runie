# Centralize unix_now helper

## Status

`todo`

## Context

`auth/storage.rs`, `proto/message/mod.rs`, and `actors/fff_indexer/mod.rs` each duplicate `SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs_f64())`.

## Goal

Add a shared `unix_now() -> f64` helper and replace all duplicates.

## Acceptance Criteria
- [ ] Add helper in `runie-core` util module.
- [ ] Replace duplicates.
- [ ] `cargo check --workspace` passes.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, or async runtime changes.

## Tests

- **Layer 1 — State/Logic:** Unit test for monotonic-ish behavior.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Tests pass.
- **Live tmux validation:** N/A.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
