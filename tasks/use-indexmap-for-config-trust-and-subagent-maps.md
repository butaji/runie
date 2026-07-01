# Use indexmap for config trust and subagent maps

## Status

`todo`

## Context

`HashMap` is used for config sections, trust decisions, resource frontmatter, and subagent registry. Iteration order is nondeterministic, hurting serialization stability and UI ordering.

## Goal

Switch to `indexmap::IndexMap` where user-visible or serialization-roundtrip order matters.

## Acceptance Criteria
- [ ] Identify target maps.
- [ ] Replace with `IndexMap`.
- [ ] Update snapshots if order changes.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, async runtime, or documentation changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for insertion-order preservation.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** Snapshot tests updated.
- **Layer 4 — E2E:** Config/trust/subagent tests pass.
- **Live tmux testing session (required):** `/settings` ordering stable.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
