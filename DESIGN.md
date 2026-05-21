# Tidy TUI Design System v3.0 — "Crush+Grok" Hybrid

## Philosophy

Tidy is the love child of Crush (Charm's glam Lip Gloss base — vibrant, personality-forward, electric) and GrokBuild (cosmic truth-seeking, terminal-native, high-signal-density). The result is a premium terminal-native agentic IDE that feels like a cyberpunk command deck: surgically precise yet delightfully alive.

**Tagline vibe:** "Cosmic. Precise. Electric."

### Core Rules (Non-Negotiable)

1. **Terminal as Canvas** — Respect grid constraints. No fake shadows unless via Unicode/ANSI. All updates buttery-smooth (ratatui double-buffering).
2. **Information Density** — 70% content, 20% whitespace/breathing room, 10% accents/glyphs.
3. **High Contrast First** — Minimum 4.5:1 WCAG contrast. Body text must be readable for 8+ hour sessions.
4. **Keyboard + Vim First** — All actions accessible via keyboard. Mouse is optional enhancement.
5. **Semantic Color = Instant State** — Color conveys meaning without reading. No decorative color.
6. **Max 3 Accents On Screen** — Never overload. One primary, one secondary, one semantic (success/error/warning).
7. **Flicker-Free** — All rendering happens off-screen via ratatui buffers. No partial draws.
8. **Alt-Screen Full Takeover** — Enter alternate screen on startup. No scrollback. Clean exit restores terminal.
9. **60+ FPS Target** — Re-render loop at 60Hz minimum. No blocking operations on main thread.
10. **No Cursor Blink in Static Areas** — Hardware cursor only in input bar. Static content areas show no cursor.
11. **Bold Only for Headings + Active Status** — Body text never bold. Bold reserved for section headers and active/current state indicators.
12. **Underlines Only for Links/Focus** — No underlined body text. Underline reserved for hyperlinks and keyboard focus indicators.

---

## Color Palette

### Base (Cosmic Deep Space)

| Token | Hex | Usage | WCAG Ratio |
|-------|-----|-------|------------|
| `bg.base` | `#0F0C14` | Main background — Cosmic deep space black-purple | — |
| `bg.panel` | `#201F26` | Panel/card backgrounds — Pepper charcoal purple | — |
| `bg.code` | `#1A1920` | Code block backgrounds — subtle gray panel | — |
| `bg.overlay` | `#0F0C14` | Modal backdrop — 70% opacity Cosmic tint | — |
| `bg.highlight` | `#2A2833` | Hover/selection states | — |
| `bg.selection` | `#3A3943` | Active selection | — |

### Text (Opaline Hierarchy)

| Token | Hex | Usage | Ratio on `#0F0C14` |
|-------|-----|-------|-------------------|
| `text.primary` | `#FFFAF1` | Primary text — Butter bright warm white | ~15:1 |
| `text.secondary` | `#DFDBDD` | Body text — Ash comfortable for long reading | ~9:1 |
| `text.muted` | `#BFBCC8` | Labels, hints, disabled — Smoke gray-purple | ~5:1 |
| `text.dim` | `#6B6878` | Subtle borders, separators, inactive | ~3:1 |

### Accents (Grok Electric)

| Token | Hex | Usage |
|-------|-----|-------|
| `accent.primary` | `#FF6B00` | **Grok Orange** — action, user messages, primary brand moments |
| `accent.secondary` | `#00F5D4` | **Neon Teal** — insight, assistant responses, success states |
| `accent.tertiary` | `#FF60FF` | **Dolly Pink** — agent identity, special highlights |

### Semantic (Clear State Communication)

| Token | Hex | Usage |
|-------|-----|-------|
| `success` | `#00F5D4` | Neon Teal — success, approved plans, completed agents |
| `warning` | `#FF6B00` | Grok Orange — warnings, pending approvals, attention |
| `error` | `#EB4268` | Sriracha red — errors, failures, destructive actions |
| `info` | `#00F5D4` | Same as accent.secondary — running processes, tools |

### Diffs

| Token | Hex | Usage |
|-------|-----|-------|
| `diff.added` | `#00F5D4` | Added lines in diff view (Neon Teal) |
| `diff.removed` | `#EB4268` | Removed lines in diff view (Sriracha) |
| `diff.hunk` | `#FF6B00` | Diff hunk headers (Grok Orange) |
| `diff.context` | `#BFBCC8` | Unchanged context lines (Smoke) |

### Borders

| Token | Hex | Usage |
|-------|-----|-------|
| `border.focused` | `#FF6B00` | Active/focused borders — Grok Orange glow |
| `border.unfocused` | `#3A3943` | Inactive borders — Charcoal |
| `border.accent` | `#FF6B00` | Special accent borders — Grok Orange for user content |

### Contrast Rules

- **Body text**: `text.secondary` (Ash) on `bg.base` (Cosmic) — ratio ~9:1, comfortable for 8+ hour sessions
- **Critical/headers**: `text.primary` (Butter) — ratio ~15:1, maximum readability
- **Orange for action**: `accent.primary` (Grok Orange) creates immediate landmarks for user input, active states
- **Teal for insight**: `accent.secondary` (Neon Teal) marks assistant responses, success states
- **Muted for background**: `text.dim` for borders, separators, inactive elements only
- **Semantic for state**: Success/error/warning convey status without reading text
- **Never use low contrast for body text** — only for decorative/inactive elements

---

## Visual Language

### Glyphs & Symbols (The Complete Set)

#### Core Box Drawing

| Weight | Horizontal | Vertical | Top-Left | Top-Right | Bottom-Left | Bottom-Right | Usage |
|--------|-----------|----------|----------|-----------|-------------|--------------|-------|
| Heavy | `━` | `┃` | `┏` | `┓` | `┗` | `┛` | Active panels, main borders |
| Light | `─` | `│` | `┌` | `┐` | `└` | `┘` | Inactive panels, separators |
| Rounded | `─` | `│` | `╭` | `╮` | `╰` | `╯` | Floating modals, user content |
| Tee | — | — | `┣` | `┫` | `┳` | `┻` | Tree branches, panel joins |

#### Status & State Glyphs

| Symbol | Name | Color Token | Usage |
|--------|------|-------------|-------|
| `❯` | Chevron | `accent.primary` | Input prompt, active navigation |
| `▌` | Left bar | `feed.*.bar` | Message timeline left edge marker (2 chars wide) |
| `▸` | Triangle right | `text.muted` → `accent.primary` | List items, collapsible headers |
| `▼` | Triangle down | `accent.primary` | Expanded sections |
| `●` | Filled circle | `feed.user.bar` | User message glyph |
| `◆` | Diamond | `feed.assistant.bar` | Assistant message glyph |
| `⟳` | Circled arrow | `feed.tool.bar` | Tool call glyph |
| `▶` | Triangle right fill | `feed.agent.bar` | Agent/thinking glyph |
| `⋯` | Mid horizontal dots | `feed.system.bar` | System/event glyph |
| `○` | Empty circle | `text.muted` | Waiting/idle state |
| `◐` | Half circle left | `accent.primary` | Animation frame 1 |
| `◑` | Half circle right | `accent.primary` | Animation frame 2 |
| `✓` | Check | `success` | Completed, approved, success |
| `×` | Cross | `error` | Failed, rejected, error |
| `→` | Arrow right | `text.secondary` | Tool result continuation |
| `│` | Vertical line | `feed.tool.bar` | Tool result continuation bar |
| `✎` | Pencil | `accent.tertiary` | Edit in progress |
| `◌` | Dotted circle | `text.muted` | Background tasks |
| `▒` | Medium shade | `text.dim` | Modal backdrop dimming |
| `░` | Light shade | `text.dim` | Shadow edges |

#### Progress & Fill

| Symbol | Usage |
|--------|-------|
| `█` | Full block — filled progress |
| `▓` | Dark shade — partial progress |
| `▒` | Medium shade — partial progress |
| `░` | Light shade — empty progress |
| `·` | Middle dot — separator, inactive items |
| `•` | Bullet — active items in lists |

#### Navigation

| Symbol | Usage |
|--------|-------|
| `↑` | Scroll up indicator |
| `↓` | Scroll down indicator |
| `→` | Forward, next |
| `←` | Back, previous |
| `↵` | Enter/return action |

#### File & Code

| Symbol | Usage |
|--------|-------|
| `├` | Tree branch middle |
| `└` | Tree branch end |
| `│` | Tree vertical line |
| `+` | Added line prefix (diffs) |
| `-` | Removed line prefix (diffs) |

### Border Rules

1. **Main panels**: Heavy borders (`━┃┏┓┗┛`) in `border.unfocused`
2. **Active panel**: Heavy borders in `border.focused` (cyan)
3. **Floating modals**: Rounded borders (`╭╮╰╯`) in `border.unfocused`
4. **User content**: Rounded borders in `border.accent` (magenta)
5. **Internal separators**: Light borders (`─│`) in `text.dim`
6. **Never mix heavy and light in same panel** — choose one weight per container

### Shadow System

Modals cast a terminal-native shadow:
```
  ╭──────╮
  │Modal │░
  │      │░
  ╰──────╯░
   ░░░░░░░▒
```

- Right edge: `░` at x+1, same y range
- Bottom edge: `░` at y+1, same x range (offset by 1)
- Corner: `▒` at x+width, y+height
- Color: `text.dim` on `bg.base`

---

## Layout System

### Zones (The Blueprint)

```
┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
┃ repo/branch · path            checks ✓  ┃  TopBar (1 line)
┣━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━┫
┃                             ┃ AGENTS    ┃
┃   Center Feed               ┃━━━━━▶━━━━━┃
┃   (reverse chronological)   ┃ ● coder   ┃
┃                             ┃   editing ┃
┃   ▌ ● user message          ┃   cl· 45s ┃
┃   ───                       ┃ ········· ┃
┃   ▌ ◆ assistant reply      ┃ ○ test    ┃
┃   ───                       ┃   running ┃
┃   ▌ ▶ thinking... 2.3s     ┃   gp· 12s ┃
┃   ───                       ┃           ┃
┃   ▌ ⟳ tool_call(args)       ┃           ┃
┃   │ → ✓ result              ┃           ┃
┃   ───                       ┃           ┃
┣━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┻━━━━━━━━━━━┫
┃╭───────────────────────────────────────╮ ┃
┃❯ user input here                        ┃  InputBar (dynamic)
┃╰─────────── model: claude-4 ───────────╯ ┃
┣━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┫
┃ Enter send · ^b sidebar · ^k cmd · ^q q ┃  StatusBar (1 line)
┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛
```

### Composition Rules

1. **Top Bar** (1 line):
   - Left: repo name / branch / path in `text.secondary`
   - Right: status indicators (checks, progress %) in `accent.primary`
   - Heavy bottom border (`━`)

2. **Main Content Area** (60-70% width):
   - **Center Feed**: Reverse-chronological message timeline with color-coded left bars (`▌`)
   - **Overlays**: Plan reviewer, diff viewer, subagent traces (floating, centered)
   - Background: `bg.base`

3. **Right Sidebar** (30-40% width):
   - Agent cards, modified files, model info, token counts
   - Background: `bg.panel`
   - Heavy left border (`┃`) separating from main
   - Collapsible sections

4. **Input Bar** (dynamic height):
   - Rounded corners (`╭╮╰╯`)
   - `❯` prompt in `accent.primary`
   - Info text at bottom-right (model name, token count)
   - Minimum 3 lines (border + 1 line + border)

5. **Status Bar** (1 line):
   - Keyboard shortcuts in `text.muted`
   - Active key in `accent.primary`
   - Heavy top border (`━`)

### Spacing

- **Screen margin**: 1 char all sides (tighter than before for density)
- **Between zones**: 0 lines (touching, separated by borders)
- **Inside panels**: 1 char padding minimum, 2 chars for breathing room
- **Between cards**: 1 blank line
- **Between messages**: 1 blank line

### Responsive Behavior

| Width | Behavior |
|-------|----------|
| < 80 cols | Sidebar collapses to overlay (toggle with `^b`) |
| < 60 cols | Minimal mode: hide agent cards, show only status icons |
| < 40 cols | Compact status bar, single-line input only |
| < 30 cols | Error: "Terminal too small. Minimum 30 columns required." |

### Z-Depth Hierarchy

1. `bg.base` — Frame background
2. `bg.panel` — Sidebar background
3. Main content (messages, code)
4. Modal backdrop (`bg.overlay` + `▒` dim)
5. Modal shadow (`░` offset)
6. Modal content (floating panels)
7. Cursor (hardware)

---

## Components

### Input Bar

```
╭──────────────────────────────────────────╮
│❯ user input here                          │
│  second line if multi-line                │
╰─────────── model: claude-4 ──────────────╯
```

**Design:**
- Rounded corners (`╭╮╰╯`) in `border.unfocused`
- Focused: border changes to `border.focused` (Grok Orange)
- `❯` prompt in `accent.primary` (Grok Orange)
- Hardware cursor: `SteadyBar` (thin line)
- Bottom-right info: model name, token count (e.g., "model: claude-4 · 4.2k tokens")
- Info format: `─ text ─` with single spaces
- Multi-line: `Shift+Enter` or `Ctrl+J`
- Each logical line = 1 visual line (no wrapping)

### Message Feed (Vertical Timeline)

The center feed displays a reverse-chronological timeline of messages with color-coded left bars.

**Layout Rules:**
- Newest messages at TOP (reverse chronological order)
- Left margin: 2 characters before left bar
- Left bar: thick `▌` glyph (2 chars wide), color-coded per message type
- Separator: `───` in `feed.separator` between entries
- No emoji — only Unicode glyphs

#### User Message

```
▌ ● Hello, can you help me with this code?
───
```

**Design:**
- Left bar: `▌` in `feed.user.bar` (Grok Orange)
- Glyph: `●` in `feed.user.bar` (orange)
- Text: `text.primary` (Butter bright warm white)
- Background: `feed.user.bg` (subtle gray panel)
- Timestamp: right-aligned in `text.muted` (optional)

#### Assistant Response

```
▌ ◆ I'll help you with that. Let me read the file first.
───
```

**Design:**
- Left bar: `▌` in `feed.assistant.bar` (Neon Teal)
- Glyph: `◆` in `feed.assistant.bar` (teal)
- Text: `text.secondary` (Ash body text)
- Background: transparent (`bg.base`)

#### Tool Call

```
▌ ⟳ read_file({"path": "src/main.rs"})
│ → ✓ File contents loaded successfully
───
```

**Design:**
- Left bar: `▌` in `feed.tool.bar` (Charple purple)
- Glyph: `⟳` in `feed.tool.bar` (purple)
- Header: `toolname(args)` in `text.secondary`
- Result line: `│` continuation bar + `→` + `✓`/`×` + result text
- Success: `✓` in `success` (Neon Teal)
- Fail: `×` in `error` (Sriracha red)

#### Agent/Thinking

```
▌ ▶ Step 1: analyzing codebase structure...
───
```

**Design:**
- Left bar: `▌` in `feed.agent.bar` (Dolly Pink)
- Glyph: `▶` in `feed.agent.bar` (pink)
- Text: italic in `text.muted`

#### System/Event

```
▌ ⋯ Mock mode active — no API key needed
───
```

**Design:**
- Left bar: `▌` in `feed.system.bar` (Charcoal)
- Glyph: `⋯` in `feed.system.bar` (charcoal)
- Text: `text.muted`

### Tool Calls (Collapsed View)

```
▼ Tool: read_file
▶ Tool: write_file

▼ Tool: bash
│ $ cargo test
│    Compiling tidy-core v0.1.0
│    Finished test [unoptimized]
│     Running unittests
│ 
│ test result: ok. 78 passed
│
```

**Design:**
- Collapsible header: `▼`/`▶` + "Tool: name" in `text.muted`
- Expanded: command/output in `bg.code` with left border
- Success: `✓` icon in `success` color (Neon Teal)
- Error: `×` icon in `error` color (Sriracha red)

### Thought/Thinking State

```
  ▶ thinking... 2.3s
```

**Design:**
- Italic text in `text.muted`
- Pulsing `▶` or `●` indicator
- Duration updates every 100ms
- Left bar in `feed.agent.bar` (Dolly Pink)

### Agent Cards (Sidebar)

```
┏━━━━━━━━━━━━━━━━━━━━━━━━┓
┃ ● coder                ┃
┃   editing files        ┃
┃   claude-4 · 45s       ┃
┃························┃
┃ ○ test                 ┃
┃   running tests        ┃
┃   gpt-4 · 12s          ┃
┗━━━━━━━━━━━━━━━━━━━━━━━━┛
```

**Design:**
- Heavy borders (`┏┓┗┛`) in `border.unfocused`
- Header "AGENTS" in `accent.primary` with bold
- Status icon on left:
  - Running: `●` pulsing in `accent.primary`
  - Completed: `✓` in `success`
  - Failed: `×` in `error`
  - Waiting: `○` in `text.muted`
- Agent name: `text.primary` bold
- Description: `text.secondary`
- Model + duration: `text.muted`
- Separator between cards: `·` dots in `text.dim`

### Plan Review Overlay

```
    ╭────────────────────────────────────╮
    ╭─ Review Plan ───────────── [Esc] ─╮
    │                                    │
    │  1. Read file src/main.rs          │
    │  2. Edit function handle_input()   │
    │  3. Run tests                      │
    │                                    │
    │  [Ctrl+Y] Approve  [Ctrl+N] Reject │
    ╰────────────────────────────────────╯
```

**Design:**
- Floating modal, centered
- Rounded corners (`╭╮╰╯`)
- Shadow offset 1 cell right/down
- Backdrop: `bg.overlay` with `▒` dim
- Title: `accent.primary` bold
- Plan items: numbered list in `text.secondary`
- Actions: `[key]` in `accent.primary`, description in `text.secondary`

### Diff Viewer Overlay

```
    ╭────────────────────────────────────╮
    ╭─ Diff: src/main.rs ───── [Esc] ───╮
    │                                    │
    │  @@ -45,7 +45,7 @@                │
    │  -    let x = old_value;           │
    │  +    let x = new_value;           │
    │       println!("{}", x);           │
    │                                    │
    │  [↑/↓] navigate  [Enter] apply     │
    ╰────────────────────────────────────╯
```

**Design:**
- Hunk headers: `accent.primary` (cyan)
- Removed lines: `diff.removed` (red) background
- Added lines: `diff.added` (green) background
- Context lines: `text.secondary`
- Line numbers: `text.muted`

### Command Palette

```
    ╭────────────────────────────────────╮
    ╭─ Command Palette ─────── [Esc] ───╮
    │                                    │
    │  OBJECT    ACTION     ARGS         │
    │  ▸ File...                         │
    │    Agent...                        │
    │    Model...                        │
    │                                    │
    │  ▸ type to filter...               │
    │  [↑↓] navigate  [Enter] select     │
    ╰────────────────────────────────────╯
```

**Design:**
- Active pane header: `accent.primary` bold
- Selected item: `▸` indicator + `text.primary`
- Unselected: `text.muted`
- Query input: `❯` prompt in `accent.primary`

### Permission Modal

```
    ╭────────────────────────────────────╮
    ╭─ Permission Required ─── [Esc] ───╮
    │                                    │
    │  Tool: bash                        │
    │  Args: rm -rf /                    │
    │                                    │
    │  This will delete all files.       │
    │                                    │
    │  [Y] Confirm  [N] Cancel           │
    │  [A] Always   [S] Skip             │
    ╰────────────────────────────────────╯
```

**Design:**
- Red left accent bar (`▌`) indicating danger
- Title: `warning` color (orange)
- Tool name: `accent.primary` bold
- Args: `code.path` (cyan) in code style
- Warning text: `text.secondary`
- Actions: selected = `accent.primary`, unselected = `text.muted`

---

## Animations & Motion

### Principles

1. **Subtle** — 200-300ms perception, not attention-grabbing
2. **Functional** — Convey state changes only
3. **Terminal-friendly** — Stepped frames, no smooth transitions
4. **No strobing** — Never flash faster than 500ms

### Animation Definitions

| Animation | Frames | Timing | Implementation |
|-----------|--------|--------|----------------|
| **Running pulse** | `●` → `◐` → `◑` → `●` | 500ms cycle | Cycle through chars on timer |
| **Thinking indicator** | `▶` → `▌` → `▶` | 800ms cycle | Subtle pulse in `feed.agent.bar` |
| **Typing dots** | `·` → `··` → `···` | 400ms cycle | Append dots cyclically |
| **Progress fill** | `░` → `▒` → `█` | Per segment | Fill blocks left to right |
| **Modal appear** | Shadow → Content | 2 frames | Draw shadow first, then content |
| **Status change** | Color wipe | Instant | Change color on next frame |
| **Border glow** | Dim → Bright | Focus event | Change border color on focus |

### Specific Behaviors

- **Agent running**: `●` pulses with gradient `feed.agent.bar` (Dolly Pink) → `accent.primary` (Grok Orange)
- **Progress bars**: `███████░░░` with gradient from `accent.tertiary` (Dolly Pink) to `accent.primary` (Grok Orange)
- **Indeterminate**: `◐◑◒◓` spins in `accent.primary`
- **Thinking**: Duration counter updates every 100ms ("thinking... 1.2s")
- **Sidebar toggle**: Instant show/hide (no animation — terminal constraint)
- **Scroll**: Instant jump with `↑`/`↓` ghost indicators at edges

---

## Typography

### Fonts

- **Required**: Monospace (grid-based rendering)
- **Recommended**: JetBrains Mono, Fira Code, Cascadia Code
- **Nerd Fonts**: Optional but recommended for extra glyphs
- **Minimum**: Unicode box-drawing + block elements support

### Text Styles

| Purpose | Style | Color |
|---------|-------|-------|
| Headers | Bold | `accent.primary` |
| Body | Normal | `text.secondary` |
| Primary content | Normal | `text.primary` |
| Code | Normal | Syntax-colored on `bg.code` |
| Labels | Normal | `text.muted` |
| Active/Selected | Bold | `accent.primary` |
| Error | Normal | `error` |
| Warning | Normal | `warning` |
| Success | Normal | `success` |
| Italic (thoughts) | Italic | `text.muted` |

### Line Height

- **Single spacing**: Messages, agent cards, list items
- **Double spacing**: Between major sections (after assistant reply, before next user message)
- **Compact mode**: All single spacing when terminal < 50 lines

---

## Interaction Patterns

### Keyboard-First (All Actions)

| Key | Action | Context |
|-----|--------|---------|
| `Enter` | Send message / Confirm | Chat / Modal |
| `Shift+Enter` | New line in input | Input bar |
| `Esc` | Close modal / Cancel | Any modal |
| `Ctrl+B` | Toggle sidebar | Chat |
| `Ctrl+K` / `Ctrl+P` | Command palette | Chat |
| `Ctrl+Y` | Approve plan | Plan review modal |
| `Ctrl+N` | Reject plan | Plan review modal |
| `Ctrl+C` / `Ctrl+Q` | Quit | Any |
| `↑/↓` | Navigate lists / Scroll | Lists / Feed |
| `j/k` | Vim-style navigate | Lists / Feed |
| `Tab` | Next field / focus | Forms / Palette |
| `?` | Show help overlay | Chat |
| `gg` | Go to top | Feed |
| `G` | Go to bottom | Feed |

### Bash-Style Input (Readline)

| Key | Action |
|-----|--------|
| `Ctrl+A` | Start of line |
| `Ctrl+E` | End of line |
| `Ctrl+W` | Delete word backward |
| `Ctrl+U` | Delete to start of line |
| `Ctrl+K` | Delete to end of line |
| `Ctrl+D` | Forward delete char |
| `Ctrl+B` | Back char |
| `Ctrl+F` | Forward char |
| `Ctrl+N` | Next line (multi-line) |
| `Ctrl+P` | Previous line (multi-line) |
| `Alt+B` | Back word |
| `Alt+F` | Forward word |

### Mouse Support (Optional Enhancement)

- Click to focus panels
- Click to expand/collapse tool calls
- Click to approve/reject in modals
- Scroll wheel for feed navigation

---

## Theme System

All colors via Opaline semantic tokens:

```rust
// Colors
theme.color("bg.base")           // Background
theme.color("text.primary")      // Primary text
theme.color("accent.primary")    // Electric cyan
theme.color("accent.secondary")  // Hot magenta
theme.color("success")           // Green

// Convert to ratatui Color
let fg: ratatui::style::Color = theme.color("text.primary").into();
```

### Adding Themes

1. Create `.toml` theme file in Opaline format
2. Register in theme discovery
3. All components auto-adapt

**No hardcoded colors anywhere.**

---

## Implementation Notes

### Rendering Order

1. Clear frame with `bg.base`
2. Draw top bar (heavy bottom border)
3. Draw sidebar (heavy left border, agent cards)
4. Draw center feed (messages, code, thoughts)
5. Draw input bar (rounded borders)
6. Draw status bar (heavy top border)
7. Draw modal backdrop (`bg.overlay` + `▒`)
8. Draw modal shadow (`░` offset)
9. Draw modal content
10. Position hardware cursor

### Performance

- Re-render entire frame every tick
- Minimize allocations in render loop
- Cache theme lookups
- Use `Buffer::empty()` for off-screen compositing
- No partial redraws (ratatui handles optimization)

### Testing Requirements

- Render output verification
- State machine transitions
- Keyboard input handling
- Edge cases (empty, overflow, wrap)
- Contrast ratio verification (automated)

---

## Flow States

### Planning State

```
┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
┃ repo/main · src/              planning ● ┃
┣━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━┫
┃ ╭───────────────────────╮   ┃ AGENTS    ┃
┃ │ Plan: Refactor auth   │   ┃━━━━━▶━━━━━┃
┃ │ 1. Read auth.rs       │   ┃ ● planner ┃
┃ │ 2. Extract validate() │   ┃   planning┃
┃ │ 3. Update tests       │   ┃   cl· 3s  ┃
┃ │                       │   ┃           ┃
┃ │ [Ctrl+Y] Approve      │   ┃           ┃
┃ │ [Ctrl+N] Reject       │   ┃           ┃
┃ ╰───────────────────────╯   ┃           ┃
┃                             ┃           ┃
┣━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┻━━━━━━━━━━━┫
┃╭───────────────────────────────────────╮ ┃
┃❯ approve plan                           ┃
┃╰─────────── model: claude-4 ───────────╯ ┃
┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛
```

### Editing State

```
┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
┃ repo/main · src/auth.rs        editing ● ┃
┣━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━┫
┃ @@ -45,7 +45,7 @@           ┃ AGENTS    ┃
┃  -    let x = old;           ┃━━━━━▶━━━━━┃
┃  +    let x = new;           ┃ ● editor  ┃
┃       println!("{}", x);     ┃   editing ┃
┃                              ┃   cl· 12s ┃
┃ [↑/↓] navigate [Enter] apply ┃           ┃
┃                             ┃           ┃
┣━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┻━━━━━━━━━━━┫
┃╭───────────────────────────────────────╮ ┃
┃❯ apply changes                          ┃
┃╰─────────── model: claude-4 ───────────╯ ┃
┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛
```

### Running State

```
┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
┃ repo/main · src/                running ● ┃
┣━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━┫
┃                             ┃ AGENTS    ┃
┃   Running tests...          ┃━━━━━▶━━━━━┃
┃                             ┃ ● runner  ┃
┃   $ cargo test              ┃   running ┃
┃      Compiling...           ┃   cl· 8s  ┃
┃      Finished               ┃           ┃
┃      Running 78 tests       ┃           ┃
┃      test result: ok        ┃           ┃
┃                             ┃           ┃
┣━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┻━━━━━━━━━━━┫
┃╭───────────────────────────────────────╮ ┃
┃❯                                        ┃
┃╰─────────── model: claude-4 ───────────╯ ┃
┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛
```

---

## References

- **Grok Build** — Precise engineering, multi-layer overlays, plan review, diff viewer, status micro-details
- **Crush** — Vibrant glam, electric accents, personality-forward, playful glyphs
- **btop** — Visual density, color-coded data, gradient bars, system monitor aesthetics
- **Opaline** — Token-based theming, semantic color naming
- **Ratatui** — Rust TUI framework, buffer-based rendering, flicker-free
- **Crossterm** — Cross-platform terminal control, input handling
