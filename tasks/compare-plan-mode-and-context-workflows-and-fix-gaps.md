# Compare plan mode and context workflows and fix gaps

**Status**: todo
**Milestone**: R7
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: build-runie-vs-grok-build-comparison-harness
**Blocks**: none

## Description

Grok Build has a `/plan` mode that blocks write tools until the user approves a structured plan. Runie has no equivalent. Compare plan mode, `/compact`, `/context`, and `/rewind` workflows. Decide whether Runie needs plan mode or can document the difference.

## Scenario Set

1. Grok `/plan` for a multi-step refactor.
2. Grok `/compact` to shrink context.
3. Grok `/context` to inspect token usage.
4. Grok `/rewind` to rollback.
5. Runie `/compact` and `/session_info`.

## Acceptance Criteria

- [ ] Each scenario runs in both tools (or Grok only if Runie lacks the feature).
- [ ] A decision record is added to the comparison report:
  - Implement plan mode in Runie, OR
  - Document that Runie uses a different trust/approval model.
- [ ] If implementing, create child tasks with unit + E2E + live tmux AC.
- [ ] `/compact` form submit works in Runie.
- [ ] `cargo test --workspace` passes after fixes.

## Tests

### Layer 1 — State/Logic
- [ ] `compact_form_submits` — `/compact` form produces a compaction intent.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `tmux_compact_works` — live tmux script runs `/compact` successfully.

## Files touched

- Determined by scope; possibly new plan-mode actor/state in `crates/runie-core/src/actors/`.

## Validation

This task is not complete until the fix is validated with all three levels:

1. **Unit tests** — cover the state/logic change in isolation.
2. **E2E tests** — cover the event handling and/or provider-replay path.
3. **Live tmux tests** — `scripts/tmux-smoke-test.sh mock` (or the relevant scenario) passes in a real terminal.

## Notes

- Plan mode may be out of scope for the current milestone; documenting the intentional difference is acceptable if decided.
