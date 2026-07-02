# Sweep duplicate and overlapping tasks

**Status**: todo
**Milestone**: recurring
**Category**: Architecture / Actors
**Priority**: P3

**Depends on**: none
**Blocks**: none
**Supersedes**: none
**Blocked by**: none
**Blocked reason**: none

## Description

Recurring task to sweep the `tasks/` backlog for duplicate, overlapping, or redundant tasks. Deduplicate by marking one as `done` with `Supersedes: other-task-id.md` and noting the duplicate in the winner's Notes section.

## Scope

### Duplicate patterns to find
- Tasks with identical or near-identical titles
- Tasks that are subsets of larger tasks
- Tasks that were blocked by work that is now `done`
- Tasks that reference each other in `Depends on` but neither is actually blocking the other

### Actions
1. Mark one task as `done` with `Supersedes: other-task-id.md`
2. Update the winner's Notes to explain why it supersedes the other
3. Regenerate `tasks/index.json`
4. Update roadmap count in `docs/superpowers/plans/2026-06-28-runie-cleanup-roadmap.md`

## Acceptance Criteria

- [ ] At least 3 duplicate/overlapping task pairs identified.
- [ ] One task in each pair marked `done` with `Supersedes` field populated.
- [ ] `tasks/index.json` regenerated after changes.
- [ ] Roadmaps updated.

## Tests

### Unit tests
- N/A; process task.

### E2E tests
- N/A; process task.

### Live tmux tests
- N/A; process task.

## Files touched

- `tasks/*.md` — updated with `Supersedes` fields
- `tasks/index.json` — regenerated
- `docs/superpowers/plans/2026-06-28-runie-cleanup-roadmap.md` — updated counts

## Notes

This is a recurring task. Run after major cleanup pushes or when the backlog exceeds 200 active tasks.
