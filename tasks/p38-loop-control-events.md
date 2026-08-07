# p38 — Move loop control state behind events

Status: in progress

## Evidence

`crates/runie-core/src/loop/actor.rs` currently keeps `running`,
`steering_mode`, `follow_up_mode`, and the active run handle in private
`tokio::sync::Mutex` fields. Queue contents are actor-owned, but the public
mode setters still write those control fields directly.

`crates/runie-tui/src/event_renderer.rs` remains a compatibility adapter and
mutates legacy status/scrollback widget state while consuming events. The
actor-owned model path is already event-driven; this is the remaining
stateful compatibility boundary.

## First slice (2026-08-06)

Steering and follow-up queue modes now travel through actor-owned Tokio
`watch` channels. Public setters send a mode update and readers consume the
latest immutable projection; the loop snapshots the mode at run start. This
removes the direct mutex writes without changing Pi's event wire contract.

## Remaining design

Add a private `LoopCommand` mailbox and a loop-control reducer owned by
`LoopActor`. Public control methods send commands and await acknowledgements;
the reducer emits typed control events/snapshots for mode changes, busy state,
abort, and run completion. Keep Pi's closed `AgentEvent` wire contract
unchanged: these are Runie application events and must not be smuggled into
`PiAgentEvent`.

Replace compatibility widget mutation with a pure event-to-view projection
fed by the actor snapshot. The renderer should only consume immutable
snapshots and terminal frame inputs.

## Verification

- event sequences prove mode changes, busy rejection, abort, and completion
  ordering;
- YAML replay can express loop-control commands without recompiling;
- no test uses `sleep()`;
- `lint-check`, unit/replay tests, and the full `just ci` gate remain green.
