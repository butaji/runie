# Clean Up Stale and Contradictory Documentation

**Status**: done
**Milestone**: R3
**Category**: Configuration
**Priority**: P1

## Description

Several planning documents are out of sync with the code and with each other:

- `FEATURE_PARITY.md` marks 31 features as “planned” with links to non-existent task
  files, while `README.md` lists the same features as implemented.
- `IMPL_PLAN.md` and `REFACTOR_PLAN.md` describe stale lint/state numbers.
- `docs/adr/README.md` links to `0017-actor-runtime.md` but the file is named
  `0017-actor-runtime-and-event-bus.md`.

## Acceptance Criteria

- [x] `IMPL_PLAN.md` and `REFACTOR_PLAN.md` are archived under `docs/archive/`.
- [x] `FEATURE_PARITY.md` is updated so every feature status matches `README.md`
  (the current source of truth); broken task links are removed or corrected.
- [x] `docs/adr/README.md` link and table entry for ADR 0017 are fixed.
- [x] No internal markdown link points to a missing file (verified with a Python
  link checker).
- [x] `docs/SPEC.md` code-organization map is updated to reflect which modules are
  present, orphaned, or missing.

## Tests

No Rust tests. Verification:

```bash
grep -R "r2-\|r3-" FEATURE_PARITY.md | grep -v '^#' | wc -l   # broken links should be 0
grep -R '\[.*\]\(.*\.md\)' docs/ | while read l; do test -f "$(echo "$l" | grep -oE '\(.*\.md\)' | tr -d '()')"; done
grep -R '\[.*\]\(tasks/.*\.md\)' FEATURE_PARITY.md | while read l; do test -f "$(echo "$l" | grep -oE 'tasks/.*\.md' )"; done
```

## Files touched

- `IMPL_PLAN.md` (archive/delete)
- `REFACTOR_PLAN.md` (archive/delete)
- `FEATURE_PARITY.md`
- `docs/adr/README.md`
- `docs/SPEC.md`

## Out of scope

- Rewriting user-facing `README.md` content beyond correcting contradictions.
- Updating ADR content; only links/tables are fixed here.
