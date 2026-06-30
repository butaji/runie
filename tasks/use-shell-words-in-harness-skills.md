# Use shell-words in harness skills

## Status

`todo`

## Context

`crates/runie-core/src/harness_skills/verification_loop.rs:56` and `crates/runie-core/src/harness_skills/startup_context.rs:48` split commands with `command.split_whitespace().collect()`, which breaks quoted arguments. `shell-words` is already a workspace dependency.

## Goal

Use `shell_words::split` in harness skills for correct quoted-arg handling.

## Acceptance Criteria

- [ ] Replace `split_whitespace()` with `shell_words::split`.
- [ ] Handle parse errors gracefully.
- [ ] Tests cover quoted paths and arguments with spaces.

## Design Impact

No change to TUI element design or composition. Only harness skill command parsing behavior changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for quoted and unquoted command strings.
- **Layer 2 — Event Handling:** Harness skill emits the same `IoMsg` events.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Harness test with quoted path succeeds.
- **Live tmux validation:** N/A.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
