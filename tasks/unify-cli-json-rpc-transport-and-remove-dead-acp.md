# Unify CLI JSON-RPC transport and remove dead ACP plumbing

**Status**: todo
**Milestone**: R5
**Category**: CLI / IPC
**Priority**: P1

**Depends on**: fold-runie-protocol-into-core
**Blocks**: migrate-tui-and-cli-to-leader-bootstrap

## Description

`runie-cli` has two overlapping JSON-RPC servers (`server.rs` and `acp.rs`) with duplicated stdio read/write loops and two stdout forwarders. `AcpRuntime.event_tx` sends events into a dropped receiver, and `submit_input` synthesizes events that never reach an actor. Extract a shared transport module, remove the duplicate forwarders, and route inputs through actor messages (`InputMsg`/`TurnMsg`) or delete ACP until it can be wired correctly.

## Acceptance Criteria

- [ ] Extract shared newline-delimited JSON read/write helpers into `crates/runie-cli/src/transport.rs`.
- [ ] `server.rs` and `acp.rs` use the shared transport.
- [ ] Remove duplicate stdout forwarders (`spawn_event_forwarder` / `spawn_combined_receiver`).
- [ ] Either fix ACP event plumbing (route to actor messages) or delete ACP mode.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 2 — Event Handling
- [ ] `cli_acp_sends_input_message` — ACP/stdio input produces an `InputMsg` event.
- [ ] `cli_acp_no_duplicate_stdout_forwarder` — only one task writes JSON-RPC notifications to stdout.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `server_mode_rpc_roundtrip` — a mock-stdio client can initialize and submit a turn.

## Files touched

- `crates/runie-cli/src/server.rs`
- `crates/runie-cli/src/acp.rs`
- `crates/runie-cli/src/transport.rs` (new)
- `crates/runie-cli/src/main.rs`
- `crates/runie-core/src/actors/input/messages.rs`

## Notes

- This is a prerequisite for `migrate-tui-and-cli-to-leader-bootstrap.md` because broken ACP plumbing blocks leader-based bootstrap.
- If ACP is deleted, also remove `runie agent stdio` subcommand references.
