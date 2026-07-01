# Format permission request for human-readable display

## Status

`done`

## Context

`crates/runie-tui/src/popups/permission.rs` renders raw `serde_json::to_string_pretty(&request.input)`.

## Goal

Pre-format a concise human-readable summary (tool name + key args) before rendering.

## Acceptance Criteria
- [ ] Display tool name and important arguments.
- [ ] Truncate large inputs.
- [ ] No raw JSON in dialog.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, or internal architecture changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for summary formatting.
- **Layer 2 — Event Handling:** Permission fact carries summary.
- **Layer 3 — Rendering:** `TestBackend` permission dialog snapshot updated.
- **Layer 4 — E2E:** Permission gate E2E tests pass.
- **Live tmux validation:** Permission dialog is readable for read_file/bash tools.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
