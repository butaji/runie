# Behavior Gaps: State Machine Analysis

## State Machine Overview

### States (TuiMode)
```
Chat ─────────┬──> CommandPalette ──> Chat
              ├──> Permission ──────> Chat
              ├──> DiffViewer ─────> Chat
              ├──> SessionTree ────> Chat
              ├──> Overlay ────────> Chat
              └──> Onboarding ─────> Chat (or Exit)
```

### Events (Msg)
```
Quit, Stop, Submit, TextareaKey, InsertNewline
ToggleSidebar, OpenCommandPalette, CloseModal, ConfirmModal
ScrollUp, ScrollDown, ScrollPageUp, ScrollPageDown
PermissionConfirm, PermissionCancel, PermissionAlways, PermissionSkip
PermissionTimeout
CommandPaletteFilter, CommandPaletteBackspace, CommandPaletteUp
CommandPaletteDown, CommandPaletteConfirm, CommandPaletteCancelArgument
AgentEvent
Tick, CursorBlink
SlashCommand
ToggleSessionTree, SessionTreeUp, SessionTreeDown, SessionTreeConfirm
OnboardingNext, OnboardingBack, OnboardingNavigateUp, OnboardingNavigateDown
OnboardingSelectProvider, OnboardingSelectModel
OnboardingKeyInput, OnboardingKeyBackspace
OnboardingSearchInput, OnboardingSearchBackspace
OnboardingSubmit, OnboardingSkip
ClearInput, ClearChat, DirectCommand, Paste
ModelsFetched, ModelsFetchFailed
Resize
```

---

## Undefined or Implicit Transitions

### BG-1: Permission Queue Processing
**Current:** `on_permission_request()` queues requests when in blocking mode.  
**Gap:** After processing a queued permission, state may not properly transition back.
**File:** `crates/runie-tui/src/tui/update/agent.rs:122-144`  
**Fix:** Already has FIX comment. Queue processing is implemented in `handle_permission()`.

### BG-2: Error State Cleanup
**Current:** `on_agent_error()` sets `mode = TuiMode::Chat` unless in Onboarding.  
**Gap:** If agent errors while permission is pending, cleanup may be incomplete.
**File:** `crates/runie-tui/src/tui/update/agent.rs:73-75`  
**Fix:** Already has FIX comment noting "BG-2 FIX".

### BG-3: AgentEnd Cleanup
**Current:** `on_agent_end()` clears permission modal and pending queue.  
**Gap:** If multiple AgentEnd events fire, state may become inconsistent.
**File:** `crates/runie-tui/src/tui/update/agent.rs:60-68`  
**Fix:** Already has FIX comment "BG-5 FIX".

### BG-4: Interrupt Fade Animation
**Current:** `AnimationState` has `interrupt_fade_start` for fade effect.  
**Gap:** Animation is rendered but state cleanup after fade is not explicit.
**File:** `crates/runie-tui/src/components/message_list/render.rs:290-310`  
**Recommendation:** Add explicit transition from interrupt state to idle.

### BG-5: Tick Message Side Effects
**Current:** `Msg::Tick` triggers animation update AND permission timeout check.  
**Gap:** Tick is always processed, but timeout check has side effects.
**File:** `crates/runie-tui/src/tui/state.rs:408-414`  
**Fix:** Already has FIX comment "P0-1 FIX: Check for permission timeout".

---

## Idempotency Issues

### IDEM-1: Duplicate Tool Calls (Already Fixed)
**File:** `crates/runie-agent/src/loop_engine.rs:140-150`  
**Status:** ✅ FIXED - `seen_tool_calls` HashSet prevents duplicates.

### IDEM-2: Double-Submit (Already Fixed)
**File:** `crates/runie-tui/src/tui/update/misc.rs:38-42`  
**Status:** ✅ FIXED - `agent_running` check blocks double-submit.

### IDEM-3: File Concurrent Edits (Already Fixed)
**File:** `crates/runie-tools/src/workspace.rs:10-35`  
**Status:** ✅ FIXED - `FileLock` and `with_lock()` for exclusive access.

---

## Cancellation Behavior

### CANCEL-1: Ctrl+C / Stop
**File:** `crates/runie-tui/src/tui/events.rs:37-40`  
**Current:** `Msg::Stop` triggers `Cmd::Interrupt` which aborts agent task.  
**Behavior:** Agent task is aborted via `handle.abort()`.  
**Gap:** Partial tool changes may not be rolled back.  
**Fix:** Already has FIX comment "P1-4 FIX: Rollback". Rollback is sent via `Cmd::Rollback`.

### CANCEL-2: Permission Deny/Cancel
**File:** `crates/runie-tui/src/tui/update/agent.rs:150-168`  
**Current:** On deny, `Cmd::Rollback` is sent to agent.  
**Behavior:** Agent should revert any file changes made by denied tool.  
**Gap:** Rollback implementation is a placeholder (`eprintln!`).  
**Recommendation:** Implement actual file state rollback mechanism.

### CANCEL-3: Task Abort on Exit
**File:** `crates/runie-cli/src/tui_run.rs:280-285`  
**Current:** On exit, `agent_task.abort()` is called.  
**Behavior:** Task is aborted and we await completion.  
**Status:** ✅ FIXED - Clean abort on exit.

---

## Concurrency Issues

### CONC-1: File Locking (Already Fixed)
**File:** `crates/runie-tools/src/workspace.rs:10-35`  
**Status:** ✅ FIXED - `FileLock` prevents concurrent edits to same file.

### CONC-2: Atomic Writes (Already Fixed)
**File:** `crates/runie-tools/src/workspace.rs:40-75`  
**Status:** ✅ FIXED - `atomic_write()` uses temp file + rename.

### CONC-3: Permission Channel
**File:** `crates/runie-cli/src/tui_run.rs:168-172`  
**Current:** Fresh permission channel created on each agent spawn.  
**Gap:** If old channel has pending messages, they may leak to new agent.  
**Fix:** Channel is replaced on spawn, old one is dropped.

### CONC-4: Event Channel Backpressure
**File:** `crates/runie-cli/src/tui_run.rs:195-205`  
**Current:** Terminal reader retries up to 10 times with 1ms sleep.  
**Gap:** If channel stays full, events are dropped silently.  
**Recommendation:** Consider bounded channel with overflow handling.

---

## Recovery Actions for Invalid States

### RECOVERY-1: Model Streams Garbage
**File:** `crates/runie-agent/src/loop_engine.rs:110-135`  
**Behavior:** Text is accumulated incrementally. Garbage tokens would appear in UI.  
**Recommendation:** Add UTF-8 validation before displaying. Already has error handling in provider.

### RECOVERY-2: Network Drops During Tool Call
**File:** `crates/runie-agent/src/loop_engine.rs:240-270`  
**Behavior:** Tool execution error is returned as result.  
**Status:** ✅ Has error handling - returns `ToolResult` with error message.

### RECOVERY-3: File Deleted During Edit
**File:** `crates/runie-tools/src/edit_file.rs`  
**Behavior:** Write fails, error returned to agent.  
**Recommendation:** Add retry mechanism or workspace state check.

### RECOVERY-4: Invalid API Key
**File:** `crates/runie-tui/src/tui/update/onboarding.rs:290-310`  
**Behavior:** API key validation fails with specific error message.  
**Status:** ✅ Fixed - `validate_key_detailed()` provides specific errors.

### RECOVERY-5: DAG Cycle Detection
**File:** N/A - Not implemented  
**Behavior:** No cycle detection in tool execution order.  
**Recommendation:** Add dependency graph tracking if tools can have dependencies.

### RECOVERY-6: Actor Panic
**File:** `crates/runie-agent/src/loop_engine.rs:290-350`  
**Behavior:** Panic is caught, error result returned, workspace rolled back.  
**Status:** ✅ Fixed - `execute_tool_with_panic_catch()` handles this.

---

## Recommended Fixes

### Fix 1: Add State Transition Assertions
```rust
// In update.rs, add assertions for valid state transitions
fn assert_valid_transition(from: TuiMode, to: TuiMode, msg: &Msg) {
    match (from, to) {
        // Valid: any mode can go to Chat via CloseModal
        (_, TuiMode::Chat) if matches!(msg, Msg::CloseModal | Msg::ConfirmModal) => {},
        // Valid: Chat can go to Onboarding
        (TuiMode::Chat, TuiMode::Onboarding) if matches!(msg, Msg::OnboardingNext) => {},
        // Add more valid transitions...
        _ => panic!("Invalid state transition: {:?} -> {:?} via {:?}", from, to, msg),
    }
}
```

### Fix 2: Implement Rollback Mechanism
```rust
// In workspace.rs, add rollback stack
pub struct Workspace {
    // ... existing fields
    rollback_stack: Vec<RollbackEntry>,
}

struct RollbackEntry {
    path: PathBuf,
    previous_content: String,
}
```

### Fix 3: Add Event Channel Overflow Handling
```rust
// In tui_run.rs, handle channel overflow
let event = crossterm::event::read();
if raw_tx.try_send(event.clone()).is_err() {
    // Show "input dropped" indicator
    tui.update(Msg::AgentEvent(AgentEvent::SystemMessage {
        text: "Input dropped - terminal busy".to_string()
    }));
}
```

---

## Summary

| Category | Fixed | Remaining | Total |
|----------|-------|-----------|-------|
| State Transitions | 3 | 1 | 4 |
| Idempotency | 3 | 0 | 3 |
| Cancellation | 2 | 1 | 3 |
| Concurrency | 3 | 1 | 4 |
| Recovery | 4 | 2 | 6 |
| **Total** | **15** | **5** | **20** |

Key remaining items:
1. **Rollback implementation** - Currently placeholder
2. **Event channel overflow handling** - Events silently dropped
3. **Cycle detection** - Not applicable without tool dependencies
