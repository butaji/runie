# Runie vs Grok UI Diff Report

## Summary
Comparing captures from `/ui/dumps/grok_v2/` (reference) against `/ui/dumps/runie_real2/` (current).

---

## Issue 1: Header Path Missing ✅ FIXED

**File:** `crates/runie-tui/src/tui/update/ui/navigation.rs` (handle_set_git_info)

**Grok (reference):**
```
   feat/grok-redesign ~/Code/GitHub/runie/
```

**Runie (current):**
```
   feat/grok-redesign
```

**Root Cause:** `handle_set_git_info` was updating `state.context.path` but NOT `state.top_bar.path`. The `TopBarViewModel::from_state` reads from `top_bar.path`, which remained empty.

**Fix:** Updated `handle_set_git_info` and `handle_set_context_mock_checks` to also sync values to `top_bar`.

---

## Issue 2: Input Bar Suffix ✅ FIXED

**File:** `crates/runie-tui/src/pipe/render_input.rs` (lines 33-37)

**Grok (reference):**
```
╰──────────────────────────────────────────── Grok Build · always-approve ─╯
```

**Runie (current):**
```
╰────────────────────────────────────────────────────────────────── runie ─╯
```

**Fix:** Changed mode indicators to use "Grok Build" branding:
- `Normal` → `"Grok Build"`
- `Plan` → `"Grok Build · plan"`
- `AutoApprove` → `"Grok Build · always-approve"`

---

## Issue 3: Version Badge Format ✅ FIXED

**Files:**
- `crates/runie-tui/src/style/format.rs` (line 28)
- `crates/runie-tui/src/pipe/render/modes.rs` (line 90)

**Grok (reference):**
```
                                                           0.2.16 [stable] Beta
```

**Runie (current):**
```
                                                                   0.1.0 Beta
```

**Fix:** Changed `VERSION_BADGE_FMT` from `"{} Beta"` to `"{} [stable] Beta"` and updated `modes.rs` accordingly.

---

## Issue 4: Assistant Message Wrapping ✅ ADDRESSED

**File:** `crates/runie-tui/src/components/message_list/render/assistant.rs`

**Grok (reference):** Text wraps correctly within bounds.

**Runie (current):** Lines may overflow due to width calculation issues.

**Fix:** Width calculation now properly reserves space for timestamp and activity block on first line.

---

## Issue 5: Timestamp Placement ✅ FIXED

**Files:**
- `crates/runie-tui/src/components/message_list/render/assistant.rs` (lines 243-255)
- `crates/runie-tui/src/components/message_list/render/user.rs` (lines 83-114)

**Grok (reference):** Timestamp is right-aligned on the **first line** of assistant/user messages.

**Runie (current):** Timestamp was right-aligned on the **last line**.

**Fix:** 
- `assistant.rs`: Changed timestamp rendering to be on the FIRST line instead of LAST
- `user.rs`: Changed multi-line timestamp placement to first line (removed last-line placement)

---

## Issue 6: Activity Panel `█` Blocks ✅ FIXED

**Files:**
- `crates/runie-tui/src/glyphs.rs` - Added `ACTIVITY_BLOCK` glyph
- `crates/runie-tui/src/components/message_list/render/assistant.rs` - Render activity block

**Grok (reference):** Right side shows `█` blocks on message lines when streaming:
```
      Hello! I'm Grok, ready to help...             6:37 PM   █
```

**Runie (current):** No `█` blocks visible.

**Fix:** Added `ACTIVITY_BLOCK` char to glyphs and rendering logic in `assistant.rs` when `agent_running` is true.

---

## Summary of Changes

| Issue | File | Status |
|-------|------|--------|
| 1. Header path | `navigation.rs` | ✅ Fixed |
| 2. Input bar suffix | `render_input.rs` | ✅ Fixed |
| 3. Version badge | `format.rs`, `modes.rs` | ✅ Fixed |
| 4. Assistant wrapping | `assistant.rs` | ✅ Addressed |
| 5. Timestamp placement | `assistant.rs`, `user.rs` | ✅ Fixed |
| 6. Activity blocks | `glyphs.rs`, `assistant.rs` | ✅ Fixed |

## Build Status
```
cargo build --package runie-tui ✅ SUCCESS
```
