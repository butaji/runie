# Shrink tasks/ planning debt

**Status**: todo
**Milestone**: R4
**Category**: Configuration
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

`tasks/` has 76 `.md` files at the root (excluding TEMPLATE), 216 done, 48 todo, 3 superseded. The task system has become its own maintenance surface — finding active work requires filtering through done tasks. This contradicts min-code. The `tasks/archive/` dir exists but completed tasks at the root were not moved. Sweep: archive all done/superseded task files, close or re-scope stale todos, shrink `tasks/` to active work + TEMPLATE + index.json only.

## Acceptance Criteria

- [ ] All `status: done` and `status: superseded` task `.md` files at `tasks/` root moved to `tasks/archive/`.
- [ ] `tasks/` root contains only: `TEMPLATE.md`, `index.json`, and `todo`/`in_progress` task `.md` files.
- [ ] `tasks/index.json` `tasks` array entries with `status: done`/`superseded` have their `file` path updated to `tasks/archive/<id>.md`.
- [ ] Stale `todo` tasks reviewed: if the work is already done or no longer relevant, mark `superseded`/`done` and archive; if still relevant, keep and confirm priority.
- [ ] A `tasks/README.md` (or note in `AGENTS.md`) documents: root = active, `archive/` = done.
- [ ] `cargo test --workspace` succeeds (no test depends on task file locations).

## Tests

### Layer 1 — State/Logic
- [ ] `no_done_tasks_at_root` — `ls tasks/*.md` (excluding TEMPLATE, index.json, README) returns only todo/in_progress task files.
- [ ] `index_json_paths_consistent` — every `index.json` entry's `file` field points to a file that exists at the declared path.

### Layer 2 — Event Handling
- [ ] N/A.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Smoke / Crash
- [ ] `cargo test --workspace` green (sanity — no test breaks from file moves).

## Files touched

- `tasks/*.md` (done/superseded) → `tasks/archive/`
- `tasks/index.json` — update `file` paths for archived entries
- `tasks/README.md` → new (or `AGENTS.md` note)

## Notes

This is a one-time sweep, not an ongoing process change. After this, the rule is: when a task is marked `done` in `index.json`, move its `.md` to `archive/` in the same commit. The 76 root files are the result of not following that rule. Rejected alternative: delete done task files entirely — rejected because they contain implementation notes useful for archaeology; archiving preserves them out of the active view.
