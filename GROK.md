# Grok Build TUI - UI Documentation

**Version:** 0.2.14 Beta  
**Model:** Grok 4.3 (xAI, April 2026)

---

## Overview

Grok Build is xAI's terminal-based AI coding assistant featuring a sophisticated Terminal User Interface (TUI). The interface combines a dark, modern aesthetic with efficient keyboard-driven navigation, real-time tool execution visualization, and extensive customization options.

---

## Design System

### Color Philosophy

Grok uses a **theme-driven color system** with automatic terminal capability detection:

| Terminal Level | Color Support |
|----------------|---------------|
| **Truecolor (24-bit)** | Full RGB color, all themes render as designed |
| **256-color** | RGB values mapped to nearest palette entry |
| **16-color** | Colors mapped to ANSI names |

All themes are defined with full RGB values and automatically quantized at startup to match detected capability. This ensures consistent appearance across all terminal types.

### Available Themes

| Theme | Aliases | Description |
|-------|---------|-------------|
| **GrokNight** | `groknight`, `dark` | Neutral gray base with accent colors. Default dark theme. Survives 256-color quantization cleanly. |
| **GrokDay** | `grokday`, `light`, `day` | Light theme for bright terminal backgrounds |
| **TokyoNight** | `tokyonight`, `tokyo` | Blue-tinted backgrounds (requires truecolor) |
| **RosePineMoon** | `rosepine`, `rose-pine` | Warm, muted palette (requires truecolor) |

### Color Slots (Theme System)

**Backgrounds:**
- `bg_base` - Main viewport background
- `bg_light` - Lighter background variant
- `bg_dark` - Darker background variant
- `bg_highlight` - Selection/hover highlight
- `bg_hover` - Mouse hover state
- `bg_terminal` - Embedded terminal blocks

**Accents (Vertical Bars):**
- `accent_user` - User prompt accent (cyan/teal)
- `accent_assistant` - Assistant response accent
- `accent_thinking` - Reasoning/thinking block accent
- `accent_tool` - Tool call execution accent
- `accent_system` - System messages
- `accent_error` - Error states (red)
- `accent_success` - Success indicators (green)
- `accent_running` - Active/running indicator (animated)
- `accent_skill` - Skill invocations
- `accent_plan` - Plan mode indicator
- `accent_feedback` - Feedback/approval prompts
- `accent_model` - Model switch indicator

**Text:**
- `text_primary` - Main body text
- `text_secondary` - Secondary/muted text

**Semantic:**
- `command` - Shell commands
- `path` - File paths
- `running` - In-progress indicator
- `warning` - Warning messages
- `fuzzy_accent` - Fuzzy search highlights

---

## UI Elements

### 1. Header Bar (Status Line)

```
   main ~/Code/GitHub/runie                                    │ 21K / 512K │
```

- **Git Branch Indicator:** `` (branch symbol) + branch name
- **Current Directory:** Path relative to home (`~`)
- **Token Meter:** `21K / 512K` - Current/total token budget
- **Activity Indicator:** Animated spinner when processing

### 2. Welcome Screen

```
                      New worktree                   ctrl-w
                      ─────────────────────────────────────
                      Resume session                 ctrl-s
                      ─────────────────────────────────────
                      Quit                           ctrl-q

  Tip: Press Ctrl-W to start a parallel task in its own worktree.
```

**Elements:**
- **Menu Items:** Horizontal dividers (`───`) between options
- **Keyboard Hints:** Shown in lowercase (`ctrl-w` format)
- **Tip Banner:** Contextual help at bottom
- **Version Badge:** Bottom-right corner (`0.2.14 Beta`)

### 3. Scrollback (Main Conversation Area)

The scrollback displays the conversation history with collapsible blocks:

```
     ❯ grok                                                         11:28 PM

     ◆ Thought for 0.9s
     ◆ Read ~/.grok/docs/user-guide/README.md
     ◆ Read Cargo.toml
     ◆ List .
     ◆ Search "AGENTS\\.md|Claude\\.md|Agent\\.md|Agents\\.md" (no files)

     ⠴ Run List `.` 2.9s                                         11s ⇣22.2k [✗]
```

**Block Types:**

| Symbol | Type | Description |
|--------|------|-------------|
| `❯` | User Prompt | Current/active prompt entry |
| `◆` | Tool Call | Thinking, file operations, searches |
| `∘` | Assistant Response | Text/markdown response |
| `⠦⠴⠋⠼` | Animations | Spinner frames for running states |
| `✓` / `✗` | Status | Success/failure indicators |

**Block Indicators:**
- `⇣22.2k` - Downloaded bytes (streaming)
- `11s` - Duration/elapsed time
- `[✗]` - Error/failure state

### 4. Activity Panel (Right Side)

```
                                                                               █
                                                                               █
     ◆ List .                                                                  █
     ◆ Read /Users/admin/.grok/docs/user-guide/README.md                       █
     ◆ Read Cargo.toml                                                         █
     ◆ List .                                                                  █
                                                                               █
```

- **Real-time Tool Stream:** Shows currently executing tools
- **Progress Bars:** ASCII progress indicators (`█` blocks)
- **Auto-scrolling:** Follows agent activity

### 5. Input Prompt

```
  ╭──────────────────────────────────────────────────────────────────────────╮
  │ ❯                                                                        │
  ╰───────────────────────────────────────────────────────────── Grok Build ─╯
```

**Structure:**
- **Top Border:** `╭───...───╮` with title
- **Prompt Symbol:** `❯` (filled triangle)
- **Input Area:** User input space
- **Bottom Border:** `╰───...───╯` with version info

**Title Variants:**
- `Grok Build` - Normal mode
- `Grok Build · plan` - Plan mode
- `Grok Build · always-approve` - Auto-approve mode

### 6. Shortcuts Bar (Contextual Footer)

```
  Shift+Tab:mode  │  Ctrl+.:shortcuts
```

Contextual hints that change based on:
- Current focus (scrollback vs prompt)
- Agent state (running/idle)
- Selected entry type

**Common Hints:**
| Context | Shortcuts |
|---------|----------|
| Idle Prompt | `Shift+Tab:mode` `Ctrl+.:shortcuts` |
| Agent Running | `Shift+Tab:mode` `Ctrl+c:cancel` `Ctrl+Enter:interject` `Ctrl+.:` |
| Scrollback | `Ctrl+Shift+e:expand thinking` `Space:prompt` |

### 7. Mode Indicators

Modes displayed in header/title:

| Mode | Title Suffix | Description |
|------|--------------|-------------|
| Normal | (none) | Standard operation |
| Plan | `· plan` | Plan-only mode, shows reasoning without execution |
| Always-Approve | `· always-approve` | YOLO mode, auto-approves all actions |

### 8. Thinking Block

```
  ┃  ◆ Thinking…
  ┃
  ┃  The user said "list src". They want to list the source files, probably
  ┃  the src directories across the crates, or perhaps the main source
  ┃  structure.
```

- **Collapsible:** Can be folded with `h` or `e`
- **Accent Animation:** Vertical bar animates while reasoning
- **Header:** `Thinking...` (configurable)
- **Collapsed Indicator:** Shows truncated preview

### 9. Tool Call Block

```
  ⠴ Run List `.` 1.8s                                        5.7s ⇣21.2k [✗]
```

**Components:**
- **Spinner:** Animated progress indicator (`⠦⠴⠋⠼⠦`)
- **Label:** Tool name + arguments
- **Duration:** Time elapsed
- **Status:** Final duration + transfer + status icon

### 10. Response Block

```
     Want me to:
     • Show a specific crate in detail?
     • List only source files (exclude target/)?
     • Read a particular file (e.g. root Cargo.toml, README, or
     main entrypoint)?
```

- **Bullet Points:** `•` for lists
- **Tree Structure:** `└── tests/` for directory listings
- **Suggestions:** Next-step recommendations

---

## Styling Details

### Glyphs & Icons

| Glyph | Usage |
|-------|-------|
| `❯` | Prompt indicator, selection cursor |
| `◆` | Diamond bullet for tool calls |
| `∘` | Circle bullet for assistant messages |
| `›` | Navigation/expansion indicator |
| `│` | Accent vertical bar (collapsed state) |
| `─` | Horizontal dividers |
| `╭╮╰╯` | Box drawing corners |
| `──` | Box drawing horizontal edges |
| `│` | Box drawing vertical edges |
| `█` | Progress bar blocks |
| `⇣` | Download indicator |
| `⇡` | Upload indicator |
| `✓` | Success indicator |
| `✗` | Error indicator |
| `⛔` | Blocked action |
| `💬` | Comments/thinking |
| `📁` | Directory |
| `📄` | File |
| `🔍` | Search |
| `⚡` | Tool execution |
| `🧠` | Thinking/reasoning |

### Borders & Boxes

**Prompt Box:**
```
╭──────────────────────────────────────────────────────────────────────────╮
│ content                                                               │
╰───────────────────────────────────────────────────────────── Grok Build ─╯
```

**Welcome Menu:**
```
┌────────────────────────────────────────────────────────────────────────────┐
│                                                                            │
│   Menu items with horizontal dividers                                       │
│                                                                            │
└────────────────────────────────────────────────────────────────────────────┘
```

**Diff Blocks:**
```
  ├─────────────────────────────────────────────────────────────────────────
  │ old_fn() {
  ├─────────────────────────────────────────────────────────────────────────
  │+ new_fn() {
```

### Animations

| Animation | Element | Description |
|-----------|---------|-------------|
| Spinner | Running tools | `⠦⠴⠋⠼⠴⠦⠂⠇` cycle at ~8fps |
| Accent Wave | Thinking blocks | Vertical accent line animates while reasoning |
| Progress | Terminal output | `█` blocks fill left-to-right |
| Cursor | Input prompt | Block cursor blink |

**Animation Config:**
```toml
[animation]
fps = 30           # Frame rate (1-60)
wave_rows = 32     # Rows per wave cycle for accent animation
```

### Markdown Rendering

- **Code Blocks:** Syntax highlighted with matching theme
- **Headers:** Styled with heading colors (h1-h6)
- **Bold/Italic:** Proper emphasis rendering
- **Links:** Clickable in supported terminals
- **Task Lists:** Checkbox rendering with custom colors

### Diff Styling

```toml
[scrollback.blocks.edit]
hunk_separator = "..."
dual_line_numbers = false       # Two-column line numbers
line_summary = false            # Show +N/-M line counts
```

---

## Layout Structure

### Viewport Layout

```
┌────────────────────────────────────────────────────────────────────────────┐
│ [Header Bar: Git/Directory/Tokens]                                         │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│  [Scrollback Area]                    │ [Activity Panel]                   │
│  - User prompts                       │ - Real-time tools                  │
│  - Assistant responses                │ - Progress bars                    │
│  - Collapsible blocks                 │                                    │
│                                                                            │
│                                                                            │
├────────────────────────────────────────────────────────────────────────────┤
│ [Prompt Input Area]                                                        │
├────────────────────────────────────────────────────────────────────────────┤
│ [Shortcuts Bar: Contextual hints]                                          │
└────────────────────────────────────────────────────────────────────────────┘
```

### Responsive Behavior

| Mode | Layout |
|------|--------|
| Fullscreen | Complete viewport with alt-screen |
| Inline | Wraps in main terminal scrollback |
| Compact | Reduced padding, minimal margins |
| tmux | Uses tmux control mode for smooth rendering |

---

## Navigation

### Two Input Modes

**Simple Mode (Default):**
- Arrow keys for navigation
- `Shift+Arrow` for turn navigation
- `Space` to focus prompt
- Any printable key auto-focuses prompt

**Vim Mode (Opt-in):**
```toml
[ui]
vim_mode = true
```

| Vim Key | Action | Simple Equivalent |
|---------|--------|-------------------|
| `j/k` | Up/down entries | `Up/Down` |
| `H/L` | Prev/next turn | `Shift+Left/Right` |
| `h/l` | Fold/unfold | `Left/Right` |
| `e/E` | Toggle/expand all | - |
| `g/G` | Top/bottom | - |
| `i/Tab` | Focus prompt | `Tab` |
| `y/Y` | Copy content/metadata | - |
| `r` | Raw markdown | - |
| `o/O` | Open entry/options | - |

### Scrollback Navigation

| Key | Action |
|-----|--------|
| `j` / `Down` | Next entry |
| `k` / `Up` | Previous entry |
| `H` / `Shift+Left` | Jump to previous turn (user prompt) |
| `L` / `Shift+Right` | Jump to next turn (user prompt) |
| `g` | Go to top |
| `G` | Go to bottom |
| `Ctrl+K` | Scroll up one line |
| `Ctrl+J` | Scroll down one line |
| `PageUp/Down` | Page scroll |
| `Ctrl+U/D` | Half-page scroll |

### Focus Switching

| From → To | Keys |
|------------|------|
| Scrollback → Prompt | `Tab`, `i`, `Space`, any printable key |
| Prompt → Scrollback | `Esc`, `Tab` |
| Prompt → Send | `Enter` |

---

## Hotkeys Reference

### Global (Always Active)

| Key | Action | Confirmation |
|-----|--------|--------------|
| `Ctrl+N` | New session | Double-press |
| `Ctrl+Shift+N` | New session in worktree | Double-press |
| `Ctrl+H` | Return to welcome screen | Yes |
| `Ctrl+Q` / `Ctrl+D` | Quit application | Double-press |

### Agent Actions

| Key | Action |
|-----|--------|
| `Ctrl+P` | Command palette |
| `?` | Command palette (alt) |
| `Ctrl+M` | Model picker (scrollback) / Multiline toggle (prompt) |
| `Ctrl+O` | Toggle auto-approve (YOLO) mode |
| `Ctrl+S` | Session picker (resume) |
| `Ctrl+;` | Toggle prompt queue pane |
| `Ctrl+Shift+A` | Toggle subagent catalog |

### During Active Turn

| Key | Action |
|-----|--------|
| `Ctrl+C` | Cancel current turn |
| `Ctrl+Enter` | Interject (continues turn) |
| `Ctrl+I` | Interject (alt) |
| `Shift+Enter` | Newline in multiline mode |

### Scrollback Actions

| Key | Action |
|-----|--------|
| `h` / `Left` | Collapse selected entry |
| `l` / `Right` | Expand selected entry |
| `e` | Toggle fold on entry |
| `E` | Expand/collapse all |
| `Ctrl+Shift+E` | Expand/collapse all thinking |
| `r` | Toggle raw markdown |
| `y` | Copy block content |
| `Y` | Copy block metadata |
| `Enter` | Fullscreen viewer |

### Welcome Screen Only

| Key | Action |
|-----|--------|
| `Ctrl+S` | Resume session |
| `Ctrl+W` | Toggle worktree mode |
| `Ctrl+I` | Import Claude settings |
| `Ctrl+Shift+I` | Dismiss Claude import row |

---

## Themes in Detail

### GrokNight (Default)

**Backgrounds:**
- Base: `#1a1a2e` (deep navy-gray)
- Light: `#16213e`
- Dark: `#0f0f1a`

**Accents:**
- User: `#6ee7b7` (mint green)
- Thinking: `#fbbf24` (amber)
- Tool: `#60a5fa` (sky blue)
- Error: `#f87171` (coral red)
- Success: `#34d399` (emerald)

### TokyoNight

**Backgrounds:**
- Base: `#1a1b26` (deep blue-black)
- Light: `#24283b`
- Dark: `#16161e`

**Accents:**
- User: `#7aa2f7` (bright blue)
- Thinking: `#bb9af7` (purple)
- Tool: `#9ece6a` (green)
- Error: `#f7768e` (pink-red)

### RosePineMoon

**Backgrounds:**
- Base: `#232136` (deep purple-gray)
- Light: `#2a2740`
- Dark: `#1f1a2e`

**Accents:**
- User: `#c4a7e7` (lavender)
- Thinking: `#f6c177` (peach)
- Tool: `#ebbcba` (rose)
- Error: `#eb6f92` (magenta)

---

## Customization (pager.toml)

### Layout

```toml
[scrollback.layout]
outer_vpad = 1              # Top/bottom padding
outer_hpad_left = 2         # Left margin (min: 1)
outer_hpad_right = 2        # Right margin (min: 1)
block_pad_left = 2          # Indent from accent bar
block_pad_right = 2         # Right padding
```

### Scrollbar

```toml
[scrollback.scrollbar]
enabled = true
gap_left = 0                # Gap before scrollbar
gap_right = 0               # Gap after scrollbar
```

### Block Styling

**Thinking Blocks:**
```toml
[scrollback.blocks.thinking]
accent_enabled = true
animate = true             # Animate accent line
truncated_lines = 3
header = true
```

**Tool Calls:**
```toml
[scrollback.blocks.tool]
muted_collapsed = true
dim_details = true
bullet = "diamond"          # dot, small-circle, circle, small-triangle, triangle, diamond, none
```

**Execute (Shell):**
```toml
[scrollback.blocks.execute]
first_lines = 2
last_lines = 3
accent_enabled = true
header_style = "label"      # or "shell"
```

---

## Special Features

### File References (`@`)

```
@src/main.rs              # Attach file
@src/main.rs:10-50        # Attach lines 10-50
@src/                     # Browse directory
@!.env                    # Force hidden files
```

### Image Support

| Platform | Action |
|----------|--------|
| macOS/Linux | Drag image into prompt |
| macOS/Linux | Paste with `Cmd+V` / `Ctrl+V` |
| Windows | Paste with `Alt+V` (special binding) |

### Slash Commands

Type `/` in prompt for:
- `/model` - Switch model
- `/theme` - Theme picker
- `/compact-mode` - Toggle compact mode
- `/yolo` - Auto-approve toggle
- `/new` - New session
- `/load` - Resume session
- And many more...

### MCP Server Indicators

```
⛔ 6 MCP servers unavailable
```

Shows connection status for Model Context Protocol servers.

---

## Version Info

```
╰───────────────────────────────────────────────────────────── Grok Build ─╯

                                                                   0.2.14 Beta
```

- **Version Number:** Semantic versioning (0.2.14)
- **Beta Tag:** Pre-release status
- **Model Version:** Grok 4.3

---

## Terminal Compatibility

| Terminal | Support Level |
|----------|---------------|
| **macOS** | Full support including image paste |
| **Linux** | Full support |
| **Windows Terminal** | Full support (use `Alt+V` for images) |
| **tmux** | Full support with control mode |
| **Zellij** | Inline mode |
| **VSCode** | Special key bindings (Ctrl+D for quit) |
| **WezTerm** | Requires `enable_kitty_keyboard = true` |

---

*Documentation generated from Grok Build TUI v0.2.14 Beta*
