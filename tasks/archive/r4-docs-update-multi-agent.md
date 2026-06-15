# Docs Update for Multi-Agent

**Status**: done
**Milestone**: R4
**Category**: Configuration
**Priority**: P1

**Depends on**: r4-adr-team-mode-orchestration
**Blocks**: (none)

## Description

Update all project-level docs to reflect the Solo/Team multi-agent design:
`docs/SPEC.md`, `docs/CONTEXT.md`, and `README.md`. Keep terminology
consistent and add pointers to ADR 0020.

## Acceptance Criteria

- [ ] `docs/SPEC.md` includes `OrchestratorActor` in runtime diagrams, lists Team
  mode commands (`/team`, `/solo`), and describes OHP at a high level.
- [ ] `docs/CONTEXT.md` glossary defines Solo, Team, Role, Model Trait,
  Orchestrator-Harness Protocol (OHP), and updates Orchestrator definition.
- [ ] `README.md` replaces any `/spawn`-style multi-agent references with
  Team-mode language and links to ADR 0020 / `docs/SPEC.md`.
- [ ] All internal markdown links validated (no broken anchors to moved files).

## Tests

No Rust tests. Verification:

```bash
# Check key terms appear in each doc
grep -qi "OrchestratorActor" docs/SPEC.md
grep -qi "Model Trait" docs/CONTEXT.md
grep -qi "Team" README.md
```

## Files touched

- `docs/SPEC.md`
- `docs/CONTEXT.md`
- `README.md`

## Out of scope

- Creating ADR (covered by `r4-adr-team-mode-orchestration`).
