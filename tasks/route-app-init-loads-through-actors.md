# Route app init loads through actors

**Status**: todo
**Milestone**: R7
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: remove-direct-appstate-mutation-from-tui-handlers
**Blocks**: subscribe-tui-to-initial-facts-before-leader-start

## Description

`app_init::bootstrap` loads skills and auth storage via `spawn_blocking` and assigns them directly to `AppState`, bypassing actors and facts. Load failures are silently swallowed with `unwrap_or_default()`.

## Root Cause

`crates/runie-tui/src/app_init.rs` was not fully migrated to the actor-driven initialization pattern.

## Acceptance Criteria

- [ ] Skill loading is routed through `IoActor` or a dedicated loader actor and applied via a fact.
- [ ] Auth provider loading is routed through `ConfigActor` / `SessionActor` and applied via a fact.
- [ ] Load failures surface as notifications or log warnings, not silently default.
- [ ] `cargo test --workspace` passes.
- [ ] Live tmux startup still loads skills and auth correctly.

## Tests

### Layer 1 — State/Logic
- [ ] `bootstrap_emits_load_intents` — `app_init::bootstrap` sends actor messages instead of mutating state.

### Layer 2 — Event Handling
- [ ] `skills_loaded_fact_applied` — `Event::SkillsLoaded` updates `AppState.skills`.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `tmux_launch_loads_skills` — live tmux script starts and asserts no skill-load panic.

## Files touched

- `crates/runie-tui/src/app_init.rs`
- `crates/runie-core/src/actors/io/ractor_io.rs`
- `crates/runie-core/src/actors/config/ractor_config.rs`
- `crates/runie-core/src/event.rs`

## Validation

This task is not complete until the fix is validated with all three levels:

1. **Unit tests** — cover the state/logic change in isolation.
2. **E2E tests** — cover the event handling and/or provider-replay path.
3. **Live tmux tests** — `scripts/tmux-smoke-test.sh mock` (or the relevant scenario) passes in a real terminal.

## Notes

- This is part of the broader direct-mutation cleanup.
