# Sync docs with code-review findings

**Status**: done
**Milestone**: R3
**Category**: Configuration
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

The documentation updates have already been applied:

- `AGENTS.md` no longer claims "Current violations: 0"; it now instructs to run `cargo build --workspace` to verify.
- `docs/Architecture.md` includes the concrete async-IO remediation order.

## Acceptance Criteria

- [ ] `AGENTS.md` and `docs/Architecture.md` contain the updated text.

## Tests

- N/A — documentation only.

## Files touched

- `AGENTS.md`
- `docs/Architecture.md`

## Implementation

No further action needed. Verify the relevant sections:

- `AGENTS.md` around the build-guardrail paragraph.
- `docs/Architecture.md` in the async-IO discipline section.

If the stale text reappears, re-apply the edits described in the original task.
