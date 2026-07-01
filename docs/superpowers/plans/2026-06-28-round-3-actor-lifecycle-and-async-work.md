# Round 3 — Actor Lifecycle & Async Work Ownership

## Findings

### 1. Orphan spawned tasks

Spawned tasks are not stored or awaited:

- `crates/runie-core/src/actors/provider/ractor_provider.rs:264-275` — `ValidateKey`/`ListModels` spawn tasks and discard handles.
- `crates/runie-core/src/actors/io/ractor_io.rs:157-228` — blocking IO tasks (`run_bash`, `write_files`, `detect_env`, `share_session`, etc.) are spawned and handles dropped.
- `crates/runie-core/src/actors/session/session_handlers.rs:230` — `handle_set_trust` spawns blocking trust-save and ignores the handle.
- `crates/runie-tui/src/ui_actor.rs:533,538` — effect tasks (`login`, generic effects) are spawned without tracking.

### 2. Timeouts / sleeps in actors

`crates/runie-tui/src/ui_actor.rs:651-678` uses a 100 ms bus timeout while waiting for `FollowUpDelivered`/`SteeringDelivered`. This is a polling/sleep-like anti-pattern.

### 3. No fire-and-forget rule is violated

The SSOT ADR requires every spawned task to have an owner. Several actors violate this.

## Recommended changes

1. Track every `tokio::spawn` and `tokio::task::spawn_blocking` in a `JoinSet` or as a stored `JoinHandle`.
2. Replace the 100 ms bus poll in `UiActor::clear_turn_state` with an explicit completion event or a oneshot/future from `TurnActor`.
3. Add an actor-state invariant: any spawned task must be cancellable/awaitable before the actor stops.
4. For `RactorProviderActor`, return replies only when the spawned task completes; store handles in actor state.
5. For `RactorIoActor`, store blocking-task handles and emit completion events; consider an IO worker pool if throughput matters.

## Task mapping

| Finding | Task file | Status |
|---------|-----------|--------|
| Track orphan spawned tasks in provider/IO/session actors | `tasks/track-orphan-spawned-tasks-in-provider-io-session-actors.md` | **new** |
| Remove timeout polling from `UiActor`/actors | `tasks/remove-timeout-polling-from-uiactor-and-actors.md` | **new** |
| Enforce observed async work in all actors | `tasks/enforce-observed-async-work-in-all-actors.md` | **new** |
| In-flight agent turn join handle | `tasks/track-and-cancel-inflight-agent-turn-joinhandle.md` | existing `done` |
| DeliverQueued race in `UiActor` | `tasks/fix-deliverqueued-race-in-uiactor.md` | existing `todo` |
| Make TUI render loop async with watch channel | `tasks/make-tui-render-loop-async-with-watch-channel.md` | existing `todo` |
| Move blocking IO out of command/update handlers | `tasks/move-blocking-io-out-of-command-handlers.md` | existing `todo` |
| Await provider network calls in ractor handler | `tasks/await-provider-network-calls-in-ractor-handler.md` | existing `todo` |
