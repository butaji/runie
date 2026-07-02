# Unify bash execution into a single shell module with command-group

## Status

`done`

## Context

Bash execution is currently triplicated across:
- `crates/runie-agent/src/tool/bash.rs:64-96` — async `tokio::process::Command` with hand-rolled timeout/kill.
- `crates/runie-core/src/actors/io/ractor_io.rs:241-308` — sync `sh -c` + direct-execution branch.
- `crates/runie-core/src/update/tools.rs:9-76` — duplicate sync runner.

All three re-implement stdout/stderr collection, exit-code handling, and output formatting. `child.kill()` also does not kill grand-children.

## Goal

Create a single `runie_core::shell` module using `tokio::process::Command` + `command-group` for reliable process-group kill, with `shell-words` parsing. Delete the duplicate runners.

## Acceptance Criteria

- [x] One `run_bash` async function in `runie-core` used by agent tool, IO actor, and update tools.
- [x] Use `command-group` (or `nix` process groups) to kill the whole tree on timeout.
- [x] Preserve `ToolStatus::TimedOut` semantics.
- [x] `shell-words` parses commands; direct execution by default, `sh -c` only when explicitly requested.
- [x] All existing bash tool tests pass.

## Design Impact

No change to TUI element design or composition. Only bash tool execution behavior changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for process-group kill, timeout, exit-code handling, and quoted args.
- **Layer 2 — Event Handling:** `IoMsg::RunBash` emits the same stdout/stderr events.
- **Layer 3 — Rendering:** `TestBackend` shows bash output identically.
- **Layer 4 — E2E:** Provider replay fixture invokes a bash tool and receives the expected output.
- **Live tmux testing session (required):** In the TUI, ask the agent to run `echo hello`, a piped command, and a command that spawns a grand-child; verify timeout kills the whole tree.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [x] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [x] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [x] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
