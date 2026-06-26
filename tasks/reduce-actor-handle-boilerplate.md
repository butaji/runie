# Reduce actor handle and message boilerplate

**Status**: done
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: generic-actor-reply
**Blocks**: none

## Description

Every actor repeats: `pub struct XxxActorHandle { tx: mpsc::Sender<XxxMsg> }`, `new`, and fire-and-forget `send`/`try_send` methods. A generic `GenericActorHandle<Msg>` eliminates this boilerplate for all fire-and-forget actors.

## Implementation

Added `GenericActorHandle<Msg>` in `trait.rs` — wraps `Arc<Sender<Msg>>` so it's always `Clone`:

```rust
/// Generic actor handle for sending typed messages.
#[derive(Clone, Debug)]
pub struct GenericActorHandle<Msg: Clone> {
    tx: std::sync::Arc<mpsc::Sender<Msg>>,
}

impl<Msg: Clone> GenericActorHandle<Msg> {
    pub fn new(tx: mpsc::Sender<Msg>) -> Self { ... }
    pub fn inner(&self) -> &mpsc::Sender<Msg> { &self.tx }
    pub async fn send(&self, msg: Msg) { ... }
    pub fn try_send(&self, msg: Msg) { ... }
}
```

Migrated handles:
- `InputActorHandle = GenericActorHandle<InputMsg>` (type alias, no own methods)
- `IoActorHandle = GenericActorHandle<IoMsg>` (type alias + `run_bash`, `write_files`, `detect_env`)
- `ViewActorHandle = GenericActorHandle<ViewMsg>` (type alias, no own methods)
- `SessionActorHandle` newtype wrapping `GenericActorHandle<SessionMsg>` via `Deref` (keeps typed methods: `set_trust`, `append_history`, `load`, `save`, etc.)
- `PersistenceActorHandle` newtype wrapping `GenericActorHandle<SessionMsg>` (keeps `set_trust`, `append_history`)
- `SessionStoreActorHandle` newtype wrapping `GenericActorHandle<SessionMsg>` (keeps `load`, `save`, `delete`, `import`, `export`, `list`)

Kept as specialized wrappers (typed request methods):
- `ConfigActorHandle` — typed async methods: `get_config`, `get_configured_providers`
- `ProviderActorHandle` — typed async methods: `build`, `validate_key`, `list_models`
- `PermissionActorHandle` — typed async methods: `ask_permission`, `resolve_permission`, etc.

## Acceptance Criteria

- [x] `GenericActorHandle<Msg>` defined in `trait.rs` with `send`, `try_send`, `inner` helpers.
- [x] `InputActorHandle`, `IoActorHandle`, `ViewActorHandle` reduced to type aliases + optional typed methods.
- [x] `SessionActorHandle` uses newtype + `Deref` to avoid duplicate impl blocks while exposing typed methods.
- [x] `PersistenceActorHandle`, `SessionStoreActorHandle` use newtype wrappers.
- [x] `ConfigActorHandle`, `ProviderActorHandle`, `PermissionActorHandle` kept as specialized wrappers.
- [x] `cargo check --workspace` succeeds with no new warnings.
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `generic_actor_handle_is_always_clone` — `GenericActorHandle<Msg>` is `Clone` for any `Msg`.
- [x] `generic_actor_handle_sends_and_receives` — `send`/`try_send` deliver messages to spawned actor.
- [x] `generic_actor_handle_impl_methods_work` — `inner()` gives access to raw sender.

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] `actor_handles_smoke_test` in `handles.rs` verifies `send_save_provider` reaches `ConfigActor`.

## Files touched

- `crates/runie-core/src/actors/trait.rs` — added `GenericActorHandle<Msg>`
- `crates/runie-core/src/actors/mod.rs` — re-export `GenericActorHandle`
- `crates/runie-core/src/actors/input/messages.rs` — `InputActorHandle = GenericActorHandle<InputMsg>`
- `crates/runie-core/src/actors/io/messages.rs` — `IoActorHandle = GenericActorHandle<IoMsg>` + typed helpers
- `crates/runie-core/src/actors/view/messages.rs` — `ViewActorHandle = GenericActorHandle<ViewMsg>`
- `crates/runie-core/src/actors/session/messages.rs` — `SessionActorHandle` newtype + `PersistenceActorHandle`/`SessionStoreActorHandle` newtypes
- `crates/runie-core/src/actors/handles.rs` — no structural change (uses all handles via their typed methods)

## Notes

- `FffIndexerHandle` kept as dedicated struct (has `search`/`try_search` that don't fit the generic pattern).
- `ConfigActorHandle`, `ProviderActorHandle`, `PermissionActorHandle` kept as specialized wrappers because they have typed request methods that need `Reply<T>`.
- `Deref` on `SessionActorHandle` exposes `GenericActorHandle<SessionMsg>` methods without duplicating `send`/`try_send` impl blocks.
