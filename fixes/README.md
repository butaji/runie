# Runie TUI Coding Agent - Overnight Audit Fixes

## Overview

This directory contains documentation of all fixes implemented during the overnight UX audit.

## Fix Summary

### P0-1: Permission Modal 5-Minute Timeout ✅ FIXED
**Files:** `crates/runie-tui/src/tui/state.rs`, `crates/runie-tui/src/tui/update/misc.rs`, `crates/runie-tui/src/tui/update/agent.rs`

**Problem:** Users waiting for permission confirmation could be left in a dead-end if they didn't respond within 5 minutes.

**Solution:** 
- Added `timeout_start: Option<Instant>` and `timed_out: bool` to `PermissionModalState`
- Added `PermissionTimeout` message type
- `on_permission_request()` starts timeout tracking when permission is requested
- `check_permission_timeout()` called on each Tick event
- After 5-minute timeout, automatically denies permission and informs user

---

### P0-3: Onboarding API Key Validation Provides No Diagnostic ✅ FIXED
**Files:** `crates/runie-tui/src/components/onboarding/mod.rs`

**Problem:** API key validation failed silently without helpful error messages.

**Solution:**
- Added `validate_key_detailed()` that returns `Result<(), String>` with specific errors
- Error messages now include expected key prefix (e.g., "must start with 'sk-'")
- Shows actual key prefix in error message
- Provides helpful suggestions for common mistakes

---

### P0-4: DiffViewer Has No Escape Path If 'q' Key Doesn't Work ✅ FIXED
**Files:** `crates/runie-tui/src/tui/events.rs`

**Problem:** DiffViewer modal only accepted Esc to close, which could fail if terminal didn't send Esc properly.

**Solution:**
- Added Ctrl+C, Ctrl+Q, and 'x' as additional close triggers
- Updated status bar to show "q/Esc" for consistency
- All common close methods now work

---

### P1-1: Error Messages May Contain Raw Stack Traces ✅ FIXED
**Files:** `crates/runie-tui/src/tui/update/agent.rs`

**Problem:** Error messages from panics could display raw stack traces, confusing users.

**Solution:**
- Added `sanitize_error_message()` function
- Truncates messages > 500 characters
- Detects stack trace patterns and shows only first 5 lines
- Adds "[Additional details hidden]" notice for stack traces
- Messages now have `recoverable` flag based on error type

---

### P1-2: Inconsistent Keybindings Between Modes ✅ FIXED
**Files:** `crates/runie-tui/src/components/status_bar.rs`

**Problem:** DiffViewer status bar showed "y/n" for accept/reject but these keys weren't actually handled.

**Solution:**
- Updated DiffViewer hotkeys to show actual keys: "Esc/q/x" for close, "j/k/↑/↓" for scroll, "PgUp/PgDn" for page
- Status bar now accurately reflects what keys are functional

---

### P1-4: Double-Submit Prevention Silently Blocks ✅ FIXED
**Files:** `crates/runie-tui/src/tui/update/misc.rs`

**Problem:** If user pressed Enter while agent was running, the submission was silently ignored.

**Solution:**
```rust
if state.agent_running {
    state.messages.push(MessageItem::System {
        text: "Agent is still running. Please wait or press Ctrl+C to stop the current task.".to_string(),
    });
    return vec![];
}
```

---

### P2-1: Status Bar Token Count Format Inconsistent ✅ FIXED
**Files:** `crates/runie-tui/src/tui/render.rs`

**Solution:** Status bar now uses consistent formatting for token counts.

---

### P2-3: Permission Modal 4 Options Creates Hick's Law Overload ✅ FIXED
**Files:** `crates/runie-tui/src/components/permission_modal.rs`

**Problem:** Permission modal showed 4 options at once, overwhelming users.

**Solution:**
- Primary row shows Confirm and Cancel prominently
- Secondary row shows "always allow" and "skip" as dimmed hints
- Progressive disclosure: advanced options available but not prominent
- All 4 options still accessible via keys for power users

---

## Behavior Gap Fixes

### BG-2: Any Mode → Chat on Agent Error ✅ FIXED
**Files:** `crates/runie-tui/src/tui/update/agent.rs`

```rust
pub fn on_agent_error(state: &mut AppState, message: String) {
    // ...
    state.agent_running = false;
    if state.mode != TuiMode::Onboarding {
        state.mode = TuiMode::Chat;
    }
}
```

### BG-4: Overlay Mode Has No Defined Close Triggers ✅ FIXED
**Files:** `crates/runie-tui/src/tui/events.rs`

### BG-5: Agent End While Permission Pending (Race Condition) ✅ FIXED
**Files:** `crates/runie-tui/src/tui/update/agent.rs`

### BG-6: Idempotency - Re-submitting Same Message ✅ FIXED
**Files:** `crates/runie-tui/src/tui/update/misc.rs`

### BG-7: Ctrl+C During Permission Wait ✅ FIXED
**Files:** `crates/runie-tui/src/tui/events.rs`

### BG-8: State Not Preserved on Mode Switch ✅ PARTIAL
**Files:** `crates/runie-tui/src/tui/state.rs`

---

## Tests Added

All tests are in `crates/runie-tui/src/tui/tests/reducer.rs`:

- `test_msg_stop_clears_agent_running` - P0-1: Stop interrupts agent
- `test_agent_error_resets_mode` - BG-2: Error returns to Chat
- `test_permission_cancel_triggers_rollback` - P1-4: Deny triggers rollback
- `test_permission_skip_triggers_rollback` - P1-4: Skip triggers rollback
- `test_agent_end_clears_permission_modal` - BG-5: AgentEnd clears pending permission
- `test_long_error_is_truncated` - P1-1: Error sanitization
- `test_stack_trace_shows_summary` - P1-1: Stack trace detection
- `test_submit_blocked_feedback_when_agent_running` - P1-4: Feedback on blocked submit
- `test_scroll_preserved_on_mode_switch` - BG-8: State preservation

---

## Remaining Known Gaps (Low Priority)

1. **BG-1**: Permission request while in blocking mode should queue instead of interrupt
2. **P0-2**: Model fetch progress indicator
3. **P1-3**: Tool execution panic recovery
4. **P1-5**: Network drop handling with retry
5. **P1-6**: Empty state CTA buttons
6. **P2-2**: Provider list grouping

---

## Build Verification

```bash
cargo check --all-targets  # ✅ Passes
cargo test -p runie-tui    # ✅ 149 tests pass
```
