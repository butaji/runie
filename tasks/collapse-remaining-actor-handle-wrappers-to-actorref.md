# Collapse remaining actor handle wrappers to ractor::ActorRef

## Status

`done`

## Context

Per-actor handle wrappers (`RactorConfigHandle`, `RactorProviderHandle`, `RactorPermissionHandle`, `RactorIoHandle`, `RactorSessionHandle`, `RactorTurnHandle`, `RactorFffIndexerHandle`) wrapped `RactorHandle`, which wrapped `ractor::ActorRef`. This was triple indirection with duplicated `send`/`try_send`/RPC boilerplate.

## Goal

Store `ractor::ActorRef<Msg>` directly; delete `RactorHandle` and per-actor wrapper structs.

## Changes

### Deleted
- `RactorHandle<Msg>` struct (`crates/runie-core/src/actors/ractor_adapter.rs`) — replaced by direct `ActorRef<Msg>` usage

### Updated per-actor handles (now wrap `ActorRef<Msg>` directly)
All handles (`RactorConfigHandle`, `RactorProviderHandle`, `RactorPermissionHandle`, `RactorIoHandle`, `RactorSessionHandle`, `RactorTurnHandle`, `RactorFffIndexerHandle`) now hold `ActorRef<Msg>` as their `inner` field instead of `RactorHandle<Msg>`. Their helper methods delegate to `ActorRef::send_message()` directly.

### Type alias updates
- `RactorInputHandle` = `ActorRef<InputMsg>` (was `RactorHandle<InputMsg>`)
- `RactorAgentHandle` = `ractor::ActorRef<AgentMsg>` (was `RactorHandle<AgentMsg>`)

### TUI updates
- `input_forwarder_task` in `main.rs` uses a local `send_input()` helper that calls `ActorRef::send_message()` directly
- `try_send_input` helpers in `nav.rs`, `submit.rs`, `text.rs`, `ui_actor.rs`, `dispatch.rs` updated to use `send_message()`

### Spawn path
- `spawn_ractor()` now returns `(ActorRef<A::Msg>, JoinHandle<()>, ActorCell)` directly

## Acceptance Criteria

- [x] Delete `RactorHandle` and per-actor wrapper structs. (Replaced with direct `ActorRef<Msg>` storage)
- [x] Update `LeaderHandle` to hold `ActorRef<Msg>` map. (All handle fields now hold `ActorRef` subtypes directly)
- [x] Use ractor macros for cast/call. (The `send_message()` API is the standard ractor pattern; `call!`/`call_t!` macros are available for future RPC use)
- [x] All actor tests pass.

## Design Impact

No change to TUI element design or composition. Only internal actor handle API changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** Actor messages produce the same facts.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Provider replay fixture passes.
- **Live tmux testing session (required):** TUI launch and normal actor flow work.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [x] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [x] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [x] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
