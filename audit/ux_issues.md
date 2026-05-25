# UX Audit - Issues & Dead-ends

## P0 Issues (Critical - Must Fix)

### P0-1: Submit with No Model Configured ✅ FIXED
**Severity:** Dead-end  
**Location:** `crates/runie-tui/src/tui/update/misc.rs:38-49`

**Issue:** When user submits without a model configured, guidance is shown.
**Status:** ✅ Fixed - guidance message added with CTA to onboard.

---

### P0-2: Permission Modal Timeout Has No Countdown ✅ FIXED
**Severity:** Dead-end risk  
**Location:** `crates/runie-tui/src/components/permission_modal.rs`

**Issue:** Permission modal has 5-minute timeout but shows no countdown.
**Status:** ✅ Fixed - countdown display added with P0-3 fix.

---

### P0-3: Invalid API Key Shows Raw Error ✅ FIXED
**Severity:** Invalid state  
**Location:** `crates/runie-cli/src/tui_run.rs:120-125`

**Issue:** Provider errors propagate as raw AgentEvent::Error.
**Status:** ✅ Fixed - errors are sanitized before display.

---

### P0-4: Session Save/Load Dead UI ✅ FIXED
**Severity:** Dead-end  
**Location:** `crates/runie-cli/src/tui_run.rs:180-188`

**Issue:** Session save/load show "not yet implemented" in palette.
**Status:** ✅ Fixed - handled gracefully with informative message.

---

## P1 Issues (High Priority - Should Fix)

### P1-1: Network Drop During Tool Call ✅ FIXED
**Severity:** Invalid state  
**Location:** `crates/runie-tools/src/bash.rs`, `crates/runie-agent/src/rig_loop.rs`

**Issue:** Network drop during tool execution has no retry logic.
**Status:** ✅ Fixed - retry module added with exponential backoff.

---

### P1-2: File Deleted During Active Edit ✅ FIXED
**Severity:** Invalid state  
**Location:** `crates/runie-tools/src/edit_file.rs:78-82`

**Issue:** mtime detection returns error but no recovery action.
**Status:** ✅ Fixed - informative error message with re-read guidance.

---

### P1-3: Model Streams Garbage Mid-token ✅ FIXED
**Severity:** Invalid state  
**Location:** `crates/runie-agent/src/rig_loop.rs:82-95`

**Issue:** Stream processing assumes well-formed tokens.
**Status:** ✅ Fixed - UTF-8 validation added to stream handling.

---

### P1-4: Double-Submit Prevention is Confusing ✅ FIXED
**Severity:** Cognitive load  
**Location:** `crates/runie-tui/src/tui/update/misc.rs:26-32`

**Issue:** Double-submit shows message in chat, confusing conversation.
**Status:** ✅ Fixed - uses input_right_info instead of chat message.

---

### P1-5: Permission Queue Not Displayed ✅ FIXED
**Severity:** Cognitive load  
**Location:** `crates/runie-tui/src/tui/state.rs:120-125`

**Issue:** PendingPermission queue exists but not displayed.
**Status:** ✅ Fixed - queue indicator added with count display.

---

## P2 Issues (Medium Priority - Nice to Fix)

### P2-1: Inconsistent Keybindings for Quit ✅ FIXED
**Severity:** Cognitive load  
**Location:** `crates/runie-tui/src/tui/events.rs:44-52`

**Issue:** Ctrl+Q used for both quit and permission cancel.
**Status:** ✅ Fixed - Esc handles cancel, Ctrl+Q for quit only in Chat.

---

### P2-2: Empty State Not Context-Aware ✅ FIXED
**Severity:** Cognitive load  
**Location:** `crates/runie-tui/src/components/message_list/render.rs:350-368`

**Issue:** Empty state always shows same message regardless of context.
**Status:** ✅ Fixed - context-aware empty state messages.

---

### P2-3: No Progress for Long Operations ✅ FIXED
**Severity:** Cognitive load  
**Location:** `crates/runie-cli/src/tui_run.rs`

**Issue:** Long operations have no progress indication.
**Status:** ✅ Fixed - spinner added for async operations.

---

### P2-4: Raw Error Dumps in stderr ✅ FIXED
**Severity:** UX quality  
**Location:** Multiple `eprintln!` calls

**Issue:** Errors logged with raw technical messages.
**Status:** ✅ Fixed - uses tracing instead, user-friendly UI messages.

---

### P2-5: Permission Timeout Auto-Denies (Could Be Scary) ✅ FIXED
**Severity:** UX quality  
**Location:** `crates/runie-agent/src/loop_engine.rs:175-190`

**Issue:** Permission timeout silently denies without notification.
**Status:** ✅ Fixed - "timed out" system message before denial.

---

### P2-6: Progressive Disclosure - Advanced Options Hidden ✅ FIXED
**Severity:** Cognitive load  
**Location:** `crates/runie-tui/src/tui/state.rs`, `permission_modal.rs`

**Issue:** All permission options visible, overwhelming user.
**Status:** ✅ Fixed - primary actions prominent, advanced hidden behind hints.

---

### P2-7: Idempotency - Same Command Twice ✅ FIXED
**Severity:** Reliability  
**Location:** `crates/runie-agent/src/loop_engine.rs`, `rig_loop.rs`

**Issue:** Duplicate tool calls executed without deduplication.
**Status:** ✅ Fixed - HashSet tracks seen tool calls in turn.

---

### P2-8: Workspace Concurrent Edit Safety ✅ FIXED
**Severity:** Reliability  
**Location:** `crates/runie-tools/src/workspace.rs`

**Issue:** Multiple tools editing same file without locking.
**Status:** ✅ Fixed - file locking mechanism and atomic writes.

---

## Summary Table

| ID | Category | Severity | File:Line | Status |
|----|----------|----------|-----------|--------|
| P0-1 | Dead-end | Critical | misc.rs:38-49 | ✅ FIXED |
| P0-2 | Dead-end | Critical | permission_modal.rs | ✅ FIXED |
| P0-3 | Invalid state | Critical | tui_run.rs:120-125 | ✅ FIXED |
| P0-4 | Dead-end | Critical | tui_run.rs:180-188 | ✅ FIXED |
| P1-1 | Invalid state | High | bash.rs | ✅ FIXED |
| P1-2 | Invalid state | High | edit_file.rs:78-82 | ✅ FIXED |
| P1-3 | Invalid state | High | rig_loop.rs:82-95 | ✅ FIXED |
| P1-4 | Cognitive load | High | misc.rs:26-32 | ✅ FIXED |
| P1-5 | Cognitive load | High | state.rs:120-125 | ✅ FIXED |
| P2-1 | Cognitive load | Medium | events.rs:44-52 | ✅ FIXED |
| P2-2 | Cognitive load | Medium | render.rs:350-368 | ✅ FIXED |
| P2-3 | Cognitive load | Medium | tui_run.rs | ✅ FIXED |
| P2-4 | UX quality | Medium | Various | ✅ FIXED |
| P2-5 | UX quality | Medium | loop_engine.rs:175-190 | ✅ FIXED |
| P2-6 | Progressive disclosure | Medium | state.rs | ✅ FIXED |
| P2-7 | Idempotency | Medium | loop_engine.rs, rig_loop.rs | ✅ FIXED |
| P2-8 | Concurrency | Medium | workspace.rs | ✅ FIXED |

**All 17 issues fixed: 100%**

---

## Verified Working

| Check | Status |
|-------|--------|
| Permission modal has Esc/Cancel | ✅ |
| Command palette has Esc close | ✅ |
| DiffViewer has q/Esc close | ✅ |
| SessionTree has Esc close | ✅ |
| Quit via Ctrl+Q works | ✅ |
| Panic recovery exists | ✅ |
| Permission rollback on cancel | ✅ |
| Agent error resets mode to Chat | ✅ |
| Onboarding has Esc/skip | ✅ |
| Empty state placeholder | ✅ |
| No model warning | ✅ |
| Empty submit feedback | ✅ |
| Permission timeout | ✅ |
| Double-submit block feedback | ✅ |
| Progressive disclosure | ✅ |
| Tool call deduplication | ✅ |
| File locking for concurrent edits | ✅ |
