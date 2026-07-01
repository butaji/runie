# Delete dead RactorAgentHandleExt trait

## Status

`todo`

## Context

`crates/runie-agent/src/actor.rs:265-276` exports `RactorAgentHandleExt::run_if_queued`, but the TUI defines its own `run_if_queued` on `AgentActorHandle` and `LeaderAgentActorHandle`. The trait is dead public API.

## Goal

Remove `RactorAgentHandleExt` and its re-export.

## Acceptance Criteria
- [ ] Delete trait and impl.
- [ ] Update `lib.rs` re-exports.
- [ ] `cargo check --workspace` passes.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, async runtime, or documentation changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** `cargo check` passes.
- **Live tmux validation:** N/A.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
