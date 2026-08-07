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

Prompt admission now uses one actor-owned semaphore permit spanning the whole
run. Busy rejection is therefore an ownership failure, and permit release is
scope-based even when the run returns an error.

## Control snapshot slice (2026-08-06)

`LoopActor` now reduces typed `LoopControlEvent` values into one immutable
`LoopControlSnapshot` containing run state, abort intent, and both queue modes.
Mode changes, run start/finish, and abort are represented by the reducer rather
than by unrelated state projections. `control_snapshot()` is read-only and
does not alter Pi's closed `AgentEvent` wire contract. The reducer has an
event-sequence unit test covering all transitions.

The runtime YAML state oracle now exposes `loop_running` and
`abort_requested`; `follow-up.yaml` asserts the settled control snapshot,
keeping the new projection in the no-recompile event-sequence test path.

**Single control SSOT (2026-08-06):** Removed the duplicate steering and
follow-up mode watch channels. Setters emit `LoopControlEvent` values, and the
loop, queue drainers, public readers, and YAML outcome all read the unified
`LoopControlSnapshot` projection.

**Reducer-owned snapshot publication (2026-08-07):** The control mailbox now
keeps the mutable `LoopControlSnapshot` inside its worker, applies each
`LoopControlEvent` there, and publishes a cloned immutable snapshot through
`watch::Sender::send`. No public method or sibling actor mutates the control
projection, and the reducer no longer uses `send_modify` as a second mutation
surface. This preserves the event → actor → snapshot sequence required by the
SSOT rule.

## Control mailbox closure (2026-08-06)

The unified control snapshot is now reduced by an owned
`LoopControlCommand` mailbox rather than `watch::send_modify` from public
methods. Run start, finish, abort, and queue-mode changes all enter the same
acknowledged reducer; the abort signal is emitted by that reducer, so
`abort().await` returns only after cancellation intent has crossed the actor
boundary. Existing loop and replay tests were updated to await this delivery
and remain sleep-free.

## Remaining design

The loop-control reducer itself is now mailbox-owned and acknowledged; it
emits typed control snapshots without changing Pi's closed `AgentEvent` wire
contract. The remaining boundary here is migrating the compatibility widget
adapter to pure event-to-view projection fed by actor snapshots.

## Production renderer boundary (2026-08-06)

`EventRenderer` now separates its legacy compatibility adapter from the live
path. Live and replay actor paths use `apply_actor_metadata` only for ephemeral
event sequencing metadata; status/feed changes are delivered as acknowledged
actor messages and rendered from snapshots. The old `apply_event` method still
exists for focused legacy-widget tests, but production actor projections no
longer call it or mutate compatibility widgets. This keeps the migration
incremental without creating a second production state owner.

The remaining work is retiring the legacy adapter and moving any focused tests
that still require it onto actor-backed replay fixtures.

## Verification

- event sequences prove mode changes, busy rejection, abort, and completion
  ordering;
- YAML replay can express loop-control commands without recompiling;
- no test uses `sleep()`;
- `lint-check`, unit/replay tests, and the full `just ci` gate remain green.
