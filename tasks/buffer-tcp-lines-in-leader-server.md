# Buffer TCP lines in leader server

**Status**: todo
**Milestone**: R7
**Category**: Architecture / IPC
**Priority**: P3

**Depends on**: fix-leader-shutdown-to-await-all-actors
**Blocks**: none

## Description

`LeaderActor::listen_tcp` reads fixed 1024-byte chunks and calls `process_client_line` per chunk. A JSON line that spans two reads, or a multi-byte UTF-8 sequence split across reads, may be mis-parsed.

## Root Cause

`crates/runie-core/src/actors/leader/actor.rs:218-228` does not buffer incoming bytes until a newline is seen.

## Acceptance Criteria

- [ ] Incoming TCP bytes are buffered until a complete newline-delimited JSON line is received.
- [ ] Split UTF-8 sequences are handled correctly.
- [ ] `cargo test --workspace` passes.
- [ ] A server-mode smoke test with large intents does not corrupt input.

## Tests

### Layer 1 — State/Logic
- [ ] `tcp_buffer_reassembles_split_lines` — feed a JSON line in two chunks and assert it is parsed once.

### Layer 2 — Event Handling
- [ ] `tcp_line_parsed_to_intent` — a complete line produces the expected bus event.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A — server mode; unit/e2e coverage is sufficient.

## Files touched

- `crates/runie-core/src/actors/leader/actor.rs`

## Validation

This task is not complete until the fix is validated with all three levels:

1. **Unit tests** — cover the state/logic change in isolation.
2. **E2E tests** — cover the event handling and/or provider-replay path.
3. **Live tmux tests** — `scripts/tmux-smoke-test.sh mock` (or the relevant scenario) passes in a real terminal.

## Notes

- Only affects server/TCP mode, not the normal TUI.
