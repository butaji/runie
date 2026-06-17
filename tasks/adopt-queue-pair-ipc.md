# Adopt Queue Pair IPC Pattern

**Status**: done
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P2

**Depends on**: (none)
**Blocks**: (none)

## Description

Implement structured Submission/Event Queue (SQ/EQ) pattern for bidirectional IPC between TUI and core:

```rust
// Submission Queue (TUI → Core)
pub struct Submission {
    pub id: SubmissionId,
    pub op: Op,
    pub trace: Option<W3cTraceContext>,
}

// Event Queue (Core → TUI)
pub struct Event {
    pub id: Option<SubmissionId>,  // Correlates to submission
    pub msg: EventMsg,
}

// Operations (TUI commands)
pub enum Op {
    UserTurn { input: String, origin: PromptOrigin },
    Interrupt,
    ExecApproval { id: ApprovalId, decision: ApprovalDecision },
    UserInputAnswer { question_id: String, answer: String },
    ConfigureSession { config: SessionConfig },
    Shutdown,
}

// Events (Core notifications)
pub enum EventMsg {
    TurnStarted { turn_id: u64 },
    TurnComplete { turn_id: u64, response_id: String },
    AgentMessage { content: String },
    ExecApprovalRequest { id: ApprovalId, tool: String, args: Value },
    Error { code: ErrorCode, message: String },
}
```

Reference: `~/Code/agents/codex-rs/protocol/src/`

## Acceptance Criteria

- [x] `Op` and `EventMsg` enums defined in `runie-protocol/`.
- [x] Submission queue with async channel for TUI→Core.
- [x] Event queue with async channel for Core→TUI.
- [x] Submission ID correlation for request/response matching.
- [x] W3C trace context propagation.
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `submission_id_correlates_event` — event links to originating submission.
- [x] `op_variant_roundtrip` — all Op variants serialize/deserialize.
- [x] `event_variant_roundtrip` — all EventMsg variants serialize/deserialize.

### Layer 2 — Event Handling
- [x] `submission_queue_delivers_to_core` — TUI sends reach core handler.
- [x] `event_queue_delivers_to_tui` — Core events reach TUI.

### Layer 3 — Rendering
N/A.

### Layer 4 — Smoke / Crash
N/A.

## Files touched

- `crates/runie-protocol/src/op.rs` (new)
- `crates/runie-protocol/src/event.rs` (new)
- `crates/runie-tui/src/ipc.rs` (new)
- `crates/runie-core/src/ipc.rs` (new)
- `crates/runie-protocol/src/lib.rs`
- `crates/runie-core/src/lib.rs`
- `crates/runie-tui/src/lib.rs`
- `crates/runie-core/Cargo.toml`
- `crates/runie-tui/Cargo.toml`

## Test Results

```text
$ cargo test --workspace
...all workspace tests pass...

$ cargo clippy --workspace -- -D warnings
...no warnings...
```

Specific new tests:
- `runie_protocol::op::tests::submission_id_correlates_event` ✅
- `runie_protocol::op::tests::op_variant_roundtrip` ✅
- `runie_protocol::op::tests::submission_with_trace_roundtrips` ✅
- `runie_protocol::event::tests::event_variant_roundtrip` ✅
- `runie_core::ipc::tests::submission_queue_delivers_to_core` ✅
- `runie_tui::ipc::tests::event_queue_delivers_to_tui` ✅
- `runie_tui::ipc::tests::submission_queue_delivers_to_core_via_tui` ✅

## Notes

This enables future multi-client support and separates TUI from core process.
Queues are intentionally not wired into the live TUI/Core main loops yet.
