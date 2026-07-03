# Compare multi-turn conversation and fix gaps

**Status**: wontfix
**Blocked reason**: Grok Build fixtures not present; comparison harness not yet built.

> **Blocked by**: `build-runie-vs-grok-build-comparison-harness` (todo), `prepare-grok-build-reference-for-comparison` (todo)
**Milestone**: R7
**Category**: TUI / Rendering
**Priority**: P1

**Depends on**: build-runie-vs-grok-build-comparison-harness, fix-tui-multi-turn-follow-up-stuck-behind-active-turn
**Blocks**: none

## Description

Compare multi-turn conversation behavior in Grok Build and Runie: follow-up messages, history continuity, steering/follow-up hints, and queue handling. Fix Runie gaps with unit + E2E tests.

## Scenario Set

1. Ask "list files", then "show me the Cargo.toml".
2. Ask "hello", then "say it again".
3. Use `Enter` for steering vs `Alt+Enter` for follow-up.
4. Observe history after multiple turns.

## Acceptance Criteria

- [ ] Each scenario runs in both tools.
- [ ] Runie processes follow-up messages after the first turn completes.
- [ ] History contains all user and assistant messages.
- [ ] Queue counter behaves correctly (increments while waiting, decrements when running).
- [ ] Actionable findings become tasks with unit + E2E + live tmux AC.
- [ ] `cargo test --workspace` passes after fixes.

## Tests

### Layer 1 — State/Logic
- [ ] `follow_up_starts_new_turn` — after `Done`, a queued user message triggers a new turn.

### Layer 3 — Rendering
- [ ] `multi_turn_renders_two_responses` — `TestBackend` shows two distinct assistant outputs.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `tmux_list_files_then_show_cargotoml` — live tmux script runs both prompts and sees two responses.

## Files touched

- `crates/runie-core/src/actors/turn/ractor_turn.rs`
- `crates/runie-tui/src/ui_actor.rs`
- `crates/runie-core/src/model/app_state.rs`

## Fixture / Replay Strategy

Use recorded Grok Build headless and TUI fixtures for multi-turn conversations. Convert Grok's event stream into Runie provider-replay fixtures and expected `TestBackend` buffers. Do not invoke live Grok Build from `cargo test` or CI.

## Validation

This task is not complete until the fix is validated with all three levels:

1. **Unit tests** — cover the state/logic change in isolation.
2. **E2E tests** — cover the event handling and/or provider-replay path.
3. **Live tmux tests** — `scripts/tmux-smoke-test.sh mock` (or the relevant scenario) passes in a real terminal.

## Notes

- Overlaps with `fix-tui-multi-turn-follow-up-stuck-behind-active-turn`.
> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.

### SSOT/Event Compliance
- [ ] **Actor/SSOT:** `TurnActor` owns turn state and queue.
- [ ] **Trigger events:** `FollowUpDelivered`, `TurnCompleted` trigger next turn.
- [ ] **Observer events:** `TurnStarted`, `TurnCompleted` notify observers.
- [ ] **No direct mutations:** Turn queue changes must go through `TurnActor`.
- [ ] **No new mirrors:** Turn state is authoritative in `TurnActor`; no duplicates.
- [ ] **Async work observed:** Turn processing has JoinHandle owners.
