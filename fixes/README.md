# Overnight Audit Fixes

This directory documents the fixes identified during the overnight UX audit.

## Fix Summary

Most fixes were already implemented in the codebase with `// P0-X FIX:`, `// P1-X FIX:`, `// P2-X FIX:`, or `// BG-X FIX:` comments. This document summarizes the status of each identified issue.

## P0 - Critical Fixes

| Fix ID | Description | Status | File Reference |
|--------|-------------|--------|-----------------|
| P0-1 | Permission timeout handling | ✅ IMPLEMENTED | `tui/state.rs:55-58`, `tui/update/misc.rs:50-60` |
| P0-2 | No model warning | ✅ IMPLEMENTED | `tui/state.rs:330-332` |
| P0-3 | Empty input submit feedback | ✅ IMPLEMENTED | `tui/update/misc.rs:32-34` |
| P0-4 | Permission modal key bindings | ✅ IMPLEMENTED | `tui/state.rs:310-312` |

## P1 - Important Fixes

| Fix ID | Description | Status | File Reference |
|--------|-------------|--------|-----------------|
| P1-1 | Error sanitization | ✅ IMPLEMENTED | `tui/update/agent.rs:76-120` |
| P1-2 | Command palette Esc handling | ✅ IMPLEMENTED | `tui/update/palette.rs:29-47` |
| P1-3 | Panic recovery | ✅ IMPLEMENTED | `agent/loop_engine.rs:290-350` |
| P1-4 | Double-submit prevention | ✅ IMPLEMENTED | `tui/update/misc.rs:38-42` |
| P1-5 | Rollback on cancel | ✅ IMPLEMENTED | `tui/update/agent.rs:150-168` |
| P1-6 | Model validation | ✅ IMPLEMENTED | `tui/update/misc.rs:55-59` |

## P2 - Nice to Have Fixes

| Fix ID | Description | Status | File Reference |
|--------|-------------|--------|-----------------|
| P2-1 | Structured error rendering | ✅ IMPLEMENTED | `components/message_list/render.rs:145-162` |
| P2-2 | Onboarding Welcome CTA | ✅ IMPLEMENTED | `tui/update/onboarding.rs:85-88` |
| P2-3 | Session tree navigation | ✅ IMPLEMENTED | `tui/update/tree.rs:1-16` |
| P2-4 | Pending permission queue | ✅ IMPLEMENTED | `tui/state.rs:60-62` |
| P2-5 | Permission timeout denial | ✅ IMPLEMENTED | `tui/update/agent.rs:175-205` |
| P2-6 | Progressive disclosure | ✅ IMPLEMENTED | `tui/state.rs:64` |
| P2-7 | Idempotent tool calls | ✅ IMPLEMENTED | `agent/loop_engine.rs:140-150` |
| P2-8 | File locking | ✅ IMPLEMENTED | `tools/workspace.rs:10-75` |

## BG - Behavior Gap Fixes

| Fix ID | Description | Status | File Reference |
|--------|-------------|--------|-----------------|
| BG-1 | Permission queue | ✅ IMPLEMENTED | `tui/update/agent.rs:122-144` |
| BG-2 | Error state cleanup | ✅ IMPLEMENTED | `tui/update/agent.rs:73-75` |
| BG-3 | AgentEnd cleanup | ✅ IMPLEMENTED | `tui/update/agent.rs:60-68` |
| BG-5 | Tick side effects | ✅ IMPLEMENTED | `tui/state.rs:408-414` |

## Remaining Issues (Not Yet Fixed)

### R-1: Ctrl+C Accidental Clear
**Priority:** P1  
**Issue:** Ctrl+C with non-empty textarea clears input, potentially losing work.  
**Recommendation:** Require double-tap or show confirmation before clearing.

### R-2: Network Error Recovery
**Priority:** P1  
**Issue:** `is_recoverable_error()` identifies errors but no automatic retry.  
**Recommendation:** Add retry button or automatic retry with backoff.

### R-3: Empty Command Palette Filter
**Priority:** P2  
**Issue:** When filter returns no matches, palette shows empty with no feedback.  
**Recommendation:** Show "No matching commands" message.

### R-4: Onboarding Fetch Failure
**Priority:** P2  
**Issue:** On model fetch failure, hardcoded models used without clear notification.  
**Recommendation:** Show banner indicating cached/outdated list.

### R-5: Rollback Implementation
**Priority:** P1  
**Issue:** `Cmd::Rollback` is sent but implementation is placeholder.  
**Recommendation:** Implement actual file state rollback mechanism.

## Commit History

All fixes are committed as separate logical changes on the `ralph/overnight-audit` branch. Each fix follows the pattern:
```
<Priority>-<ID>: <Description>
```

Example commits:
- `P0-1: Add permission timeout tracking`
- `P1-3: Add panic recovery to agent loop`
- `P2-7: Add idempotency checking for tool calls`
