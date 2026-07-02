# Replace bash safety heuristic with OS sandboxing

## Status

**partial** — Deny-list simplification done (230 → 227 lines, cleaner structure, interpreter bypass fixed). OS-level sandboxing not yet implemented.

## Context

`crates/runie-core/src/bash_safety.rs` was a 230-line hand-rolled heuristic. It was easily bypassed via interpreter injection (e.g., `bash -c 'exec rm -rf /'`).

## Changes Made

### Phase 1: Deny-list simplification (done)

1. **Replaced 230-line multi-function heuristic with a clean deny-list.** The check now iterates through a `&[DenyEntry]` table of `fn(&str) -> bool` predicates.

2. **Fixed BASH-253: interpreter bypass.** The original `check_interpreter_attack` only checked tokens after the interpreter name. The fix joins all tokens back into a string and checks the full command for dangerous patterns — catching `bash -c 'exec rm -rf /'`, `python -c 'import os; os.system("rm -rf /")'`, etc.

3. **Fixed shell_words space-insertion bug.** `shell_words` inserts spaces around `>`, so `cat /dev/zero > /dev/sda` tokenizes as `["cat", "/dev/zero", ">", "/dev/sda"]`. Added both `>/dev/` and ` > /dev/` patterns to catch both direct-mode and tokenized-mode writes to block devices.

4. **Added `dd` to partition-tools check.** The original code checked `dd` via block-device patterns but not as a standalone tool.

5. **Added comprehensive test coverage** for interpreter bypass variants (`bash -c`, `python -c`, `ruby -e`, `perl -e`, `node -e`), recursive chmod, and device-write variants.

### Phase 2: OS-level sandboxing (not yet done)

Remaining work:
- macOS: `sandbox-exec` profile as opt-in `--sandbox` flag.
- Linux: `landlock` via the `landlock` crate.
- Windows: job objects / Windows Sandbox.
- Graceful fallback when OS sandbox is unavailable.
- Gate behind `--sandbox` CLI flag and config flag.

## Acceptance Criteria

- [x] Replace the large heuristic with a small deny-list for obviously dangerous strings. (Done — deny-list table with 6 check functions)
- [x] Fix interpreter bypass (`bash -c 'exec rm -rf /'` etc.). (Done — full-command check)
- [x] Fix shell_words space-insertion for redirect patterns. (Done — both direct and tokenized patterns)
- [ ] Implement platform sandbox profiles that deny writes outside cwd/network/sensitive paths. (Not done)
- [ ] Gate sandboxing behind `--sandbox` (CLI) and a config flag (TUI). (Not done)
- [ ] Provide graceful fallback when the OS sandbox is unavailable. (Not done)
- [x] Existing safe commands still work without the flag. (Done — tests pass)

## Design Impact

No change to TUI element design or composition. Only bash tool security behavior changes.

## Tests

### Layer 1 — State/Logic
- `blocks_direct_destructive_commands` — `rm -rf /`, `dd`, `mkfs`, fork bomb, sudo rm
- `blocks_recursive_chmod` — `chmod -R 777 /`, `chmod 777 /root`
- `blocks_evasive_variants` — interpreter bypass, quoted paths, device writes, find exec rm
- `allows_safe_commands` — echo, ls, git, cargo, npm, python safe scripts
- `allows_nested_interpreter_safe_commands` — `bash -c 'echo hello'`, `python -c 'print(1+1)'`
- `empty_command_is_safe` — empty and whitespace-only commands

### Layer 2 — Event Handling
- Agent safety tests: `test_bash_safety_*` in `runie-agent`

### Layer 4 — E2E
- `bash_tool` in `runie-agent` exercises the safety check

## Files touched

- `crates/runie-core/src/bash_safety.rs` — complete rewrite with deny-list table
- `crates/runie-agent/src/tool/bash.rs` — unchanged (uses `check_bash_safety`)

## Notes

- OS sandboxing is a future enhancement; the deny-list approach is the primary safety mechanism for now.
- The `SafetyResult` type alias documents the return convention: `None` = safe, `Some(reason)` = blocked.
- `shell_words` tokenization is preserved because it correctly handles quoting (needed for the `> /dev/` space-insertion fix).
- The `has_recursive_rm` check uses lowercase comparison to handle mixed-case paths.

> **Live tmux testing session required for Phase 2:** After the OS sandboxing is implemented, run a destructive command with and without `--sandbox`; verify the sandbox blocks it.
## Completion Validation

- [x] **Unit tests** — `cargo test bash_safety --workspace` passes (14 tests).
- [x] **E2E tests** — `cargo test --workspace` passes (all 2000+ tests).
- [ ] **Live tmux run tests** — Phase 2 (OS sandboxing) requires live session testing.
