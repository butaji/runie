# p19 — Verification: parity harness (pi event-sequence replay + grok cast snapshot diffs)

**Parity target:** proof that runie-core and runie-tui behave identically to pi and grok.

## Reference

- pi event sequences: `runAgentLoop` ordering — `agent_start`, `turn_start`, prompt `message_start`/`end`, assistant `message_start`/`message_update*`/`message_end`, tool events, `turn_end`, `agent_end` — `~/Code/agents/pi/packages/agent/src/agent-loop.ts:95-274`.
- grok TUI reference states: recorded asciinema casts `~/Code/GitHub/runie-tests/runie/artifacts/grok-full.cast` and `grok-rich.cast` (already used by `crates/runie-tui/tests/visual_snapshots.rs`).

## Adapt to runie

1. **Core event-sequence oracle**: a table of pi traces (JSON) → expected event-kind sequence. For each supported scenario (plain prompt, tool call, tool_use continuation, length-truncated tool call, error/aborted, steering-at-start, follow-up, continue-after-assistant), assert runie-core emits the exact ordered event kinds (using the existing `common::event_kinds` harness). Each row cites the pi source line it mirrors.
2. **State projection oracle**: for each trace, assert the `AgentState` projections (`is_streaming`, `streaming_message`, `pending_tool_calls`, `error_message` — p12) match pi's `AgentState` at key checkpoints.
3. **TUI snapshot diff**: extend `visual_snapshots.rs` so every transcript/status/prompt frame in the runie TUI is diffed against the frames extracted from `grok-full.cast`/`grok-rich.cast` with **zero diffs** (byte-exact symbols). New fixtures for reasoning fold, verb-group folding, tool error, markdown code blocks.
4. **Serialization oracle**: round-trip every new field (p01-p04, p09) and assert the JSON shape matches pi's TS wire forms exactly.

## State machine / variants

The oracle is a table: `scenario → pi_reference(file:line) → expected_events → expected_state_projection → expected_tui_frame`. Each row is a pass/fail check; the harness fails on any mismatch.

## Acceptance

- `cargo test --workspace` green, including the new parity harness.
- `just ci` (fmt-check + clippy + lint + test) fully green.
- All `tasks/index.json` steps `p01..p19` marked `done`.
- The harness reports **100%** of pi traces and grok frames pass (this is the 10/10 confidence gate for the parity claim).