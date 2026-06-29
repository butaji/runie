# Wire or delete `InputActor`

**Status**: todo
**Milestone**: R6
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: migrate-production-actors-to-ractor
**Blocks**: migrate-tui-and-cli-to-leader-bootstrap

## Description

`InputActor` is spawned in the TUI bootstrap and its handle is stored in `ActorHandles`, but crossterm input events are published directly to the `EventBus`. No production code sends `InputMsg` to the actor. Either route input through `InputMsg` or delete the orphan actor.

## Acceptance Criteria

- [ ] Decide whether `InputActor` should own `InputState`.
- [ ] If kept: route crossterm events through `InputMsg` to the actor; remove direct `EventBus` publication.
- [ ] If deleted: move `InputState` ownership to `UiActor` and remove `InputActor` spawn.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 2 — Event Handling
- [ ] `input_actor_receives_key_event` — a crossterm key event reaches `InputActor` (if kept).
- [ ] `ui_actor_owns_input_state` — input state updates correctly (if deleted).

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `crates/runie-tui/src/main.rs`
- `crates/runie-tui/src/ui_actor.rs`
- `crates/runie-core/src/actors/input/actor.rs`
- `crates/runie-core/src/actors/input/messages.rs`

## Notes

- Prefer deleting the actor unless it provides clear value; fewer actors means less lifecycle complexity.
