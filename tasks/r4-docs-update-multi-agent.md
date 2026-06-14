# Docs Update for Multi-Agent

**Status**: todo
**Milestone**: R4
**Category**: Documentation
**Priority**: P1

**Depends on**: r4-adr-team-mode-orchestration
**Blocks**: (none)

## Description

Update all project-level docs to reflect the Solo/Team multi-agent design:
`docs/SPEC.md`, `docs/FEATURE_PARITY.md`, `docs/ARCHITECTURE_ROUND2.md`,
`docs/CONTEXT.md`, and `README.md`. Keep terminology consistent and add
pointers to `docs/MULTI.md` and the new ADR.

## Acceptance Criteria

- [ ] `docs/SPEC.md` includes `OrchestratorActor` in runtime diagrams, lists Team
  mode commands (`/team`, `/solo`), and describes OHP at a high level.
- [ ] `FEATURE_PARITY.md` has an "Agents & Orchestration" section with
  Solo/Team, Q&A alignment, one-shot planning, subagent sidebar, and trait
  routing.
- [ ] `docs/ARCHITECTURE_ROUND2.md` references `docs/MULTI.md`.
- [ ] `docs/CONTEXT.md` glossary defines Solo, Team, Role, Model Trait,
  Orchestrator-Harness Protocol (OHP), and updates Orchestrator definition.
- [ ] `README.md` replaces any `/spawn`-style multi-agent references with
  Team-mode language and links to `docs/MULTI.md`.
- [ ] All internal markdown links validated (no broken anchors to moved files).

## Tests

No Rust tests. Verification:

```bash
# Check key terms appear in each doc
grep -qi "OrchestratorActor" docs/SPEC.md
grep -qi "Solo" FEATURE_PARITY.md
grep -qi "Team" FEATURE_PARITY.md
grep -qi "docs/MULTI.md" docs/ARCHITECTURE_ROUND2.md
grep -qi "Model Trait" docs/CONTEXT.md
grep -qi "Team" README.md
```

## Files touched

- `docs/SPEC.md`
- `FEATURE_PARITY.md`
- `docs/ARCHITECTURE_ROUND2.md`
- `docs/CONTEXT.md`
- `README.md`

## Out of scope

- Writing `docs/MULTI.md` (already exists).
- Creating ADR (covered by `r4-adr-team-mode-orchestration`).
