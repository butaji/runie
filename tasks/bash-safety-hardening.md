# Bash Tool Safety Hardening

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Security
**Priority**: P1

**Depends on**: `permission-system-runtime-wiring`
**Blocks**: none

## Description

Safety checks rely on lowercase substring matching (`rm -rf /`, `dd of=/dev/...`, etc.). This is easily evaded by variants such as `rm -rf / --no-preserve-root`, `cd / && rm -rf *`, `rm -rf "$HOME"`, `shred -n1 /dev/sda`, `python -c "..."`, etc. There is also no protection for destructive commands run inside subshells or scripts.

## Acceptance Criteria

- [ ] Replace substring checks with a real shell parser/AST or an explicit allowlist/denylist of command forms.
- [ ] Destructive commands are blocked or require explicit approval regardless of casing or whitespace.
- [ ] Subshells, scripts, and interpreter invocations (`python`, `ruby`, `node`) are handled.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `bash_safety_blocks_evasive_variants` — common evasions are rejected.
- [ ] `bash_safety_allows_read_only_commands` — `ls`, `cat`, `git status`, etc. are allowed.

### Layer 2 — Event Handling
N/A.

### Layer 3 — Rendering
N/A.

### Layer 4 — Smoke / Crash
- [ ] `smoke_destructive_bash_is_blocked` — a destructive command is not executed.

## Files touched

- `crates/runie-agent/src/safety.rs`
- `crates/runie-engine/src/tool/bash.rs`

## Notes

This should be paired with the permission system: any command that cannot be proven read-only should require approval.
