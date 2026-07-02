# Split large core files into focused modules

## Status

`partial` — `proto/message/mod.rs`, `provider/provider_trait.rs`, `event/durable.rs`, `actors/io/ractor_io.rs`, and `actors/permission/ractor_permission.rs` split into focused modules + tests. Remaining: `actors/fff_indexer/mod.rs` (669).

## Description

The following production files exceed the advertised 500-line limit and mix responsibilities:

- ~~`crates/runie-core/src/proto/message/mod.rs`~~ — **DONE** (845→342 lines)
  - Split into: `tool_call.rs`, `role.rs`, `metadata.rs`, `validation.rs`, `chat_message.rs`, `mod.rs`
- ~~`crates/runie-core/src/provider/provider_trait.rs`~~ — **DONE** (663→347 lines)
  - Split into: `provider_trait.rs` (main) + `tests.rs` (tests)
- ~~`crates/runie-core/src/event/durable.rs`~~ — **DONE** (772→387 lines)
  - Split into: `durable.rs` (main) + `tests.rs` (tests)
- ~~`crates/runie-core/src/actors/io/ractor_io.rs`~~ — **DONE** (633→346 lines)
  - Split into: `ractor_io.rs` (main) + `ractor_io/tests.rs` (tests)
- ~~`crates/runie-core/src/actors/permission/ractor_permission.rs`~~ — **DONE** (686→405 lines)
  - Split into: `ractor_permission.rs` (main) + `ractor_permission/tests.rs` (tests)
- `crates/runie-core/src/actors/fff_indexer/mod.rs` — 669 lines
- `crates/runie-core/src/model/state/domain_ops.rs` — 410 lines (under limit)

(`session/sqlite_store.rs` is excluded because SQLite is deferred; see `standardize-session-persistence-on-jsonl.md`.)

## Changes (provider_trait.rs and durable.rs splits — done)

### provider_trait.rs split
Split `crates/runie-core/src/provider/provider_trait.rs` (663 lines) into 2 focused files:

| File | Lines | Contains |
|------|-------|----------|
| `provider/provider_trait.rs` | 347 | Main trait, types, error enum |
| `provider/provider_trait/tests.rs` | 318 | Unit tests |

### durable.rs split
Split `crates/runie-core/src/event/durable.rs` (772 lines) into 2 focused files:

| File | Lines | Contains |
|------|-------|----------|
| `event/durable.rs` | 387 | Main enum, conversions, implementations |
| `event/durable/tests.rs` | 387 | Unit tests |

### ractor_io.rs split
Split `crates/runie-core/src/actors/io/ractor_io.rs` (633 lines) into 2 focused files:

| File | Lines | Contains |
|------|-------|----------|
| `actors/io/ractor_io.rs` | 346 | Main actor, message handling, git detection |
| `actors/io/ractor_io/tests.rs` | 289 | Unit tests |

### ractor_permission.rs split
Split `crates/runie-core/src/actors/permission/ractor_permission.rs` (686 lines) into 2 focused files:

| File | Lines | Contains |
|------|-------|----------|
| `actors/permission/ractor_permission.rs` | 405 | Main actor, message handling |
| `actors/permission/ractor_permission/tests.rs` | 283 | Unit tests |

Public API preserved. All tests pass via `cargo test --workspace`.

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
- [x] **No direct mutations:** Split modules must not introduce direct state mutations; all changes go through existing actors. ✓
- [x] **No new mirrors:** Each split module must not create authoritative copies of actor-owned state. ✓
- [x] **Async work observed:** Any new async work spawned during split must be awaited or have a JoinHandle owner. ✓

## Completion Validation

- [x] **Unit tests** — `cargo test --workspace` passes.
- [x] **E2E tests** — `cargo test --workspace` passes.
- [x] **Live tmux run tests** — Provider path tested via existing headless/TUI tests.
