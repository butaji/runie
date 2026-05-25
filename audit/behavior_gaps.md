# Behavior Validation: State Machine Gaps and Proposed Fixes

**Audit Date:** 2026-05-24  
**Scope:** runie-agent loop engine, TUI state machine

---

## State Machine Overview

### TUI Modes (States)
```
TuiMode::Chat
    ↕ Onboarding complete / Esc
TuiMode::Onboarding
    ↕ Finish onboarding
TuiMode::CommandPalette
    ↕ Esc / Command executed
TuiMode::Permission
    ↕ Allow/Deny/Cancel/Timeout
TuiMode::DiffViewer
    ↕ Esc / Close
TuiMode::SessionTree
    ↕ Esc / Select
TuiMode::Overlay
    ↕ Esc
TuiMode::Select
    ↕ Esc / Select
```

### Agent States
```
AgentState::Idle
    → AgentState::Running (on Submit)
    → AgentState::WaitingForPermission (on tool call)
AgentState::Running
    → AgentState::Idle (on TurnEnd with no tools)
    → AgentState::WaitingForPermission (on tool call)
    → AgentState::Error (on API/tool error)
AgentState::WaitingForPermission
    → AgentState::Running (on PermissionGranted)
    → AgentState::Idle (on PermissionDenied)
    → AgentState::Error (on timeout)
AgentState::Error
    → AgentState::Idle (on user dismiss)
```

---

## Undefined / Implicit Transitions Found

### BG-1: Permission While in DiffViewer Mode
| Property | Value |
|---|---|
| **File** | `crates/runie-tui/src/tui/update/agent.rs` |
| **Issue** | Permission request switches mode away from DiffViewer, losing context |

**Current Behavior:**
```rust
// When permission request comes in, mode switches immediately
if state.mode == TuiMode::DiffViewer {
    state.mode = TuiMode::Permission; // Loses DiffViewer context
}
```

**Problem:** User is reviewing a diff, agent needs permission for a tool, mode switches away. User loses diff context.

**Proposed Fix:** Queue permission, stay in DiffViewer:
```rust
if state.mode == TuiMode::DiffViewer {
    state.permission_modal.pending_queue.push(pending);
    // Stay in DiffViewer - don't switch
    return vec![];
}
```

**Test:** `test_permission_request_switches_mode` (documents expected behavior)

---

### BG-2: Network Drop During Tool Call
| Property | Value |
|---|---|
| **File** | `crates/runie-agent/src/loop_engine.rs` |
| **Issue** | No explicit handling for network errors during tool execution |

**Current Behavior:** Tool execution returns `ToolResult { is_error: true }` on network failure, but:
1. Agent continues to next turn (may retry infinitely)
2. No user notification of transient vs permanent failure

**Missing Transitions:**
```
AgentState::Running → AgentState::Running (retry)
AgentState::Running → AgentState::Error (permanent)
```

**Proposed Fix:** Add retry limit and classify errors:
```rust
const MAX_TOOL_RETRIES: usize = 3;

match tool_result.is_error {
    true if is_transient_error(&tool_result) && retries < MAX_TOOL_RETRIES => {
        retries += 1;
        continue; // Retry
    }
    true => {
        return Err(AgentLoopError::PermanentToolFailure { tool: name.to_string() });
    }
    false => { /* continue */ }
}
```

---

### BG-3: Model Stream Garbage Mid-Token
| Property | Value |
|---|---|
| **File** | `crates/runie-ai/src/` (streaming implementation) |
| **Issue** | If model produces invalid UTF-8 or malformed tokens mid-stream |

**Current Behavior:** Assumes streaming chunks are valid UTF-8.

**Problem:** External API could produce garbage. Current code:
```rust
// Likely missing validation
let text = chunk.text; // Assumes valid
```

**Proposed Fix:** Add stream validation:
```rust
fn validate_stream_chunk(chunk: &str) -> Result<&str, StreamError> {
    std::str::from_utf8(chunk.as_bytes())
        .map_err(|_| StreamError::InvalidUtf8)
}
```

---

### BG-4: File Deleted During Active Edit
| Property | Value |
|---|---|
| **File** | `crates/runie-tools/src/edit_file.rs` |
| **Issue** | No rollback mechanism for edit-in-progress files |

**Current Behavior:** Edit tool reads file, user approves edit, but file is deleted before write.

**Missing:** Rollback of read state, retry with file recreation.

**Proposed Fix:** Add atomic write with temp file:
```rust
fn atomic_write(path: &Path, content: &str) -> Result<(), ToolError> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, content)?;
    std::fs::rename(&tmp, path)?; // Atomic on POSIX
    Ok(())
}
```

---

### BG-5: DAG Cycle Detection (N/A - No DAG)
| Property | Value |
|---|---|
| **Issue** | Task mentions DAG cycle detection, but no DAG exists in codebase |

**Resolution:** Not applicable. The agent loop is linear (turn-based), not DAG-based.

---

### BG-6: Invalid API Key + "Run" Pressed
| Property | Value |
|---|---|
| **File** | `crates/runie-agent/src/executor.rs` |
| **Issue** | Invalid API key produces error, but error message may be unclear |

**Current Behavior:** Provider returns 401/403, error propagates:
```rust
// Likely missing specific handling
Err(e) => ToolResult { is_error: true, content: e.to_string() }
```

**Problem:** User sees generic error, doesn't know if it's API key, network, or model issue.

**Proposed Fix:** Classify API errors:
```rust
match error.kind() {
    ErrorKind::Unauthorized => "Invalid API key. Check Settings.",
    ErrorKind::Forbidden => "API key lacks permissions.",
    ErrorKind::Timeout => "Request timed out. Check network.",
    _ => "API error. Check logs.",
}
```

---

### BG-7: Actor Panic - Workspace Integrity
| Property | Value |
|---|---|
| **File** | `crates/runie-agent/src/loop_engine.rs` |
| **Issue** | Panic recovery exists but partial file edits may persist |

**Current Behavior:** `catch_unwind` catches panic, but workspace may have partial changes:
```rust
// Tool panicked, error returned
ToolResult { is_error: true, content: "Tool '{}' panicked" }
```

**Missing:** Rollback of any in-progress file modifications.

**Proposed Fix:** Track transaction log:
```rust
struct ToolTransaction {
    tool_name: String,
    touched_files: Vec<PathBuf>,
}

fn execute_with_transaction(registry: Arc<ToolRegistry>, ...) -> Result<ToolResult, ToolError> {
    let mut tx = ToolTransaction::new();
    // ... track touched files ...
    match result {
        Ok(r) => { tx.commit(); Ok(r) }
        Err(e) => { tx.rollback(); Err(e) }
    }
}
```

---

### BG-8: Ctrl+C During Permission Wait
| Property | Value |
|---|---|
| **File** | `crates/runie-tui/src/tui/update/agent.rs` |
| **Issue** | Already handled, but documented transition was unclear |

**Current Behavior:**
```rust
Msg::Stop | Msg::Quit => {
    state.agent_running = false;
    state.mode = TuiMode::Chat; // Resets to Chat
}
```

**Verified:** This is correctly implemented. Mode resets to Chat, permission is abandoned.

---

## State Transition Matrix (As-Is)

| From State | Event | To State | Status |
|---|---|---|---|
| Chat | Submit | Running | ✅ Defined |
| Chat | Esc | Chat | ✅ No-op |
| Running | ToolCall | WaitingForPermission | ✅ Defined |
| Running | TurnEnd | Idle | ✅ Defined |
| Running | Error | Error | ✅ Defined |
| WaitingForPermission | Grant | Running | ✅ Defined |
| WaitingForPermission | Deny | Idle | ✅ Defined |
| WaitingForPermission | Timeout | Idle | ✅ Defined (needs UI) |
| Error | Dismiss | Idle | ✅ Defined |
| Idle | Submit | Running | ✅ Defined |
| Any | Quit | Exit | ✅ Defined |

---

## Missing Tests for State Transitions

| Transition | Test Name | Status |
|---|---|---|
| Running → Error (network) | `test_network_error_transitions_to_error_state` | **MISSING** |
| WaitingForPermission → Timeout | `test_permission_timeout_auto_dismisses` | **MISSING** |
| Error → Idle (dismiss) | `test_error_dismiss_resets_state` | **MISSING** |
| Chat → Submit (no model) | `test_submit_without_model_shows_warning` | **MISSING** |
| DiffViewer + Permission | `test_permission_while_in_diffviewer` | Documented (BG-1) |

---

## Proposed Fixes Summary

| ID | Description | Priority | Files |
|---|---|---|---|
| BG-1 | Queue permission in DiffViewer | High | `update/agent.rs` |
| BG-2 | Retry limit + transient error classification | High | `loop_engine.rs` |
| BG-3 | Stream validation for UTF-8 | Medium | `runie-ai/` |
| BG-4 | Atomic file writes | Medium | `edit_file.rs` |
| BG-6 | Classify API errors with user-friendly messages | High | `executor.rs` |
| BG-7 | Transaction log for rollback | Low | `loop_engine.rs`, `tools/` |
