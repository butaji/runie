# Expose Runie via ACP over stdio

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: app-state-read-only-projection
**Blocks**: none

## Summary

Add an Agent Client Protocol (ACP) stdio adapter that exposes Runie’s event bus as JSON-RPC. The TUI becomes one client; IDEs and scripts can reuse the same agent runtime without custom integration code.

## Acceptance Criteria

- A new `AcpActor` or adapter starts with `runie --acp` or `runie agent stdio`.
- ACP messages are JSON-RPC 2.0 over stdin/stdout.
- Intents can be sent into the system; facts/events are streamed out.
- Existing TUI and headless modes continue to work unchanged.
- Authentication and permission gates are preserved.
- `cargo check --workspace` is green.

## Tests

- **Layer 2**: ACP message serialization/deserialization tests.
- **Layer 4**: ACP client-server round-trip with a mock provider fixture.
