# Write Runie vs Grok Build findings report

**Status**: todo
**Milestone**: R7
**Category**: Documentation
**Priority**: P1

**Depends on**: compare-headless-one-shot-scenarios-and-fix-gaps, compare-interactive-tui-launch-scenarios-and-fix-gaps, compare-tool-execution-permission-flows-and-fix-gaps, compare-session-persistence-resumption-and-fix-gaps, compare-plan-mode-and-context-workflows-and-fix-gaps, compare-file-context-picker-and-fix-gaps, compare-model-provider-switching-and-fix-gaps, compare-diff-edit-output-and-fix-gaps, compare-subagent-mcp-support-and-fix-gaps, compare-auth-config-setup-and-fix-gaps, compare-quit-abort-error-recovery-and-fix-gaps, compare-multi-turn-conversation-and-fix-gaps
**Blocks**: none

## Description

After the comparison scenarios run, produce a consolidated markdown report at `docs/superpowers/plans/2026-06-30-runie-vs-grok-build-findings.md`. The report must list every discrepancy with classification, reproduction steps, and the linked `tasks/` item. The report is based on recorded Grok Build fixtures, not live invocations.

## Acceptance Criteria

- [ ] Report contains a side-by-side table of all scenarios.
- [ ] Each finding has classification: missing feature, dead-end, confusing UX, bug, or reference-only.
- [ ] Each actionable finding links to a `tasks/` entry.
- [ ] Blockers (e.g. Grok Build auth failure) are documented with impact.
- [ ] The report is committed with the comparison tasks.

## Tests

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A — documentation task.

## Files touched

- `docs/superpowers/plans/2026-06-30-runie-vs-grok-build-findings.md`

## Validation

This task is not complete until the fix is validated with all three levels:

1. **Unit tests** — cover the state/logic change in isolation.
2. **E2E tests** — cover the event handling and/or provider-replay path.
3. **Live tmux tests** — `scripts/tmux-smoke-test.sh mock` (or the relevant scenario) passes in a real terminal.

## Notes

- The report is updated iteratively as scenario tasks complete.
