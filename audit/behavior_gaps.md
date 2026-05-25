# Behavior Gaps — State Machine Audit

Mapped the agent loop and TUI as finite state machines. Every `StateA → StateB` transition was traced.

---

## 1. Undefined / Implicit Transitions

### BG-1: `TuiMode::Chat` → `TuiMode::Permission` — no return path if permission dismissed while agent running
**Files:** `crates/runie-tui/src/tui/update/agent.rs` → `on_permission_request()`, `crates/runie-tui/src/tui/events.rs` → `key_to_permission_msg()`

**Current:** When a permission request fires, `TuiMode` becomes `Permission`. User presses `n`/`Esc` → `PermissionCancel` → `TuiMode::Chat`. Agent resumes with a denied permission decision.

**Gap:** If the `permission_rx` channel is closed (agent dropped), the denial decision has nowhere to go. The TUI returns to `Chat` mode but the agent is hung waiting for a decision.

**Fix:** Check if `permission_rx` is closed before sending. If so, treat as implicit `Deny` and show a banner: `"Permission channel closed — denied by default."`

---

### BG-2: `TuiMode::Onboarding::Complete` — no defined exit
**File:** `crates/runie-tui/src/tui/update/onboarding.rs`

`OnboardingStep::Complete` is the terminal step of the onboarding flow. The handler for `OnboardingNext` at this step commits settings and transitions to `Chat`. But `OnboardingBack` from `Complete` has no defined behavior — pressing Esc on the completion screen could loop back to the previous step or exit to Chat.

**Fix:** Add explicit handler: `OnboardingBack` from `Complete` → re-enter the `ModelSelect` step (allow re-selection). `OnboardingNext` from `Complete` → `TuiMode::Chat`.

---

### BG-3: `agent_running = true` with no active stream — zombie state
**File:** `crates/runie-tui/src/tui/update/agent.rs` → `on_message_start()`, `on_agent_end()`

`on_message_start` sets `agent_running = true`. If the provider disconnects without sending `AgentEnd`, `agent_running` stays `true` forever. The spinner keeps spinning. The user cannot type.

**Fix:** Add a `Tokio` timeout task. If no `AgentEnd` or `AgentEvent::Error` is received within 60 seconds of `MessageStart`, emit `AgentEvent::Error { message: "Provider stream timed out" }` and set `agent_running = false`.

---

### BG-4: `TuiMode::DiffViewer` → `TuiMode::Chat` — what happens to pending input?
**File:** `crates/runie-tui/src/tui/events.rs` → `key_to_diff_msg()`

When DiffViewer is open and user presses Esc/q, `Msg::CloseModal` is sent. The `update` function calls `palette::handle_close_modal` which resets the mode. But if the user was typing a message before opening the diff, that input is preserved in `state.textarea`. This is correct — no data loss.

**Gap:** If a diff is shown while an agent is running, the DiffViewer is layered on top. When closed, `agent_running` is still `true`. The user can keep watching the agent. This is fine.

---

### BG-5: `Msg::ModelsFetched` / `Msg::ModelsFetchFailed` — no transition enforcement ✓ FIXED
**File:** `crates/runie-tui/src/tui/update/misc.rs` → `handle_submit()`

**Before:** If a `Submit` was sent while models were still being fetched, the agent started without knowing what model was configured.

**Fix Applied:** Added check in `handle_submit()`:
```rust
if let Some(ref onboarding) = state.onboarding {
    if onboarding.is_fetching_models {
        state.messages.push(MessageItem::System {
            text: "Still loading models... Please wait.".to_string(),
        });
        return vec![];
    }
}
```
This blocks submit while model fetch is in progress and shows a helpful banner.

---

### BG-6: `AgentEvent::MessageUpdate` with empty content — infinite loop risk
**File:** `crates/runie-tui/src/tui/update/agent.rs` → `on_message_update()`

If the provider sends `MessageUpdate` with empty content repeatedly (e.g., streaming garbage), the UI updates but the spinner never stops. There's no stop condition.

**Fix:** If `MessageUpdate` returns empty content 5 times consecutively, treat as `AgentEvent::Error { message: "Model returned empty response" }`.

---

## 2. Idempotency Violations

### BG-7: Double-submit is not blocked ✓ FIXED
**File:** `crates/runie-tui/src/tui/update/misc.rs` → `handle_submit()`

**Before:** If the user double-pressed Enter quickly, two `Submit` messages could be queued. The second submit would start a new agent turn while one was already running.

**Fix Applied:** Added check at the start of `handle_submit()`:
```rust
if state.agent_running {
    return vec![];
}
```
This prevents starting a new agent turn while one is already running.

---

### BG-8: Ctrl+C during `Submit` — inconsistent with agent_running
**File:** `crates/runie-tui/src/tui/events.rs` → `key_to_chat_msg()`

When `Ctrl+C` is pressed with non-empty textarea, it calls `Msg::ClearInput`. But if `agent_running = true` at the same time, should Ctrl+C interrupt the agent? Currently it only clears input.

**Fix:** If `agent_running = true` and textarea is non-empty, `Ctrl+C` should show a banner: `"Agent running. Press Ctrl+C again to interrupt."` with a 2-second debounce before escalating.

---

## 3. Cancellation Safety

### BG-9: `Ctrl+C` during permission request — agent not cancelled ✓ FIXED
**File:** `crates/runie-tui/src/tui/events.rs` → `key_to_permission_msg()`

**Before:** When `TuiMode::Permission`, pressing `Ctrl+C` was NOT handled (went to `None`). The permission modal stayed open. The agent was blocked waiting for a decision.

**Fix Applied:** Added Ctrl+C handling at the start of `key_to_permission_msg()`:
```rust
if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c')) {
    return Some(Msg::PermissionCancel);
}
```
This gives an explicit escape path from the permission modal via Ctrl+C.

---

### BG-10: Rollback of partial file edits on agent crash
**File:** `crates/runie-tools/src/edit_file.rs`

The edit tool applies changes sequentially. If the agent crashes mid-edit (tool call 3 of 5), files 1-3 are already modified. There's no rollback mechanism.

**Fix:** The workspace should snapshot state before each tool execution. On `AgentEvent::Error`, offer a `rewind` action that reverts to the last known-good snapshot.

---

## 4. Concurrency Guards

### BG-11: Shared `AppState` across threads
**File:** `crates/runie-tui/src/tui/state.rs` + `crates/runie-cli/src/tui_run.rs`

`AppState` is shared between the TUI event loop (main thread) and the agent loop (async task) via `event_tx`/`permission_rx` channels. The `messages` vector is mutated only by the agent event handler. The `textarea` is mutated only by the TUI event handler. These are separate, but `current_model`, `agent_running`, `session_token_usage` are mutated by the agent loop and read by the TUI.

**Gap:** These reads/writes are not atomic. On a slow terminal, `agent_running` could be read as stale.

**Fix:** Use `Arc<AtomicBool>` for `agent_running`. Use `Arc<Mutex<T>>` for `session_token_usage`. Or accept that these are event-driven (only updated when `AgentEvent` arrives) so the TUI always reads the latest from the event handler.

---

## 5. Proposed Fixes Summary

| ID | Type | Gap | Proposed Fix |
|----|------|-----|-------------|
| BG-1 | Transition | Permission channel closed → hung agent | Check channel closed, implicit Deny |
| BG-2 | Transition | Onboarding Complete — no back path | Back → re-enter ModelSelect |
| BG-3 | Zombie State | agent_running=true with no stream | Timeout after 60s, emit Error |
| BG-5 | Transition | Submit while models fetching | Block Submit, show banner |
| BG-6 | Stream Safety | Empty MessageUpdate spam | Count consecutive empties, error on 5 |
| BG-7 | Idempotency | Double-submit not blocked | Check agent_running before Submit |
| BG-8 | Cancellation | Ctrl+C during agent run | 2-step interrupt escalation |
| BG-9 | Dead-End | Ctrl+C in Permission mode → no effect | Add Ctrl+C → PermissionCancel |
| BG-10 | Rollback | No partial-edit rollback | Snapshot before each tool call |
| BG-11 | Concurrency | Shared state across threads | Use Arc<AtomicBool>/Mutex or document event-driven model |
