# Compare plan mode and context workflows and fix gaps

**Status**: wontfix — blocked on Grok Build (unavailable). Compare tasks are deferred; Runie plan mode implemented independently via add-plan-file-artifact-and-plan-mode-rpc.
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

## Fixture / Replay Strategy

Use recorded Grok Build fixtures for `/plan`, `/compact`, `/context`, and `/rewind` outputs. Runie tests use these as the reference; do not invoke live Grok Build from `cargo test` or CI.

## Validation

This task is not complete until the fix is validated with all three levels:

1. **Unit tests** — cover the state/logic change in isolation.
2. **E2E tests** — cover the event handling and/or provider-replay path.
3. **Live tmux tests** — `scripts/tmux-smoke-test.sh mock` (or the relevant scenario) passes in a real terminal.

## Notes

- Plan mode may be out of scope for the current milestone; documenting the intentional difference is acceptable if decided.
> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.

### SSOT/Event Compliance
- [ ] **Actor/SSOT:** Depends on outcome: either `PlanActor` or existing actors remain authoritative.
- [ ] **Trigger events:** Plan mode events trigger write tool blocking.
- [ ] **Observer events:** Plan approval/rejection events notify observers.
- [ ] **No direct mutations:** Plan mode must emit events, not mutate state directly.
- [ ] **No new mirrors:** Plan state is authoritative in the owning actor; no duplicates.
- [ ] **Async work observed:** Plan approval is synchronous; no new async work.
