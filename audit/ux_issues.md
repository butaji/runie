# UX Audit Report - Runie TUI Coding Agent

**Audit Date:** 2024-05-24  
**Auditor:** Ralph (Overnight Audit)  
**Status:** In Progress

---

## P0 Issues (Critical - Dead-ends)

These issues prevent users from making progress or force them to kill the process.

### P0-1: Permission Modal 5-Minute Timeout Has No UI Feedback

**File:** `crates/runie-agent/src/loop_engine.rs`  
**Lines:** ~300-330

**Problem:** When the agent requests permission, it waits up to 5 minutes for user response. If the user doesn't respond within this window:
- The permission silently times out
- The tool is marked as denied with no user notification
- The agent continues with a "denied" result

**Current Behavior:**
```rust
let decision = tokio::time::timeout(
    std::time::Duration::from_secs(300), // 5 minute timeout
    permission_rx.recv()
).await;
```

**Expected:** Visual countdown or "waiting for response" indicator, with clear timeout message.

**Fix Required:**
1. Add `permission_timeout` field to `PermissionModalState`
2. Send `PermissionTimeout` event to TUI after 5 minutes
3. Display timeout message in modal

**Severity:** High - Users may wait indefinitely thinking their response was captured.

---

### P0-2: Submit Blocked During Model Fetch Without Retry Path

**File:** `crates/runie-tui/src/tui/update/misc.rs`  
**Lines:** 45-52

**Problem:** If user hits Enter while models are being fetched in onboarding:
```rust
if let Some(ref onboarding) = state.onboarding {
    if onboarding.is_fetching_models {
        state.messages.push(MessageItem::System {
            text: "Still loading models... Please wait.".to_string(),
        });
        return vec![];  // Blocks submit, no retry mechanism
    }
}
```

**Current Behavior:** Shows "Still loading models..." but doesn't:
1. Indicate progress (spinner/percentage)
2. Provide a way to cancel and retry
3. Show estimated wait time

**Fix Required:**
1. Add progress indicator to onboarding state
2. Provide "Cancel and retry" action
3. Show model count being loaded

**Severity:** High - Blocks user input with no resolution path.

---

### P0-3: Onboarding API Key Validation Provides No Diagnostic

**File:** `crates/runie-tui/src/components/modal.rs`  
**Lines:** ~280-295

**Problem:** When API key validation fails:
```rust
OnboardingStep::KeyInput => {
    if self.validate_key() { OnboardingStep::ModelSelect }
    else { self.error_message = Some("Invalid API key format".to_string()); OnboardingStep::KeyInput }
}
```

**Current Behavior:** Shows "Invalid API key format" without explaining:
- What format was expected
- What was actually entered
- How to find/obtain the key

**Fix Required:**
1. Provide specific validation messages based on failure reason
2. Show expected prefix clearly
3. Link to provider documentation

**Severity:** High - New users cannot complete setup.

---

### P0-4: DiffViewer Has No Escape Path If 'q' Key Doesn't Work

**File:** `crates/runie-tui/src/tui/events.rs`  
**Lines:** ~140-150

**Problem:** DiffViewer only responds to 'q' or Escape:
```rust
fn key_to_diff_msg(key: crossterm::event::KeyEvent) -> Option<Msg> {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => Some(Msg::CloseModal),
        ...
    }
}
```

**If the terminal doesn't send 'q' correctly** (some terminals have issues), users are stuck.

**Fix Required:**
Add alternative close triggers:
- Ctrl+C → CloseModal
- Ctrl+Q → CloseModal (consistent with Chat mode)

**Severity:** Medium - Uncommon but blocking when it occurs.

---

## P1 Issues (Important - Invalid States)

These cause unexpected behavior but have recovery paths.

### P1-1: Error Messages May Contain Raw Stack Traces

**File:** `crates/runie-tui/src/tui/update/agent.rs`  
**Lines:** ~90-105

**Problem:** `on_agent_error()` accepts raw error strings:
```rust
pub fn on_agent_error(state: &mut AppState, message: String) {
    let recoverable = is_recoverable_error(&message);
    state.messages.push(MessageItem::Error { message, recoverable });
    ...
}
```

**Current Behavior:** If provider returns a JSON error with stack trace, it appears directly in chat.

**Fix Required:**
1. Truncate error messages > 500 chars
2. Detect and sanitize stack traces
3. Provide "Show details" expansion

**Severity:** Medium - Degrades UX, potential info leak.

---

### P1-2: Inconsistent Keybindings Between Modes

**File:** `crates/runie-tui/src/tui/events.rs`

| Mode | Quit/Close | Conflict |
|------|------------|----------|
| Chat | Ctrl+Q | Also mapped |
| Permission | None (blocks) | Blocks everything |
| DiffViewer | q, Esc | Different from Chat |
| CommandPalette | Esc | Consistent |
| SessionTree | Esc | Consistent |

**Problem:** Users must remember different keys per mode.

**Fix Required:**
- Standardize: Esc always closes modals
- Ctrl+Q only in Chat mode (quit)
- Add "Press ? for help" hint in status bar

**Severity:** Medium - Cognitive load, learnability issue.

---

### P1-3: Agent Panic During Tool Execution Leaves Workspace Inconsistent

**File:** `crates/runie-agent/src/loop_engine.rs`  
**Lines:** ~350-400

**Problem:** If tool execution panics (e.g., file deleted mid-edit):
```rust
let result = if let Some(tool) = registry.get(name) {
    match tool.execute(final_input.clone()).await {
        Ok(output) => ...
        Err(e) => ToolResult { is_error: true, ... }
    }
} else {
    ToolResult { is_error: true, ... }
};
```

**Current Behavior:** Tool errors are caught but partial state changes persist.

**Fix Required:**
1. Wrap tool execution in catch_unwind
2. Track tool state changes
3. Implement rollback on panic

**Severity:** Medium - Can corrupt workspace.

---

### P1-4: Double-Submit Prevention Silently Blocks

**File:** `crates/runie-tui/src/tui/update/misc.rs`  
**Lines:** 40-44

**Problem:**
```rust
if state.agent_running {
    return vec![];  // Silent block
}
```

**Current Behavior:** No feedback to user that submission was blocked.

**Fix Required:**
1. Push informational message explaining why
2. Suggest stopping current task first
3. Visual indicator that agent is running

**Severity:** Low - Confusing but recoverable.

---

### P1-5: Network Drops Mid-Stream Leave Partial Responses

**File:** `crates/runie-agent/src/loop_engine.rs`  
**Lines:** ~200-240

**Problem:** If network drops during streaming:
```rust
let mut stream = stream;
while let Some(event) = stream.next().await {
    match event {
        LlmEvent::MessageDelta { content } => {
            text_content.push_str(&content);
            // Partial content saved
        }
        ...
    }
}
```

**Current Behavior:** Partial response remains in chat, user must manually delete.

**Fix Required:**
1. Detect stream interruption
2. Show "Connection lost" message
3. Offer "Retry" action
4. Clear partial content on retry

**Severity:** Low - Annoying but recoverable.

---

### P1-6: Empty State - No Workspace Shows Blank Screen

**File:** `crates/runie-tui/src/components/message_list/render.rs`

**Problem:** When `messages.is_empty()` and `!agent_running`:
```rust
if vm.messages.is_empty() && !vm.agent_running {
    render::render_empty_state(area, buf, text_muted, text_dim, text_x);
}
```

**Current Behavior:** Shows placeholder but no primary CTA to:
- Create/open a workspace
- Load a previous session

**Fix Required:**
Add contextual empty states:
- "No workspace selected" → button to select
- "No messages" → "Start a conversation"
- "No tools available" → "Configure tools"

**Severity:** Low - Missing discoverability.

---

## P2 Issues (Improvements - Cognitive Load)

### P2-1: Status Bar Token Count Format Inconsistent

**File:** `crates/runie-tui/src/tui/render.rs`  
**Lines:** ~40-55

**Problem:** Mixed number formats:
```rust
if vm.session_token_usage.total_tokens > 0 {
    parts.push(Span::styled(format!("{} tokens", vm.session_token_usage.total_tokens), ...));
    if vm.session_token_usage.estimated_cost > 0.0 {
        parts.push(Span::styled(format!(" · ${:.4}", vm.session_token_usage.estimated_cost), ...));
    }
}
```

**Fix:** Use consistent formatting (commas for thousands, 2 decimals for cost).

---

### P2-2: Onboarding Provider List Has 20+ Items Without Grouping

**File:** `crates/runie-tui/src/components/modal.rs`  
**Lines:** ~210-250

**Problem:** Alphabetical list of 20+ providers is overwhelming.

**Fix Required:**
1. Group by category (OpenAI-compatible, Anthropic, Local, etc.)
2. Show "Popular" section at top
3. Search/filter with fuzzy matching

---

### P2-3: Permission Modal 4 Options Creates Hick's Law Overload

**File:** `crates/runie-tui/src/components/permission_modal.rs`  
**Lines:** ~130-145

**Problem:** 4 options presented simultaneously:
- Confirm (Y/Enter)
- Cancel (N/Esc)
- Always (A)
- Skip (S)

**Fix Required:**
Use progressive disclosure:
- Primary row: Confirm | Cancel
- Secondary row: "Advanced: [a] always allow, [s] skip"

---

## Summary Matrix

| ID | Category | Severity | File | Lines | Status |
|----|----------|----------|------|-------|--------|
| P0-1 | Dead-end | Critical | loop_engine.rs | 300-330 | Open |
| P0-2 | Dead-end | Critical | misc.rs | 45-52 | Open |
| P0-3 | Dead-end | Critical | modal.rs | 280-295 | Open |
| P0-4 | Dead-end | Medium | events.rs | 140-150 | Open |
| P1-1 | Invalid state | Medium | agent.rs | 90-105 | Open |
| P1-2 | Keybinding | Medium | events.rs | (various) | Open |
| P1-3 | Invalid state | Medium | loop_engine.rs | 350-400 | Open |
| P1-4 | UX | Low | misc.rs | 40-44 | Open |
| P1-5 | Invalid state | Low | loop_engine.rs | 200-240 | Open |
| P1-6 | Empty state | Low | message_list | (render) | Open |
| P2-1 | Format | Low | render.rs | 40-55 | Open |
| P2-2 | Cognitive | Low | modal.rs | 210-250 | Open |
| P2-3 | Cognitive | Low | permission_modal.rs | 130-145 | Open |
