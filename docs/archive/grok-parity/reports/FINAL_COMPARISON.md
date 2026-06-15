# Grok vs Runie Visual Comparison

## Chat State

### Header
```
Grok:    feat/grok-redesign ~/Code/GitHub/runie              │ 20K / 512K │
Runie:   main ~/Code/GitHub/runie                          │ 0 / 128.0K │
```

**Differences:**
- Branch name: `feat/grok-redesign` vs `main` (context-dependent, expected)
- Token format: `20K` vs `0` (Grok drops decimal for 0, Runie shows `.0K`)
- Max tokens: `512K` vs `128.0K` (2x difference)
- Whitespace alignment: Grok has tighter spacing before `│`

**Parity: 60%** — Layout matches, values differ (context-dependent)

---

### User Message
```
Grok:      ❯ list the files in this directory              4:37 PM
Runie:     ❯ lst                                            5:26 PM
```

**Differences:**
- Bullet: Both use `❯`
- Indent: Both use 6 spaces
- Timestamp: Both use `H:MM PM` format
- Content differs (expected — different test inputs)

**Parity: 100%** — Format matches

---

### Assistant Message
```
Grok:     ⠼ Waiting… 0.3s                              4.9s ⇣20.9k [✗]
Runie:    ∘ I received your message: "lst". This is a mock response for te5:26 PM
Runie:                                                        0s ⇣0 [✓]
```

**Differences:**
- Bullet: `⠼` (Braille pattern) vs `∘` (ring operator)
- Status: `Waiting…` with spinner vs text response
- Duration: `0.3s` (elapsed) vs `0s` (indeterminate)
- Timestamp: Inline at end of message vs trailing status on separate line
- Status format: `⇣20.9k [✗]` vs `⇣0 [✓]`
- Grok shows tool timing (`4.9s`), Runie shows `0s`

**Parity: 40%** — Significant structural divergence

---

### Tool Call / Thought Block
```
Grok:      ◆ Thought for 0.8s
Grok:    ❙  ◆ List .
Runie:   (none in dump)
```

**Differences:**
- Grok has explicit thought/thinking indicator with duration
- Grok has tool call prefix `❙` and bullet `◆`
- Runie has no tool call block in this dump

**Parity: 0%** — Runie dump lacks tool call state

---

### Input Bar
```
Both:   ╭──────────────────────────────────────────────────────────────────────────╮
Both:   │ ❯                                                                │
Grok:   ╰───────────────────────────────────────────────────────────── Grok Build ─╯
Runie:  ╰────────────────────────────────────────────────────────────────── runie ─╯
```

**Differences:**
- Footer text: `Grok Build` vs `runie`
- Footer alignment: Both right-padded to ~60 chars, slight length difference

**Parity: 90%** — Near-identical structure

---

### Status Bar Hints
```
Grok:   Shift+Tab:mode  │  Ctrl+c:cancel  │  Ctrl+Enter:interject  │  Ctrl+.:
Runie:  Shift+Tab:mode  │  Ctrl+.:shortcuts
```

**Differences:**
- Number of hints: 4 vs 2
- Grok has `Ctrl+c:cancel` and `Ctrl+Enter:interject` missing in Runie
- Runie condenses to `Ctrl+.:shortcuts`

**Parity: 50%** — Partial match only

---

## Slash Menu State

### Border Style
```
Grok:   (uses horizontal dividers `──` between sections)
Runie:  (uses horizontal dividers `──` between sections)
```

**Differences:**
- Grok divider length: 87 chars (`87─`)
- Runie divider length: 79 chars
- Grok has `❯` prefix on selected item, `█` suffix marker
- Runie uses `❯` prefix on selected item only

**Parity: 70%** — Similar approach, different lengths/markers

---

### Selected Indicator
```
Grok:      ❯ /quit                             Quit the application                █
Runie:     ❯ /new                                                Start new session
```

**Differences:**
- Grok: `❯` + description + `█` marker (selection = `█`)
- Runie: `❯` prefix only (visual selection via `❯`)
- Selection marker: `█` vs implicit `❯` position

**Parity: 60%** — Both indicate selection differently

---

### Command Description Alignment
```
Grok:   /quit                             Quit the application
Runie:  /new                                                Start new session
```

**Differences:**
- Description position: Both right-pad to similar width
- Grok has 2-line descriptions for some commands (`/fork` wraps)
- Runie keeps descriptions single-line

**Parity: 80%** — Similar alignment approach

---

### Menu Content Overlap
```
Grok:   /quit, /home, /new, /fork, /compact
Runie:  /new, /clear, /tree, /fork, /home, /resume, /sessions, /rename, /share, /session-info
```

**Differences:**
- Grok commands: 5 items
- Runie commands: 10 items
- Runie has more commands (`/clear`, `/tree`, `/resume`, `/sessions`, `/rename`, `/share`, `/session-info`)
- Grok has `/compact` not in Runie

**Parity: 30%** — Minimal overlap in command set

---

## Welcome Screen State

### Menu Items
```
Grok:   New worktree                   ctrl-w
        Resume session                 ctrl-s
        Quit                           ctrl-q

Runie:  ▸ New worktree             (ctrl-w)
          Start a parallel ...
        Resume session           (ctrl-s)
          Continue where yo...
        Quit                     (ctrl-q)
          Exit runie
```

**Differences:**
- Selected indicator: `❯` vs `▸` (Runie uses `▸` for unselected)
- Keyboard hint format: `ctrl-w` (lowercase, no parens) vs `(ctrl-w)` (parens)
- Descriptions: Grok has none inline, Runie has descriptions on next line
- Runie has extra descriptions for each item

**Parity: 50%** — Similar items, different presentation

---

### Dividers
```
Grok:   ─────────────────────────────────────
Runie:  ─────────────────────────────────────
```

**Differences:**
- Length: Grok 37 chars, Runie 39 chars
- Position: Grok has blank line before/after, Runie same

**Parity: 85%** — Near-identical

---

### Tip Banner
```
Grok:   Tip: Press Ctrl-W to start a parallel task in its own worktree.
Runie:  Tip: Press Ctrl-W to start a parallel task in its own worktree.
```

**Differences:**
- None — identical text

**Parity: 100%** — Exact match

---

### Version Badge
```
Grok:                                                           0.2.16 Beta
Runie:                                                          0.1.0 Beta
```

**Differences:**
- Version: `0.2.16 Beta` vs `0.1.0 Beta`
- Position: Both right-aligned at column ~65, but Runie has more preceding blank space

**Parity: 90%** — Same position, different version

---

## Overall Parity Score

| Element | Parity |
|---------|--------|
| Chat Header | 60% |
| User Message | 100% |
| Assistant Message | 40% |
| Tool Call Block | 0% |
| Input Bar | 90% |
| Status Bar Hints | 50% |
| Slash Menu Border | 70% |
| Slash Selected Indicator | 60% |
| Slash Description Alignment | 80% |
| Slash Command Overlap | 30% |
| Welcome Menu Items | 50% |
| Welcome Dividers | 85% |
| Welcome Tip Banner | 100% |
| Welcome Version Badge | 90% |

**Overall Parity Score: 65%**

---

## Key Divergences Requiring Attention

1. **Status Bar Hints** — Runie missing `Ctrl+c:cancel` and `Ctrl+Enter:interject`
2. **Assistant Message Format** — Completely different: spinner vs text, different status placement
3. **Slash Menu Commands** — Minimal overlap (5 vs 10 commands)
4. **Tool Call Block** — Missing in Runie dump (need to verify state exists)
5. **Welcome Descriptions** — Runie has inline descriptions, Grok does not
6. **Keyboard Hint Format** — `ctrl-w` vs `(ctrl-w)` inconsistency

## Recommendations

1. Add missing status bar hints to match Grok's completeness
2. Standardize keyboard hint format across both UIs (prefer `(ctrl-w)` style)
3. Align assistant message structure — consider Grok's spinner approach for tool states
4. Review slash menu command parity — Grok's `/compact` vs Runie's additional commands
5. Verify tool call block rendering in Runie for parity with Grok's `◆` + `❙` pattern
