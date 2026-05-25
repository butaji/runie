# Behavior Gaps - State Machine Analysis

**Audit Date:** 2024-05-24  
**Auditor:** Ralph (Overnight Audit)  
**Status:** Identified 8 gaps, test infrastructure created

---

## State Machine Overview

```
                    ┌─────────────────────────────────────────────────────────────┐
                    │                         TuiMode                            │
                    │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐           │
                    │  │  Chat   │─▶│Overlay  │ │ Select  │ │Permission│          │
                    │  └────┬────┘ └────┬────┘ └────┬────┘ └────┬────┘           │
                    │       │          │           │           │                 │
                    │       │          └───────────┴───────────┘                 │
                    │       │                       │                            │
                    │       │    ┌──────────────────┼──────────────────┐        │
                    │       │    │                  │                  │        │
                    │       ▼    ▼                  ▼                  ▼        │
                    │  ┌─────────────────┐  ┌─────────────────┐  ┌─────────┐ │
                    │  │CommandPalette   │  │ DiffViewer      │  │SessionTree│ │
                    │  └─────────────────┘  └─────────────────┘  └─────────┘ │
                    │                                                           │
                    │  ┌─────────────┐                                          │
                    │  │ Onboarding  │ (always accessible, never blocked)       │
                    │  └─────────────┘                                          │
                    └───────────────────────────────────────────────────────────┘
```

---

## Undefined Transitions Found

### BG-1: Chat → Permission (Implicit, undocumented)

**Trigger:** Agent requests tool permission during execution  
**Handler:** `crates/runie-tui/src/tui/update/agent.rs:on_permission_request()`  
**Problem:** No explicit state transition - mode changes silently

**Current Flow:**
```
Chat (agent_running=true)
  → AgentEvent::PermissionRequest
  → on_permission_request() sets mode = TuiMode::Permission
```

**Issue:** What if user is in DiffViewer when permission request arrives?

**Proposed Fix:** Add mode check and queue permission if in blocking mode

```rust
pub fn on_permission_request(state: &mut AppState, ...) {
    if matches!(state.mode, TuiMode::Overlay | TuiMode::DiffViewer) {
        // Queue permission for after current modal closes
        state.pending_permission = Some(PermissionRequest { ... });
    } else {
        state.mode = TuiMode::Permission;
    }
}
```

---

### BG-2: Any Mode → Chat on Agent Error (Implicit)

**File:** `crates/runie-tui/src/tui/update/agent.rs:on_agent_error()`  
**Lines:** 92-98

**Problem:** Mode silently changes to Chat on any agent error

```rust
pub fn on_agent_error(state: &mut AppState, message: String) {
    // ...
    if state.mode != TuiMode::Onboarding {
        state.mode = TuiMode::Chat;  // Implicit transition
    }
}
```

**Issue:** User loses context of what they were doing (viewing diff, session tree, etc.)

**Proposed Fix:** Add explicit transition handling with context preservation

```rust
enum ModeTransition {
    ToChat { reason: ChatTransitionReason },
    ToPermission { request: PendingPermission },
    // ...
}

enum ChatTransitionReason {
    AgentError,
    Cancel,
    AgentEnd,
}

pub fn on_agent_error(state: &mut AppState, message: String) {
    // Preserve scroll position and context
    let saved_scroll = state.scroll.clone();
    // ... handle error ...
    state.previous_mode = Some(state.mode.clone());
    state.mode = TuiMode::Chat;
    state.scroll = saved_scroll;
}
```

---

### BG-3: Permission → Chat on Tool Deny (No Rollback)

**File:** `crates/runie-agent/src/loop_engine.rs`  
**Lines:** ~280-310

**Problem:** When user denies permission, no cleanup of partial changes

```rust
Ok(Some(PermissionDecision::Deny { .. })) => {
    // Add fake error result but don't rollback any partial changes
    let result = ToolResult {
        content: vec![ContentPart::Text { 
            text: "Tool execution denied by user".to_string() 
        }],
        is_error: true,
        // No rollback mechanism
    };
}
```

**Issue:** If tool started making changes before permission check, those persist.

**Proposed Fix:** Implement transaction-style tool execution

```rust
// Before tool execution, create checkpoint
let checkpoint = tool.create_checkpoint()?;

// After execution, confirm or rollback
match permission_decision {
    PermissionDecision::Allow => checkpoint.commit(),
    PermissionDecision::Deny | PermissionDecision::Skip => checkpoint.rollback(),
}
```

---

### BG-4: Overlay Mode Has No Defined Close Triggers

**File:** `crates/runie-tui/src/tui/events.rs`  
**Lines:** ~125-135

**Problem:** Overlay mode accepts Escape but no other keys

```rust
fn key_to_overlay_msg(key: crossterm::event::KeyEvent) -> Option<Msg> {
    if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('q')) {
        return Some(Msg::CloseModal);  // Ctrl+Q works
    }
    match key.code {
        KeyCode::Esc => Some(Msg::CloseModal),
        _ => None,  // All other keys ignored!
    }
}
```

**Issue:** What if Escape doesn't work in some terminals?

**Proposed Fix:** Add additional close triggers

```rust
KeyCode::Char('q') | KeyCode::Char('Q') => Some(Msg::CloseModal),
KeyCode::Char('x') | KeyCode::Char('X') => Some(Msg::CloseModal),
```

---

### BG-5: Agent End While Permission Pending (Race Condition)

**File:** `crates/runie-agent/src/loop_engine.rs`  
**Lines:** ~430-440

**Problem:** If `AgentEnd` arrives while waiting for permission:

```rust
// Wait for permission decision
let decision = tokio::time::timeout(
    std::time::Duration::from_secs(300),
    permission_rx.recv()
).await;
```

**Issue:** If agent loop ends (e.g., error, max turns), permission channel may never respond.

**Proposed Fix:** Check agent status before blocking on permission

```rust
tokio::select! {
    decision = permission_rx.recv() => {
        // Handle decision
    }
    _ = agent_cancellation.notified() => {
        // Agent was cancelled, don't wait for permission
        return Err(AgentLoopError::Cancelled);
    }
    _ = tokio::time::sleep(Duration::from_secs(300)) => {
        // Timeout
        send_permission_denied(id.clone());
    }
}
```

---

### BG-6: Idempotency - Re-submitting Same Message

**File:** `crates/runie-tui/src/tui/update/misc.rs`  
**Lines:** ~35-55

**Problem:** If user presses Enter twice quickly:
1. First message recorded, agent spawned
2. Second submit blocked (agent_running=true)

**Issue:** What if both submits happen before agent_running is set?

**Proposed Fix:** Add submission deduplication

```rust
pub fn handle_submit(state: &mut AppState) -> Vec<Cmd> {
    let text = state.textarea.lines().join("\n");
    if text.is_empty() { return vec![]; }
    
    // Deduplicate: check if last message is identical
    if let Some(MessageItem::User { text: last, .. }) = state.messages.last() {
        if last == &text {
            state.textarea.select_all();
            state.textarea.delete_line_by_end();
            return vec![];  // Duplicate, ignore
        }
    }
    
    // Proceed with submission
}
```

---

### BG-7: Ctrl+C During Permission Wait

**File:** `crates/runie-tui/src/tui/events.rs`  
**Lines:** ~95-105

**Current Behavior:** Ctrl+C in Permission mode sends `PermissionCancel`

**Problem:** User may want to cancel the entire operation, not just this permission.

**Proposed Fix:** Add "Cancel All" option

```rust
fn key_to_permission_msg(key: crossterm::event::KeyEvent) -> Option<Msg> {
    // Ctrl+Shift+C = Cancel all (quit to chat)
    if key.modifiers.contains(KeyModifiers::CONTROL | KeyModifiers::SHIFT) 
        && matches!(key.code, KeyCode::Char('c')) {
        return Some(Msg::Stop);  // Cancel agent entirely
    }
    // Ctrl+C = Cancel this permission (current behavior)
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('c') | KeyCode::Char('q') => return Some(Msg::PermissionCancel),
            _ => {}
        }
    }
    // ...
}
```

---

### BG-8: State Not Preserved on Mode Switch

**File:** `crates/runie-tui/src/tui/state.rs`

**Problem:** Switching modes loses state:
- Scroll position in message list
- Selected item in command palette
- Diff viewer scroll position

**Proposed Fix:** Add state preservation

```rust
#[derive(Clone)]
pub struct AppState {
    // ... existing fields ...
    
    // Preserved state for each mode
    pub preserved_scroll: HashMap<TuiMode, ScrollState>,
    pub preserved_selection: HashMap<TuiMode, usize>,
}

pub fn switch_mode(state: &mut AppState, new_mode: TuiMode) {
    // Save current mode state
    state.preserved_scroll.insert(state.mode.clone(), state.scroll.clone());
    state.preserved_selection.insert(state.mode.clone(), state.command_palette.selected);
    
    // Restore new mode state
    if let Some(saved_scroll) = state.preserved_scroll.get(&new_mode) {
        state.scroll = saved_scroll.clone();
    }
    if let Some(saved_selection) = state.preserved_selection.get(&new_mode) {
        state.command_palette.selected = *saved_selection;
    }
    
    state.mode = new_mode;
}
```

---

## Validation Checklist

| Gap | Validation Test | Status |
|-----|-----------------|--------|
| BG-1 | Send permission request while in DiffViewer | Test needed |
| BG-2 | Trigger error while viewing session tree | Test needed |
| BG-3 | Deny permission after partial file edit | Test needed |
| BG-4 | Test Overlay close with non-Escape keys | Test needed |
| BG-5 | Send AgentEnd while waiting for permission | Test needed |
| BG-6 | Double-press Enter rapidly | Test needed |
| BG-7 | Ctrl+C during permission wait | Covered |
| BG-8 | Switch modes and return | Test needed |

---

## Proposed Test Cases

```rust
#[cfg(test)]
mod state_transition_tests {
    use super::*;
    
    #[test]
    fn test_permission_request_in_diff_viewer() {
        let mut state = AppState::default();
        state.mode = TuiMode::DiffViewer;
        
        // Simulate permission request
        on_permission_request(&mut state, "tool_call_1".into(), "bash".into(), "{}".into());
        
        // Should queue permission, not switch modes
        assert_eq!(state.mode, TuiMode::DiffViewer);
        assert!(state.pending_permission.is_some());
    }
    
    #[test]
    fn test_error_preserves_scroll() {
        let mut state = AppState::default();
        state.mode = TuiMode::SessionTree;
        state.scroll.feed_offset = 100;
        
        // Trigger error
        on_agent_error(&mut state, "Test error".into());
        
        // Should restore scroll after returning to chat
        assert_eq!(state.mode, TuiMode::Chat);
    }
    
    #[test]
    fn test_double_submit_deduplicated() {
        let mut state = AppState::default();
        state.textarea = TextArea::from("Hello");
        
        // First submit
        let cmds1 = handle_submit(&mut state);
        assert!(!cmds1.is_empty());
        
        // Immediately second submit (same text)
        let cmds2 = handle_submit(&mut state);
        assert!(cmds2.is_empty());  // Should be blocked
    }
}
```

---

## Recommended State Machine Diagram (Target)

```
                    ┌───────────────────────────────────────────────┐
                    │                   TuiMode                      │
                    │                                               │
┌─────────┐        │  ┌─────────┐  ┌──────────┐  ┌──────────────┐  │
│Onboarding│        │  │  Chat   │  │ Permission│  │ CommandPalette│  │
└────┬────┘        │  └───┬─────┘  └────┬─────┘  └──────┬───────┘  │
     │             │      │             │              │          │
     │             │      │             │              │          │
     │             │      ▼             ▼              ▼          │
     │             │  ┌──────────────────────────────────────┐    │
     │             │  │         Any Mode (Overlay)          │    │
     │             │  │   Permission requests queued here    │    │
     │             │  └──────────────────────────────────────┘    │
     │             │                                               │
     │             │  ┌──────────┐  ┌────────────┐  ┌──────────┐  │
     │             │  │DiffViewer│  │SessionTree │  │(future)  │  │
     │             │  └──────────┘  └────────────┘  └──────────┘  │
     │             │                                               │
     └─────────────┼───────────────────────────────────────────────┘
                   │
     ┌─────────────┴────────────┐
     │                         │
     ▼                         ▼
┌─────────────────┐     ┌─────────────────┐
│  State Preserved │     │ Transition Events │
│  - scroll        │     │ - on_error       │
│  - selection     │     │ - on_permission  │
│  - scroll_pos    │     │ - on_end         │
└─────────────────┘     └─────────────────┘
```
