# UX Audit: Dead-Ends, Invalid States, and Cognitive Load

**Audit Date:** 2026-05-24  
**Auditor:** ralph/overnight-audit branch  
**Scope:** runie-tui crate, runie-agent crate

---

## P0 Issues (Critical - Breaks Flow or Leaves User Stuck)

### P0-1: **Message List Has No Empty State Placeholder** ✓ FIXED
| Property | Value |
|---|---|
| **File** | `crates/runie-tui/src/components/message_list/render.rs` |
| **Severity** | P0 |
| **Category** | Empty State |
| **Status** | ✅ FIXED - commit 2d9866a |

**Problem:** When `messages` is empty, the `MessageList` renders nothing. Users see a blank screen with no guidance on what to do.

**Fix Applied:** Empty state rendering added at line 55-57 of `message_list.rs`:
```rust
// Empty state: no messages and no active agent
if vm.messages.is_empty() && !vm.agent_running {
    render::render_empty_state(area, buf, text_muted, text_dim, text_x);
}
```

The `render_empty_state` function shows:
- Title: "runie"
- Tagline: "Your coding companion"
- CTA: "Type a message and press Enter to start"
- Hints: "Press ^k for commands · ^b for sidebar · ^q to quit"

**Test Reference:** `harness/tasks/empty_state/` - PASSES (4/4 checks)

---

### P0-2: **No Warning When Model Not Configured** ✓ FIXED
| Property | Value |
|---|---|
| **File** | `crates/runie-tui/src/tui/render.rs` |
| **Severity** | P0 |
| **Category** | Invalid State |
| **Status** | ✅ FIXED - commit 2d9866a |

**Problem:** Status bar shows `current_model: None` silently. User can press Enter to submit, but the agent won't run without a model.

**Fix Applied:** Added warning in `build_center_line`:
```rust
// P0-2 FIX: Show warning when no model is configured
if vm.current_model.is_none() {
    parts.push(Span::styled("⚠ No model configured", Style::default().fg(warning)));
} else if let Some(model) = vm.current_model.as_deref() {
    parts.push(Span::styled(model, Style::default().fg(text_tertiary)));
    parts.push(Span::styled(" · ", Style::default().fg(text_tertiary)));
}
```

**Test Reference:** `harness/tasks/no_model_warning/` - PASSES (4/4 checks)

---

### P0-3: **Submit with Empty Input Shows No Feedback** ✓ FIXED
| Property | Value |
|---|---|
| **File** | `crates/runie-tui/src/tui/update/misc.rs` |
| **Severity** | P0 |
| **Category** | Cognitive Load |
| **Status** | ✅ FIXED - commit 2d9866a |

**Problem:** Pressing Enter with empty input produces no visible feedback. User might think their message was sent.

**Fix Applied:** Added feedback message:
```rust
fn handle_submit(state: &mut AppState) -> Vec<Cmd> {
    let text = state.textarea.lines().join("\n");
    if text.is_empty() {
        // P0-3 FIX: Show feedback when submitting empty input
        state.input_right_info = "Type a message first".to_string();
        return vec![];
    }
    // ...
}
```

**Test Reference:** `harness/tasks/idle_submit_feedback/` - PASSES (4/4 checks)

---

## P1 Issues (User Experience Degradation)

### P1-1: **Command Palette in Argument Mode Has No Escape** ✓ FIXED
| Property | Value |
|---|---|
| **File** | `crates/runie-tui/src/components/command_palette/mod.rs` |
| **Severity** | P1 |
| **Category** | Dead-End |
| **Status** | ✅ FIXED

**Problem:** When user is typing arguments for a palette command (e.g., file path), pressing Esc does not cancel argument input and return to command selection.

**Fix Applied:**
1. Added `Msg::CommandPaletteCancelArgument` message in `state.rs`
2. Modified `key_to_palette_msg` in `events.rs` to send `CommandPaletteCancelArgument` on Esc
3. Added `handle_palette_escape` in `update/palette.rs` that:
   - If in argument mode: cancels argument mode, clears input, returns to command selection
   - If not in argument mode: closes the palette
4. Made `pending_command` field public for access from update module

---

### P1-2: **Permission Modal Has No Timeout Auto-Dismiss** ✓ FIXED
| Property | Value |
|---|---|
| **File** | `crates/runie-tui/src/tui/update/agent.rs` |
| **Severity** | P1 |
| **Category** | Invalid State |
| **Status** | ✅ FIXED

**Problem:** Permission modal stays open indefinitely. If user walks away, agent hangs forever.

**Fix Applied:**
1. `check_permission_timeout` in `misc.rs` returns `Some(Msg::PermissionTimeout)` after 5 minutes
2. `handle_permission_timeout` in `agent.rs` handles the timeout by:
   - Showing "Permission request timed out after 5 minutes. Request denied." message
   - Clearing the permission modal
   - Returning to Chat mode (or showing next pending permission)
3. `handle_tick_permission_check` calls `check_permission_timeout` on every tick

---

### P1-3: **Onboarding KeyInput Step Has No Back/Escape** ✓ VERIFIED WORKING
| Property | Value |
|---|---|
| **File** | `crates/runie-tui/src/components/onboarding/mod.rs` |
| **Severity** | P1 |
| **Category** | Dead-End |
| **Status** | ✅ ALREADY FIXED

**Problem:** During API key input in onboarding, there's no way to go back and select a different provider.

**Verified Working:**
- `key_to_onboarding_msg` sends `OnboardingBack` on Esc key
- `handle_onboarding_back` calls `prev_step()`
- `prev_step()` transitions from `KeyInput` → `ProviderSelect`
- User can press Esc to go back and select a different provider

---

### P1-4: **DiffViewer Has No Keyboard Shortcut Hints in Status Bar**
| Property | Value |
|---|---|
| **File** | `crates/runie-tui/src/components/status_bar.rs` |
| **Severity** | P1 |
| **Category** | Cognitive Load |

**Problem:** DiffViewer shows `j/k` for scroll but user might not know PgUp/PgDn are available.

**Current:**
```rust
TuiMode::DiffViewer => vec![
    StatusItem { key: "Esc/q/x".to_string(), description: "close".to_string() },
    StatusItem { key: "j/k/↑/↓".to_string(), description: "scroll".to_string() },
    StatusItem { key: "PgUp/PgDn".to_string(), description: "page".to_string() },
],
```

**Status bar is correct** - this is actually fine. Marking as resolved.

---

## P2 Issues (Polish and Progressive Disclosure)

### P2-1: **Error Messages Show as Raw Text, Not Banners**
| Property | Value |
|---|---|
| **File** | `crates/runie-tui/src/components/message_helpers.rs` |
| **Severity** | P2 |
| **Category** | Error Presentation |

**Problem:** Agent errors are rendered as plain text in the message stream. No visual distinction from normal messages.

**Current:** `AgentEvent::Error` creates a `MessageItem::Error` but renders like regular text.

**Fix:** Add distinct styling for error messages:
```rust
// In render.rs error message handling
let error_style = Style::default()
    .fg(colors.error)
    .bg(colors.error_bg)
    .add_modifier(Modifier::CROSSED_OUT);
```

---

### P2-2: **Agent Running Indicator Only in Status Bar**
| Property | Value |
|---|---|
| **File** | `crates/runie-tui/src/components/message_list/render.rs` |
| **Severity** | P2 |
| **Category** | Cognitive Load |

**Problem:** When agent is running, user might not notice until they try to type.

**Current:** `agent_running: true` only shows spinner in status bar.

**Fix:** Add subtle pulsing indicator in message list when waiting for agent response.

---

### P2-3: **Session Tree Navigation Has No Breadcrumb**
| Property | Value |
|---|---|
| **File** | `crates/runie-tui/src/components/session_tree.rs` |
| **Severity** | P2 |
| **Category** | Cognitive Load |

**Problem:** In deep session trees, user loses context of where they are.

**Fix:** Add breadcrumb trail in SessionTree navigator showing current path.

---

## Summary Table

| ID | Category | File | Severity | Status |
|---|---|---|---|---|
| P0-1 | Empty State | message_list/render.rs | Critical | ✅ FIXED |
| P0-2 | Invalid State | render.rs | Critical | ✅ FIXED |
| P0-3 | Cognitive Load | update/misc.rs | Critical | ✅ FIXED |
| P1-1 | Dead-End | command_palette/mod.rs | High | ✅ FIXED |
| P1-2 | Invalid State | permission_modal.rs | High | ✅ FIXED |
| P1-3 | Dead-End | onboarding/mod.rs | High | ✅ VERIFIED |
| P2-1 | Error Presentation | message_helpers.rs | Medium | Nice to have |
| P2-2 | Cognitive Load | message_list/render.rs | Medium | Nice to have |
| P2-3 | Cognitive Load | session_tree.rs | Medium | Nice to have |

---

## Verified Working (No Action Needed)

| Check | Status | Notes |
|---|---|---|
| Permission modal has Esc/Cancel | ✅ | `key_to_permission_msg` handles Esc, n, Ctrl+C |
| Command palette has Esc close | ✅ | `key_to_palette_msg` handles Esc |
| DiffViewer has q/Esc close | ✅ | `key_to_diff_msg` handles q and Esc |
| SessionTree has Esc close | ✅ | `key_to_tree_msg` handles Esc |
| Quit via Ctrl+Q works | ✅ | `key_to_chat_msg` handles Ctrl+Q |
| Panic recovery exists | ✅ | `execute_tool_with_panic_catch` wraps tools |
| Permission rollback on cancel | ✅ | `handle_permission_msg` sends Rollback cmd |
| Agent error resets mode to Chat | ✅ | `handle_agent_event` sets mode = Chat on error |
| Onboarding has Esc/skip | ✅ | `key_to_onboarding_msg` handles Esc |
