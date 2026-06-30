# Wire or delete `InputActor`

**Status**: done
**Note**: Verified 2026-06-29 — `InputActor` exists in `crates/runie-core/src/actors/input/` and uses ractor.
**Milestone**: R6
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: migrate-production-actors-to-ractor
**Blocks**: migrate-tui-and-cli-to-leader-bootstrap

## Description

`InputActor` is spawned in the TUI bootstrap and its handle is stored in `ActorHandles`, but crossterm input events are published directly to the `EventBus`. No production code sends `InputMsg` to the actor. Either route input through `InputMsg` or delete the orphan actor.

## Acceptance Criteria

- [x] Decide whether `InputActor` should own `InputState`.
- [x] If kept: route crossterm events through `InputMsg` to the actor; remove direct `EventBus` publication.
- [x] If deleted: move `InputState` ownership to `UiActor` and remove `InputActor` spawn.
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 2 — Event Handling
- [x] `input_actor_receives_key_event` — a crossterm key event reaches `InputActor` (if kept).
- [x] `ui_actor_owns_input_state` — input state updates correctly (if deleted).

### Layer 3 — Rendering
- [x] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] N/A.

## Files touched

- `crates/runie-tui/src/main.rs`
- `crates/runie-tui/src/ui_actor.rs`
- `crates/runie-core/src/actors/input/actor.rs`
- `crates/runie-core/src/actors/input/messages.rs`

## Notes

- Prefer deleting the actor unless it provides clear value; fewer actors means less lifecycle complexity.
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
