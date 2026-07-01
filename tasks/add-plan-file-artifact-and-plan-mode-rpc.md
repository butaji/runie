# Add plan file artifact and plan mode RPC

## Status

`todo`

## Context

Kimi Code persists the active plan as a markdown file and toggles plan mode via RPC. Runie has no plan artifact.

## Goal

Persist plans as `<session_dir>/plans/<id>.md`, emit `PlanModeEnabled`/plan-file facts, and copy the plan on fork.

## Acceptance Criteria
- [ ] Add plan file storage.
- [ ] Add plan mode RPC events.
- [ ] Restore plan on resume/fork.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, async runtime, or documentation changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for plan file round-trip.
- **Layer 2 — Event Handling:** Plan facts emitted.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Session resume tests include plan.
- **Live tmux testing session (required):** Plan mode persists across restarts.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
