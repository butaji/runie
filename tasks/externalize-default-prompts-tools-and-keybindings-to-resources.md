# Externalize default prompts, tools, and keybindings to resources

## Status

`todo`

## Context

Default system prompt, tool list, and keybindings are hard-coded Rust strings (`prompts.rs`, tool list, `keybindings/defaults.rs`). Editing them requires a recompile.

## Goal

Move them to `resources/prompts/`, `resources/tools/`, and `resources/keybindings/default.yaml`, loading with `include_str!`/`include_dir!`.

## Acceptance Criteria

- [ ] Move default prompt(s) to resources.
- [ ] Move default tool descriptions to resources.
- [ ] Move default keybindings to YAML/JSON.
- [ ] Load at startup; preserve runtime overrides.
- [ ] All tests pass.

## Design Impact

No change to TUI element design or composition. Only resource loading changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for resource loading.
- **Layer 2 — Event Handling:** Config/keybinding facts unchanged.
- **Layer 3 — Rendering:** `TestBackend` snapshots match.
- **Layer 4 — E2E:** Headless CLI loads defaults.
- **Live tmux validation:** Default shortcuts and system prompt work.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
