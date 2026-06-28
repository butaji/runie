# Extract a shared streaming-response parser

**Status**: todo
**Milestone**: R2
**Category**: Agent / Provider
**Priority**: P1

**Depends on**: migrate-production-actors-to-ractor
**Blocks**: none

## Description

`crates/runie-agent/src/stream_response.rs` and `crates/runie-agent/src/headless/mod.rs` both consume `ProviderEvent` streams and perform nearly identical work: accumulate text/thinking deltas, route tool-call events through `ToolStream`, fall back to `parse_tool_calls_fallible`, and strip tool markers. `ToolStream` was already extracted, but the surrounding event loop, fallback parsing, and finish handling are still duplicated. A shared pure `StreamingResponseParser` would remove ~150–200 lines and ensure streaming fixes only need to happen once.

## Acceptance Criteria

- [ ] Extract a shared parser (in `runie-core` or `runie-agent`) that transforms a `ProviderEvent` stream into `(text, reasoning, tool_calls, parse_errors)`.
- [ ] Update `stream_response.rs` to feed its `EmitFn` from the parser.
- [ ] Update `headless/mod.rs` to feed its `HeadlessEvent` callbacks from the parser.
- [ ] Preserve all existing fallback and tool-marker-stripping behavior.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `stream_response_and_headless_produce_same_output` — the same provider replay fixture produces the same text/tools/errors in both paths.
- [ ] `fallback_parsing_still_works` — when structured tool events are absent, fallback parsing still extracts tool calls.

## Files touched

- `crates/runie-agent/src/stream_response.rs`
- `crates/runie-agent/src/headless/mod.rs`
- New shared parser module (e.g., `crates/runie-agent/src/stream_parser.rs` or `crates/runie-core/src/tool/stream_parser.rs`)

## Notes

- The parser should be pure/async-agnostic so it can be driven by both production and headless runtimes.
- Coordinate with `migrate-production-actors-to-ractor.md` so the event contracts are stable before unifying the parser.
