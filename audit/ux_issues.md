# UX Audit Report — Runie TUI

**Auditor:** ralph/overnight-audit
**Date:** 2026-05-24
**Scope:** `crates/runie-tui/` + `crates/runie-agent/` state machine

---

## P0 — CRITICAL (Dead-ends, Crash Paths)

### P0-1: No Ctrl+C / SIGINT handler for agent execution
**File:** `crates/runie-tui/src/tui.rs`
**Severity:** Users cannot interrupt a running agent without killing the process.

When the agent is streaming tokens or executing tools, there is no signal handler for `SIGINT`. The only `running = false` path is via `Msg::Quit` from `^q`. If the agent hangs (e.g., model timeout, infinite loop in a tool), the user must `kill` the process.

**Evidence:** `crates/runie-tui/src/tui.rs` has `install_panic_hook()` but no `install_signal_handler()`. The `events.rs` handles keyboard events but `crossterm::event::poll()` does not intercept terminal signals.

**Fix:** Add `tokio::signal::ctrl_c()` listener in the event loop that fires `Msg::Stop` / `TuiAction::Interrupt`, setting `agent_running = false` and sending a cancellation signal to the agent task.

---

### P0-2: Onboarding Welcome step is a dead-end
**File:** `crates/runie-tui/src/components/onboarding/render.rs:29–51`
**Severity:** User is trapped on the Welcome screen with no escape path.

The Welcome step shows a centered panel but has no interactive elements. `key_to_onboarding_msg` returns `None` for all keys except `Enter`, which does nothing (there's no handler for it). Pressing `^q` (quit) only works if `Esc` is handled by the onboarding handler.

**Evidence:**
```rust
// crates/runie-tui/src/tui/events.rs:124–135
fn key_to_onboarding_msg(key: KeyEvent, state: &AppState) -> Option<Msg> {
    // Only handles Tab, Shift+Tab, Enter, Arrows
    // No Esc / ^q handler
```

**Fix:** Add `Esc` and `^q` as aliases for `OnboardingSkip` on the Welcome step, or show an explicit "Press Enter to start →" CTA. Make the Welcome panel clickable/selectable.

---

### P0-3: Permission modal — Escape doesn't return to Chat
**File:** `crates/runie-tui/src/tui/events.rs:76–90`
**Severity:** Permission modal has no dedicated Esc handler; Escape falls through.

In `key_to_permission_msg`, only `y`/`Enter`/`n`/`Esc` are handled as `PermissionCancel`. However, if the user is in `TuiMode::Permission` and presses `^q` (global quit), the key maps to the permission handler first. The bigger issue: when `PermissionCancel` fires, `handle_permission` sets `mode = TuiMode::Chat`, but if the user presses `Esc` outside the modal's area, the event is not consumed.

**Evidence:**
```rust
// events.rs:76-90 - Esc maps to PermissionCancel
KeyCode::Char('n') | KeyCode::Esc => Some(Msg::PermissionCancel),
```

**Fix:** Add explicit `^q` handling in `TuiMode::Permission` that calls `PermissionCancel` and also sets `running = false` (quit). Ensure the permission modal fully intercepts all key events.

---

### P0-4: Overlay mode has no key handler
**File:** `crates/runie-tui/src/tui/events.rs`
**Severity:** `TuiMode::Overlay` is set but no keys are handled in this mode.

`key_to_msg` matches on `state.mode` but does not include `TuiMode::Overlay`. If the app enters overlay mode, all key presses are silently dropped.

**Evidence:**
```rust
// events.rs:31-40
match state.mode {
    TuiMode::Chat => ...,
    TuiMode::Overlay => {
        // MISSING - returns None for all keys!
    },
    TuiMode::Select => ...,
    ...
}
```

**Fix:** Add `TuiMode::Overlay` match arm with `Esc` → `CloseModal` / `^q` → `Quit`.

---

### P0-5: Empty chat has no placeholder
**File:** `crates/runie-tui/src/components/message_list/render.rs`
**Severity:** When `state.messages` is empty, the content area is blank — no CTA, no guidance.

**Evidence:** `MessageList::render_ref` iterates over `items`, but if empty, renders nothing. The `MessageItem` enum has no `Empty` variant.

**Fix:** Add an empty-state check at the top of `MessageList::render_ref` that shows:
```
No messages yet. Start typing to chat with the agent.
  ↑/↓  scroll  |  ^k  commands  |  ^b  sidebar
```

---

## P1 — IMPORTANT (Invalid States, Recovery Gaps)

### P1-1: Model fetch failure during onboarding leaves user stuck
**File:** `crates/runie-tui/src/components/onboarding/render.rs:182–199`
**Severity:** If the model fetch fails (network error, invalid key), the UI shows "loading models..." indefinitely.

The `is_fetching_models` flag is set to `true` when fetching starts, but there's no error state that transitions to `KeyInput` with an error message.

**Evidence:**
```rust
// onboarding/mod.rs - no error handler for failed fetch
pub fn set_models(&mut self, models: Vec<ModelOption>) { ... }
pub fn set_fetch_error(&mut self, err: String) { /* MISSING */ }
```

**Fix:** Add `fetch_error: Option<String>` field to `Onboarding`, set it on fetch failure, and render the error in the KeyInput step alongside the "loading..." text.

---

### P1-2: Agent error doesn't distinguish recoverable vs fatal
**File:** `crates/runie-tui/src/tui/update/agent.rs:85–97`
**Severity:** All agent errors are shown the same way, even though transient errors (timeout, rate limit) could be retried.

The `is_recoverable_error()` function exists but its result is stored in `MessageItem::Error { recoverable, .. }` and rendered identically.

**Evidence:**
```rust
// on_agent_error stores recoverable flag but render doesn't use it
MessageItem::Error { message, recoverable }  // recoverable is never checked in render
```

**Fix:** In `message_list/render.rs`, check `recoverable` and show a "Retry" button / key hint for recoverable errors. For fatal errors, show "Report issue" hint.

---

### P1-3: DiffViewer mode has no Escape exit
**File:** `crates/runie-tui/src/tui/events.rs:103–113`
**Severity:** `key_to_diff_msg` handles `j/k/↑/↓/y/n` but NOT `Esc`. Only `q` closes the DiffViewer.

**Evidence:**
```rust
fn key_to_diff_msg(key: KeyEvent) -> Option<Msg> {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => ...,
        KeyCode::Char('k') | KeyCode::Up => ...,
        KeyCode::Char('q') | KeyCode::Esc => Some(Msg::CloseModal),  // Esc IS handled
        _ => None,
    }
}
```
**Actually OK** — `Esc` IS handled. But the status bar for DiffViewer shows `Esc = close` — this is inconsistent with the onboarding status bar which doesn't show Esc.

**Fix:** N/A (already handled), but mark as verified.

---

### P1-4: No rollback on PermissionCancel during partial edit
**File:** `crates/runie-tui/src/tui/update/agent.rs:143–151`
**Severity:** If the agent started editing a file and the user cancels, the partial edit remains.

**Evidence:** `handle_permission_msg` sends `PermissionDecision::Deny` but the agent loop doesn't have a rollback mechanism for partially executed tool calls.

**Fix:** Add `Cmd::RollbackTool` variant and implement rollback in the agent executor. On `PermissionCancel`, send rollback command before returning to Chat mode.

---

### P1-5: SessionTree mode has no dedicated hotkey in status bar
**File:** `crates/runie-tui/src/components/status_bar.rs`
**Severity:** `TuiMode::SessionTree` status bar shows navigation keys but not how to switch between SessionTree and Chat (no `Esc` shown).

**Evidence:** The status bar for SessionTree:
```
Esc close | ↑/↓ navigate | Enter expand
```
But `ToggleSessionTree` is triggered by `^b` (sidebar toggle), which is NOT shown in SessionTree mode status bar.

**Fix:** Add `^b` as an alternative exit from SessionTree mode in the status bar hint, or make `Esc` consistently the way out.

---

### P1-6: No API key validation before agent spawn
**File:** `crates/runie-tui/src/tui/update/misc.rs:29–48`
**Severity:** User can press Enter on an empty textarea or with invalid input. The submit handler blocks empty submits but not "run with invalid config".

**Evidence:** `handle_submit` checks `text.is_empty()` but doesn't validate that `current_model` is set. If onboarding was skipped without configuration, the agent spawns with `None` model.

**Fix:** Add a pre-spawn validation check:
```rust
if state.current_model.is_none() {
    state.messages.push(MessageItem::System {
        text: "No model configured. Run /setup to configure a provider.".to_string(),
    });
    return vec![];
}
```

---

## P2 — MINOR (Cognitive Load, Consistency)

### P2-1: Inconsistent status bar format across modes
**File:** `crates/runie-tui/src/components/status_bar.rs`
**Severity:** Some modes show `Esc = close` (Overlay, CommandPalette, DiffViewer, SessionTree) but Onboarding doesn't. This violates Hick's Law — users must learn different escape conventions.

**Evidence:**
```rust
// Onboarding status bar:
TuiMode::Onboarding => vec![
    StatusItem { key: "Enter", description: "next" },
    StatusItem { key: "^q", description: "quit" },
    // NO Esc shown!
]
```

**Fix:** Add `Esc` with `back` / `skip` description to the Onboarding status bar, consistent with other modes.

---

### P2-2: Permission modal — "Skip" option is discoverable but not discoverable enough
**File:** `crates/runie-tui/src/components/permission_modal.rs:157–164`
**Severity:** The `[s] skip this step` option is shown in dim text as a secondary row. Users who don't read carefully won't find it.

**Evidence:** Progressive disclosure is partially implemented (2 primary + 2 dimmed), but "Skip" is useful for iterating quickly.

**Fix:** Show all 4 options on equal footing (4-column layout), with primary colors. The current 2+2 layout is good but the dimmed options are too easy to miss. Consider showing a legend at the bottom: `[a] always · [s] skip`.

---

### P2-3: Welcome step has no call-to-action text
**File:** `crates/runie-tui/src/components/onboarding/render.rs:29–51`
**Severity:** The Welcome panel shows title + subtitle but no "press Enter to begin" text.

**Evidence:**
```
welcome
multi-model coding agent
configure providers, models, keys
(END - no CTA)
```

**Fix:** Add a footer line: `Press Enter or click to begin →`

---

### P2-4: Hotkey `^q` quits app but onboarding uses it for "quit setup"
**File:** `crates/runie-tui/src/tui/events.rs`
**Severity:** `^q` in Onboarding mode sends `OnboardingSkip` (which completes setup), but `^q` in Chat mode sends `Quit` (which exits the app). This is confusing — same key, different action.

**Evidence:**
```rust
// Onboarding:
KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Msg::OnboardingSkip),
// Chat:
KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Msg::Quit),
```

**Fix:** Rename `OnboardingSkip` to something clearer, or use a different key for "skip setup" (e.g., `^c` for cancel setup). Document the keybinding difference in the onboarding status bar.

---

### P2-5: Error banner in message list doesn't distinguish recoverable vs fatal
**File:** `crates/runie-tui/src/components/message_list/render.rs`
**Severity:** Both recoverable and fatal errors render identically as red `ERROR` badges.

**Evidence:** `render_error_item` doesn't check the `recoverable` field of `MessageItem::Error`.

**Fix:** Add a `⟳` retry icon for recoverable errors:
```
[ERROR] Network timeout [⟳ retry]
```
vs.
```
[ERROR] Invalid API key [report]
```

---

## Summary Table

| ID | Severity | Issue | File |
|----|----------|-------|------|
| P0-1 | CRITICAL | No Ctrl+C handler | `tui.rs` |
| P0-2 | CRITICAL | Welcome step dead-end | `onboarding/render.rs` |
| P0-3 | CRITICAL | Permission modal Esc gap | `events.rs` |
| P0-4 | CRITICAL | Overlay mode has no handler | `events.rs` |
| P0-5 | CRITICAL | Empty chat no placeholder | `message_list/render.rs` |
| P1-1 | HIGH | Model fetch failure stuck | `onboarding/mod.rs` |
| P1-2 | HIGH | Error not distinguished by severity | `update/agent.rs` |
| P1-3 | MEDIUM | DiffViewer Escape — VERIFIED OK | — |
| P1-4 | HIGH | No rollback on permission cancel | `update/agent.rs` |
| P1-5 | MEDIUM | SessionTree missing ^b in status | `status_bar.rs` |
| P1-6 | HIGH | No pre-spawn config validation | `update/misc.rs` |
| P2-1 | LOW | Onboarding missing Esc in status | `status_bar.rs` |
| P2-2 | LOW | Skip option not discoverable | `permission_modal.rs` |
| P2-3 | LOW | Welcome has no CTA | `onboarding/render.rs` |
| P2-4 | LOW | ^q means different things | `events.rs` |
| P2-5 | LOW | Error not styled by severity | `message_list/render.rs` |

---

## Hotkey Consistency Map

| Mode | Esc | Enter | ^q | j/k | Other |
|------|-----|-------|----|-----|-------|
| Chat | ✓ (scroll) | ✓ send | ✓ quit | ✓ scroll | ^b sidebar, ^k cmd |
| Onboarding | ✗ missing | ✓ next | ✓ skip | ✓ nav | Tab nav |
| Permission | ✓ cancel | ✓ allow | ✗ missing | ✓ cycle | y/n/a/s |
| CommandPalette | ✓ close | ✓ run | — | ✓ nav | ↑/↓ nav |
| DiffViewer | ✓ close | — | — | ✓ nav | y/n accept/reject |
| SessionTree | ✓ close | ✓ expand | — | ✓ nav | — |
| Overlay | ✓ close | ✓ select | ✗ missing | ✓ nav | — |

**Legend:** ✓ = handled, ✗ = should exist but missing, — = N/A
