# Step 11: MockStreamFn + fixture harness

**Status:** pending
**Depends on:** 10

## Goal
Build the deterministic test harness that lets every behavioural test replay an event sequence without `sleep()`.

## Changes
- `crates/runie-core/tests/common/mod.rs` (new):
  - `MockStreamFn` impl of `StreamFn` that consumes a `Vec<AssistantMessageEvent>` and yields each event via the stream.
  - `MockStreamFn::from_yaml(path)` / `from_json(value)` loaders.
  - `RecordedEvent { kind: String, payload: serde_json::Value }` struct.
  - `TestBus` wrapper around `EventBus` that records every dispatched event into a `Mutex<Vec<AgentEvent>>` for assertions.
  - `TestLoop` builder: `TestLoop::new().with_mock_stream(events).with_tools(...).with_before_tool_call(...).build()`.
- `crates/runie-core/tests/fixtures/` directory with sample YAML fixtures:
  - `hello.yaml` — single text response.
  - `tool_call.yaml` — one tool call, then text.
  - `parallel_tools.yaml` — three tool calls in one assistant turn.
  - `stream_error.yaml` — assistant starts, then error event.

## Verification
- `cargo test -p runie-core --test common_helpers` → exit 0 (a smoke test that loads each fixture and runs `MockStreamFn` to completion).

## Notes
- Determinism: tests use `tokio::time::pause()` + `advance` where timing matters, never `sleep`.
- No real network or provider code in this crate; everything goes through `MockStreamFn`.