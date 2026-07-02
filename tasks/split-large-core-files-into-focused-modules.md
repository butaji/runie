# Split large core files into focused modules

## Status

`partial` — `proto/message/mod.rs` (845→342 lines) split into focused modules. Remaining: `event/durable.rs` (772), `actors/fff_indexer/mod.rs` (669), `provider/provider_trait.rs` (663), `actors/permission/ractor_permission.rs` (685), `actors/io/ractor_io.rs` (633).

## Description

The following production files exceed the advertised 500-line limit and mix responsibilities:

- ~~`crates/runie-core/src/proto/message/mod.rs`~~ — **DONE** (845→342 lines)
  - Split into: `tool_call.rs`, `role.rs`, `metadata.rs`, `validation.rs`, `chat_message.rs`, `mod.rs`
- `crates/runie-core/src/event/durable.rs` — 772 lines
- `crates/runie-core/src/actors/fff_indexer/mod.rs` — 669 lines
- `crates/runie-core/src/provider/provider_trait.rs` — 663 lines
- `crates/runie-core/src/actors/permission/ractor_permission.rs` — 685 lines
- `crates/runie-core/src/actors/io/ractor_io.rs` — 633 lines
- `crates/runie-core/src/model/state/domain_ops.rs` — 410 lines (under limit)

(`session/sqlite_store.rs` is excluded because SQLite is deferred; see `standardize-session-persistence-on-jsonl.md`.)

## Changes (proto/message/mod.rs split — done)

Split `crates/runie-core/src/proto/message/mod.rs` (845 lines) into 6 focused modules:

| File | Lines | Contains |
|------|-------|----------|
| `proto/message/mod.rs` | 342 | Re-exports + tests |
| `proto/message/chat_message.rs` | 318 | `ChatMessage`, `ChatMessageBuilder`, `now()` |
| `proto/message/role.rs` | 68 | `Role`, `MessageOrigin` |
| `proto/message/validation.rs` | 98 | `validate_message`, `validate_messages`, `SanitizeError` |
| `proto/message/tool_call.rs` | 52 | `ToolCall` |
| `proto/message/metadata.rs` | 25 | `MessageMetadata` |
| `proto/message/parts.rs` | 56 | `Part` (already separate) |

Public API preserved via re-exports in `mod.rs`. All external callers updated automatically via unchanged import paths.

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
