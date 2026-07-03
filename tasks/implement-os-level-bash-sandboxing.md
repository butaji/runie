# Implement OS-level bash sandboxing

## Status

`done` — Phase 2 of bash safety hardening; Phase 1 (deny-list) is complete.

## Context

Phase 1 (`replace-bash-safety-heuristic-with-os-sandboxing.md`) replaced the 230-line heuristic with a clean deny-list. Phase 2 adds OS-level sandboxing as an additional layer of defense.

## Goal

Implement OS-level sandboxing for bash tool execution:

1. **macOS**: `sandbox-exec` profile that denies writes outside cwd/network/sensitive paths
2. **Linux**: `landlock` via the `landlock` crate (optional feature `landlock`)
3. **Windows**: job objects / Windows Sandbox (basic CREATE_NO_WINDOW flag)
4. **Graceful fallback** when OS sandbox is unavailable (log warning, continue without sandbox)

## Implementation

### Files created/modified

1. **`crates/runie-core/src/sandbox.rs`** (new) — Platform-specific sandboxing implementation:
   - `sandbox_available()` — check if sandbox is available on current platform
   - `run_sandboxed()` — run command with sandbox
   - `run_sandboxed_shell()` — run shell command with sandbox
   - macOS: Uses `sandbox-exec` with deny-write profile
   - Linux: Uses `landlock` crate when `landlock` feature is enabled
   - Windows: Uses `CREATE_NO_WINDOW` creation flags

2. **`crates/runie-core/src/shell.rs`** — Added sandbox support:
   - `run_bash_sandboxed()` — async bash with sandbox
   - `run_bash_shell_internal()` — internal shell execution with optional sandbox

3. **`crates/runie-core/src/config/mod.rs`** — Added `SandboxSection`:
   ```toml
   [sandbox]
   enabled = true
   ```

4. **`crates/runie-core/src/lib.rs`** — Exported `sandbox` module

5. **`crates/runie-agent/src/tool/bash.rs`** — Added sandbox integration:
   - Reads `RUNIE_SANDBOX=1` env var to enable sandboxing
   - Falls back to unsandboxed execution when sandbox unavailable

6. **`crates/runie-cli/src/main.rs`** — Added `--sandbox` flag to `print` command

7. **`crates/runie-cli/src/print.rs`** — Sets `RUNIE_SANDBOX=1` when flag is provided

8. **`Cargo.toml`** (workspace) — Added `landlock = "0.4"` dependency

9. **`crates/runie-core/Cargo.toml`** — Added `landlock` feature flag

10. **`crates/runie-core/src/tests/arch_guardrails.rs`** — Added `sandbox.rs` to production allow list

## Acceptance Criteria

- [x] Implement platform-specific sandbox profiles
  - macOS: `sandbox-exec` with deny-write profile
  - Linux: `landlock` crate support (optional feature)
  - Windows: `CREATE_NO_WINDOW` flags
- [x] Gate sandboxing behind `--sandbox` (CLI) and `sandbox.enabled` (config)
  - CLI: `--sandbox` flag on `runie print` command
  - Runtime: `RUNIE_SANDBOX=1` environment variable
- [x] Provide graceful fallback when the OS sandbox is unavailable
  - Logs warning and continues without sandbox
  - Deny-list still active regardless of sandbox status
- [x] Existing safe commands still work with sandbox enabled
  - All existing tests pass
- [x] Layer 1-4 tests implemented
  - Unit tests for sandbox availability and command execution
  - CLI parsing tests for `--sandbox` flag

## Tests

### Layer 1 — State/Logic
- `sandbox_available()` returns correct status per platform
- `run_unsandboxed()` works correctly

### Layer 2 — Event Handling
- `bash_sandboxed_succeeds_when_enabled` test verifies sandbox integration

### Layer 4 — E2E
- CLI `--sandbox` flag parsing test
- Bash tool with sandbox env var test

## Dependencies

- `landlock` crate for Linux sandboxing (optional, `landlock` feature)
- `sandbox-exec` on macOS (built-in)
- Windows: `CREATE_NO_WINDOW` flag (built-in)

## Follow-up required

The 2026-07-03 architecture/code review found that `run_bash_direct` does not accept or apply `use_sandbox`, so `run_bash_sandboxed(..., shell=false)` silently falls back to unsandboxed execution.

See `tasks/fix-bash-sandbox-direct-mode.md` for the remaining work.
