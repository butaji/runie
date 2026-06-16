# R3 Implementation Roadmap

**Status**: done
**Milestone**: R3
**Category**: Core / State
**Priority**: P0

**Depends on**: (none)
**Blocks**: (none)

## Description

Create and maintain a sequenced implementation roadmap for R3/R4 work. The roadmap groups active tasks into dependency-ordered phases so the team can ship incrementally without blocking unrelated work.

## What Was Done

- Created/updated `docs/ROADMAP.md` with 6 phases covering all active R3 tasks
- Each phase has a clear goal and a checklist of task IDs with current status
- Dependencies between phases are accurate (FFF tools depend on fff-indexer-actor; TUI depends on merge-runie-term-into-tui)
- `docs/SPEC.md` already links to `docs/ROADMAP.md`
- Roadmap is updated to reflect current state

## Acceptance Criteria

- [x] `docs/ROADMAP.md` exists and lists all active R3/R4 tasks in phase order.
- [x] Each phase has a clear goal and a checklist of task IDs.
- [x] Dependencies between phases are accurate.
- [x] `docs/SPEC.md` links to `docs/ROADMAP.md`.
- [x] The roadmap is updated when tasks are added, archived, or reprioritized.

## Tests

### Layer 1 — State/Logic
- [x] `roadmap_covers_all_active_r3_tasks` — every active R3 task appears in at least one phase.
- [x] `roadmap_dependencies_acyclic` — no circular dependencies between phases.

## Files touched

- `docs/ROADMAP.md`
- `docs/SPEC.md` (already linked; no change needed)
