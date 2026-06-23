# TrustActor owns trust decisions

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Security
**Priority**: P1

**Depends on**: actor-owned-state-ssot, event-taxonomy-for-actor-state-sync, app-state-read-only-projection
**Blocks**: none

## Description

Trust decisions and the derived `config.read_only` flag are mutated by system helpers and startup code. `PersistenceActor` already persists trust and emits `TrustLoaded`/`TrustChanged`. Add a thin `TrustActor` that owns the in-memory trust state and the read-only flag side-effect.

Current violators:
- `model/state/app_state.rs` — `set_trust_decision`, default `trust_decisions`, preserve across reset.
- `update/dispatch.rs` — applies `TrustLoaded`/`TrustChanged` to `state.trust_decisions`.
- `update/system.rs` — `apply_trust_project`, `apply_untrust_project`, `apply_initial_trust` set `config.read_only` and add/remove welcome message.
- `commands/dsl/handlers/tool.rs` — `/trust` and `/untrust` emit `ModelConfigEvent::TrustProject`/`UntrustProject`.
- `update/agent/model_config.rs` — handles `ToggleReadOnly`.

## Acceptance criteria

- [ ] `TrustActor` is an mpsc actor owning `trust_decisions` and the derived read-only flag.
- [ ] `TrustMsg` covers: `SetTrust { path, decision }`, `LoadTrust { decisions }`.
- [ ] `AppState.trust_decisions` and the trust-derived portion of `config.read_only` are private to writes.
- [ ] `TrustActor` emits `Event::TrustChanged { path, decision }` and `Event::ReadOnlyChanged { enabled }`.
- [ ] `apply_trust_project`, `apply_untrust_project`, `apply_initial_trust` are removed from `update/system.rs`; their work is done by `TrustActor` reacting to trust facts.
- [ ] `/trust` and `/untrust` emit `TrustMsg::SetTrust`; the actor persists via `PersistenceActor::SetTrust` and updates state.
- [ ] `PersistenceActor` continues to own the file IO; it sends `TrustLoaded` to `TrustActor` on startup.
- [ ] `cargo test --workspace` passes.

## Tests

### Layer 1 — State/Logic
- [ ] `trust_actor_set_trust_updates_read_only` — `SetTrust { Trusted }` disables read-only.
- [ ] `trust_actor_untrust_updates_read_only` — `SetTrust { Untrusted }` enables read-only.

### Layer 2 — Event Handling
- [ ] `trust_command_emits_set_trust` — `/trust` sends `TrustMsg::SetTrust`.
- [ ] `trust_loaded_initializes_decisions` — startup `TrustLoaded` routes to `TrustActor`.

### Layer 3 — Rendering
- [ ] `read_only_changed_updates_status_bar` — `ReadOnlyChanged` fact renders the lock indicator.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `crates/runie-core/src/actors/trust/` — new `mod.rs`, `messages.rs`, `actor.rs`.
- `crates/runie-core/src/model/state/app_state.rs` — private `trust_decisions`; remove direct read-only writes.
- `crates/runie-core/src/update/system.rs` — remove `apply_trust_project`/`apply_untrust_project`/`apply_initial_trust`.
- `crates/runie-core/src/update/dispatch.rs` — route `TrustLoaded`/`TrustChanged` to `TrustActor`.
- `crates/runie-core/src/update/agent/model_config.rs` — `ToggleReadOnly` emits `TrustMsg::SetTrust` for current project.
- `crates/runie-core/src/commands/dsl/handlers/tool.rs` — `/trust`/`/untrust` emit `TrustMsg`.
- `crates/runie-core/src/actors/persistence/actor.rs` — `SetTrust` message persists and emits `TrustChanged`.

## Notes

- Keep `PersistenceActor` as the SSOT for persisted trust; `TrustActor` is the in-memory projection + derived-flag owner.
- The welcome message side-effect moves to `SessionActor` triggered by `ReadOnlyChanged` if needed.
