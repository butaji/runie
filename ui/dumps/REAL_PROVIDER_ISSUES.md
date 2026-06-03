# Runie Real Provider vs Grok Comparison Report

## ISSUE 1: Input Text Accumulation (Ctrl+h displayed literally)

**Severity: HIGH**

### What Runie Shows
```
╭──────────────────────────────────────────────────────────────────────────╮
│ ❯ /qCtrl+h                                                               │
╰────────────────────────────────────────────────────────────────── runie ─╯
```

### What Grok Shows
```
╭──────────────────────────────────────────────────────────────────────────╮
│ ❯                                                                        │
╰──────────────────────────────────────────── Grok Build · always-approve ─╯
```

### Analysis
When user presses Escape to close slash menu, the input bar shows literal `/qCtrl+h` instead of being cleared or showing just `❯`. The Ctrl+h (ASCII 8, backspace) is being rendered as literal text rather than performing the backspace action.

### Root Cause
Escape key handler in slash menu mode is not properly consuming/handling the accumulated input. The `/q` prefix from typing `/quit` plus the Ctrl+h backspace sequence are being concatenated as visible characters.

### Fix Required
- File: `ui/src/components/InputBar.ts` or similar
- When Escape is pressed in slash/command mode, clear the input buffer completely before returning to normal mode
- Ensure Ctrl+h (0x08) is intercepted and treated as backspace, not printed

---

## ISSUE 2: Thinking Block Format

**Severity: MEDIUM**

### What Runie Shows
```
◦ The user said "sayhello". This is a simple request - they want me to
◦ say hello. I'll respond with a friendly greeting.
   ∘ Hello! 👋 How can I help you today?                           8:41 PM
```

### What Grok Shows
```
┃  ◆ Thinking…
┃
┃  …                                                                         █
┃  It's a Rust project called "runie", with multiple crates under crates/   █
┃  a TUI, CLI, AI components, etc. There's a target/ directory (build        █
┃  artifacts), docs/, harness/, etc.                                         █
```

### Differences
| Aspect | Runie | Grok |
|--------|-------|------|
| Bullet character | `◦` (U+25E6) | `┃` (U+2503) |
| Thinking prefix | `◦` + text | `┃  ◆ Thinking…` |
| Continuation | `◦` line-start | `┃  …` + `┃  text` |
| Agent response | `∘` (U+2219) | `⠧` (U+28A7) + text |
| Spinner | None shown | `⠧` (Braille pattern) |

### Fix Required
- File: `ui/src/components/MessageBlock.ts` or `ThinkingBlock.ts`
- Replace bullet character `◦` with box-drawing character `┃`
- Add `◆` prefix for "Thinking" label
- Use consistent `┃` prefix for all thinking lines
- Align with Grok's thinking block structure

---

## ISSUE 3: Help Panel (`?` key doesn't open help)

**Severity: MEDIUM**

### What Runie Shows
Status bar hint: `Shift+Tab:mode  │  Ctrl+.`
No visible help panel when `?` is pressed.

### What Grok Shows
Status bar hint: `Shift+Tab:mode  │  Ctrl+.:shortcuts`

### Analysis
Runie doesn't implement `?` key for help. The hint format is also incomplete - should show `:shortcuts` label.

### Fix Required
- File: `ui/src/App.ts` or input handler
- Implement `?` key binding to show shortcuts/help overlay
- File: status bar component to add `:shortcuts` suffix

---

## ISSUE 4: Slash Menu Escape Doesn't Close Cleanly

**Severity: HIGH**

### What Runie Shows (after Escape)
```
   ── Commands ────────────────────────────────────────────────────────────────
...
   ╭──────────────────────────────────────────────────────────────────────────╮
   │ ❯ /qCtrl+h                                                               │
   ╰────────────────────────────────────────────────────────────────── runie ─╯
```

### What Grok Shows
Slash menu not visible in Grok captures, but input clears properly.

### Analysis
When user types `/quit` then presses Escape:
1. Slash menu opens showing commands
2. User presses Escape to close
3. Input bar shows residual `/qCtrl+h`

### Root Cause
Slash command mode state not properly reset on Escape. Partial input from slash command (`/q`) plus Ctrl+h backspace sequence are visible.

### Fix Required
- File: `ui/src/components/SlashMenu.ts` or command mode handler
- Escape should: (1) clear accumulated input, (2) hide slash menu, (3) return to clean input state

---

## ISSUE 5: Message Spacing Inconsistency

**Severity: LOW**

### What Runie Shows (04_response.txt)
```
◦ The user said "sayhello". This is a simple request - they want me to
◦ say hello. I'll respond with a friendly greeting.
   ∘ Hello! 👋 How can I help you today?                           8:41 PM

                                                            2s ⇣53 [✓]
```

### What Grok Shows (04_thinking.txt)
```
Hello! I'm Grok, ready to help with your software engineering   6:37 PM   █
tasks in the /Users/admin/Code/GitHub/runie repo.                         █
...
Turn completed in 2.4s.                                                   █

```

### Differences
| Aspect | Runie | Grok |
|--------|-------|------|
| Response indent | `   ∘` (3 spaces) | None (full width) |
| Trailing indicator | `[✓]` | `[✗]` (for failed) |
| Status line | `2s ⇣53 [✓]` | `8.0s ⇣23.2k [✗]` |

### Fix Required
- File: `ui/src/components/MessageBlock.ts`
- Match Grok's full-width response format without indent
- Format: `∘ ` prefix still OK but without extra spaces

---

## ISSUE 6: Status Bar Hint Format Mismatch

**Severity: MEDIUM**

### What Runie Shows
```
Shift+Tab:mode  │  Ctrl+.:
```

### What Grok Shows
```
Shift+Tab:mode  │  Ctrl+.:shortcuts
```

### Differences
| Aspect | Runie | Grok |
|--------|-------|------|
| Hint text | `Ctrl+.:` (colon, no label) | `Ctrl+.:shortcuts` |
| Full hint | `Ctrl+.:` | `Ctrl+.:shortcuts` |

### Fix Required
- File: status bar component
- Add `:shortcuts` suffix to Ctrl+. hint
- Consistent format: `key:action`

---

## ISSUE 7: Tool Status Format

**Severity: HIGH**

### What Runie Shows (05_tools.txt)
```
      ∘ Hello! 👋 How can I help you today?                           8:41 PM

                                                             2s ⇣53 [✓]


      ❯ lstfles                                                        8:42 PM
```

After slash command (empty state):
```
╭──────────────────────────────────────────────────────────────────────────╮
│ ❯                                                                        │
╰────────────────────────────────────────────────────────────────── runie ─╯
Shift+Tab:mode  │  Ctrl+.:
```

### What Grok Shows (05_tools.txt)
```
     ⠧ Thinking… 1.5s                                           8.0s ⇣23.2k [✗]

╭──────────────────────────────────────────────────────────────────────────╮
│ ❯                                                                        │
╰──────────────────────────────────────────── Grok Build · always-approve ─╯

Shift+Tab:mode  │  Ctrl+c:cancel  │  Ctrl+Enter:interject  │  Ctrl+.:
```

### Differences
| Aspect | Runie | Grok |
|--------|-------|------|
| Thinking indicator | `◦` bullet | `⠧` (Braille spinner) |
| Tool status | None shown during tool | `⠧ Thinking… 1.5s  8.0s ⇣23.2k [✗]` |
| Status bar hints | `Shift+Tab:mode  │  Ctrl+.:` | Full hints with cancel/interject |
| Tool output | Just prompt | Shows thinking block + status |

### Analysis
Runie doesn't show tool/thinking status during execution. Grok shows:
1. Spinner with thinking text
2. Elapsed time (1.5s)
3. Total time (8.0s)
4. Download indicator (⇣23.2k)
5. Status ([✗] for failed)

### Fix Required
- File: `ui/src/components/ToolStatus.ts` or similar
- Add spinner animation during tool execution
- Show elapsed/total time
- Show data transfer indicator
- Show [✓] or [✗] status

---

## ISSUE 8: Status Bar Missing Action Hints

**Severity: MEDIUM**

### What Runie Shows
```
Shift+Tab:mode  │  Ctrl+.:
```

### What Grok Shows
```
Shift+Tab:mode  │  Ctrl+c:cancel  │  Ctrl+Enter:interject  │  Ctrl+.:
```

### Missing Hints in Runie
- `Ctrl+c:cancel` - Cancel current operation
- `Ctrl+Enter:interject` - Interject with user input

### Fix Required
- File: status bar component
- Add Ctrl+c and Ctrl+Enter hints when agent is running
- Only show relevant hints based on current state

---

## ISSUE 9: Memory/Token Display Format

**Severity: LOW**

### What Runie Shows
```
feat/grok-redesign                                    │ 4 / 128.0K │
```

### What Grok Shows
```
feat/grok-redesign ~/Code/GitHub/runie                     │ 7.6K / 512K │
```

### Differences
| Aspect | Runie | Grok |
|--------|-------|------|
| Branch display | Just branch name | Branch + repo path |
| Memory used | `4` (abstract units) | `7.6K` (KB) |
| Memory total | `128.0K` | `512K` |
| Scale | Small (128K max) | Larger (512K max) |

### Fix Required
- File: `ui/src/components/StatusBar.ts` or memory display component
- Add repo path to branch display
- Consider switching to KB units for consistency
- May need to align token limits with actual Grok limits

---

## ISSUE 10: Input Bar Prompt

**Severity: LOW**

### What Runie Shows
```
╭──────────────────────────────────────────────────────────────────────────╮
│ ❯                                                                        │
╰────────────────────────────────────────────────────────────────── runie ─╯
```

### What Grok Shows
```
╭──────────────────────────────────────────────────────────────────────────╮
│ ❯                                                                        │
╰──────────────────────────────────────────── Grok Build · always-approve ─╯
```

### Differences
| Aspect | Runie | Grok |
|--------|-------|------|
| Left prompt | None | `Grok Build · always-approve` |
| Right prompt | `runie` | (empty) |

### Fix Required
- File: `ui/src/components/InputBar.ts`
- Consider matching Grok's prompt style
- Or clarify what `runie` represents

---

## ISSUE 11: Version Display

**Severity: LOW**

### What Runie Shows
```
                                                                   0.1.0 Beta
```

### What Grok Shows
```
                                                           0.2.16 [stable] Beta
```

### Fix Required
- Update version number if this is a release discrepancy
- Consider adding `[stable]` tag for consistency

---

## ISSUE 12: Slash Menu Item Selection Indicator

**Severity: LOW**

### What Runie Shows
```
   ── Commands ────────────────────────────────────────────────────────────────
     ❯ /new                                                Start new session
       /clear                                             Clear conversation
```

### What Grok Shows
Not visible in captures.

### Analysis
Runie uses `❯` as selection indicator. The header `── Commands ───` uses em-dashes. The formatting is slightly different from Grok's style.

### Fix Required
- File: `ui/src/components/SlashMenu.ts`
- May need to align with Grok's slash menu style if visible in future captures

---

## ISSUE 13: Empty Lines Between Messages

**Severity: LOW**

### What Runie Shows
```
◦ The user said "sayhello". This is a simple request - they want me to
◦ say hello. I'll respond with a friendly greeting.
   ∘ Hello! 👋 How can I help you today?                           8:41 PM

                                                             2s ⇣53 [✓]


```

### What Grok Shows
```
Hello! I'm Grok, ready to help with your software engineering   6:37 PM   █

      Turn completed in 2.4s.                                                   █

```

### Fix Required
- File: message rendering component
- Ensure consistent blank line count between message groups
- Runie has 2 blank lines, may want 1

---

## SUMMARY: Priority Fixes

| Priority | Issue | Files to Modify |
|----------|-------|-----------------|
| P0 | Input text accumulation | `InputBar.ts`, command mode handler |
| P0 | Tool status not shown | `ToolStatus.ts`, agent state handler |
| P0 | Status bar missing hints | `StatusBar.ts` |
| P1 | Thinking block format | `ThinkingBlock.ts` |
| P1 | Slash menu Escape | `SlashMenu.ts` |
| P1 | Help panel `?` key | `App.ts` |
| P2 | Memory display format | `StatusBar.ts` |
| P2 | Input bar prompt | `InputBar.ts` |
| L3 | Version / stable tag | Version config |
| L3 | Message spacing | `MessageBlock.ts` |

---

## Files Likely Requiring Changes

```
ui/src/
├── App.ts                              # ? key handler, global key handling
├── components/
│   ├── InputBar.ts                     # Input text accumulation fix
│   ├── SlashMenu.ts                    # Escape handling, item display
│   ├── StatusBar.ts                    # Hint format, memory display
│   ├── ThinkingBlock.ts                # ◦ → ┃ format change
│   ├── ToolStatus.ts                   # New: tool execution status
│   └── MessageBlock.ts                # Message spacing, response format
└── state/
    └── agent.ts                        # Tool running state for hints
```
