# Use which crate for executable lookup

## Status

`todo`

## Context

`crates/runie-core/src/tool/format.rs:6-24` shells out to `/usr/bin/which` to locate executables. This adds a subprocess, fails on systems without `which`, and is unnecessary.

## Goal

Replace the subprocess with the `which` crate (already used by Goose). No shelling out.

## Acceptance Criteria

- [ ] Remove `tokio::process::Command("which", ...)` call.
- [ ] Use `which::which_global` or `which::which_in`.
- [ ] Handle missing executable the same way as before.
- [ ] Tests pass on systems without a `which` binary.

## Design Impact

No change to TUI element design or composition. Only tool availability detection behavior changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for finding known executables and handling missing ones.
- **Layer 2 — Event Handling:** Tool formatting emits the same availability fact.
- **Layer 3 — Rendering:** Tool list shows availability as before.
- **Layer 4 — E2E:** Headless CLI tool that checks for `git` works.
- **Live tmux validation:** Ask the agent to run a missing command; error message is the same.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
