# Adopt Protocol Crate for IPC Types

**Status**: done
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P2

**Depends on**: (none)
**Blocks**: (none)

## Description

Create a `runie-protocol` crate containing shared types for IPC between components (server, TUI, CLI):

- `Message` — unified message type for all IPC
- `Request` / `Response` — request/response envelope
- `Notification` — fire-and-forget events
- `Error` — typed error variants
- `Version` — protocol version for compatibility

Reference: `~/Code/agents/codex-rs/protocol/` structure.

## Acceptance Criteria

- [x] `runie-protocol` crate created with `messages.rs`, `request.rs`, `notification.rs`.
- [x] Server IPC uses protocol types (TUI IPC ready to consume).
- [x] Version field on all messages for forward compatibility.
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `message_serialization_roundtrip` — JSON serialize/deserialize works.
- [x] `request_response_correlation` — request ID links request/response.

### Layer 2 — Event Handling
N/A.

### Layer 3 — Rendering
N/A.

### Layer 4 — Smoke / Crash
N/A.

## Files touched

- `crates/runie-protocol/` (new crate)
- `crates/runie-server/src/` — use protocol types
- `crates/runie-tui/src/` — use protocol types

## Notes

Enables future server/TUI separation and multi-client support.
