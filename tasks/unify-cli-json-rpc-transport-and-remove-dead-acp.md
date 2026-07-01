# Unify CLI JSON-RPC transport and remove dead ACP plumbing

**Status**: done
**Milestone**: R5
**Category**: CLI / IPC
**Priority**: P1

**Depends on**: fold-runie-protocol-into-core
**Blocks**: migrate-tui-and-cli-to-leader-bootstrap

## Description

`runie-cli` had two overlapping JSON-RPC servers (`server.rs` and `acp.rs`) with duplicated stdio read/write loops and broken event forwarding. `AcpRuntime.event_tx` sent events into a dropped receiver, and `submit_input` synthesized events that never reached an actor.

**Decision**: Delete the broken ACP mode. Keep the working server mode and unified transport.

## What was done

### Transport module (already existed)
- `crates/runie-cli/src/transport.rs` provides shared `parse_request`, `write_message`, and `build_response`.

### Server mode fixes
- `server.rs` now uses `write_message` from `transport.rs` instead of duplicate `write_response`.
- Removed unused `AsyncWriteExt` import.

### ACP deletion
- Deleted `crates/runie-cli/src/acp.rs`.
- Removed `Acp` variant from `Command` enum in `main.rs`.
- Removed `mod acp;` declaration.
- Updated documentation in `docs/Architecture.md` to remove ACP references.

## Acceptance Criteria

- [x] Extract shared newline-delimited JSON read/write helpers into `crates/runie-cli/src/transport.rs`.
- [x] `server.rs` uses the shared transport.
- [x] Remove duplicate stdout forwarders (`spawn_event_forwarder` / `spawn_combined_receiver`).
- [x] Either fix ACP event plumbing (route to actor messages) or delete ACP mode — **deleted**.
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 2 — Event Handling
- [x] `server::tests::rpc_parses_request` — server mode parses requests correctly.
- [x] `server::tests::rpc_returns_response` — server mode returns responses correctly.
- [x] `transport::tests::parse_request_parses_valid_json` — transport parses JSON-RPC requests.
- [x] `transport::tests::parse_request_returns_error_on_invalid` — transport handles parse errors.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] `server::tests::rpc_list_models` — server mode lists models from catalog.

## Files touched

- `crates/runie-cli/src/server.rs` — use shared transport
- `crates/runie-cli/src/acp.rs` — **deleted**
- `crates/runie-cli/src/transport.rs` — already existed
- `crates/runie-cli/src/main.rs` — remove ACP command
- `docs/Architecture.md` — update external interfaces section

## Notes

- Server mode (`runie server`) provides working JSON-RPC over TCP/stdio.
- ACP was an experimental event-driven interface that never worked correctly.
- Future work: consider unifying the leader TCP protocol (`crates/runie-core/src/actors/leader/actor.rs`) with the CLI transport module or migrating to `jsonrpsee`.
> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
