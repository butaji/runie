# Grok Build TUI - UI Documentation

**Version:** 0.2.15 Beta  
**Updated:** Extended with Extensions, Modes, Commands, Headless/ACP, Skills, Plugins, Hooks, and Marketplaces  
**Model:** Grok 4.3 (xAI, April 2026)

---

## Overview

Grok Build is xAI's terminal-based AI coding assistant featuring a sophisticated Terminal User Interface (TUI). The interface combines a dark, modern aesthetic with efficient keyboard-driven navigation, real-time tool execution visualization, and extensive customization options.

---

## Features

Grok Build provides a comprehensive development workflow in your terminal:

| Feature | Description |
|---------|-------------|
| **Plan mode** | Propose a structured approach before writing code |
| **Subagents** | Spawn parallel agents for testing and research |
| **Skills** | Turn workflows into reusable slash commands |
| **Hooks** | Run scripts on file edits and tool calls |
| **MCP servers** | Connect to Linear, Sentry, Grafana, and more |
| **AGENTS.md** | Set conventions and rules per directory |
| **Memory** | Persist decisions and context across sessions |
| **Code search** | Grep and navigate large codebases fast |
| **Multi-file edits** | Refactor across files with search-and-replace |
| **Git integration** | Stage, commit, push, and manage branches |
| **Deep reasoning** | Step-by-step thinking for hard problems |
| **Web search** | Look up docs and packages from the terminal |
| **Terminal execution** | Run builds and tests with live streaming |
| **Headless mode** | Script Grok Build in CI/CD pipelines |
| **Code review** | Line-by-line feedback before opening a PR |
| **Sandboxed execution** | Run untrusted code in isolated environments |
| **Background tasks** | Monitor long-running builds and processes |
| **Theming** | Customize colors, fonts, and appearance |

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

### 7. Modes

Grok operates in different modes that control behavior:

| Mode | Title Suffix | Description |
|------|--------------|-------------|
| Normal | (none) | Standard operation |
| Plan | `· plan` | Plan-only mode, shows reasoning without execution |
| Always-Approve | `· always-approve` | YOLO mode, auto-approves all actions |
| Subagents | `· subagents` | Parallel subagent panel visible |
| Ask | `· ask` | Interactive questionnaire active |

**Switching Modes:**
- `Shift+Tab` cycles session modes in the TUI

#### Plan Mode

Plan mode is for planning first. When active, write tools are blocked except for the session plan file.

```
grok-build · plan
```

**Use cases:**
- Sketch approach before making changes
- Review proposed edits before applying
- Ask clarifying questions before edits

**Commands:**
- `/plan` - View the current session plan

#### Always-Approve Mode

Always-approve skips permission prompts for tool calls.

```
grok-build · always-approve
```

**Starting in always-approve:**
```bash
grok --always-approve
```

**Toggling in TUI:**
- `Ctrl+O` - Toggle auto-approve mode
- `/always-approve` - Slash command

#### Permission Configuration

Set default permission behavior in `~/.grok/config.toml`:

```toml
[ui]
permission_mode = "always-approve"   # Skip all prompts
permission_mode = "ask"              # Prompt on each tool call (default)
```

**Legacy options (still supported):**
- `approval_mode`
- `yolo = true`

> Note: Put config in `~/.grok/config.toml`, not project-scoped `.grok/config.toml`.

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

### 11. Extensions Modal

The Extensions modal provides a unified interface for managing Grok Build's extensibility system:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ [Hooks] [Plugins] [Marketplace] [Skills] [MCP Servers]                      │
├─────────────────────────────────────────────────────────────────────────────┤
│ / to search                                        Workspace ⌄             │
├─────────────────────────────────────────────────────────────────────────────┤
│ › team-tool                                    (project)  [install]         │
│ › browser-review v0.8.2                      (workspace) [install]         │
│ › github-flow v2.1.0                         (workspace) [installed]        │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Tabs:**
| Tab | Purpose |
|-----|---------|
| Hooks | Pre/post action hooks |
| Plugins | Local plugin management |
| Marketplace | Browse and install from registry |
| Skills | Skill bundles and configurations |
| MCP Servers | Model Context Protocol server connections |

**Plugin Entry Format:**
- `›` - Expand indicator for nested items
- Plugin name with optional version (`browser-review v0.8.2`)
- Scope badge: `(project)` or `(workspace)`
- Action button: `[install]`, `[installed]`, or `[update]`

**Scope System:**
| Scope | Description |
|-------|-------------|
| `(project)` | Project-level plugin, affects current repository |
| `(workspace)` | Workspace-local plugin, isolated to current session |

**Opening the Modal:**
- Command palette: `Ctrl+P` → type `extensions` or `build`
- Slash command: `/extensions` or `/build`

### 12. Interactive Questionnaire Panel

When the agent needs user input, a questionnaire panel appears:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ ● ○ ● ○ ● ○  Waiting on answers for 3 questions              [turn: 7.1s]  │
├─────────────────────────────────────────────────────────────────────────────┤
│  1  ○  Minimal & terminal-native                                         │
│          Clean, keyboard-first, no excess chrome                           │
│  2  ○  Bold & expressive                                                  │
│          Strong visuals, gradients, animations                             │
│  3  ○  Developer-focused                                                  │
│          Code-first aesthetic, technical precision                          │
│  4  ○  Other                                                              │
│          Define custom principles                                           │
│  z  ○  Type your answer here                                              │
├─────────────────────────────────────────────────────────────────────────────┤
│  [1/3]  ↑/↓ navigate  ←/→ question  Enter:select                        │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Questionnaire Elements:**
- **Dot Grid:** Shows progress across multiple questions (e.g., `● ○ ●` pattern)
- **Numbered Options:** Questions with radio-button selection
- **Subtitles:** Descriptive text below each option
- **Text Input:** `z` option for custom input
- **Progress:** `[1/3]` indicates current question of total

**Radio Button States:**
| State | Appearance |
|-------|------------|
| Unselected | Empty circle with border |
| Selected | Filled circle |
| Focused | Highlighted border |

### 13. Plan Modal

Plan mode displays a structured implementation document:

```
┌─────────────────────────────── plan.md ───────────────────────────────┐
│                                                                        │
│   1  Install Docs Refresh Plan                                         │
│   2  Quick Assessment                                                  │
│   3  • docs/install.md skips headless mode...                          │
│   4  Implementation Plan                                               │
│   5  1. Replace the install snippet with curl bootstrap                │
│   6  2. Document `-p` headless mode                                   │
│   7  3. Point users to config.toml for models                         │
│   8  4. Cross-link the auth and feedback sections                      │
│                                                                        │
└────────────────────────────────────────────────────────────────────────┘
```

**Plan Document Structure:**
- **Section Headers:** Colored labels (`Quick Assessment`, `Implementation Plan`)
- **Numbered Steps:** Sequential implementation items
- **Bullet Points:** Supporting details and context
- **Divider:** Centered title (`─── plan.md ───`)

**Plan Mode Shortcuts:**
| Key | Action |
|-----|--------|
| `Enter` | Approve plan and apply changes |
| `Esc` | Close plan without applying |
| `Type` | Add comment or modification request |

### 14. Subagent Panel

Parallel subagent execution displays in either inline or dedicated panel mode:

**Inline Mode (in scrollback):**
```
     ⠴ Run List `.` 1.8s                                         5.7s ⇣21.2k
```

**Dedicated Panel Mode:**
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ projects/main jasong/folder                                       77.54% │
├─────────────────────────────────────────────────────────────────────────────┤
│ [explore] Explore checkout flow                     explore · grok-build    │
│ [explore] Explore infra and CI                     explore · grok-build    │
│ [explore] Explore shared Go libraries              explore · grok-build    │
│ [explore] Explore order services                    explore · grok-build    │
│ [explore] Explore fulfillment jobs                  explore · grok-build    │
│ [explore] Explore pricing engine                    explore · grok-build    │
├─────────────────────────────────────────────────────────────────────────────┤
│ ❯ find the source of the p99 latency regression                        │
└─────────────────────────────────────────────────────────────────────────────┘
                              ┌──────────────────────────────────┐
                              │ ❯ find the source of the...      │
                              │         grok-build · subagents    │
                              └──────────────────────────────────┘
```

**Dot Grid Activity Indicator:**
The 2x3 dot matrix shows agent activity state:
```
● ○ ●    ● = Active/working
● ○ ●    ○ = Idle/waiting
```

| Pattern | Meaning |
|---------|---------|
| `● ○ ●` alternating | Multi-step task in progress |
| All `●` | Fully active |
| All `○` | Idle/waiting for input |

**Agent Labels:**
- `explore` - Exploration/analysis agent
- `general` - General-purpose agent
- `code` - Code implementation agent

**Context Suffixes:**
| Suffix | Mode |
|--------|------|
| `grok-build` | Normal mode |
| `grok-build · plan` | Plan mode |
| `grok-build · always-approve` | Auto-approve mode |
| `grok-build · subagents` | Subagent panel visible |
| `grok-build · ask` | Questionnaire active |

### 15. Inline Diff Viewer

When editing files, the diff viewer shows changes inline:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  ◆ Thought for 2.3s                                                        │
│                                                                             │
│  41  Install the CLI with...                                               │
│  42+ curl -fsSL x.ai/cli/install.sh | bash                                 │
│  43                                                                         │
│  44- Run the CLI and follow the prompts.                                   │
│  44+ Run `grok-build -p` to use the CLI in headless ACP-compatible mode.   │
│  45+ Sign in once, then configure models and API keys in `config.toml`.     │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Diff Color System:**
| Variable | Default | Purpose |
|----------|---------|---------|
| `--terminal-diff-gutter` | `#6c6c6c` | Line number color |
| `--terminal-diff-text` | `#e1e1e1` | Unchanged line text |
| `--terminal-diff-insert-bg` | `#202a16` | Added line background |
| `--terminal-diff-delete-bg` | `#32181c` | Removed line background |
| `--terminal-diff-insert-fg` | `#9ece6a` | Added line number (green) |
| `--terminal-diff-delete-fg` | `#f7768e` | Removed line number (red) |

**Gutter Styling:**
- Right-aligned line numbers
- 14px right padding
- Monospace font for alignment
- Fixed 54px gutter width

### 16. Additional Color Variables

Extended color palette for UI elements:

| Variable | Default | Purpose |
|----------|---------|---------|
| `--terminal-editor-bg` | (theme base) | Editor/panel background |
| `--terminal-surface` | `#202020` | Elevated surface background |
| `--terminal-popover` | `#1a1a1a` | Modal/popover background |
| `--terminal-teal` | `#29c6be` | Teal accent for prompts |
| `--terminal-orange` | `#d59556` | Orange for file labels (plan.md) |
| `--terminal-purple` | `#bc97ff` | Purple for thinking blocks |
| `--terminal-yellow` | `#cfb47c` | Yellow for plan indicator |
| `--terminal-blue-bright` | `#88a6ff` | Bright blue for active elements |

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

## Slash Commands

The command palette (`Ctrl+P`) groups session, context, model, and tool actions. Type `/` in the prompt to access slash commands. User-invocable skills also appear as slash commands.

### Session Commands

| Command | Alias | Description |
|---------|-------|-------------|
| `/quit` | `/exit` | Quit the application |
| `/home` | - | Return to the welcome screen |
| `/new` | - | Start a new session |
| `/resume` | - | Resume a previous session |
| `/sessions` | - | Browse and pick from past sessions |
| `/fork` | - | Fork the current session into a new one |
| `/rename <title>` | - | Rename the current session |
| `/share` | - | Share the current session via URL |
| `/session-info` | - | Show session info |

### Context & Model Commands

| Command | Description |
|---------|-------------|
| `/context` | View context usage |
| `/model <name>` | Switch the active model |
| `/compact [context]` | Compact conversation history |
| `/compact-mode` | Toggle denser UI layout |
| `/rewind` | Rewind to an earlier point in the conversation |
| `/usage` | Show token and credit usage for the session |

### UI & Display Commands

| Command | Description |
|---------|-------------|
| `/theme [name]` | Switch the color theme |
| `/multiline` | Toggle multiline input |

### Permission & Plan Commands

| Command | Description |
|---------|-------------|
| `/always-approve` | Toggle always-approve mode |
| `/plan` | View the current session plan |
| `/feedback [text]` | Send feedback about the current session |

### Utility Commands

| Command | Description |
|---------|-------------|
| `/btw <question>` | Ask a side question without interrupting |
| `/logout` | Sign out of the current account |

### Extensions Commands

These open the unified extensions modal with a pre-selected tab:

| Command | Tab Selected |
|---------|--------------|
| `/hooks` | Hooks |
| `/plugins` | Plugins |
| `/skills` | Skills |
| `/mcps` | MCP Servers |

### Shell-Provided Commands

| Command | Description |
|---------|-------------|
| `/flush` | Flush conversation memory to disk now |
| `/memory` | Search and edit persistent memory entries |
| `/dream` | Trigger an offline memory-consolidation pass |
| `/imagine <prompt>` | Generate an image from text |
| `/imagine-video <prompt>` | Generate a video from text |

### Skills as Commands

Any user-invocable skill appears as a slash command:
```
/<skill-name>
```

If names collide, use the qualified form:
```
/local:commit
```

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

## Headless Mode & Scripting

### Headless Mode

Use headless mode for scripts, bots, or other machine-friendly tasks:

```bash
grok -p "Your prompt here"
```

**Common flags:**

| Flag | Description |
|------|-------------|
| `-p, --single <PROMPT>` | Send one prompt |
| `-m, --model <MODEL>` | Choose a model |
| `-s, --session-id <ID>` | Create or resume a named headless session |
| `-r, --resume <ID>` | Resume an existing session |
| `-c, --continue` | Continue the most recent session in the current directory |
| `--cwd <PATH>` | Set the working directory |
| `--output-format <FMT>` | Choose `plain`, `json`, or `streaming-json` |
| `--always-approve` | Auto-approve tool executions |
| `--no-alt-screen` | Run inline (no fullscreen TUI takeover) |

**Suppressing auto-updates in scripts:**
```bash
grok --no-auto-update -p "..."
```

Or persist in `~/.grok/config.toml`:
```toml
[cli]
auto_update = false
```

### Output Formats

| Format | Description |
|--------|-------------|
| `plain` | Human-readable text |
| `json` | One JSON object at the end |
| `streaming-json` | Newline-delimited JSON events |

```bash
grok -p "List TODO comments" --output-format json
grok -p "Explain the architecture" --output-format streaming-json
```

### ACP (Agent Protocol)

Use ACP for IDE or tool integration instead of terminal sessions:

```bash
grok agent stdio
```

This runs Grok as an ACP agent over JSON-RPC on stdin/stdout. Environment variables:

| Variable | Description |
|----------|-------------|
| `XAI_API_KEY` | API key for authentication |

**Authentication methods:**
- `xai.api_key` - Direct API key
- `cached_token` - Cached login token

**Example Node.js integration:**
```javascript
import { spawn } from "node:child_process";
import readline from "node:readline";
import process from "node:process";

const proc = spawn("grok", ["agent", "stdio"], { stdio: ["pipe", "pipe", "pipe"] });
const rl = readline.createInterface({ input: proc.stdout });

// See full example in the Headless & Scripting section of docs
```

**ACP Methods:**
- `initialize` - Initialize the agent
- `authenticate` - Authenticate with a method
- `session/new` - Create a new session
- `session/prompt` - Send a prompt to a session

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

*Documentation generated from Grok Build TUI v0.2.15 Beta*
