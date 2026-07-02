# Add `supersedes` field and deduplication sweep task

**Status**: done
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P2

**Depends on**: none
**Blocks**: none
**Supersedes**: none
**Blocked by**: none
**Blocked reason**: none

## Description

Introduce `supersedes` field to task frontmatter for tracking task lineage and deduplication. Create a recurring task to sweep for duplicate or overlapping tasks.

## Acceptance Criteria

- [x] `TEMPLATE.md` includes `Supersedes` field in frontmatter.
- [x] `tasks/index.json` includes `supersedes` field.
- [x] Created `sweep-duplicate-tasks.md` recurring task for deduplication sweeps.

## Tests

### Unit tests
- N/A; process task.

### E2E tests
- N/A; process task.

### Live tmux tests
- N/A; process task.

## Files touched

- `tasks/TEMPLATE.md` — added `Supersedes` field
- `tasks/index.json` — regenerated with `supersedes` field
- `tasks/sweep-duplicate-tasks.md` — new recurring deduplication sweep task

## Notes

The `supersedes` field uses the pattern `**Supersedes**: (task-id or none)` to indicate when a task replaces or supercedes an older task. This helps track task evolution and identify redundant work.
