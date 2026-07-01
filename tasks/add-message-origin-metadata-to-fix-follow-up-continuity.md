# Add message origin metadata to fix follow-up continuity

## Status

`todo`

## Context

Runie cannot distinguish a real user follow-up from injected tool/system context, causing follow-ups to get stuck behind an active turn.

## Goal

Add `origin` to `ChatMessage`/`AgentEvent` flow so the turn engine can separate user messages from injections.

## Acceptance Criteria
- [ ] Define `MessageOrigin` enum (User, Tool, System, Compaction, etc.).
- [ ] Tag messages at creation points.
- [ ] Use origin in turn scheduling.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, or async runtime changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for origin-based scheduling.
- **Layer 2 — Event Handling:** Follow-up events carry `User` origin.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Multi-turn replay tests pass.
- **Live tmux validation:** User follow-up starts a new turn reliably.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
