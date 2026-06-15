# Gap Analysis: runie vs Grok Build TUI

## Overview

After extensive UI capture and comparison, here are the remaining gaps between runie's TUI and Grok's actual interface.

---

## 1. WELCOME SCREEN

### Header Line
| Aspect | runie | Grok | Gap |
|--------|-------|------|-----|
| Format | `runiemain  src/components` | `   feat/grok-redesign ~/Code/GitHub/runie/` | **HIGH** |
| Leading spaces | None | 2 spaces | Minor |
| Prefix | "runie" text before  | None | **HIGH** |
| Branch | Always "main" | Actual git branch | **HIGH** |
| Path | Relative ("src/components") | Full path with `~` | **HIGH** |
| Memory meter | Missing | Hidden on welcome (correct) | OK |

### Menu Items
| Aspect | runie | Grok | Gap |
|--------|-------|------|-----|
| Selection indicator | `❯ New worktree` | `New worktree` (no indicator) | **MEDIUM** |
| Alignment | Left-heavy (~12 spaces) | Centered (~22 spaces) | **MEDIUM** |
| Divider width | Full width to hint | Centered, shorter | **LOW** |
| Hint alignment | `ctrl-w` at col 52 | `ctrl-w` at col 47 | **LOW** |

### Layout
| Aspect | runie | Grok | Gap |
|--------|-------|------|-----|
| Vertical spacing | 21 lines total | 24 lines total | **LOW** |
| Blank lines after menu | 3 | 6 | **LOW** |
| Tip indentation | 10 spaces | 2 spaces | **LOW** |

### Bottom Area
| Aspect | runie | Grok | Gap |
|--------|-------|------|-----|
| Version badge position | Before input bar (line 18) | After input bar (line 23) | **MEDIUM** |
| Input bar left padding | `│❯` (no space) | `│ ❯` (space) | **LOW** |
| Bottom label | `runie ─ mock` | `Grok Build` | **LOW** |

---

## 2. CHAT SCREEN

### Header
| Aspect | runie | Grok | Gap |
|--------|-------|------|-----|
| Format | Similar | ` branch ~/path │ X.XK / XXXK │` | **MEDIUM** |
| Memory meter | Not visible | Visible | **HIGH** |

### Message Area
| Aspect | runie | Grok | Gap |
|--------|-------|------|-----|
| User messages | Unknown | `❯ test    4:10 PM` | **HIGH** |
| Thinking blocks | Now has box borders | Box borders | ✅ FIXED |
| Tool calls | Unknown | `◆ List .` | **HIGH** |
| Session title | Unknown | `❯ test` with timestamp | **HIGH** |

### Status Bar
| Aspect | runie | Grok | Gap |
|--------|-------|------|-----|
| Visible | Not captured correctly | `Shift+Tab:mode │ Ctrl+.:shortcuts` | **HIGH** |
| Context-aware | Unknown | Changes based on state | **HIGH** |

---

## 3. SLASH MENU

### From Dumps (Both Show Same Content)
- ✅ Menu content matches
- ✅ Selection indicator `❯` matches
- ✅ Layout with `█` progress bar on right matches
- ✅ Commands and descriptions match

**Status: NEARLY IDENTICAL**

---

## 4. COMMAND PALETTE

### From Dumps (Both Show Same Content)
- ✅ Modal structure matches
- ✅ Title `Commands` with `[✗]` close button
- ✅ Search field
- ✅ Category headers
- ✅ Commands with keyboard shortcuts
- ✅ Footer hints

**Status: NEARLY IDENTICAL**

---

## 5. CRITICAL FUNCTIONAL GAPS

These are behavioral differences, not just visual:

| # | Issue | Impact |
|---|-------|--------|
| 1 | **'n' key opens keyboard shortcuts instead of new session** | **CRITICAL** |
| 2 | **Chat view not accessible** - Can't get to actual chat screen | **CRITICAL** |
| 3 | **Header shows wrong info** - "runie" prefix, wrong branch, wrong path | **HIGH** |
| 4 | **Home screen is 24 lines vs grok's 24** - but spacing distribution differs | **MEDIUM** |
| 5 | **No session list view** - Grok has session browser after Ctrl+H | **HIGH** |
| 6 | **Keyboard shortcuts modal appears unexpectedly** | **HIGH** |

---

## 6. THEME / COLOR GAPS

Unable to verify from text dumps, but likely differences:
- Color values may not match Grok's exact hex codes
- Accent colors might differ
- Border colors might differ
- Background shades might differ

---

## Summary

### What's Working ✅
1. Home screen menu structure (3 items, correct labels)
2. Keyboard hints (ctrl-w, ctrl-s, ctrl-q)
3. Separator dividers
4. Tip text content
5. Input bar with `❯` prompt
6. Slash menu layout and content
7. Command palette layout
8. Thinking block box borders (fixed)
9. Header memory meter hidden on welcome (fixed)
10. Status bar hidden on welcome (fixed)

### What's Broken ❌
1. **Navigation**: Can't access chat screen (keyboard shortcuts modal blocks)
2. **Header**: Wrong format, wrong branch, wrong path
3. **Selection indicator**: Shows `❯` on menu items where grok doesn't
4. **Spacing**: Different vertical distribution
5. **Session list**: No session browser view
6. **Chat view**: Not captured/working

### Top Priority Fixes
1. Fix 'n' key to create new session instead of opening shortcuts
2. Fix header to show ` branch ~/path` format without "runie" prefix
3. Add session list view (accessible from welcome screen)
4. Fix menu alignment to be centered
5. Remove `❯` selection indicator from menu items
6. Capture and verify chat view works correctly
