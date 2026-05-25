# Behavior Gaps — State Machine Audit

**Date:** 2026-05-24
**Scope:** Agent loop + TUI state transitions in `crates/runie-tui/` and `crates/runie-agent/`

---

## 1. Undefined / Implicit Transitions

### BG-1: Onboarding → Chat transition has no explicit commit
**From:** `OnboardingStep::Complete` (user selects "no, finish")
**To:** `TuiMode::Chat`
**Gap:** There is no explicit transition function. The flow relies on `Onboarding::is_complete()` returning true, but the TUI never checks this — it stays in `TuiMode::Onboarding` forever.

**Evidence:** `crates/runie-tui/src/tui/events.rs` — `OnboardingSkip` and `OnboardingSubmit` both call `route_onboarding` → `handle_onboarding_msg`, but none of these call `tui.update(Msg::CloseModal)` or set `state.mode = TuiMode::Chat`.

**Proposed fix:**
```rust
// In onboarding handler, when step == Complete and user confirms "finish":
pub fn handle_onboarding_msg(state: &mut AppState, msg: Msg) -> Vec<Cmd> {
    match msg {
        Msg::OnboardingSubmit => {
            if state.onboarding.as_ref().map(|o| o.is_complete()).unwrap_or(false) {
                state.mode = TuiMode::Chat;
                state.onboarding = None;
                // Return Cmd::SaveSettings if configured
            }
            vec![]
        }
        ...
    }
}
```

---

### BG-2: `AgentEvent::Error` doesn't transition to a recovery state
**From:** Any agent state (streaming, tool execution, etc.)
**To:** ???
**Gap:** `on_agent_error` pushes an `Error` message and sets `agent_running = false`, but the TUI remains in its current mode. If the error occurred during `Permission` mode, the mode is NOT reset to `Chat`.

**Evidence:** `crates/runie-tui/src/tui/update/agent.rs:85–97`
```rust
pub fn on_agent_error(state: &mut AppState, message: String) {
    let recoverable = is_recoverable_error(&message);
    state.messages.push(MessageItem::Error { message, recoverable });
    state.agent_running = false;
    // NOTE: state.mode is NOT reset!
    // If mode was TuiMode::Permission, user is now stuck
}
```

**Proposed fix:**
```rust
pub fn on_agent_error(state: &mut AppState, message: String) {
    let recoverable = is_recoverable_error(&message);
    state.messages.push(MessageItem::Error { message, recoverable });
    state.agent_running = false;
    // Always reset to Chat on agent error (unless in Onboarding)
    if state.mode != TuiMode::Onboarding {
        state.mode = TuiMode::Chat;
    }
}
```

---

### BG-3: `AgentEvent::PermissionRequest` assumes success path
**From:** Agent running, needs permission
**To:** `TuiMode::Permission`
**Gap:** If the agent sends `PermissionRequest` while already in `TuiMode::Permission` (nested permissions), the second request overwrites `permission_modal` state without queuing.

**Evidence:** `crates/runie-tui/src/tui/update/agent.rs:106–113`
```rust
pub fn on_permission_request(...) {
    state.permission_modal.tool = Some(tool_name.clone());
    state.permission_modal.tool_call_id = Some(tool_call_id);
    state.mode = TuiMode::Permission;
    // No queue — previous permission request is lost
}
```

**Proposed fix:** Add a `permission_queue: Vec<PermissionModalState>` field to `AppState`. Push incoming requests onto the queue instead of overwriting.

---

### BG-4: `AgentEvent::ToolExecutionEnd` assumes matching `ToolExecutionStart`
**From:** `AgentEvent::ToolExecutionStart`
**To:** `AgentEvent::ToolExecutionEnd`
**Gap:** If `ToolExecutionEnd` arrives without a preceding `ToolExecutionStart` (e.g., error during tool init), the `on_tool_end` tries to update `state.messages.last_mut()` which may be a `User` or `Assistant` message.

**Evidence:** `crates/runie-tui/src/tui/update/agent.rs:73–78`
```rust
pub fn on_tool_end(state: &mut AppState, tool_result: ...) {
    if let Some(MessageItem::ToolCall { ref mut result, .. }) = state.messages.last_mut() {
        *result = Some(text);
        *is_error = tool_result.is_error;
    }
    // No guard — if last message is not ToolCall, this is a no-op
    // and the tool result is silently dropped
}
```

**Proposed fix:**
```rust
pub fn on_tool_end(state: &mut AppState, tool_result: ...) {
    if let Some(MessageItem::ToolCall { ref mut result, ref mut is_error, .. }) = state.messages.last_mut() {
        *result = Some(text);
        *is_error = tool_result.is_error;
    } else {
        // Log to error log for debugging, but don't crash
        tracing::warn!("ToolExecutionEnd with no matching ToolExecutionStart");
    }
}
```

---

### BG-5: `AgentEvent::AgentEnd` doesn't clear pending permissions
**From:** Agent running → agent finishes
**To:** `TuiMode::Chat`
**Gap:** If `AgentEnd` arrives while a permission modal is open (agent ended without waiting for response), the modal state persists.

**Evidence:** `crates/runie-tui/src/tui/update/agent.rs:80–83`
```rust
pub fn on_agent_end(state: &mut AppState) {
    state.agent_running = false;
    // NOTE: permission_modal is NOT cleared
    // NOTE: mode is NOT reset to Chat
}
```

**Proposed fix:**
```rust
pub fn on_agent_end(state: &mut AppState) {
    state.agent_running = false;
    if state.mode == TuiMode::Permission {
        state.permission_modal.tool = None;
        state.mode = TuiMode::Chat;
    }
}
```

---

### BG-6: `AgentEvent::MessageStart` / `MessageUpdate` / `MessageEnd` race
**From:** Model streaming tokens
**Gap:** The agent can send `MessageStart`, then `MessageUpdate` (partial content), then `MessageEnd`. If `MessageEnd` arrives before `MessageStart` (reordering due to async channel), `on_message_end` panics or silently fails.

**Evidence:** `crates/runie-tui/src/tui/update/agent.rs:47–51`
```rust
pub fn on_message_end(state: &mut AppState, message: ...) {
    update_last_assistant(state, &message.content);
    // update_last_assistant:
    // if let Some(MessageItem::Assistant { .. }) = state.messages.last_mut() { ... }
    // If last is not Assistant, silently does nothing
}
```

**Proposed fix:** Add a `current_streaming_message: Option<String>` field to track in-progress messages, with sequence number validation.

---

## 2. Idempotency Issues

### BG-7: Double-submit is partially blocked but race exists
**File:** `crates/runie-tui/src/tui/update/misc.rs:29–48`
**Gap:** `handle_submit` checks `agent_running` before spawning, but this check happens in the synchronous update path. Between the check and the `Cmd::SpawnAgent` dispatch, another message could re-trigger submit.

**Evidence:** The check is in `update()` which is synchronous, but `cmds` are executed asynchronously by the caller. Between `update()` returning `Cmd::SpawnAgent` and the agent actually starting (`on_message_start`), `agent_running` is still `false`.

**Proposed fix:** Set `agent_running = true` immediately before returning `Cmd::SpawnAgent`:
```rust
Msg::Submit => {
    if text.is_empty() { return vec![]; }
    if state.agent_running { return vec![]; }
    // Set flag immediately to block re-submit
    state.agent_running = true;
    // ... push User message
    vec![Cmd::SpawnAgent { messages: ... }]
}
```

---

### BG-8: Multiple `Tick` events can stack
**File:** `crates/runie-tui/src/tui/tui.rs`
**Gap:** The animation `Tick` is dispatched via a timer. If the timer fires faster than frames are rendered, ticks stack up and the animation jumps.

**Evidence:** No debouncing in `handle_anim` — each tick increments the frame counter regardless of render rate.

**Proposed fix:** Use a `std::time::Instant` for the last tick time and only advance if enough time has elapsed:
```rust
const TICK_INTERVAL: Duration = Duration::from_millis(80);
if last_tick.elapsed() >= TICK_INTERVAL {
    state.animation.braille_frame = (state.animation.braille_frame + 1) % 10;
    last_tick = Instant::now();
}
```

---

## 3. Cancellation Issues

### BG-9: Ctrl+C during tool execution doesn't cancel the tool
**File:** `crates/runie-agent/src/`
**Gap:** When `Ctrl+C` is pressed, the TUI sets `agent_running = false`, but the agent's running task (e.g., a `bash` tool executing `rm -rf`) continues running.

**Proposed fix:**
1. Add `CancellationToken` to agent task spawn
2. On `Msg::Stop` / Ctrl+C, call `token.cancel()` on the running agent
3. In tool executors, check `token.is_cancelled()` periodically
4. On cancellation, rollback any partial file changes

---

### BG-10: Permission cancel doesn't rollback tool state
**File:** `crates/runie-tui/src/tui/update/agent.rs:143–151`
**Gap:** If a `WriteFile` tool started execution (permission was auto-granted by "Always") and the user later cancels, the file changes are not reverted.

**Proposed fix:** Add `Cmd::Rollback { tool_call_id: String }` and implement a diff-based rollback in the tools layer. On `PermissionCancel`, send rollback for the most recent tool.

---

## 4. Concurrency Issues

### BG-11: `state.messages` accessed from both event loop and agent event channel
**File:** `crates/runie-tui/src/tui/update/agent.rs`
**Gap:** `AgentEvent` messages are processed via `on_agent_event` which calls `update()`, which mutates `state.messages`. The terminal event loop also reads `state.messages` for rendering. There's no `Mutex` or channel protection.

**Evidence:** `crates/runie-tui/src/tui/tui.rs` — `render()` reads `state.messages` while `on_agent_event()` writes to it. In Rust, `&mut AppState` is exclusive, but the agent events arrive via a separate async task that calls `tui.on_agent_event()` — this is `&mut` access from a concurrent task.

**Proposed fix:** Use a `tokio::sync::mpsc::UnboundedReceiver<AgentEvent>` that the main event loop polls, so all mutations happen on the single-threaded TUI task. OR wrap `messages` in a `std::sync::Arc<Mutex<Vec<MessageItem>>>`.

---

### BG-12: `state.animation` updated from timer task, read from render
**File:** `crates/runie-tui/src/tui/state.rs` + `crates/runie-tui/src/tui/tui.rs`
**Gap:** `AnimationState` fields are updated by the `Tick` handler (async timer task) while `render()` reads them. Without synchronization, render may see torn values.

**Proposed fix:** Same as BG-11 — poll `Tick` events through the main event loop channel instead of a separate timer task.

---

## 5. Missing Validation

### BG-13: No DAG cycle detection in slash command execution
**File:** `crates/runie-tui/src/tui/update/slash.rs`
**Gap:** Slash commands can trigger agents that trigger more slash commands. No cycle detection prevents infinite loops.

**Proposed fix:** Add a `slash_command_depth: usize` field to `AppState`. Increment on each slash command, fail if `depth > 3`.

---

### BG-14: `DiffViewer` can be opened when no diff is available
**File:** `crates/runie-tui/src/tui/state.rs`
**Gap:** `state.diff_viewer: Option<DiffViewer>` is set to `None` by default. If `DiffViewer` mode is entered with `None`, the render panics.

**Evidence:** `crates/runie-tui/src/tui/tui.rs:260–264`
```rust
fn render_diff_viewer(...) {
    if let Some(ref diff) = state.diff_viewer {
        diff.render_ref(diff_area, ...);  // OK
    }
    // If mode is DiffViewer but diff_viewer is None → silent no-op?
    // Actually: mode is set to DiffViewer but diff_viewer is never set → no render!
}
```

**Proposed fix:** Guard mode entry:
```rust
pub fn open_diff_viewer(&mut self, diff: DiffViewer) {
    self.state.diff_viewer = Some(diff);
    self.update(Msg::OpenDiffViewer);  // Sets mode to DiffViewer
}
```
And add an assertion in render: `debug_assert!(state.diff_viewer.is_some())`.

---

## Transition Matrix (Current State)

```
CURRENT STATE          EVENT                  RESULT
─────────────────────────────────────────────────────────────────────
TuiMode::Chat          Enter                 Push User msg, SpawnAgent
TuiMode::Chat          AgentEvent::Error     Push Error, agent_running=false
TuiMode::Chat          AgentEvent::AgentEnd  agent_running=false
TuiMode::Chat          ^q                    running=false (QUIT)
TuiMode::Chat          PermissionRequest     mode=Permission
TuiMode::Permission    PermissionConfirm     SendPermission::Allow, mode=Chat
TuiMode::Permission    PermissionCancel      SendPermission::Deny, mode=Chat
TuiMode::Permission    AgentEvent::AgentEnd ??? (NOT HANDLED - BG-5)
TuiMode::Permission    AgentEvent::Error    ??? (NOT HANDLED - BG-2)
TuiMode::Onboarding    Enter                next_step() or is_complete()?→Chat
TuiMode::Onboarding    OnboardingSkip       ??? (NOT IMPLEMENTED - BG-1)
TuiMode::Onboarding    ^q                    OnboardingSkip (same as Esc!)
TuiMode::Overlay       <any key>             ??? (NOT HANDLED - P0-4)
TuiMode::DiffViewer    Esc                  CloseModal → ??? (mode stays DiffViewer!)
TuiMode::SessionTree   Esc                   ??? (NOT HANDLED in key_to_msg)
TuiMode::Chat          Tick                  animation.braille_frame++
```

---

## Proposed Transition Matrix (Fixed)

```
STATE                  EVENT                  RESULT
─────────────────────────────────────────────────────────────────────
TuiMode::Chat          Enter                 Push User, agent_running=true, SpawnAgent
TuiMode::Chat          AgentEvent::Error     Push Error, agent_running=false, mode=Chat
TuiMode::Chat          AgentEvent::AgentEnd  agent_running=false, mode=Chat
TuiMode::Chat          ^q                    running=false (QUIT)
TuiMode::Chat          PermissionRequest     Push to queue, show modal
TuiMode::Permission    PermissionConfirm     SendPermission::Allow, dequeue or mode=Chat
TuiMode::Permission    PermissionCancel     SendPermission::Deny, rollback, dequeue or mode=Chat
TuiMode::Permission    AgentEvent::Error     Clear queue, mode=Chat, Push Error
TuiMode::Permission    AgentEvent::AgentEnd Clear queue, mode=Chat
TuiMode::Onboarding    Enter (Welcome)       step=ProviderSelect
TuiMode::Onboarding    Esc                   step=prev_step (or quit if Welcome)
TuiMode::Onboarding    is_complete + Enter  mode=Chat, onboarding=None
TuiMode::Onboarding    fetch_error          Show error in KeyInput step
TuiMode::Overlay       Esc / ^q              mode=Chat (or Quit if ^q)
TuiMode::DiffViewer    Esc                   mode=Chat, diff_viewer=None
TuiMode::SessionTree   Esc                   mode=Chat
<any>                  Ctrl+C                Interrupt agent, rollback, mode=Chat
```

---

## Test Cases Required

| # | Test | Steps | Expected |
|---|------|-------|----------|
| T1 | Welcome dead-end | Launch app with no config, type any key | Should respond or show CTA |
| T2 | Permission cancel rollback | Agent starts edit, user cancels | File should be unchanged |
| T3 | Ctrl+C during stream | Start agent, press Ctrl+C mid-stream | Agent stops, mode=Chat, no crash |
| T4 | Error during permission | Agent errors while permission modal open | Mode=Chat, error shown |
| T5 | Double submit | Press Enter twice rapidly | Only one User message created |
| T6 | Onboarding → Chat | Complete onboarding, press Enter | Mode switches to Chat |
| T7 | Overlay mode key handling | Open overlay, press Esc | Overlay closes |
| T8 | Empty chat placeholder | Launch fresh, no messages | Placeholder shown |
| T9 | Model fetch failure | Enter invalid API key | Error shown, can retry |
| T10 | Overlay → Quit | In overlay, press ^q | App quits gracefully |
