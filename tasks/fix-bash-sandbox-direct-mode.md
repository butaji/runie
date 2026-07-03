# Fix bash sandboxing for direct (non-shell) mode

**Status**: done
**Milestone**: R7
**Category**: Architecture / Security
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-core/src/shell.rs:84-88` branches on `shell` but does not pass `use_sandbox` to `run_bash_direct`:

```rust
if shell {
    run_bash_shell_internal(command, working_dir, env, timeout, use_sandbox).await
} else {
    run_bash_direct(command, working_dir, env, timeout).await
}
```

This means `run_bash_sandboxed(..., shell=false)` silently falls back to unsandboxed execution, contradicting the API name and the security intent documented in `implement-os-level-bash-sandboxing.md`.

## Acceptance Criteria

- [x] Thread `use_sandbox` through `run_bash_direct` and apply the same sandbox/deny-list as shell mode.
- [x] OR explicitly document that sandboxing requires `shell=true` and make `run_bash_sandboxed` reject/warn when called with `shell=false`.
- [x] Add tests covering both `shell=true` and `shell=false` sandboxed paths.
- [x] `cargo test --workspace` passes.
- [x] `cargo check --workspace` passes with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `sandbox_applies_to_direct_mode` — `run_bash_sandboxed(cmd, shell=false)` uses sandbox restrictions.
- [x] `sandbox_rejects_direct_shell_false` — or, if documented limitation, rejects the call with a clear error.

### Layer 2 — Event Handling
- [x] N/A — shell execution concern.

### Layer 3 — Rendering
- [x] N/A — no rendering change.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] `bash_tool_sandboxed_direct_mode` — agent bash tool with sandbox enabled and `shell=false` is restricted.

### Live Tmux Testing Session
- [x] Run a headless turn that uses the bash tool with sandboxing enabled and verify restrictions apply regardless of `shell` flag.

## Files touched

- `crates/runie-core/src/shell.rs`
- `crates/runie-core/src/sandbox.rs`
- `crates/runie-agent/src/tool/bash.rs`

## Notes

- Supersedes the remaining work from `implement-os-level-bash-sandboxing.md`.
- Prefer the "thread through" fix unless there is a documented reason direct mode cannot be sandboxed.
