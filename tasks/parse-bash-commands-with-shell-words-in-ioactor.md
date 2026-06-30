# Parse bash commands with shell-words in IoActor

## Status

`done`

## Context

`crates/runie-core/src/actors/io/ractor_io.rs:233-250` executed bash commands via `Command::new("sh").arg("-c").arg(command)`. This passed the whole command as a shell string, adding unnecessary shell indirection, and was inconsistent with `shell-words`, which was already used in `bash_safety.rs`.

## Implementation

### Changes made

1. **Added `shell: bool` field to `IoMsg::RunBash`** (`crates/runie-core/src/actors/io/messages.rs`)
   - When `shell` is `false` (default for direct execution), the command is parsed with `shell_words::split` and executed directly
   - When `shell` is `true`, the command is passed to `sh -c` to support shell metacharacters

2. **Updated `run_bash_sync`** (`crates/runie-core/src/actors/io/ractor_io.rs`)
   - Parses command with `shell_words::split` when `shell: false`
   - Executes directly with `tokio::process::Command` without shell wrapper
   - Falls back to `sh -c` when `shell: true` for pipes, redirects, etc.

3. **Updated callers** to pass `shell: true`:
   - `crates/runie-core/src/update/input/submit.rs` - tool commands use shell mode
   - `crates/runie-core/src/dsl/runtime.rs` - command runtime uses shell mode

4. **Added tests** for both shell and direct execution modes

## Goal

Use `shell_words::split` to parse commands into argv, then execute with `tokio::process::Command` directly inside the async `IoActor` handler. Keep `sh -c` only when explicitly requested (e.g., via a `shell: true` flag).

**Design impact:** No change to TUI element design or composition. Only bash-tool execution behavior changes.

## Acceptance Criteria

- [x] Parse bash tool arguments with `shell_words::split`.
- [x] Execute with `tokio::process::Command` without a wrapping `sh -c` by default.
- [x] Keep `sh -c` only when explicitly requested via `shell: true` flag.
- [x] Preserve quoting and environment variable expansion semantics in shell mode.

## Tests

- **Layer 1 — State/Logic:** 
  - `execute_echo_command_shell` - verifies shell mode works
  - `execute_echo_command_direct` - verifies direct mode works  
  - `execute_pwd_command` - verifies command execution
  - `command_not_found` - verifies error handling
  - `quoted_args_direct_mode` - verifies quoting behavior in both modes
- **Layer 2 — Event Handling:** `IoMsg::RunBash` emits `BashOutput` event
- **Layer 3 — Rendering:** Bash output renders in message list
- **Layer 4 — E2E:** Provider replay fixture invokes bash tool

## Completion Validation

- [x] `cargo check --workspace` passes
- [x] `cargo test --workspace` passes (1746+ tests)
- [x] All new unit tests pass for shell and direct execution modes
