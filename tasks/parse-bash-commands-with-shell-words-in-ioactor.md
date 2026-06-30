# Parse bash commands with shell-words in IoActor

## Status

`todo`

## Context

`crates/runie-core/src/actors/io/ractor_io.rs:233-250` executes bash commands via `Command::new("sh").arg("-c").arg(command)`. This passes the whole command as a shell string, adds an unnecessary shell indirection, and is inconsistent with `shell-words`, which is already a workspace dependency.

## Goal

Use `shell_words::split` to parse commands into argv, then execute with `tokio::process::Command` directly inside the async `IoActor` handler. Keep `sh -c` only when explicitly requested (e.g., via a `shell: true` flag).

**Design impact:** No change to TUI element design or composition. Only bash-tool execution behavior changes.

## Acceptance Criteria

- [ ] Parse bash tool arguments with `shell_words::split`.
- [ ] Execute with `tokio::process::Command` without a wrapping `sh -c` by default.
- [ ] Remove the `spawn_blocking` wrapper if no longer needed.
- [ ] Preserve quoting and environment variable expansion semantics.

## Tests

- **Layer 1 — State/Logic:** Unit tests for `shell_words::split` on commands with quotes, escapes, and env vars.
- **Layer 1:** Command with shell metacharacters (`|`, `&&`) either rejected or explicitly routed to `sh -c`.
- **Layer 2 — Event Handling:** Send `IoMsg::RunBash` and assert the correct stdout/stderr events are emitted.
- **Layer 3 — Rendering:** `TestBackend` snapshot shows bash output streamed into the message list.
- **Layer 4 — E2E:** Provider replay fixture invokes a bash tool and receives the expected output.
- **Live tmux validation:** In the TUI, ask the agent to run `echo hello world` and a piped command; verify output is captured correctly.
