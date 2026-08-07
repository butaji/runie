# Step 09: Integration test (TestBackend)

**Status:** implemented; runtime YAML replay is now the canonical integration surface
**Depends on:** 08

## Goal
An end-to-end test that drives `App` via `ratatui::backend::TestBackend`, runs a `MockStreamFn` with two tool calls, and asserts the rendered Buffer contains the full transcript in order.

## Changes
- `crates/runie-tui/tests/e2e_test.rs`:
  - Reuse `runie_core::r#loop::LoopActor` + a `MockStreamFn` that emits: Start, TextDelta("Hello"), TextDelta(" world"), Done (with one tool call in the assistant message), then a second turn with TextDelta("Done").
  - Register a `bash` echo tool and a `read_file` stub tool.
  - Build the App pointing at the same `EventBus`.
  - Call `App::run` with a `TestBackend` of fixed size (24x80). Drive it manually via the action channel.
  - After all events are processed, capture the `Buffer` and assert it contains:
    - "Hello world" (streaming text finalised)
    - tool execution line ("⚙ bash: …")
    - tool result line
    - "Done" (second turn)
- `crates/runie-tui/tests/common/mod.rs`: helper to build a test App with a sized TestBackend.

## Verification
- `cargo test -p runie-tui --test e2e_test` → exit 0.
- `cargo test -p runie-tui` → exit 0 (all tests pass).

## Notes
- No `sleep()`. Drive the loop by sending actions, awaiting the loop task, then asserting on the Buffer.
- TestBackend size: 24 rows × 80 cols.
