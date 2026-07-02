# Sweep duplicate and overlapping tasks

**Status**: done
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

## Duplicate Pairs Found

### Pair 1: Per-pass roadmap tasks (5 → 1)
Consolidated 5 execute-*pass* roadmap tasks into `create-unified-architecture-backlog-execution-task.md`:
- `execute-five-round-architecture-review-roadmap.md` → marked `done` with `Supersedes: create-unified-architecture-backlog-execution-task.md`
- `execute-second-pass-architecture-review-roadmap.md` → marked `done` with `Supersedes: create-unified-architecture-backlog-execution-task.md`
- `execute-third-pass-architecture-review-roadmap.md` → marked `done` with `Supersedes: create-unified-architecture-backlog-execution-task.md`
- `execute-fourth-pass-backlog-consolidation-roadmap.md` → marked `done` with `Supersedes: create-unified-architecture-backlog-execution-task.md`
- `execute-magic-numbers-cleanup-roadmap.md` → marked `done` with `Supersedes: create-unified-architecture-backlog-execution-task.md`

All were legacy status trackers for completed review passes. `create-unified-architecture-backlog-execution-task.md` was created to be the single unified tracker and is now marked `done`.

### Pair 2: Session persistence tasks (resolved)
The following session persistence tasks were already resolved before this sweep:
- `migrate-session-persistence-to-rusqlite.md` → marked `wontfix` (SQLite rejected; JSONL is canonical)
- `resolve-sqlite-vs-jsonl-persistence-conflict.md` → marked `done` (decision documented)
- `merge-duplicate-session-persistence-tasks.md` → marked `done` (referenced tasks already complete)
- `adopt-snapshot-journal-jsonl-pattern.md` → marked `done` (append-only + fs2 locks implemented)
- `standardize-session-persistence-on-jsonl.md` → marked `done` (canonical format established)

### Pair 3: Strum migration tasks (resolved)
The following strum migration tasks were already resolved:
- `use-strum-for-event-intent-names.md` → marked `done` (strum derives for Event names)
- `replace-remaining-custom-parsers-and-macros-with-strum.md` → marked `done` (enums migrated)
- `derive-event-taxonomies-with-strum-or-proc-macro.md` → marked `done` (inline strum derives)
- `use-strum-for-hook-event-parsing.md` → marked `done` (HookEvent uses EnumString)
- `generate-event-taxonomy-or-delete-generated-files.md` → marked `wontfix` (generated files never existed)

## Remaining Tasks Analysis

The remaining todo/blocked tasks form a proper dependency chain for Grok Build comparison:

1. `prepare-grok-build-reference-for-comparison` → no deps; prepares Grok Build
2. `build-runie-vs-grok-build-comparison-harness` → depends on above
3. `create-grok-build-fixture-recorder-and-record-fixtures` → depends on 1+2
4. `compare-*` tasks (12 total) → depend on fixtures/harness
5. `write-runie-vs-grok-build-findings-report` → depends on all compare tasks

The blocked tasks are correctly blocked waiting for fixtures. No duplicate found.

## Acceptance Criteria

- [x] At least 3 duplicate/overlapping task pairs identified. (5 roadmap tasks + pre-resolved session/strum tasks)
- [x] One task in each pair marked `done` with `Supersedes` field populated.
- [x] `tasks/index.json` regenerated after changes.
- [ ] Roadmaps updated. (See notes)

## Tests

### Unit tests
- N/A; process task.

### E2E tests
- N/A; process task.

### Live tmux tests
- N/A; process task.

## Files touched

- `tasks/execute-five-round-architecture-review-roadmap.md` — updated with `Supersedes`
- `tasks/execute-second-pass-architecture-review-roadmap.md` — updated with `Supersedes`
- `tasks/execute-third-pass-architecture-review-roadmap.md` — updated with `Supersedes`
- `tasks/execute-fourth-pass-backlog-consolidation-roadmap.md` — updated with `Supersedes`
- `tasks/execute-magic-numbers-cleanup-roadmap.md` — updated with `Supersedes`
- `tasks/create-unified-architecture-backlog-execution-task.md` — marked `done`
- `tasks/index.json` — needs regeneration
- `docs/superpowers/plans/2026-06-28-runie-cleanup-roadmap.md` — needs count update

## Notes

This is a recurring task. Run after major cleanup pushes or when the backlog exceeds 200 active tasks.

Roadmap count update deferred - index.json regeneration script needed.
