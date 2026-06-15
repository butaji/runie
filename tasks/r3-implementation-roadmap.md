# R3 Implementation Roadmap

**Status**: todo
**Milestone**: R3
**Category**: Core / State
**Priority**: P0

**Depends on**: (none)
**Blocks**: (none)

## Description

Create and maintain a sequenced implementation roadmap for R3/R4 work. The roadmap groups active tasks into dependency-ordered phases so the team can ship incrementally without blocking unrelated work.

## Acceptance Criteria

- [ ] `docs/ROADMAP.md` exists and lists all active R3/R4 tasks in phase order.
- [ ] Each phase has a clear goal and a checklist of task IDs.
- [ ] Dependencies between phases are accurate (e.g., actor wiring before FFF indexer, FFF before skills, etc.).
- [ ] `docs/SPEC.md` links to `docs/ROADMAP.md`.
- [ ] The roadmap is updated when tasks are added, archived, or reprioritized.

## Tests

### Layer 1 — State/Logic
- [ ] `roadmap_covers_all_active_r3_tasks` — every active R3 task appears in at least one phase.
- [ ] `roadmap_dependencies_acyclic` — no circular dependencies between phases.

### Layer 2 — Event Handling
N/A.

### Layer 3 — Rendering
N/A.

### Layer 4 — Smoke / Crash
N/A.

## Files touched

- `docs/ROADMAP.md`
- `docs/SPEC.md`

## Notes

- This is a meta planning task. It is done when the roadmap is accepted and linked.
- Active task statuses remain authoritative in `tasks/index.json`.
