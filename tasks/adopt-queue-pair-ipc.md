# Adopt Queue Pair IPC Pattern

**Status**: todo
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

- [ ] `Op` and `EventMsg` enums defined in `runie-protocol/`.
- [ ] Submission queue with async channel for TUI→Core.
- [ ] Event queue with async channel for Core→TUI.
- [ ] Submission ID correlation for request/response matching.
- [ ] W3C trace context propagation.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `submission_id_correlates_event` — event links to originating submission.
- [ ] `op_variant_roundtrip` — all Op variants serialize/deserialize.
- [ ] `event_variant_roundtrip` — all EventMsg variants serialize/deserialize.

### Layer 2 — Event Handling
- [ ] `submission_queue_delivers_to_core` — TUI sends reach core handler.
- [ ] `event_queue_delivers_to_tui` — Core events reach TUI.

### Layer 3 — Rendering
N/A.

### Layer 4 — Smoke / Crash
N/A.

## Files touched

- `crates/runie-protocol/src/op.rs` (new)
- `crates/runie-protocol/src/event.rs` (new)
- `crates/runie-tui/src/ipc.rs` (new)
- `crates/runie-core/src/ipc.rs` (new)

## Notes

This enables future multi-client support and separates TUI from core process.
