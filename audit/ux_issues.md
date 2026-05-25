# UX Audit - Issues & Dead-ends

## P0 Issues (Critical - Must Fix)

### P0-1: Submit with No Model Configured
**Severity:** Dead-end  
**Location:** `crates/runie-tui/src/tui/update/misc.rs:38-49`

**Issue:** When user submits without a model configured:
```rust
if model_missing {
    state.messages.push(MessageItem::System {
        text: "No model configured. Press Ctrl+O or type /onboard to set up a model.".to_string(),
    });
    return vec![];  // ← Silent failure
}
```

**Problem:** User message is recorded but no agent spawns. The guidance message is cryptic.

**Fix:** Block submission entirely with clear CTA, or auto-open onboarding.

---

### P0-2: Permission Modal Timeout Has No Countdown
**Severity:** Dead-end risk  
**Location:** `crates/runie-tui/src/components/permission_modal.rs`

**Issue:** Permission modal has 5-minute timeout but shows no countdown. User may wait indefinitely.

**Current state:**
- `timeout_start: Option<std::time::Instant>` is tracked
- No UI renders remaining time

**Fix:** Add countdown display in permission modal:
```rust
const TIMEOUT_SECS: u64 = 300;
let remaining = TIMEOUT_SECS.saturating_sub(elapsed);
```

---

### P0-3: Invalid API Key Shows Raw Error
**Severity:** Invalid state  
**Location:** `crates/runie-cli/src/tui_run.rs:120-125`

**Issue:** When API key is invalid, the error propagates as raw `AgentEvent::Error`:
```rust
Err(e) => {
    let _ = event_tx.send(AgentEvent::Error { message: e }).await;
    return;
}
```

**Problem:** No user-friendly error banner, no recovery action.

**Fix:** Map provider errors to actionable messages:
- Invalid key → "API key is invalid. Check settings."
- Rate limit → "Rate limited. Wait and retry."
- Network → "Network error. Check connection."

---

### P0-4: Session Save/Load Dead UI
**Severity:** Dead-end  
**Location:** `crates/runie-cli/src/tui_run.rs:180-188`

**Issue:** Session save/load show "not yet implemented" but appear in command palette:
```rust
Cmd::SaveSession { name } => {
    eprintln!("SaveSession not yet implemented: {:?}", name);
    vec![]
}
Cmd::LoadSession { name } => {
    eprintln!("LoadSession not yet implemented: {}", name);
    vec![]
}
```

**Problem:** Dead UI elements that silently fail.

**Fix:** Either implement or remove from command palette until ready.

---

## P1 Issues (High Priority - Should Fix)

### P1-1: Network Drop During Tool Call
**Severity:** Invalid state  
**Location:** `crates/runie-tools/src/bash.rs`, `crates/runie-agent/src/rig_loop.rs`

**Issue:** If network drops during tool execution (e.g., bash command that calls external API), there's no retry logic or rollback.

**Current:** Tool executes or fails. No recovery path.

**Fix:** Add retry with exponential backoff:
```rust
pub async fn execute(&self, args: serde_json::Value) -> Result<ToolOutput, ToolError> {
    retry_with_backoff(|| self.execute_inner(args.clone()), 3).await
}
```

---

### P1-2: File Deleted During Active Edit
**Severity:** Invalid state  
**Location:** `crates/runie-tools/src/edit_file.rs:78-82`

**Issue:** `EditFileTool` detects mtime changes but only returns error:
```rust
if current_mtime != read_mtime {
    return Err(ToolError::ExecutionFailed(
        "File was modified since it was read."
    ));
}
```

**Problem:** User must manually re-read and retry. No recovery action.

**Fix:** Add "File was modified" banner in TUI with "Re-read" action.

---

### P1-3: Model Streams Garbage Mid-token
**Severity:** Invalid state  
**Location:** `crates/runie-agent/src/rig_loop.rs:82-95`

**Issue:** Stream processing assumes well-formed tokens:
```rust
Ok(StreamedAssistantContent::Text(text)) => {
    text_content.push_str(&text.text);
```

**Problem:** If model streams invalid UTF-8 or malformed JSON, no validation.

**Fix:** Add validation:
```rust
if let Ok(text) = std::str::from_utf8(chunk_bytes) {
    text_content.push_str(text);
} else {
    tracing::warn!("Received invalid UTF-8 from model");
}
```

---

### P1-4: Double-Submit Prevention is Confusing
**Severity:** Cognitive load  
**Location:** `crates/runie-tui/src/tui/update/misc.rs:26-32`

**Issue:** Double-submit shows a message but doesn't clearly indicate current state:
```rust
if state.agent_running {
    state.messages.push(MessageItem::System {
        text: "Agent is still running. Please wait or press Ctrl+C to stop."
    });
```

**Problem:** Message is added to chat, confusing conversation flow.

**Fix:** Show in `input_right_info` instead:
```rust
state.input_right_info = "Agent running... Ctrl+C to stop".to_string();
```

---

### P1-5: Permission Queue Not Displayed
**Severity:** Cognitive load  
**Location:** `crates/runie-tui/src/tui/state.rs:120-125`

**Issue:** `PendingPermission` queue exists but not displayed:
```rust
pub struct PermissionModalState {
    // ...
    pub pending_queue: Vec<PendingPermission>,
}
```

**Problem:** If multiple permissions needed, user only sees first one.

**Fix:** Show queue indicator: "3 permissions pending" with prev/next.

---

## P2 Issues (Medium Priority - Nice to Fix)

### P2-1: Inconsistent Keybindings for Quit
**Severity:** Cognitive load  
**Location:** `crates/runie-tui/src/tui/events.rs:44-52`

**Issue:** Ctrl+Q means "quit" in Chat mode, but also used in permission modal:
```rust
TuiMode::Permission => Some(key_to_permission_msg(*key)),
// key_to_permission_msg also maps Ctrl+Q to cancel
```

**Problem:** User may accidentally cancel permission when trying to quit.

**Fix:** Use different key for permission cancel (e.g., Esc only).

---

### P2-2: Empty State Not Context-Aware
**Severity:** Cognitive load  
**Location:** `crates/runie-tui/src/components/message_list/render.rs:350-368`

**Issue:** Empty state always shows same message:
```
runie
Your coding companion
Type a message and press Enter to start
Press ^k for commands · ^b for sidebar · ^q to quit
```

**Problem:** Doesn't adapt to state (no workspace, no model, etc.)

**Fix:** Different empty states:
- No model: "Press Ctrl+O to configure a model"
- No workspace: "No workspace configured"
- First launch: Full welcome with keybindings

---

### P2-3: No Progress for Long Operations
**Severity:** Cognitive load  
**Location:** `crates/runie-cli/src/tui_run.rs`

**Issue:** Model fetching shows spinner, but session saving/loading, compaction have no progress.

**Fix:** Add progress callbacks for all async operations.

---

### P2-4: Raw Error Dumps in stderr
**Severity:** UX quality  
**Location:** Multiple `eprintln!` calls

**Issue:** Errors like these are not user-facing:
```rust
eprintln!("[Rollback] Tool {} cancelled - workspace state preserved", tool_call_id);
eprintln!("SaveSession not yet implemented: {:?}", name);
```

**Problem:** Raw technical messages in terminal output.

**Fix:** Log to tracing instead, show user-friendly messages in UI.

---

### P2-5: Permission Timeout Auto-Denies (Could Be Scary)
**Severity:** UX quality  
**Location:** `crates/runie-agent/src/loop_engine.rs:175-190`

**Issue:** When permission times out, tool is denied:
```rust
_ => {
    if let Err(e) = event_tx.send(AgentEvent::PermissionDenied { ... }).await { }
    false
}
```

**Problem:** User may not realize their pending action was denied.

**Fix:** Show "Permission timed out" system message before denial.

---

## Summary Table

| ID | Category | Severity | File:Line | Status |
|----|----------|----------|-----------|--------|
| P0-1 | Dead-end | Critical | misc.rs:38-49 | TODO |
| P0-2 | Dead-end | Critical | permission_modal.rs | TODO |
| P0-3 | Invalid state | Critical | tui_run.rs:120-125 | TODO |
| P0-4 | Dead-end | Critical | tui_run.rs:180-188 | TODO |
| P1-1 | Invalid state | High | bash.rs | TODO |
| P1-2 | Invalid state | High | edit_file.rs:78-82 | TODO |
| P1-3 | Invalid state | High | rig_loop.rs:82-95 | TODO |
| P1-4 | Cognitive load | High | misc.rs:26-32 | TODO |
| P1-5 | Cognitive load | High | state.rs:120-125 | TODO |
| P2-1 | Cognitive load | Medium | events.rs:44-52 | TODO |
| P2-2 | Cognitive load | Medium | render.rs:350-368 | TODO |
| P2-3 | Cognitive load | Medium | tui_run.rs | TODO |
| P2-4 | UX quality | Medium | Various | TODO |
| P2-5 | UX quality | Medium | loop_engine.rs:175-190 | TODO |
