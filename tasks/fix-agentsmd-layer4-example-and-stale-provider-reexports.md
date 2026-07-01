# Fix AGENTS.md Layer-4 example and stale provider re-exports

## Status

`todo`

## Context

`AGENTS.md` Layer-4 example uses deleted `DynProvider::from_provider`; `runie-provider/src/config/mod.rs` re-exports `Config`/`ModelProvider` with a stale backward-compat comment.

## Goal

Replace the example with `BuiltProvider::from_provider`; remove the stale re-export and comment.

## Acceptance Criteria
- [ ] Update `AGENTS.md` example.
- [ ] Remove stale re-exports/comment.
- [ ] `cargo check --workspace` passes.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, or documentation changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Example compiles when copied to a test.
- **Live tmux validation:** N/A.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
