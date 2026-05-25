# Behavior Gaps — Undefined State Transitions

**Date:** 2024-05-24  
**Auditor:** ralph/overnight-audit  

---

## State Machine Overview

The Runie agent TUI can be modeled as a finite state machine with these primary states:

```
                    ┌─────────────────────────────────────────┐
                    │                                         │
                    ▼                                         │
┌─────────┐    ┌─────────────┐    ┌──────────────────────────▼┐
│ Onboard │───▶│    Chat     │◀───│ Permission (blocking)    │
│   ing   │    │             │    │                          │
└─────────┘    └──────┬──────┘    └──────────────────────────┘
       ▲              │                     │
       │              │                     │
       │              ▼                     │
       │         ┌─────────┐                │
       │         │ Running │◀───────────────┘
       │         │ (Agent) │   (Tool permission)
       │         └────┬────┘
       │              │
       │              ▼
       │    ┌─────────────────┐
       └─── │     Error        │
            │  (Recoverable)   │
            └─────────────────┘
```

**Secondary Modes (overlays):**
- `TuiMode::CommandPalette`
- `TuiMode::DiffViewer`
- `TuiMode::SessionTree`
- `TuiMode::Overlay`
- `TuiMode::Select`

---

## Gap 1: Interrupted → Error (Undefined Transition)

**Status:** Partially Defined  
**File:** `crates/runie-agent/src/loop_engine.rs`  
**Location:** `run_agent_loop()` function

**Issue:** When `Msg::Stop` is received mid-turn, the agent loop has no clean exit path. The `Interrupt` command is sent from TUI, but the loop doesn't check for interruption between tool executions.

**Current Behavior:** The TUI correctly handles interrupt by setting `agent_running = false` and returning to Chat mode. However, the agent task may still be running.

**Missing Transition:** `Running` + `Interrupt` → `Error(Interrupted)` — needs agent loop to check for interruption.

---

## Gap 2: Permission Denied → Partial Rollback (Incomplete)

**Status:** Defined  
**File:** `crates/runie-agent/src/loop_engine.rs`, `crates/runie-tui/src/tui/update/agent.rs`  
**Location:** `handle_permission()` and `handle_permission_msg()`

**Issue:** When permission is denied, the TUI sends a `Rollback` command. The command exists and is processed, but actual file system rollback logic needs verification.

**Current Behavior:**
```rust
// TUI update/agent.rs
let should_rollback = tool_call_id.is_some()
    && (matches!(decision, PermissionDecision::Deny { .. })
        || matches!(decision, PermissionDecision::Skip { .. }));

let mut cmds = vec![Cmd::SendPermission { decision }];
if should_rollback {
    cmds.push(Cmd::Rollback { tool_call_id: tool_call_id.unwrap() });
}
```

**Status:** `Cmd::Rollback` is defined and sent. Actual rollback implementation needs verification.

---

## Gap 3: Network Drop During Tool Call → No Retry

**Status:** Identified (Gap BG-3)  
**File:** `crates/runie-tools/src/bash.rs`, `edit_file.rs`, `write_file.rs`  
**Location:** `Tool::execute()` implementations

**Issue:** Tool execution can fail due to network issues. There's no retry logic.

**Harness Task:** Added `harness/tasks/network_retry/` to track this gap.

**Proposed Fix:** Add retry wrapper with exponential backoff for transient errors.

---

## Gap 4: File Deleted During Edit → No Detection

**Status:** Identified (Gap BG-3)  
**File:** `crates/runie-tools/src/edit_file.rs`  
**Location:** `EditFileTool::execute()`

**Issue:** If a file is deleted or modified between read and write, the tool will fail or overwrite changes.

**Harness Task:** Added `harness/tasks/file_stale_edit/` to track this gap.

**Proposed Fix:** Store file mtime on read, verify on write, return StaleEdit error if mismatch.

---

## Gap 5: Model Streams Garbage → No Validation

**Status:** Tracked via Harness  
**File:** `crates/runie-ai/src/` (provider implementations)  
**Location:** Streaming implementations

**Issue:** If the model returns malformed tokens, the agent may crash.

**Harness Task:** `harness/tasks/streaming_garbage/` exists to track this.

---

## Gap 6: DAG Cycle Detection (Future Consideration)

**Status:** Not Yet Applicable  
**File:** Not yet implemented  
**Location:** Tool dependency tracking

**Issue:** If tools could call other tools (agent-in-agent), cycles could cause infinite loops.

**Current Status:** Currently tools don't call other tools, so this isn't an issue.

---

## Gap 7: Idempotency — Re-running Same Command

**Status:** Partially Addressed  
**File:** `crates/runie-tui/src/tui/update/misc.rs`  
**Location:** `handle_submit()`

**Issue:** User can submit the same message twice in quick succession.

**Current Behavior:** 
```rust
// BG-7 & P1-4 FIX: Block double-submit with user feedback
if state.agent_running {
    state.messages.push(MessageItem::System {
        text: "Agent is still running...".to_string(),
    });
    return vec![];
}
```

**Gap:** There's still a potential race window between `agent_running` check and setting it.

---

## Undefined Transition Matrix

| From State | Event | To State | Status |
|------------|-------|----------|--------|
| Running | ToolPanic | Error | ✅ Caught by panic catch |
| Running | Interrupt | Error | ⚠️ Partial (TUI handles, agent loop needs) |
| Permission | Deny | Chat | ✅ Rollback command sent |
| Chat | NetworkDrop | Error | ❌ Gap BG-3 |
| Running | FileDeleted | Error | ❌ Gap BG-3 |
| Thinking | GarbageToken | Error | ⚠️ Tracked via harness |
| Chat | DuplicateSubmit | Chat | ⚠️ Partial (race window) |
| Onboarding | InvalidAPIKey | Onboarding | ❌ Gap BG-4 |
| Any | Panic | Crash | ✅ Only caught by outer handler |

---

## Implemented Fixes

### ✅ Gap BG-1: Permission Queue Visibility
Permission requests are now queued when in blocking mode (DiffViewer, SessionTree, etc.). Queue processing is handled when the modal is closed.

### ✅ Gap BG-2: Agent Error Resets Mode
When agent errors out, mode always resets to Chat (unless in Onboarding). This prevents getting stuck in Permission mode.

### ✅ Gap BG-5: Agent End Clears Permission Modal
When agent ends, any pending permission modal is cleared.

### ✅ Gap BG-6: Submit Blocked with Feedback
Double submit is blocked while agent is running, with user feedback.

### ✅ Gap BG-7: Permission Timeout Countdown
Permission modal now displays countdown timer.

---

## Recommended Test Coverage

| Test | Status |
|------|--------|
| test_interrupted_agent_cleanup | ⚠️ Partial - TUI works, agent loop needs |
| test_permission_rollback | ✅ Command sent |
| test_network_retry | ❌ Gap BG-3 |
| test_file_modified_during_edit | ❌ Gap BG-3 |
| test_garbage_token_handling | ⚠️ Tracked via harness |
| test_duplicate_submit_blocked | ✅ Works |
