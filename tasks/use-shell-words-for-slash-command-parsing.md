# Use shell-words for slash command parsing

## Status

`todo`

## Context

`crates/runie-core/src/commands/registry.rs:264-271` parses slash commands with `input.split_once(' ')`, which cannot handle quoted arguments, flags, or model names containing spaces.

## Goal

Use `shell-words` (already in workspace deps) to tokenize slash input, then dispatch by first token and parse remaining tokens.

## Acceptance Criteria

- [ ] Replace `split_once(' ')` with `shell_words::split`.
- [ ] Handle parse errors with a clear message.
- [ ] Preserve behavior for simple `/command arg` inputs.
- [ ] Tests cover quoted args and flags.

## Design Impact

No change to TUI element design or composition. Only slash command parsing behavior changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for quoted, flag, and simple args.
- **Layer 2 — Event Handling:** Slash commands emit the same events.
- **Layer 3 — Rendering:** `TestBackend` slash palette unchanged.
- **Layer 4 — E2E:** Headless CLI slash commands work.
- **Live tmux validation:** Type `/save "my session"` and verify it is parsed correctly.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
