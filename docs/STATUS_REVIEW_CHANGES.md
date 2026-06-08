# Status Review — Changes Applied

**Date:** 2026-06-08

---

## Documents Created

| File | Purpose |
|------|---------|
| `docs/STATUS_REVIEW.md` | Comprehensive review of tasks, code, and documentation gaps |
| `tasks/r1-actor-infrastructure.md` | Consolidated task replacing 4 individual actor stubs |
| `tasks/r1-input-history-persistence.md` | Deferred work from mvp-input-history |
| `tasks/mvp-input-multiline-cursor.md` | Deferred work from mvp-input-multiline |

---

## Tasks Updated

### Status Changes

| Task | Before | After | Reason |
|------|--------|-------|--------|
| `mvp-input-history` | in-progress | **done** | Up/Down navigation works; persistence deferred |
| `mvp-input-multiline` | in-progress | **done** | Core newline insertion works; cursor polish deferred |
| `mvp-config-hot-reload` | MVP/todo | **R1**/todo | Not MVP-critical; infrastructure belongs in R1 |

### Content Improvements

| Task | Change |
|------|--------|
| `r1-core-refactor` | Marked 3/5 ACs as done (update.rs split, clippy, O(1) cache). Documented remaining AppState composition work and VisibleRegion blocker. |
| `mvp-tui-syntax` | Added **Lint Note** section documenting 582-line file exceeding 500-line limit. |
| `mvp-input-history` | Backfilled actual test function names (history_prev_moves_up, history_next_moves_down, etc.). |
| `mvp-input-multiline` | Backfilled actual test function names. Clarified deferred work. |

### Tasks Removed (Consolidated)

| Removed Task | Merged Into |
|--------------|-------------|
| `r1-actor-tool-actors` | `r1-actor-infrastructure` |
| `r1-actor-queue-agent` | `r1-actor-infrastructure` |
| `r1-actor-session-manager` | `r1-actor-infrastructure` |
| `r1-actor-config-agent` | `r1-actor-infrastructure` |

---

## docs/SPEC.md Updated

R1 checklist corrected to reflect completed work:
- `[x] Split update.rs` — was `[ ]`
- `[x] Fix clippy warnings` — was `[ ]`
- `[x] Cache optimizations` — was `[ ]`
- AppState field count updated: 28 → 27
- VisibleRegion note updated: "still referenced by autoscroll tests"

---

## Task Index Updated

`tasks/index.json`: 46 entries (was 48)

| Category | Count |
|----------|-------|
| MVP done | 34 |
| MVP todo | 1 (input-multiline-cursor) |
| R1 in-progress | 1 (core-refactor) |
| R1 todo | 9 |
| R2 todo | 1 (queue-delivery-mode) |

---

## Remaining Work Noted in STATUS_REVIEW.md

1. **`syntax.rs` lint violation** — 582 lines, only file over 500-line limit. Needs decision: split keyword tables or exempt data modules.

2. **AppState composition** — 27 fields remain. Blocked by 75+ tests accessing fields directly. Needs dedicated focused effort.

3. **28 done MVP tasks still have placeholder Tests sections** — They all reference the generic TEMPLATE.md text instead of actual test function names. Only 3 tasks (session-persistence, tui-diff, tui-syntax) have specific test references.

4. **R1 prioritization** — `r1-config-keybindings` has highest user value but depends on `r1-actor-infrastructure` (ConfigAgent).
