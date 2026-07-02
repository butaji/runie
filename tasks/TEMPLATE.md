# Task Title

**Status**: (todo | in_progress | done | blocked | wontfix)
**Milestone**: (MVP | R1 | R2 | R3 | R4 | R5 | R6 | R7)
**Category**: (Core / State | Tools | TUI / Rendering | Input / Commands | Sessions | Configuration | Architecture / Actors | Architecture / Security | ...)
**Priority**: (P0 | P1 | P2)

**Depends on**: (task-ids or none)
**Blocks**: (task-ids or none)
**Supersedes**: (task-id or none)
**Blocked by**: (task-ids or none)
**Blocked reason**: (reason string or none)

## Description

One-paragraph summary of the problem or goal. Keep it concrete and scoped.

## Acceptance Criteria

- [x] Criterion that can be verified by reading code or running tests.
- [x] Another criterion.
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.
- [x] A live tmux testing session was run and passed.

## Tests

Reference the four testing layers from `AGENTS.md`. Omit a layer only if it is genuinely not applicable, and explain why.

### Layer 1 — State/Logic
- [x] `test_name` — what it verifies.

### Layer 2 — Event Handling
- [x] `test_name` — what it verifies.

### Layer 3 — Rendering
- [x] `test_name` — what it verifies (or N/A with explanation).

### Layer 4 — Smoke / Crash
- [x] `test_name` — what it verifies (or N/A with explanation).

### Live Tmux Testing Session
- [x] A real terminal tmux session was run after implementation; the changed behavior was exercised and passed.

## Files touched

- `path/to/file.rs`
- `path/to/another.rs`

## Notes

Any context, rejected alternatives, or out-of-scope items.
