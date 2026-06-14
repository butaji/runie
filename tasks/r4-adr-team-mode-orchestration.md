# ADR: Team Mode Orchestration

**Status**: todo
**Milestone**: R4
**Category**: Core Architecture
**Priority**: P0

**Depends on**: (none)
**Blocks**: r4-orchestrator-domain-types

## Description

Write `docs/adr/0020-team-mode-orchestration.md` capturing the Solo/Team
execution-mode decision, the Orchestrator-Harness Protocol, model trait routing,
and isolated subagent context design. This ADR is the anchor document for all
R4 multi-agent work.

## Acceptance Criteria

- [ ] `docs/adr/0020-team-mode-orchestration.md` exists and follows the ADR format.
- [ ] ADR explains why Supervisor + isolated subagents was chosen over
  peer-to-peer handoffs, role-based crews, and graph workflows.
- [ ] ADR references `docs/MULTI.md` for the full conceptual vision.
- [ ] `docs/adr/README.md` table includes the new ADR.
- [ ] `cargo build --workspace` succeeds (no code changes).

## Tests

No Rust tests. Verification:

```bash
test -f docs/adr/0020-team-mode-orchestration.md
grep -q "0020-team-mode-orchestration" docs/adr/README.md
```

## Files touched

- `docs/adr/0020-team-mode-orchestration.md`
- `docs/adr/README.md`

## Out of scope

- Implementation of the Orchestrator or subagents.
- Updating other docs (covered by `r4-docs-update-multi-agent`).
