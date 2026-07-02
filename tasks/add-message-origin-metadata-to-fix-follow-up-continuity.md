# Add message origin metadata to fix follow-up continuity

## Status

`partial`

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

### ✅ Step 2: Tag messages at creation points (partial)
**Status:** In Progress

Updated message creation in session handlers:
- ✅ `handle_add_user_message` → `MessageOrigin::User`
- ✅ `handle_add_system_message` → `MessageOrigin::System`
- ✅ `handle_add_tool_message` → `MessageOrigin::Tool`

Still need to update:
- [ ] Compaction summarization → `MessageOrigin::Compaction`
- [ ] Steering messages → `MessageOrigin::Steering`
- [ ] Follow-up messages → `MessageOrigin::FollowUp`
- [ ] Context injection (@file, @search) → `MessageOrigin::Context`

### ⏳ Step 3: Use origin in turn scheduling
**Status:** Pending

Need to update turn scheduling logic to consider origin when deciding:
- Whether to queue a message behind an active turn
- Whether to start a new turn immediately

## Acceptance Criteria
- [x] Define `MessageOrigin` enum (User, Tool, System, Compaction, etc.).
- [x] Tag messages at creation points (user, system, tool done).
- [ ] Use origin in turn scheduling.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, or async runtime changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for origin-based scheduling.
- **Layer 2 — Event Handling:** Follow-up events carry `User` origin.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Multi-turn replay tests pass.
- **Live tmux testing session (required):** User follow-up starts a new turn reliably.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
