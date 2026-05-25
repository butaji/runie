# UX Audit Report — Dead-Ends, Invalid States, Cognitive Load

**Date:** 2024-05-24  
**Auditor:** ralph/overnight-audit  
**Severity Scale:** P0 (blocks usage) → P1 (major UX friction) → P2 (minor issue)

---

## P0 — Critical (Blocks Usage / Data Loss Risk)

### ✅ P0-1: Agent Ctrl+C Interruption Leaves State Inconsistent
**Status:** Partially Addressed  
**File:** `crates/runie-agent/src/loop_engine.rs`, `crates/runie-tui/src/tui/update/agent.rs`

The TUI correctly handles `Msg::Stop` by setting `agent_running = false` and resetting mode. However, the agent loop still needs a clean abort path. The `Interrupt` command exists but the agent loop doesn't check for interruption between tool executions.

**Current State:** Basic interrupt handling works. Full rollback of partial changes still needs implementation.

---

### ✅ P0-2: No Model Configured — Silent Failure on Submit
**Status:** FIXED  
**File:** `crates/runie-tui/src/tui/update/misc.rs`

When user submits without a model configured (and not in onboarding), the system now:
1. Records the user's message
2. Clears the textarea (good UX)
3. Shows a helpful error message: "No model configured. Press Ctrl+O or type /onboard to set up a model."
4. Does NOT spawn the agent

This provides clear feedback and guides the user to the solution.

---

### ✅ P0-3: Permission Modal Has No Timeout Display
**Status:** FIXED  
**Files:** 
- `crates/runie-tui/src/components/permission_modal.rs` - Added `timeout_secs` field and `render_timeout()` function
- `crates/runie-tui/src/tui.rs` - Passes timeout calculation to modal
- `crates/runie-tui/src/tui/view_models.rs` - Added `timeout_secs` to view model

The permission modal now displays a countdown timer (e.g., "⏱ Expires in 4:32"). The timer changes to warning color when less than 60 seconds remain.

---

## P1 — Major UX Friction

### P1-1: Stack Traces May Still Leak on Provider Errors
**Status:** Partially Addressed  
**File:** `crates/runie-agent/src/loop_engine.rs`

Error sanitization exists in the TUI's `on_agent_error()`. Provider errors are sanitized before display, but there's room for improvement at the provider level.

---

### P1-2: Network Drops During Tool Call — No Retry UI
**Status:** Identified (Gap BG-3)  
**File:** `crates/runie-tools/src/bash.rs`, `edit_file.rs`, `write_file.rs`

No retry logic for transient network errors. Added harness task `network_retry` to track this.

---

### P1-3: Empty Chat State — Missing Helpful CTA
**Status:** Tracked via Harness Task  
**File:** `harness/tasks/empty_state/`

Harness task exists to validate empty state implementation.

---

### P1-4: Permission Modal Blocks Everything — No Queue Visibility
**Status:** Identified (Gap BG-1)  
**File:** `crates/runie-tui/src/components/permission_modal.rs`

Queue depth is tracked but not displayed in the modal. Future enhancement to show "1 of 3 pending".

---

### P1-5: Onboarding API Key Validation Is Weak
**Status:** Identified (Gap BG-4)  
**File:** `crates/runie-tui/src/components/onboarding/`

API key format validation exists but no actual API call to verify. Added harness task to track.

---

## P2 — Minor Issues

### P2-1: Command Palette Has No Fuzzy Search Ranking
**Status:** Known Limitation  
**File:** `crates/runie-tui/src/components/command_palette/`

Substring matching works but fuzzy search would improve UX.

---

### P2-2: Session Tree Navigation Is Keyboard-Only
**Status:** Known Limitation  
**File:** `crates/runie-tui/src/components/session_tree.rs`

Mouse support would improve accessibility.

---

### P2-3: Token Usage Display Is Estimates Only
**Status:** Documented  
**File:** `crates/runie-tui/src/components/status_bar.rs`

Cost estimates are based on fixed token pricing, not actual billing.

---

## Cognitive Load Issues

### C1: Inconsistent Keybindings Between Modes
**Status:** Partially Addressed  
**Evidence:** Permission now accepts `Ctrl+C` and `Ctrl+Q` for cancel. Overlay accepts `Esc` and `Ctrl+Q`.

---

### C2: Too Many Modal Modes (7 TuiMode variants)
**Status:** Known Architectural Concern  
**Evidence:** TuiMode has: Chat, Overlay, Select, Permission, CommandPalette, DiffViewer, SessionTree, Onboarding

---

## Empty State Checklist

| Empty State | Current Behavior | Status |
|-------------|------------------|--------|
| No messages | Tracked via harness | P1-3 |
| No model configured | Block submit + guide to settings | ✅ P0-2 FIXED |
| No workspace | Bash tools fail with clear error | OK |
| Permission timeout | Shows countdown timer | ✅ P0-3 FIXED |
| API fetch fails | Error banner + retry option | OK in onboarding |

---

## Summary Statistics

| Severity | Count | Fixed |
|----------|-------|-------|
| P0 | 3 | 2 |
| P1 | 5 | 0 (tracked) |
| P2 | 3 | 0 (known) |
| Cognitive | 2 | 1 (partial) |
| Empty States | 5 | 2 |
