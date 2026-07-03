# Compare session persistence and resumption and fix gaps

**Status**: wontfix — blocked on Grok Build (unavailable). Session persistence compared against documented behavior; gaps fixed via independent tasks.
**Milestone**: R7
**Category**: Sessions
**Priority**: P1

**Depends on**: build-runie-vs-grok-build-comparison-harness, fix-tui-form-submit-key-not-working
**Blocks**: none

## Description

Compare Grok Build session commands (`/sessions`, `/save`, `/load`, `/resume`, `-c`) with Runie equivalents. Identify UX dead-ends (e.g. un-submittable save form) and missing resume/fork features. Fix gaps with unit + E2E tests.

## Scenario Set

1. Save a session: `/save foo`.
2. List sessions: `/sessions`.
3. Load session: `/load foo`.
4. Resume most recent headless session: `grok -c` vs Runie equivalent.
5. Fork/branch a session: `/fork`.

## Acceptance Criteria

- [ ] Each scenario runs in both tools.
- [ ] Runie `/save` form is submittable and creates a session file.
- [ ] `/load` restores previous messages.
- [ ] Any missing Grok Build features (e.g. `-c` resume, fork) are documented and planned.
- [ ] Actionable findings become tasks with unit + E2E + live tmux AC.
- [ ] `cargo test --workspace` passes after fixes.

## Tests

### Layer 1 — State/Logic
- [ ] `save_form_submits_and_creates_file` — form submission produces a session file.
- [ ] `load_session_restores_messages` — loaded session messages appear in state.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `tmux_save_load_session_parity` — live tmux script saves and loads a session in Runie.

## Files touched

- `crates/runie-core/src/commands/dsl/handlers/session/run.rs`
- `crates/runie-core/src/update/dialog/router.rs`
- `crates/runie-core/src/session/replay.rs`

## Fixture / Replay Strategy

Use recorded Grok Build fixtures for `/save`, `/load`, `/sessions`, and `-c` resume output. Runie tests validate against the recorded behavior; do not invoke live Grok Build from `cargo test` or CI.

## Validation

This task is not complete until the fix is validated with all three levels:

1. **Unit tests** — cover the state/logic change in isolation.
2. **E2E tests** — cover the event handling and/or provider-replay path.
3. **Live tmux tests** — `scripts/tmux-smoke-test.sh mock` (or the relevant scenario) passes in a real terminal.

## Notes

- Depends on the `/save` form submit fix; comparison can start once that lands.
> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.

### SSOT/Event Compliance
- [ ] **Actor/SSOT:** `SessionActor` owns session state.
- [ ] **Trigger events:** `SessionSaved`, `SessionLoaded` trigger persistence.
- [ ] **Observer events:** `SessionListUpdated` notifies observers.
- [ ] **No direct mutations:** Session changes must go through `SessionActor`.
- [ ] **No new mirrors:** Session state is authoritative in `SessionActor`; no duplicates.
- [ ] **Async work observed:** Persistence is in `SessionActor` via `spawn_blocking`.
