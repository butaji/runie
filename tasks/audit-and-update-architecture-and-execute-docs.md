# Audit and update Architecture and EXECUTE docs

## Status

`todo`

## Context

`docs/Architecture.md` lists non-existent crates (`runie-server`, `runie-protocol`, `runie-util`) and speculative actors; `EXECUTE.md` describes stale architecture.

## Goal

Update crate map and architecture description; move speculative sections to Future/R4; correct `EXECUTE.md`.

## Acceptance Criteria
- [ ] List actual workspace members.
- [ ] Remove or mark speculative actors/IPC.
- [ ] Correct `EXECUTE.md` statements.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, or documentation changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** N/A.
- **Live tmux validation:** N/A.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
