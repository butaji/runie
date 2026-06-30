# Fix CLI headless native tool loops after permission denied

**Status**: todo
**Milestone**: R7
**Category**: Tools
**Priority**: P0

**Depends on**: fix-tui-mock-simple-text-response-repetition
**Blocks**: live-tui-smoke-test-real-minimax

## Description

Running `runie-headless print "native tool"` with the mock provider loops forever. After each `bash` tool call is denied (`Permission denied for tool 'bash'`), the agent immediately issues another identical `bash` tool call. The CLI never returns to the shell.

## Live Evidence

```
{"type":"text","data":{"data":"I'll run a command.\n"}}
{"type":"tool_call_start","data":{"id":"call_1","name":"bash"}}
{"type":"tool_call_input_delta","data":{"id":"call_1","delta":"{\"command\":\"echo hi\"}"}}
{"type":"tool_call_end","data":{"id":"call_1"}}
{"type":"end","data":{"stop_reason":"ToolCalls",...}}
{"type":"tool_result","data":{"id":"call_1","output":"Permission denied for tool 'bash'"}}
... repeats indefinitely ...
```

## Acceptance Criteria

- [ ] A denied tool call in headless mode produces a single final response explaining the denial.
- [ ] The CLI exits after the turn completes, rather than re-issuing the same tool call in a loop.
- [ ] A maximum turn count or explicit `tool_result` handling prevents infinite loops for deterministic providers.
- [ ] `cargo test --workspace` passes.
- [ ] `runie-headless print "native tool"` terminates with non-repeating output.

## Tests

### Layer 1 — State/Logic
- [ ] `denied_tool_does_not_loop` — mock provider returns a bash call; after a denied `tool_result`, the next response is a final text, not another bash call.

### Layer 2 — Event Handling
- [ ] `tool_result_denied_event_sequence_ends` — event stream ends with `Done` after the denied result.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `headless_native_tool_terminates` — run `runie-headless print "native tool"` and assert the output does not contain more than one `tool_call_start`.

## Files touched

- `crates/runie-agent/src/actor.rs`
- `crates/runie-agent/src/stream_response.rs`
- `crates/runie-cli/src/print.rs` (or wherever the headless loop lives)
- `crates/runie-core/src/headless_runtime.rs`

## Validation

This task is not complete until the fix is validated with all three levels:

1. **Unit tests** — cover the state/logic change in isolation.
2. **E2E tests** — cover the event handling and/or provider-replay path.
3. **Live tmux tests** — `scripts/tmux-smoke-test.sh mock` (or the relevant scenario) passes in a real terminal.

## Notes

- The mock provider is deterministic and always returns a bash call for "native tool", so the headless loop must have a guard (e.g. max tool turns) or treat a denied tool as a terminal error.
- This is a headless-only bug; the TUI shows a permission dialog but cannot answer it due to a separate focus bug.
