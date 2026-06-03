# UI Comparison Report: runie vs grok

## 1. Welcome Screen
**File:** `verify_01_welcome.txt` vs `01_welcome_screen.txt`

### ✅ MATCHING
- Menu items present: New worktree, Resume session, Quit
- Menu item order and text
- Keyboard shortcuts (ctrl-w, ctrl-s, ctrl-q)
- Tip text at bottom

### ❌ DIFFERENCES

| Aspect | runie | grok |
|--------|-------|------|
| Branch/directory line | **Missing** | ` feat/grok-redesign ~/Code/GitHub/runie/` |
| Input bar | **Missing** | Box with `❯` prompt and bottom border |
| Status bar footer | **Missing** | `Shift+Tab:mode  │  Ctrl+.:shortcuts` |
| Version number | `0.1.0 Beta` at col 70 | `0.2.16 Beta` at col 70 |
| Menu indentation | Left-heavy indent | Centered alignment |
| Bottom border | **Missing** | `╰─────────────────────────────── Grok Build ─╯` |

---

## 2. Home Screen (after Ctrl+H)
**File:** `verify_02_home.txt` vs `02_session_list.txt`

### ❌ CRITICAL MISMATCH

| Aspect | runie | grok |
|--------|-------|------|
| Content | **Shows welcome menu** (same as verify_01) | **Shows session list** with sessions |
| Session list | **Not shown** | GitHub-runie, test, grok, etc. |
| Search prompt | **Not shown** | `/ to search` |
| Navigation bar | **Not shown** | `Esc:back  │  Enter:select  │  ...` |

**ISSUE:** After Ctrl+H, runie does NOT show the session list/home dashboard. It re-displays the welcome menu.

---

## 3. Chat View (after pressing 'n')
**File:** `verify_03_chat.txt` vs `03_chat_view.txt`

### ❌ CRITICAL MISMATCH

| Aspect | runie | grok |
|--------|-------|------|
| Content | **Keyboard Shortcuts modal overlay** | **Actual chat view** with session header |
| Memory meter | **Not shown** | `9.5K / 512K` visible in header |
| Session title | **Not shown** | `❯ test  4:10 PM` |
| Thought content | **Not shown** | `◆ Thought for 0.1s` then `◆ List .` |
| Input bar | **Hidden behind modal** | `❯` prompt visible |
| Status bar | **Hidden behind modal** | `Shift+Tab:mode  │  Ctrl+.:shortcuts` |

**ISSUE:** Pressing 'n' opens keyboard shortcuts modal instead of creating a new chat session.

---

## 4. Typing State (after typing "hello world")
**File:** `verify_04_typing.txt` vs `05_input_typing.txt`

### ❌ CRITICAL MISMATCH

| Aspect | runie | grok |
|--------|-------|------|
| Content | **Keyboard Shortcuts modal** (different variant) | Input with typed text |
| What shows | `❯ hllw rHelp` (garbled) | `❯ .hello world` |
| Modal sections | Navigation, Actions, Session | N/A - no modal |
| Session info | **Not visible** | Visible in chat area |
| Status bar | **Not visible** | `Enter:send  │  Shift+Tab:mode  │  Ctrl+.:shortcuts` |

**ISSUE:** "hello world" appears garbled inside the shortcuts modal header (`hllw rHelp`). The 'e' characters are being consumed/overwritten.

---

## Summary of Issues

### Critical (Blocking)
1. **Home screen (Ctrl+H)** - Shows welcome menu instead of session list
2. **New session ('n')** - Opens keyboard shortcuts modal instead of chat
3. **Keyboard shortcuts modal** - Appears spontaneously, capturing keystrokes

### High Priority
4. **Missing branch/working directory line** on welcome/home screens
5. **Missing input bar** on welcome screen
6. **Missing status bar footer** on welcome/home screens
7. **Memory meter** not visible on home (but should be hidden per design)

### Medium Priority
8. **Garbled text** in shortcuts modal header when typing
9. **Menu alignment** - not centered like grok
10. **Missing bottom border** with "Grok Build" branding

---

## Specific Recommendations

1. **Fix Ctrl+H behavior** - Should display home/session list, not welcome menu
2. **Fix 'n' key behavior** - Should open new chat, not keyboard shortcuts modal
3. **Investigate keyboard shortcuts modal trigger** - Why does it appear spontaneously?
4. **Add missing UI elements** to welcome screen:
   - Branch/directory line at top
   - Input bar with `❯` prompt and box border
   - Status bar with mode shortcuts at bottom
5. **Fix garbled text issue** in the shortcuts modal header
6. **Ensure memory meter hides on home screen** but shows on chat screen
