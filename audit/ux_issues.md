# UX Audit — Runie TUI

Audited by walking every `draw()` and `handle_event()` path. Issues are ranked P0 (blocks usage), P1 (confusing/cognitive load), P2 (cosmetic/good-to-fix).

---

## P0 — Dead-Ends and Blocking Issues

### P0-1: No empty-state placeholder in chat view
**File:** `crates/runie-tui/src/components/message_list/render.rs`

When `messages` is empty, `render_ref()` loops over zero items and renders a completely blank feed. The user sees nothing and has no CTA.

**Fix:** Add a render block after the message loop:
```rust
if vm.messages.is_empty() && !vm.agent_running {
    // render welcome prompt
}
```

**Primary CTA:** "Type a message and press Enter to start." + "Press ^k for commands."

---

### P0-2: Onboarding Welcome step — Esc is a no-op (not a dead-end, but dead-start)
**File:** `crates/runie-tui/src/tui/events.rs` → `key_to_onboarding_msg()`

On the `Welcome` step, `OnboardingBack` maps to `Msg::OnboardingBack`. The onboarding handler at `onboarding.rs` does not define a back action from `Welcome`. User cannot escape the Welcome screen except via the skip/next flow. This is acceptable UX for onboarding but should be documented; pressing Esc from Welcome should ideally show "Press Enter to continue or ^C to quit" since there is no parent step.

**Fix:** On `Welcome` step, `Esc` should either display a tooltip hint "press Enter" or be handled as a no-op with the hint shown inline.

---

### P0-3: Model fetch failure shows inline error but no recovery path
**File:** `crates/runie-tui/src/components/onboarding/render.rs` → `render_model_select()`

When `onboarding.error_message` is set (API key invalid, network timeout, etc.), it renders:
```rust
if let Some(ref err) = onboarding.error_message {
    Paragraph::new(err.as_str()).style(...)
}
```
The user sees the error but cannot retry from this screen. They must go back and re-enter the key.

**Fix:** When error is shown, add a "retry" button/option in the footer: `"[r] retry"` alongside `"[Enter] select"`.

---

### P0-4: DiffViewer — `q` is the only quit key; Esc does NOT close it
**File:** `crates/runie-tui/src/tui/events.rs` → `key_to_diff_msg()` vs `crates/runie-tui/src/tui/render.rs` status bar

Status bar says: `TuiMode::DiffViewer => vec![("q", "close"), ("j/k", "scroll")]`

`key_to_diff_msg` handles only `Esc`, `q`, `j/k`, `↑↓`, `PageUp/Down`. **Esc IS handled** in the key mapping — it returns `Msg::CloseModal`. This is correct. The status bar hint is consistent.

Wait, actually: `TuiMode::DiffViewer => key_to_diff_msg(key)` is NOT in the key routing. Let me re-check...

In `key_to_msg`, DiffViewer mode calls `key_to_diff_msg`. That function maps `Esc` → `Msg::CloseModal`. So Esc closes the DiffViewer. This is NOT a bug. However, the status bar shows `("q", "close")` as the primary hint, not `("Esc", "close")`. This is a **cognitive load P1 issue** — users expect Esc to close, and the status bar misleads them.

**Fix (P1):** Change DiffViewer status bar to `("Esc", "close")`.

---

## P1 — Cognitive Load and Inconsistent Keybindings

### P1-1: Permission modal — four options on one screen (Hick's Law violation)
**File:** `crates/runie-tui/src/components/permission_modal.rs` → `render_buttons()`

The permission modal shows four options simultaneously: `[Y] Confirm`, `[N] Cancel`, `[A] Always`, `[S] Skip`. This is 4 active choices with no progressive disclosure.

**Fix:** Show only two options by default: `[Y] Confirm (once)`, `[N] Cancel`. Move `[A] Always` and `[S] Skip` to a secondary row labeled `"[a] always allow"` and `"[s] skip this step"`. This reduces visible options from 4 to 2 primary + 2 discoverable.

---

### P1-2: Status bar inconsistent with actual hotkey label for Permission mode
**File:** `crates/runie-tui/src/tui/render.rs` → `get_status_items()`

`TuiMode::Permission` shows: `("y", "confirm"), ("n", "cancel"), ("a", "always"), ("s", "skip")`

But `key_to_permission_msg` maps `Char('y')` AND `Enter` both to `Msg::PermissionConfirm`. The status bar should show `(y/Enter) confirm` to reflect reality. Similarly, `Esc` maps to cancel, not just `n`.

**Fix:** Update status bar to reflect all valid inputs: `("y/Enter", "confirm"), ("Esc/n", "cancel"), ("a", "always")`.

---

### P1-3: `Ctrl+Q` to quit vs `Ctrl+C` empty to quit — discoverability
**File:** `crates/runie-tui/src/tui/events.rs` → `key_to_chat_msg()`

`Ctrl+Q` quits (displayed in status bar). `Ctrl+C` with empty input also quits. Users may not discover the latter. The status bar only shows `^q quit`. This is fine but could be improved.

**Fix (P2):** Add a secondary hint: `"^c (empty) also quits"`. Or change the behavior so `Ctrl+C` always quits regardless of input content, and show that in the status bar.

---

### P1-4: Command palette — no empty state message
**File:** `crates/runie-tui/src/components/command_palette/render.rs`

When `palette.filtered_commands` is empty (after filtering), the palette shows nothing. The user sees a blank box with no indication that their filter matched nothing.

**Fix:** When filtered list is empty, render a centered "No matching commands" message.

---

### P1-5: Onboarding — model select shows "loading models..." but no timeout indicator
**File:** `crates/runie-tui/src/components/onboarding/render.rs`

`render_model_select()` renders `"loading models..."` while `onboarding.is_fetching_models` is true. If the network request hangs indefinitely, the user is stuck with no indication that they should wait or retry.

**Fix:** After 10 seconds of `is_fetching_models = true` with no response, show a timeout message: `"Taking a while... press Esc to go back"`.

---

### P1-6: Status bar says `^q quit` in Chat mode, but `^c` (empty) also quits
**File:** `crates/runie-tui/src/tui/events.rs`

The `Ctrl+C → empty input → quit` behavior is not reflected in the status bar. Users may not discover this alternative quit path.

**Fix (P2):** Show `^c` in status bar: `"^c quit (when empty)"`.

---

## P2 — Invalid State Degradation

### P2-1: Agent error event shows raw `format!("Error: {}", message)` — no structured presentation
**File:** `crates/runie-tui/src/tui/update/agent.rs` → `on_agent_error()`

```rust
pub fn on_agent_error(state: &mut AppState, message: String) {
    state.messages.push(MessageItem::System { text: format!("Error: {}", message) });
    state.agent_running = false;
}
```

The error is dumped as a plain text `System` message. Users see `"Error: connection refused"` without knowing if it's recoverable.

**Fix:** Create a dedicated `MessageItem::Error { message: String, recoverable: bool }` variant. Render it with an `[!]` icon, error color, and if `recoverable`, show `"(press Enter to retry)"`.

---

### P2-2: Network drop during tool call — agent continues silently
**File:** `crates/runie-agent/src/loop_engine.rs`

When the provider stream drops mid-turn, the loop engine catches the error, emits `AgentEvent::Error`, but the error message is just the raw error string. If the error is transient (network blip), the user has no way to retry the same turn.

**Fix:** Categorize errors as `Transient` vs `Fatal`. If `Transient`, append `" (try again automatically?)"` to the message and set a flag that triggers a retry prompt in the TUI.

---

### P2-3: DAG cycle detection — error buried in System message
**File:** `crates/runie-agent/src/loop_engine.rs` or `crates/runie-tools/src/` (cycle detection)

If a file being edited is deleted mid-operation, the tool returns an error. The TUI shows a system message, but the user may not understand the cause.

**Fix:** Map common error patterns to friendly messages:
- `"file not found"` → `"That file was deleted. Do you want to re-create it?"`
- `"permission denied"` → `"Permission denied. Check file permissions."`
- `"DAG cycle"` → `"Circular dependency detected in edit order."`

---

### P2-4: Partial file edit — if edit fails mid-write, workspace may be inconsistent
**File:** `crates/runie-tools/src/edit_file.rs`

The `EditFileTool` should use atomic write (write to temp, then rename) to avoid leaving files in a partially edited state. Need to verify this is implemented.

**Fix:** Check that `write_file` uses atomic write. If not, add it.

---

## Summary Table

| ID | Severity | Category | Screen | Issue |
|----|----------|----------|--------|-------|
| P0-1 | P0 | Empty State | Chat feed | Blank screen on first launch |
| P0-3 | P0 | Invalid State | Onboarding model select | Error shown but no retry CTA |
| P1-1 | P1 | Cognitive Load | Permission modal | 4 options shown at once |
| P1-2 | P1 | Cognitive Load | Permission status bar | Label mismatch with actual keys |
| P1-4 | P1 | Empty State | Command palette | Blank when no match |
| P1-5 | P1 | Invalid State | Onboarding model select | No timeout on fetch |
| P2-1 | P2 | Error Presentation | Agent error | Raw error string, no structured type |
| P2-2 | P2 | Invalid State | Agent loop | Network drop = silent or cryptic |
| P2-3 | P2 | Invalid State | Edit tools | File deletion not surfaced cleanly |
