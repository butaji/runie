# Add `blocked` status and `blocked_reason` fields to tasks

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

Promote `blocked` to a first-class task status with a `blocked_reason` field. Update `tasks/TEMPLATE.md` and regenerate `tasks/index.json` to include the new fields.

## Acceptance Criteria

- [x] `TEMPLATE.md` includes `Status`, `Blocked by`, and `Blocked reason` fields in frontmatter.
- [x] Task frontmatter syntax follows the pattern: `**Status**: (todo | in_progress | done | blocked | wontfix)`.
- [x] `tasks/index.json` includes `status`, `blocked_by`, and `blocked_reason` fields.
- [x] All tasks with `**Status**: blocked` have corresponding `> **Blocked by**:` and `**Blocked reason**:` entries.

## Tests

### Unit tests
- N/A; process task.

### E2E tests
- N/A; process task.

### Live tmux tests
- N/A; process task.

## Files touched

- `tasks/TEMPLATE.md` — added `Status`, `Blocked by`, and `Blocked reason` fields
- `tasks/index.json` — regenerated with `status`, `blocked_by`, and `blocked_reason` fields
- `tasks/*.md` — updated blocked tasks with `**Blocked by**:` and `**Blocked reason**:` entries

## Notes

The `Blocked by` field uses the `> **Blocked by**: ...` blockquote syntax to distinguish from the `Depends on` field which represents hard dependencies. `Blocked by` indicates soft blockers (upstream work not started, awaiting decision, etc.).
