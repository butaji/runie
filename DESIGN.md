# Runie TUI Design System

> Codified design patterns for Runie, informed by Grok Build, Claude Code, Pi.dev, and Codex CLI.

---

## 1. Design Philosophy

**Terminal-native minimalism**
- Everything renders in monospace terminal output. No GUI widgets, no pseudo-graphical chrome.
- Constraints breed clarity: if it can't be expressed in characters, it doesn't belong.

**Actions over explanations**
- Display what was done, not what it means.
- Show `git commit -m "fix auth"` not "Successfully committed your changes to the repository."

**Instant feedback**
- Every keystroke produces immediate visual response.
- Long operations show progress; completion is silent.

---

## 2. Message Formatting Rules

### User Messages
- Prefix with `❯` followed by a single space
- Compact: no wrapping blank lines, no decorative padding
- Example:
  ```
  ❯ implement user auth with JWT
  ```

### Assistant Messages
- Plain text, left-aligned, no boxes, no borders
- No leading/trailing blank lines unless separating logical sections
- Example:
  ```
  Added `auth/jwt.ts` with HS256 signing and expiry middleware.
  ```

### Think Blocks
- Indented with `  · ` (two spaces, middle dot, one space)
- Bullets only — no numbered lists, no headings, no borders
- Compact: 1-3 lines preferred, expand only if reasoning is non-trivial
- Example:
  ```
    · Need to validate the token expiry before granting access
    · Refresh token flow adds complexity — skip for now
  ```

### Thought Duration Tag
- Inline compact tag placed right of last think bullet
- Format: `  thought 0.9s`
- No brackets, no labels, no color
- Only shown when reasoning exceeds 0.5s

### Turn Separator
- Compact inline tag at start of new turn
- Format: `[turn: 3s, 2tc, ⇣97]`
  - `3s` — wall clock since last turn
  - `2tc` — tool call count
  - `⇣97` — tokens incoming (down arrow, truncated to 2-3 chars)
- No horizontal rules, no blank line above/below

### Tool Calls
- Single-line summary: `● name · args`
- Example: `● Read · src/auth/jwt.ts`
- Results indented with `└` tree prefix (standard └─ glyph)
- Example:
  ```
  ● Read · src/auth/jwt.ts
  └ 42 lines · middleware exported, expiresIn config present
  ```

### Blank Lines
- **Zero blank lines** between messages of different types
- Single blank line only within multi-line content (think blocks, multi-line output)
- Example (tight sequence):
  ```
  ❯ fix the login redirect
    · The redirect URI must match exactly what the OAuth provider expects
    · Could be query param ordering — sort params before signing
    thought 0.3s
  [turn: 4s, 1tc, ⇣82]
  ● Write · src/auth/oauth.ts
  └ Updated redirect() to sort query params alphabetically
  ```

---

## 3. Visual Hierarchy

### What Not to Use
- No full-width borders (—, ─, ═, or box-drawing characters spanning width)
- No diamond bullets `◆` or heavy ornamentation
- No shaded/background blocks (█, ▓, ░) for decoration
- No ASCII art dividers between sections

### Color Allocation
| Element | Color | Notes |
|---|---|---|
| User input | Bright (white/default) | Stands out as origin |
| Content / output | Default | Core message |
| Metadata | Muted gray | Timestamps, counts, tags |
| Additions | Green | Diffs, confirmations |
| Deletions | Red | Diffs, errors |
| Active indicator | Bright / accent | Current operation |

### Status vs Content
- Metadata lives at edges (top bar, bottom bar, inline tags)
- Content fills the terminal — no centering, no max-width
- Single-line status indicators only — never multi-line status blocks

### Density
- Compact by default. Prefer 1 line over 2.
- If a piece of information requires more than 3 lines to display, prefer summarization.

---

## 4. Interaction Patterns

### Keyboard-First
- No mouse required for any action
- All functionality accessible via keyboard
- If a feature requires explanation to discover, it needs a shortcut

### Hotkey Visibility
- Hotkeys shown in status bar at all times
- Never hidden in a menu — always visible in the chrome
- Format: `^b` means Ctrl+b, `M-x` means Alt+x

### Slash Commands
- `/` triggers command palette
- Common commands: `/commit`, `/diff`, `/search`, `/branch`, `/abort`
- Tab completion on slash commands
- Power users can chain: `/commit -m "fix" && /push`

### Command Palette
- Fuzzy search across all available commands
- Shows hotkey binding inline with command description
- Recent commands surfaced first
- Trigger: `^k` (or `/` from empty input)

### Sidebar
- Toggle with `^b`
- Shows: file tree, git status, branch info, recent turns
- Compact: single-char indicators for status (! modified, + added, ● active)

---

## 5. Output Principles

### File Changes — Inline Diffs
- Show diffs inline, not in a separate pane
- Format: `green +` for additions, `red -` for deletions
- Context: 2-3 lines max, truncated with `...` if large
- Example:
  ```
  - const secret = process.env.JWT_SECRET;
  + const secret = await getSecret('jwt-signing-key');
  ```
- Summary line: `3 files changed, +47, -12`

### Tool Calls — Single-Line Summaries
- Never dump raw tool output
- Summarize what happened in one line with key details
- Details accessible via drill-down (`Enter` on the call)

### Reasoning — Compact Indented Text
- No bordered boxes around thought process
- 2-space indent, `·` bullet, one idea per line
- Total think block should not exceed the visible terminal height

### Verbose Explanations
- **Never** volunteered — only shown when user asks (`why?`, `explain`, `verbose`)
- Default: "Done." or "Updated 3 files." is sufficient.
- If forced to explain: lead with what changed, not what it means.

### Error Display
- Show error message and relevant context in one block
- No stack traces by default (accessible via `^e` for expand)
- Suggest correction if obvious: `git push` failed → `hint: did you forget to commit?`

---

## 6. Status Display

### Top Bar
```
repo/branch/path  tokens/window  gauge%
```
- Left: current repository, branch, and working directory (truncated to fit)
- Center: token count / context window limit
- Right: memory gauge (percentage of context used)
- Format is single line, space-separated
- Example: `runie/main/src  42k/200k  21%`

### Bottom Bar
```
Enter send | ^b sidebar | ^k cmd | ? help | ^q quit
```
- Fixed position, always visible
- Hotkeys left-aligned, pipe-separated
- Only show hotkeys relevant to current mode (e.g., no `^b` if sidebar is open)

### Live Indicator
- Shown when an operation is in progress
- Format: `● Working (12s)` — bullet, label, elapsed seconds in parens
- Updates every second
- Replaces the turn separator on the current line
- Disappears silently when operation completes (no "Done." message)

### Mode Indicator
- When in sidebar: `[sidebar]` prefix in bottom bar
- When in palette: `[palette]` prefix, input field at top of list
- When recording a macro: `[recording: 3]`

---

## 7. Animation Rules

### Completed Items
- **No spinner on completed items** — ever
- Completion is silent: the work appearing is the feedback

### Active Operations
- Only animate current/active operations
- One active animation at a time — never multiple concurrent spinners

### Matrix Rain
- Background easter egg / ambient animation
- **Ultra-slow**: one cell updated per second (not per frame)
- Uses braille characters (⠂ ⠄ ⠂ ⠄) for density
- Runs only when terminal is idle (no pending user input)
- Toggle with `M-m` (Alt+m)

### Braille Spinner
- Single braille character cycling: `⠋ → ⠙ → ⠹ → ⠸ → ⠼ → ⠴ → ⠦ → ⠧ → ⠇ → ⠏`
- Only on actively running operations (not on completed ones)
- Appears inline where the result will print
- Example: `● Working ⠋` → after 0.5s → `● Working ⠙`

### Cursor
- Block cursor in input mode (`█` replacement)
- Underline cursor in navigation/palette mode
- Blinking rate: 530ms (CSS standard)

---

## 8. Component Quick Reference

### Turn Structure (single turn)
```
[turn: Xs, Ytc, ⇣Z]
❯ user input
  · think bullet
  · think bullet
  thought Xs
● ToolName · args
└ result summary
assistant response
```

### Status Bar Layout
```
top:    repo/branch/path  tokens/window  gauge%
bottom: Enter send | ^b sidebar | ^k cmd | ? help | ^q quit
```

### Live Operation
```
● Working (0s)
● Working (1s)
● Working (2s)
  └ tool result appears here (spinner gone)
```

### Sidebar (^b)
```
● src/
  ! auth/jwt.ts
  ! routes/login.ts
  + utils/async.ts
```

---

## 9. Anti-Patterns (Do Not Use)

| Pattern | Reason | Alternative |
|---|---|---|
| Full-width border line | Breaks terminal flow, dated aesthetic | No border |
| `◆` diamond bullet | Heavy, decorative, unnecessary | `·` or `-` |
| Spinner after completion | Redundant, noisy | Silent |
| Multi-line status block | Wastes vertical space | Single line |
| Bordered box around thought | Imitates GUI, breaks monospace | 2-space indent |
| "Done." / "Success." messages | Verbose for TTY | Work speaks for itself |
| Box-drawing frames around output | 1990s terminal aesthetic | Plain text |
| Multiple concurrent animations | Overwhelming, distracting | One at a time |
| Verbose explanations unprompted | Contradicts minimal design | Shorthand summary |

---

## 10. Implementation Notes

- All rendering assumes a standard 80-column terminal (graceful at 40)
- Unicode support required (❯, ·, ⇣, ●, └, braille chars)
- Color: use 256-color palette, avoid truecolor unless guaranteed
- Output buffering: flush on newline for live operation updates
- Accessibility: ensure sufficient contrast for gray metadata text
