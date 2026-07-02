# Split large core files into focused modules

## Status

`todo`

## Description

The following production files exceed the advertised 500-line limit and mix responsibilities:

- `crates/runie-core/src/proto/message/mod.rs` — 769 lines
- `crates/runie-core/src/actors/fff_indexer/mod.rs` — 669 lines
- `crates/runie-core/src/provider/provider_trait.rs` — 650 lines
- `crates/runie-core/src/event/durable.rs` — 648 lines
- `crates/runie-core/src/actors/permission/ractor_permission.rs` — 601 lines
- `crates/runie-core/src/actors/io/ractor_io.rs` — 587 lines
- `crates/runie-core/src/model/state/domain_ops.rs` — 410 lines

(`session/sqlite_store.rs` is excluded because SQLite is deferred; see `standardize-session-persistence-on-jsonl.md`.)

## Acceptance criteria

1. **Unit tests** — All split module unit tests pass.
2. **E2E tests** — Event dispatch through split modules passes.
3. **Live run tests** — Smoke-test the affected features in tmux (messages, FFF search, permissions, IO).

## Tests

### Unit tests
- Unit tests for split modules pass.

### E2E tests
- Event dispatch through split modules passes.

### Live run tests
- Start tmux and exercise message display, file search, permission prompts, and bash tools.

### SSOT/Event Compliance
- [ ] **Actor/SSOT:** N/A (refactoring; actors and state ownership unchanged).
- [ ] **Trigger events:** N/A (refactoring; no new state transitions).
- [ ] **Observer events:** N/A (refactoring; no new observers).
- [ ] **No direct mutations:** Split modules must not introduce direct state mutations; all changes go through existing actors.
- [ ] **No new mirrors:** Each split module must not create authoritative copies of actor-owned state.
- [ ] **Async work observed:** Any new async work spawned during split must be awaited or have a JoinHandle owner.
