# Fold `runie-protocol` crate into `runie-core`

**Status**: todo
**Milestone**: R5
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: none
**Blocks**: unify-cli-json-rpc-transport-and-remove-dead-acp

## Description

`crates/runie-protocol/` is still a full workspace member despite an archived task claiming it was folded into `runie-core`. It contains chat-message types, wire messages, provider types, and error types that are re-exported or duplicated in `runie-core`. Move the types into `runie-core/src/proto/` (or appropriate modules), update consumers, and delete the crate.

## Acceptance Criteria

- [ ] Move `runie-protocol/src/*` types into `runie-core` modules (`message`, `proto`, `provider`, `error`).
- [ ] Remove `runie-protocol` from workspace members and from crate dependencies.
- [ ] Deduplicate message tests; keep one canonical copy.
- [ ] `runie-cli`, `runie-provider`, and `runie-core` compile without the crate.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `message_serde_roundtrip` — preserved in `runie-core`.
- [ ] `provider_event_serde_roundtrip` — preserved.

### Layer 2 — Event Handling
- [ ] N/A.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `minimax_replay_still_passes` — provider replay still works after the move.

## Files touched

- `Cargo.toml`
- `crates/runie-protocol/` (delete)
- `crates/runie-core/src/proto/` or `src/message/`, `src/provider/`, `src/error.rs`
- `crates/runie-core/src/lib.rs`
- `crates/runie-core/src/message/mod.rs`
- `crates/runie-cli/Cargo.toml` and source files
- `crates/runie-provider/Cargo.toml` and source files

## Notes

- The archived task `fold-protocol-into-core.md` was premature; this task completes the work.
- Keep serde as the single serialization layer.
