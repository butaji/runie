# Finish strum migration for remaining enums

## Status

`todo`

## Context

Multiple enums still have manual `FromStr`, `Display`, `as_str` mappings: `commands/dsl/category.rs`, `proto/message/mod.rs`, `provider_event.rs`, `model/state/types.rs`, `agent_phase.rs`, `tool/search/types.rs`, `permissions/mod.rs`.

## Goal

Derive `strum::EnumString`, `Display`, and `IntoStaticStr`; remove manual match tables.

## Acceptance Criteria
- [ ] Identify all target enums.
- [ ] Add strum derives and aliases.
- [ ] Delete manual impls.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, or async runtime changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for parsing and display round-trips.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** All affected crate tests pass.
- **Live tmux testing session (required):** N/A.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
