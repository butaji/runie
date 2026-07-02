# Derive builders for model and panel builders

## Status

`partial`

## Context

`ModelCapabilities`/`ModelInfo` and `Panel` builders are hand-written consuming builders (~70–270 LOC each).

**Current state (2026-07-02):**
- `ModelCapabilities` — `#[derive(Builder)]` ✓
- `ModelInfo` — `#[derive(Builder)]` ✓
- `Panel` — custom fluent builder methods (not `derive_builder`); the builder methods have custom normalization and UI-specific behavior that makes `derive_builder` a poor fit.

The task is `partial`: model structs are done, Panel remains.

## Goal

Use `derive_builder` or `typed-builder` for `ModelCapabilities`, `ModelInfo`, and `Panel` builders; keep custom helpers where the macro cannot express them.

## Acceptance Criteria
- [x] Add derive to model structs. ✓ (`ModelCapabilities`, `ModelInfo`)
- [ ] Add derive to panel builder. (Panel has custom normalization in `with_title` and UI-specific behavior; `derive_builder` may not be a good fit)
- [x] Update call sites and tests. ✓

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, or documentation changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for builder defaults.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** Panel/command-palette snapshots unchanged.
- **Layer 4 — E2E:** Model selector tests pass.
- **Live tmux testing session (required):** `/model` and `/` palette work.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
