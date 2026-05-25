# Behavior Audit - Undefined State Transitions

## Overview

This document identifies undefined or implicit state transitions in the agent loop and TUI state machine. Each undefined transition is a potential bug.

---

## Agent Loop State Machine

### States
```rust
enum AgentState {
    Idle,           // No active agent
    Running,        // Agent streaming
    WaitingPermission, // Blocked on permission
    ToolExecuting,  // Tool running
    Error,          // Terminal error
    Completed,      // Turn done
}
```

### Current Transition Map

| From | Event | To | Defined? |
|------|-------|-----|----------|
| Idle | SpawnAgent | Running | ✅ |
| Running | MessageChunk | Running | ✅ |
| Running | ToolCall | WaitingPermission | ✅ |
| WaitingPermission | Allow | ToolExecuting | ✅ |
| WaitingPermission | Deny | Completed | ✅ |
| WaitingPermission | Timeout | Completed | ✅ |
| ToolExecuting | ToolResult | Running | ✅ |
| Running | Error | Error | ✅ |
| Running | MessageEnd | Completed | ✅ |
| **Any** | **Ctrl+C** | **Idle** | ❌ Undefined |

### Gap 1: Cancellation Not Explicit

**Location:** `crates/runie-agent/src/loop_engine.rs`

**Issue:** `Cmd::Interrupt` aborts the task but doesn't define what state the agent should be in afterward:

```rust
Cmd::Interrupt => {
    if let Some(handle) = agent_task.take() {
        handle.abort();  // ← Task killed, state undefined
    }
    vec![]
}
```

**Problem:** 
- `agent_running` in TUI is set to false
- But agent loop may have been mid-stream
- No cleanup of partial tool executions
- Workspace may have partial edits

**Proposed Fix:**
```rust
Cmd::Interrupt => {
    // Signal graceful stop
    stop_token.cancel();
    
    // Wait briefly for cleanup
    if let Some(handle) = agent_task.take() {
        match tokio::time::timeout(Duration::from_secs(2), handle).await {
            Ok(_) => {} // Clean shutdown
            Err(_) => handle.abort(), // Force kill if slow
        }
    }
    
    // Reset state
    state.agent_running = false;
    state.mode = TuiMode::Chat;
    
    vec![]
}
```

---

### Gap 2: Panic Recovery Not Defined

**Location:** `crates/runie-agent/src/loop_engine.rs:195-215`

**Issue:** When a tool panics, rollback happens but state transitions are unclear:

```rust
if let Err(panic_info) = result {
    // Rollback partial changes
    workspace.rollback()?;
    
    return ToolResult {
        // ... error result
        is_error: true,
    };
}
```

**Problem:**
- No `Panic` event to TUI
- No transition to `Error` state
- Agent continues waiting for result

**Proposed Fix:**
```rust
if let Err(panic_info) = result {
    let panic_msg = panic_info.to_string();
    
    // Send panic event to TUI
    event_tx.send(AgentEvent::Panic { 
        tool_name: name.clone(),
        message: panic_msg.clone(),
    }).await?;
    
    // Rollback workspace
    workspace.rollback()?;
    
    // Return error result - agent loop will continue
    return ToolResult {
        is_error: true,
        // ...
    };
}
```

---

### Gap 3: Network Error Mid-Stream

**Location:** `crates/runie-agent/src/rig_loop.rs:75-85`

**Issue:** Stream errors are logged but not explicitly handled:

```rust
while let Some(chunk) = stream.next().await {
    match chunk {
        Ok(StreamedAssistantContent::Text(text)) => {
            // Process
        }
        Err(e) => {
            tracing::error!("Stream error: {}", e);
            // ← Falls through, continues loop
        }
    }
}
```

**Problem:**
- Loop continues with empty/incomplete message
- No error event sent to TUI
- User sees partial response

**Proposed Fix:**
```rust
Err(e) => {
    tracing::error!("Stream error: {}", e);
    
    event_tx.send(AgentEvent::Error { 
        message: format!("Stream interrupted: {}", e),
    }).await?;
    
    // Send partial message if any
    if !text_content.is_empty() {
        assistant_message.content = vec![ContentPart::Text { text: text_content }];
        event_tx.send(AgentEvent::MessageEnd { 
            message: assistant_message.clone(),
        }).await?;
    }
    
    break;  // Exit stream loop
}
```

---

## TUI State Machine

### Modes (TuiMode)
```rust
enum TuiMode {
    Chat,           // Main input mode
    Overlay,        // Modal overlay
    Select,         // Text selection
    Permission,     // Permission dialog
    CommandPalette, // Command search
    DiffViewer,     // Git diff view
    SessionTree,    // Session browser
    Onboarding,     // First-run setup
}
```

### Current Transition Map

| From | Event | To | Defined? |
|------|-------|-----|----------|
| Onboarding | Complete | Chat | ✅ |
| Onboarding | Skip | Chat | ✅ |
| Chat | PermissionRequest | Permission | ✅ |
| Permission | Allow/Deny | Chat | ✅ |
| Permission | Timeout | Chat | ✅ |
| Chat | Ctrl+K | CommandPalette | ✅ |
| CommandPalette | Esc/Enter | Chat | ✅ |
| Chat | ^Q | (quit) | ✅ |
| **Chat** | **Error event** | **???** | ❌ Undefined |

### Gap 4: Error State in TUI Not Defined

**Location:** `crates/runie-tui/src/tui/update/agent.rs`

**Issue:** `AgentEvent::Error` is handled but doesn't change mode:

```rust
pub fn handle_agent_event(state: &mut AppState, event: AgentEvent) {
    match event {
        // ...
        AgentEvent::Error { message } => on_agent_error(state, message),
        // ...
    }
}

pub fn on_agent_error(state: &mut AppState, message: String) {
    state.agent_running = false;
    state.messages.push(MessageItem::System {
        text: format!("Error: {}", message),
        // No mode change, no recovery action
    });
}
```

**Problem:**
- Error message added to chat but no visual emphasis
- No "recover" action offered
- User may not notice error

**Proposed Fix:**
```rust
pub fn on_agent_error(state: &mut AppState, message: String) {
    state.agent_running = false;
    
    // Show error prominently
    state.messages.push(MessageItem::Error {
        text: message.clone(),
        recoverable: true,  // Can retry
    });
    
    // Update status bar
    state.top_bar.status = Some("Error".to_string());
    
    // Offer recovery
    state.messages.push(MessageItem::System {
        text: "Press Enter to retry or type a new message".to_string(),
    });
}
```

---

### Gap 5: Idempotency of Submit

**Location:** `crates/runie-tui/src/tui/update/misc.rs:18-50`

**Issue:** Submit can be called multiple times in quick succession:

```rust
if state.agent_running {
    // Blocked, but message added
}
```

**Problem:**
- Multiple "Agent is still running" messages if clicked fast
- Race condition between `agent_running` check and state update

**Proposed Fix:**
```rust
// Atomic check-and-set
let expected = false;
if state.agent_running.compare_exchange(expected, true, ...).is_err() {
    // Already running, show feedback
    state.input_right_info = "Agent running...".to_string();
    return vec![];
}
```

---

### Gap 6: Workspace Concurrency Not Handled

**Location:** `crates/runie-tools/src/workspace.rs`

**Issue:** Multiple tools may edit same file simultaneously:

```rust
pub struct Workspace {
    root: PathBuf,
    // No file locking
}
```

**Problem:**
- Tool A reads file, Tool B writes file
- Tool A writes based on stale data
- Lost update

**Proposed Fix:**
```rust
pub struct Workspace {
    root: PathBuf,
    file_locks: Arc<Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>>,
}

impl Workspace {
    pub async fn with_lock<F, R>(&self, path: &Path, f: F) -> Result<R, ToolError>
    where F: FnOnce() -> R {
        let lock = self.file_locks.entry(path.to_path_buf())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone();
        let _guard = lock.lock().await;
        f().await
    }
}
```

---

## Tool Execution Idempotency

### Gap 7: Same Tool Called Twice

**Location:** `crates/runie-agent/src/loop_engine.rs`

**Issue:** If agent sends same tool call twice (e.g., after error recovery), it's executed twice:

```rust
for (tool_use, _tool_name, _args_str) in pending_tool_calls {
    // Each call executed independently
    execute_tool(tool_use, ...).await;
}
```

**Problem:**
- `grep` run twice with same args
- `write_file` creates duplicate content

**Proposed Fix:**
```rust
let mut seen: HashSet<String> = HashSet::new();

for (tool_use, ...) in pending_tool_calls {
    let call_key = format!("{}_{}", name, args_str);
    
    if seen.contains(&call_key) {
        tracing::warn!("Duplicate tool call skipped: {}", name);
        continue;
    }
    seen.insert(call_key);
    
    execute_tool(tool_use, ...).await;
}
```

---

## Summary of Proposed Fixes

| Gap | Description | Proposed Fix | Priority |
|-----|-------------|--------------|----------|
| 1 | Cancellation undefined | Explicit shutdown sequence | P0 |
| 2 | Panic not explicit | Add Panic event | P0 |
| 3 | Stream error handling | Error event + partial send | P1 |
| 4 | TUI error state | Recovery actions | P1 |
| 5 | Submit idempotency | Atomic check | P2 |
| 6 | Workspace concurrency | File locking | P1 |
| 7 | Tool call deduplication | HashSet dedup | P2 |

---

## Test Coverage Needed

1. **Cancellation test:** Spawn agent, interrupt mid-turn, verify clean state
2. **Panic test:** Tool panics, verify error handling
3. **Stream error test:** Simulate network drop, verify partial + error
4. **Idempotency test:** Submit twice, verify single execution
5. **Concurrency test:** Two agents edit same file, verify lock
