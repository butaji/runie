# Collapse remaining actor handle wrappers to ractor::ActorRef

## Status

`todo`

## Context

Per-actor handle wrappers (`RactorConfigHandle`, `RactorProviderHandle`, `RactorPermissionHandle`, `RactorIoHandle`, `RactorSessionHandle`, `RactorTurnHandle`, `RactorFffIndexerHandle`) wrap `RactorHandle`, which wraps `ractor::ActorRef`. This is triple indirection with duplicated `send`/`try_send`/RPC boilerplate.

## Goal

Store `ractor::ActorRef<Msg>` directly in `LeaderHandle`; use `ractor::cast!`/`call!`/`call_t!` macros; delete `RactorHandle`, `spawn_ractor`, and the per-actor wrapper structs.

## Acceptance Criteria

- [ ] Delete `RactorHandle` and per-actor wrapper structs.
- [ ] Update `LeaderHandle` to hold `ActorRef<Msg>` map.
- [ ] Use ractor macros for cast/call.
- [ ] All actor tests pass.

## Design Impact

No change to TUI element design or composition. Only internal actor handle API changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** Actor messages produce the same facts.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Provider replay fixture passes.
- **Live tmux validation:** TUI launch and normal actor flow work.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
