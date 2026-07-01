# Add project config denylist for sensitive keys

## Status

`todo`

## Context

Codex denies dangerous keys in project-local config files to prevent unsafe sharing.

## Goal

Add a project-local config denylist for keys like `model_providers`, `openai_base_url`, `profile`.

## Acceptance Criteria
- [ ] Define denylist.
- [ ] Reject or warn when project config contains denied keys.
- [ ] Document precedence and restrictions.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, or documentation changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for denied-key detection.
- **Layer 2 — Event Handling:** Config-loaded fact excludes/flags denied keys.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Project-layer tests pass.
- **Live tmux validation:** N/A.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
