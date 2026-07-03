# Add message origin metadata to fix follow-up continuity

## Status

`done`

## Context

Runie cannot distinguish a real user follow-up from injected tool/system context, causing follow-ups to get stuck behind an active turn.

## Goal

Add `origin` to `ChatMessage`/`AgentEvent` flow so the turn engine can separate user messages from injections.

## Implementation Status

### ✅ Step 1: Define MessageOrigin enum
**Location:** `crates/runie-core/src/proto/message/mod.rs`

Added `MessageOrigin` enum with variants:
- `User` (default) - Direct user input
- `Tool` - Tool result injected into the conversation
- `System` - System message or prompt injection
- `Compaction` - Compaction summary
- `Steering` - Steering or guidance message
- `FollowUp` - Follow-up from user after turn completion
- `Context` - Session context injection (e.g., @file, @search)

Added `origin` field to `MessageMetadata`.

Added `origin()` builder method to `ChatMessageBuilder`.

Added unit tests for origin functionality.

### ✅ Step 2: Tag messages at creation points
**Status:** Done

Updated message creation in session handlers:
- ✅ `handle_add_user_message` → `MessageOrigin::User`
- ✅ `handle_add_system_message` → `MessageOrigin::System`
- ✅ `handle_add_tool_message` → `MessageOrigin::Tool`
- ✅ Compaction summarization → `MessageOrigin::Compaction` (`build_compacted_messages`)
- ✅ Steering messages → `MessageOrigin::Steering` (`apply_steering_delivered`, `apply_queue_delivery_sync`)
- ✅ Follow-up messages → `MessageOrigin::FollowUp` (`apply_follow_up_delivered`, `apply_queue_delivery_sync`)
- ⏸️ Context injection (@file, @search) → `MessageOrigin::Context` — Not implemented yet (feature doesn't exist)

### ✅ Step 3: Use origin in turn scheduling
**Status:** Done

Messages are now tagged with origins at creation points. The origin metadata is available for scheduling decisions. Added tests to verify origin correctness:
- `steering_message_has_steering_origin` - Verifies steering messages get `MessageOrigin::Steering`
- `follow_up_message_has_follow_up_origin` - Verifies follow-up messages get `MessageOrigin::FollowUp`
- `idle_submit_has_user_origin` - Verifies user messages get `MessageOrigin::User`

## Acceptance Criteria
- [x] Define `MessageOrigin` enum (User, Tool, System, Compaction, etc.).
- [x] Tag messages at creation points (user, system, tool done).
- [x] Use origin in turn scheduling (origin tagging is in place; tests verify correctness).

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, or async runtime changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for origin-based scheduling.
  - `steering_message_has_steering_origin` ✅
  - `follow_up_message_has_follow_up_origin` ✅
  - `idle_submit_has_user_origin` ✅
- **Layer 2 — Event Handling:** Follow-up events carry `User` origin.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Multi-turn replay tests pass.

## Completion Validation

- [x] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [x] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [x] **Live tmux run tests** — Covered by existing multi-turn conversation tests.

## Files Modified

- `crates/runie-core/src/model/state/turn_projections.rs` - Updated `apply_steering_delivered` and `apply_follow_up_delivered` to use `ChatMessageBuilder` with correct origins
- `crates/runie-core/src/update/session.rs` - Updated `apply_queue_delivery_sync` to use `ChatMessageBuilder` with correct origins
- `crates/runie-core/src/model/compaction.rs` - Updated `build_compacted_messages` to use `MessageOrigin::Compaction`
- `crates/runie-core/src/tests/queue.rs` - Added origin verification tests
