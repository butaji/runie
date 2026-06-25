# Fold runie-protocol into runie-core

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

`runie-protocol` (557 LOC, 9 modules) is a separate workspace crate with exactly 2 consumers:

- `crates/runie-server/src/main.rs` — the RPC server.
- `crates/runie-core/src/ipc.rs` — IPC queue pair shared between core and TUI.

No external IDE/plugin consumer exists today. The crate exists "for future IDE integration" but that is speculative (YAGNI). Maintaining a separate crate means: an extra `Cargo.toml`, workspace member, `runie-protocol.workspace = true` dependency line in 2 crates, and a crate boundary that forces `runie-core` to depend on it for a single `ipc.rs` file. Folding it into `runie-core` as a `proto/` module (with `ipc.rs` moving to `proto/ipc.rs`) removes the crate overhead while keeping the types reusable if an external consumer appears later.

## Acceptance Criteria

- [ ] `crates/runie-protocol/` deleted from workspace.
- [ ] `runie-protocol` removed from `Cargo.toml` `[workspace]` members and `[workspace.dependencies]`.
- [ ] `runie-protocol.workspace = true` removed from `runie-core` and `runie-server` `Cargo.toml`.
- [ ] Protocol types moved to `crates/runie-core/src/proto/` (`mod.rs`, `error.rs`, `event.rs`, `messages.rs`, `notification.rs`, `op.rs`, `request.rs`, `response.rs`, `version.rs`, `ipc.rs`).
- [ ] `runie-core/src/lib.rs` declares `pub mod proto;` and re-exports `Error`, `Event`, `EventMsg`, `Message`, `Notification`, `Op`, `ApprovalDecision`, `ApprovalId`, `PromptOrigin`, `SessionConfig`, `Submission`, `SubmissionId`, `Request`, `Response`, `Version`, `PROTOCOL_VERSION`.
- [ ] `runie-server` imports rewritten from `runie_protocol::` → `runie_core::proto::` (or crate-root re-exports).
- [ ] `cargo test --workspace` succeeds.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `proto_types_round_trip` — `Request`/`Response`/`Message` serde round-trip unchanged after move.

### Layer 2 — Event Handling
- [ ] `proto_event_serializes` — `Event` / `EventMsg` serialize to the same JSON shape.

### Layer 3 — Rendering
- N/A — protocol types, no rendering.

### Layer 4 — Smoke / Crash
- [ ] `smoke_server_uses_core_proto` — `runie-server` compiles and its existing RPC parse tests pass against `runie_core::proto`.

## Files touched

- `crates/runie-protocol/` → delete (9 files + Cargo.toml)
- `crates/runie-core/src/proto/` → new (10 files, content from runie-protocol + `ipc.rs`)
- `crates/runie-core/src/ipc.rs` → delete (moved to `proto/ipc.rs`)
- `crates/runie-core/src/lib.rs` — `pub mod proto;` + re-exports; remove `pub mod ipc;`
- `crates/runie-core/Cargo.toml` — remove `runie-protocol` dep
- `crates/runie-server/Cargo.toml` — remove `runie-protocol` dep
- `crates/runie-server/src/main.rs` — rewrite imports
- `Cargo.toml` (root) — remove from workspace members + dependencies

## Notes

Reversible: if a real external consumer (IDE plugin, language server) materializes, `proto/` can be extracted back into a crate with `cargo new`. The cost of keeping a speculative crate is real (workspace churn, extra dep graph node); the cost of re-extracting is low. Rejected alternative: keep as a crate "for separation" — rejected because separation without a consumer is YAGNI. The `proto/` module name avoids collision with the existing `ipc.rs` root file (which folds in).
