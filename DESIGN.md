# Tidy TUI Design System v2.0 — "GrokCrush" Hybrid

## Philosophy

Tidy is the love child of Grok Build (precise, ratatui-engineered, multi-layer professional control center) and Crush (vibrant Charm glam, personality-forward, energetic). The result is a premium terminal-native agentic IDE that feels like a cyberpunk command deck: surgically precise yet delightfully alive.

**Tagline vibe:** "Precise. Electric. Yours."

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

### Base (Deep Space)

| Token | Hex | Usage | WCAG Ratio |
|-------|-----|-------|------------|
| `bg.base` | `#0a0a0f` | Main background — deepest black with cool undertone | — |
| `bg.panel` | `#111118` | Panel/card backgrounds — barely lifted from base | — |
| `bg.code` | `#16161f` | Code block backgrounds | — |
| `bg.overlay` | `#0d0d14` | Modal backdrop — 70% opacity dark tint | — |
| `bg.highlight` | `#1e1e2e` | Hover/selection states | — |
| `bg.selection` | `#2a2a3c` | Active selection | — |

### Text (Cool White Hierarchy)

| Token | Hex | Usage | Ratio on `#0a0a0f` |
|-------|-----|-------|-------------------|
| `text.primary` | `#e0e0ff` | Primary text — soft cool white | ~14:1 |
| `text.secondary` | `#a0a0cc` | Body text, comfortable for long reading | ~8:1 |
| `text.muted` | `#6e6e99` | Labels, hints, disabled state | ~4.8:1 |
| `text.dim` | `#444466` | Subtle borders, separators, inactive | ~2.5:1 |

### Accents (Electric + Glam)

| Token | Hex | Usage |
|-------|-----|-------|
| `accent.primary` | `#00f0ff` | **Electric Cyan** — primary brand, active states, chevrons, focus rings, running indicators |
| `accent.secondary` | `#ff2aff` | **Hot Magenta** — secondary highlights, user input pills, brand moments (sparingly) |
| `accent.tertiary` | `#b14cff` | **Purple** — gradients, agent identity, model badges |

### Semantic (Clear State Communication)

| Token | Hex | Usage |
|-------|-----|-------|
| `success` | `#39ff8c` | Success, approved plans, completed agents, passing tests |
| `warning` | `#ff9d3a` | Warnings, pending approvals, attention needed |
| `error` | `#ff4d6d` | Errors, failures, destructive actions, rejected plans |
| `info` | `#00f0ff` | Same as accent.primary — running processes, active tools, thinking state |

### Diffs

| Token | Hex | Usage |
|-------|-----|-------|
| `diff.added` | `#39ff8c` | Added lines in diff view |
| `diff.removed` | `#ff4d6d` | Removed lines in diff view |
| `diff.hunk` | `#00f0ff` | Diff hunk headers |
| `diff.context` | `#6e6e99` | Unchanged context lines |

### Borders

| Token | Hex | Usage |
|-------|-----|-------|
| `border.focused` | `#00f0ff` | Active/focused borders — bright cyan glow |
| `border.unfocused` | `#444466` | Inactive borders — dim gray |
| `border.accent` | `#ff2aff` | Special accent borders — magenta for user content |

### Gradients (Simulated via Color Blocks)

| Name | Start | End | Usage |
|------|-------|-----|-------|
| `gradient.cyan-magenta` | `#00f0ff` | `#ff2aff` | User message pills, progress bars |
| `gradient.purple-cyan` | `#b14cff` | `#00f0ff` | Agent identity, model badges |
| `gradient.status` | `#b14cff` | `#00f0ff` | Thinking/thought indicators |

### Contrast Rules

- **Body text**: `text.secondary` on `bg.base` — ratio ~8:1, comfortable for 8+ hour sessions
- **Critical/headers**: `text.primary` — ratio ~14:1, maximum readability
- **Accents for recognition**: `accent.primary` (cyan) creates immediate landmarks for active elements
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
| `▌` | Bookmark bar | `accent.primary` | Left-edge attention marker on active lines |
| `▸` | Triangle right | `text.muted` → `accent.primary` | List items, collapsible headers (muted when closed, cyan when open) |
| `▼` | Triangle down | `accent.primary` | Expanded sections |
| `●` | Filled circle | `accent.primary` (pulsing) | Running/active state |
| `○` | Empty circle | `text.muted` | Waiting/idle state |
| `◐` | Half circle left | `accent.primary` | Animation frame 1 |
| `◑` | Half circle right | `accent.primary` | Animation frame 2 |
| `✓` | Check | `success` | Completed, approved, success |
| `✗` | Cross | `error` | Failed, rejected, error |
| `✎` | Pencil | `accent.secondary` | Edit in progress |
| `◆` | Diamond | `accent.tertiary` | Important marker, system events |
| `⟐` | Circled dot | `accent.primary` | Thinking/processing state (alternative to ●) |
| `◌` | Dotted circle | `text.muted` | Subagent traces, background tasks |
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
┃                             ┃ ● coder   ┃
┃   User message        ←──── ┃   editing ┃
┃                             ┃   cl· 45s ┃
┃   Assistant reply           ┃ ········· ┃
┃                             ┃ ○ test    ┃
┃   thinking... 2.3s          ┃   running ┃
┃                             ┃   gp· 12s ┃
┃   ▼ Tool: read_file()       ┃           ┃
┃   ✓ read_file: done         ┃           ┃
┃                             ┃           ┃
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
   - **Center Feed**: Message history, tool calls, thoughts
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
- Focused: border changes to `border.focused` (cyan)
- `❯` prompt in `accent.primary` (cyan)
- Hardware cursor: `SteadyBar` (thin line)
- Bottom-right info: model name, token count (e.g., "model: claude-4 · 4.2k tokens")
- Info format: `─ text ─` with single spaces
- Multi-line: `Shift+Enter` or `Ctrl+J`
- Each logical line = 1 visual line (no wrapping)

### User Messages (Gradient Pills)

```
                    ╭────── message ──────╮
                    │◗  user text here   ◖│
                    ╰─────────────────────╯
```

**Design:**
- Right-aligned within feed
- Rounded pill shape (`╭╮╰╯` or `◗`/`◖` caps)
- Gradient background: `accent.tertiary` (purple) → `accent.secondary` (magenta)
- Text: `text.primary` (cool white)
- Simulated gradient via per-cell color interpolation
- No `❯` prompt inside pill — clean text only

### Assistant Messages

```
  Assistant text here in primary color
  spanning multiple lines if needed
  
  ```rust
  1 │ code block with syntax highlighting
  2 │ using bg.code background
  ```
```

**Design:**
- Left-aligned, full width (minus padding)
- Text: `text.primary`
- No background (uses `bg.base`)
- Code blocks: `bg.code` background, line numbers in `text.muted`
- Syntax highlighting via Opaline `code.*` tokens

### Tool Calls

```
  ▼ Tool: read_file (collapsed)
  ▶ Tool: write_file (collapsed)
  
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
- Success: `✓` icon in `success` color
- Error: `✗` icon in `error` color

### Thought Bubbles

```
  ⟐ thinking... 2.3s
```

**Design:**
- Italic text in `text.muted`
- Pulsing `⟐` or `●` indicator (cycles `○◐◑●`)
- Duration updates every 100ms
- Gradient accent bar on left: `accent.tertiary` → `accent.primary`

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
  - Failed: `✗` in `error`
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
| **Thinking indicator** | `⟐` → `◉` → `⟐` | 800ms cycle | Subtle pulse |
| **Typing dots** | `·` → `··` → `···` | 400ms cycle | Append dots cyclically |
| **Progress fill** | `░` → `▒` → `█` | Per segment | Fill blocks left to right |
| **Modal appear** | Shadow → Content | 2 frames | Draw shadow first, then content |
| **Status change** | Color wipe | Instant | Change color on next frame |
| **Border glow** | Dim → Bright | Focus event | Change border color on focus |

### Specific Behaviors

- **Agent running**: `●` pulses with gradient `accent.tertiary` → `accent.primary`
- **Progress bars**: `███████░░░` with gradient from `accent.tertiary` to `accent.primary`
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
