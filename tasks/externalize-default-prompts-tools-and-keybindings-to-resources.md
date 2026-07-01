# Externalize default prompts, tools, and keybindings to resources

## Status

`todo`

## Context

Default system prompt, tool list, and keybindings are hard-coded Rust strings (`prompts.rs`, tool list, `keybindings/defaults.rs`). Editing them requires a recompile.

## Current State

- **Keybindings**: ✅ Externalized to `resources/keybindings/default.yaml` and loaded with `include_str!` in `keybindings/defaults.rs`.
- **Prompts**: ❌ `DEFAULT_PROMPT` is still a raw `&str` constant in `prompts.rs:34`.
- **Tools**: ❌ `DEFAULT_TOOLS` is still a raw `&str` constant in `prompts.rs:37`.

## Goal

Move default prompt(s) and tool list to resources, load with `include_str!`, while preserving runtime overrides.

## Acceptance Criteria

- [ ] Move default prompt(s) to resources.
- [ ] Move default tool descriptions to resources.
- [ ] Move default keybindings to YAML/JSON. — **Done** ✅
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
