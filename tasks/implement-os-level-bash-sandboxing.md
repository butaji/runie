# Implement OS-level bash sandboxing

## Status

`todo` — Phase 2 of bash safety hardening; Phase 1 (deny-list) is complete.

## Context

Phase 1 (`replace-bash-safety-heuristic-with-os-sandboxing.md`) replaced the 230-line heuristic with a clean deny-list. Phase 2 adds OS-level sandboxing as an additional layer of defense.

## Goal

Implement OS-level sandboxing for bash tool execution:

1. **macOS**: `sandbox-exec` profile that denies writes outside cwd/network/sensitive paths
2. **Linux**: `landlock` via the `landlock` crate  
3. **Windows**: job objects / Windows Sandbox
4. **Graceful fallback** when OS sandbox is unavailable (log warning, continue without sandbox)

## CLI/Config Integration

- Add `--sandbox` flag to CLI
- Add `sandbox.enabled` field to config
- When enabled, wrap bash execution in OS sandbox
- When unavailable, log warning and proceed without sandbox (deny-list still active)

## Acceptance Criteria

- [ ] Implement platform-specific sandbox profiles
- [ ] Gate sandboxing behind `--sandbox` (CLI) and `sandbox.enabled` (config)
- [ ] Provide graceful fallback when the OS sandbox is unavailable
- [ ] Existing safe commands still work with sandbox enabled
- [ ] Live tmux testing: destructive command blocked with `--sandbox`, allowed without

## Tests

### Layer 4 — E2E
- Live tmux script: run destructive command with and without `--sandbox`, verify sandbox blocks it

## Files to touch

- `crates/runie-core/src/bash_safety.rs` — add sandbox wrapper
- CLI binary argument parsing
- Config schema and loading

## Dependencies

- `landlock` crate for Linux sandboxing (check availability/compatibility)
- `sandbox-exec` on macOS (built-in)
- Windows: TBD (job objects or Windows Sandbox)
