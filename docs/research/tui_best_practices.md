# TUI Design Best Practices

A comprehensive guide to building delightful, efficient, and keyboard-driven terminal user interfaces.

## Table of Contents

1. [Philosophy: The Terminal Advantage](#philosophy-the-terminal-advantage)
2. [Keyboard-Driven Interface Principles](#keyboard-driven-interface-principles)
3. [Navigation Patterns from Popular TUIs](#navigation-patterns-from-popular-tuis)
4. [Common TUI Anti-Patterns](#common-tui-anti-patterns)
5. [What Makes Terminal UIs Delightful](#what-makes-terminal-uis-delightful)
6. [Status Bar and Information Density](#status-bar-and-information-density)
7. [Color Usage in Terminals](#color-usage-in-terminals)
8. [Help System Design](#help-system-design)
9. [Command Palette Design](#command-palette-design)
10. [Architecture and Patterns](#architecture-and-patterns)
11. [Performance Principles](#performance-principles)

---

## Philosophy: The Terminal Advantage

The terminal is not a relic of the past—it represents a philosophy of computing that prioritizes speed, composability, and user mastery over visual polish.

### Why Terminals Matter

From Brandur's analysis on [Learning From Terminals to Design the Future of User Interfaces](https://brandur.org/interfaces):

> "Modern applications and interfaces frustrate me. In today's world every one of us has the awesome power of the greatest computers in human history in our pockets and at our desks... Yet almost without exception we wait for our computers instead of the other way around."

Terminal programs offer:

- **Negligible startup/loading time** — instant response
- **Instant screen transitions** — no animations slowing you down
- **Uniform interface elements** — predictable and learnable
- **High ceiling for mastery** — optimized for experienced users who invest time
- **Composability** — output can be piped into other programs

### The Core Value

> "A successful interface isn't one that looks good in a still screenshot, it's one that maximizes our productivity and lets us **keep moving**."

---

## Keyboard-Driven Interface Principles

### The Principle of Never Leaving the Keyboard

Power users stay in flow by never reaching for the mouse. A well-designed TUI should:

1. **Every action accessible by keyboard** — If there's a button, there's a shortcut
2. **Predictable shortcuts** — Follow established conventions (vim, Emacs, common patterns)
3. **Easy discoverability** — Clear help system without memorization
4. **Escape as universal abort** — One key to cancel/back out of any state

### Mode-Based Design

Many successful TUIs use modes to group related functionality:

```
┌─────────────────────────────────────────┐
│  NORMAL MODE  │  INSERT MODE  │  VISUAL │
│  (commands)   │   (editing)   │ (select)│
└─────────────────────────────────────────┘
```

**Examples:**
- **vim** — Normal, Insert, Visual, Command-line modes
- **lazygit** — Files panel, Commits panel, Branches panel
- **htop** — Main view, Tree view, Search mode

### Mnemonic Keybindings

Choose shortcuts that hint at their function:

| Key | Meaning | Example Use |
|-----|---------|-------------|
| `j/k` | Down/Up | vim-style list navigation |
| `h/l` | Left/Right | Hierarchy navigation |
| `gg` | Go to top | Jump to first item |
| `G` | Go to bottom | Jump to last item |
| `/` | Search | Forward search |
| `?` | Help/Search | Backward search or help |
| `:` | Command | Enter command mode |
| `q` | Quit | Close current panel/window |
| `Esc` | Escape | Cancel/back out |

### Progressive Disclosure

Not every feature needs a permanent shortcut. Use:
- **Lazy keys** — Press first, then show options (e.g., `m` in ranger shows move options)
- **Leader keys** — Chord combinations for rarely-used actions
- **Command palette** — `:` or `/` opens searchable command interface

---

## Navigation Patterns from Popular TUIs

### vim: The Gold Standard

vim's navigation philosophy has become the de facto standard for terminal applications.

**Movement keys:**
```
h j k l          Basic movement (left, down, up, right)
w b              Word forward/backward
0 $              Line start/end
gg G             File top/bottom
/ ?              Search forward/backward
n N              Next/previous match
{ }              Paragraph movement
%                Matching bracket
```

**Why it works:**
- Home row positioning minimizes finger travel
- Consistent across the ecosystem (people already know it)
- Combines with modifiers (5j = 5 lines down)

### htop: Process Management Pattern

htop demonstrates effective use of:
- **Function keys for categories** — F1-Help, F2-Setup, F3-Search, F4-Filter, F5-Tree, F6-Sort
- **Interactive filtering** — Real-time process filtering
- **Color-coded status** — Visual state indication

### ranger: File Manager Pattern

[ranger](https://opensource.com/article/22/12/linux-file-manager-ranger) shows three-column navigation:
```
┌──────────┬──────────┬──────────────┐
│ Parent   │ Current  │ Preview      │
│ (cwdir)  │ (cwfile) │ (sel file)   │
└──────────┴──────────┴──────────────┘
```

Keybindings:
- `h/l` — Move up/down directories
- `j/k` — Navigate files
- `Space` — Mark file
- `yy` — Copy
- `dd` — Cut
- `p` — Paste

### lazygit: Panel-Based Workflow

[lazygit](https://github.com/jesseduffield/lazygit) exemplifies the panel-based TUI pattern:

```
┌────────────────────────────────────────────────────┐
│ 1: Status │ 2: Files │ 3: Branches │ 4: Commits   │
│────────────────────────────────────────────────────│
│                                                    │
│  Main Panel Area                                   │
│  (context-dependent: diffs, logs, etc.)            │
│                                                    │
├────────────────────────────────────────────────────┤
│  ? ─────────────────────────────────────────────── │
│  Staged: 2 │ Conflicts: 0 │ Unstaged: 1          │
└────────────────────────────────────────────────────┘
```

**Design principles:**
- Number keys switch panels
- Single key actions for common operations
- Inline help shows available keys
- Panel state persists during workflow

### fzf: Fuzzy Finding Pattern

[fzf](https://github.com/junegunn/fzf) demonstrates:
- **Instant filtering** — Results update as you type
- **Keyboard-first** — All actions via keys
- **Preview pane** — Shows detailed info alongside list
- **Multiple selection** — Tab to select multiple items

---

## Common TUI Anti-Patterns

Avoid these patterns that frustrate users and break flow.

### 1. Hidden State Syndrome

**Bad:** User can't tell if the app is processing, stuck, or waiting.

**Good:** Clear status indicators with specific feedback:
```
[Processing...] ████████░░ 80%
```

### 2. One-Way Escape Paths

**Bad:** Some actions require multiple Esc presses, or can't be cancelled.

**Good:** Single Esc cancels any in-progress action; always returns to safe state.

### 3. Inconsistent Shortcuts

**Bad:** `q` quits in one screen, `Esc` in another.

**Good:** Consistent shortcut mapping across all views.

### 4. Modal Overload

**Bad:** Every action opens a modal dialog.

**Good:** Inline editing where possible; reserve modals for critical confirmations.

### 5. Missing Feedback

**Bad:** User presses key, nothing visible happens.

**Good:** Visual feedback for every keypress (cursor movement, selection highlight, status change).

### 6. Unreachable States

**Bad:** User can enter a state with no way out except restart.

**Good:** Every state has an escape route (`q`, `Esc`, `:q`).

### 7. Animation as Decoration

**Bad:** Transitions that slow down workflow.

**Good:** Instant state changes; animations only when they convey information (progress, loading).

### 8. Excessive Whitespace

**Bad:** Tiny content area with huge margins.

**Good:** Efficient use of screen real estate; content density appropriate to context.

---

## What Makes Terminal UIs Delightful

### Speed is Happiness

> "The computer waits on the human rather than the other way around."

- **Instant response** — Every keypress immediately reflected
- **No loading screens** — Content appears immediately
- **No animations** — Unless they convey information

### Consistency and Predictability

- Same key does same thing everywhere
- Learned skills transfer between contexts
- No surprises or edge cases

### Respect for Expertise

- Beginner-friendly defaults
- Power-user shortcuts available
- No condescending "Are you sure?" dialogs for reversible actions

### Visual Clarity

- High contrast text
- Clear selection indicators
- Scannable layout

### Sensible Defaults

- Works out of the box
- No mandatory configuration
- Optional customization for those who want it

---

## Status Bar and Information Density

### The Status Bar Contract

The status bar is your contract with the user. It should answer:

1. **Where am I?** — Current context, mode, panel
2. **What's selected?** — Current item, count selected
3. **What's happening?** — Background process, sync status
4. **What can I do?** — Mode-specific available actions

### Information Density Principles

From [UI Density by Matt Ström](https://mattstromawn.com/writing/ui-density/):

> "UI density is not just the way an interface looks at one moment in time; it's about the amount of information an interface can provide at a glance."

**Density spectrum:**

```
Low Density          Medium Density          High Density
─────────────        ──────────────          ─────────────
┌─────────────────┐  ┌─────────────────────┐ ┌───────────────────────────────┐
│                 │  │ Item 1        [x]    │ │ # │ Name         │ Status │   │
│  [Selected]     │  │ Item 2        [ ]    │ │───┼─────────────┼────────│   │
│                 │  │ Item 3        [ ]    │ │ 1 │ project-a   │ ACTIVE │   │
│  [Action]       │  │                 [+]  │ │ 2 │ project-b   │ STANDBY│   │
│                 │  └─────────────────────┘ │ 3 │ project-c   │ ACTIVE │   │
└─────────────────┘                          └───────────────────────────────┘
```

**Choose density based on:**
- User expertise level
- Data complexity
- Screen size constraints
- Task at hand

### Status Bar Patterns

**Minimal (htop style):**
```
CPU: ██████░░░░ 60% | MEM: ███░░░░░░░ 30% | Tasks: 142, 3 running
```

**Rich (lazygit style):**
```
Branch: main → | Staged: 2 | Unstaged: 5 | Conflicts: 0 | Rebasing
```

**Customizable (tmux style):**
```
#[fg=colour234,bg=colour136] session #[fg=colour136,bg=colour234] #S 
#[fg=colour136,bg=colour234]│ #[fg=colour39,bg=colour234] windows #[fg=colour39]:#I 
```

### Scannability Tips

- **Left-align text** — Easier to scan
- **Use consistent delimiters** — `│` for separation, `:` for labels
- **Truncate with ellipsis** — Never wrap text in status bar
- **Group related info** — Context, status, actions together

---

## Color Usage in Terminals

### The 256-Color Reality

Terminals typically support 256 colors (8-bit). Use them strategically.

### Semantic Color Mapping

Assign consistent meaning to colors:

| Color | Semantic Meaning | Examples |
|-------|------------------|----------|
| Red | Error, danger, delete | Error messages, delete actions |
| Green | Success, staged, add | Success messages, staged files |
| Yellow/Gold | Warning, modified | Warnings, unstaged changes |
| Blue | Info, links, selection | Links, selected items |
| Cyan | Info, secondary | Secondary information |
| Magenta | Special, tags | Tags, special highlights |
| White/Gray | Default, muted | Default text, disabled |

### Contrast and Legibility

**Always consider:**
- Terminal background (light or dark)
- User color scheme
- Color-blind users

**Best practices:**
```
✓ High contrast: bright text on dark background
✓ Use bold for emphasis, not just color
✓ Combine color + text (not color alone)
✗ Never rely on color alone for critical info
```

### Color Palettes for TUIs

**Dark theme (common):**
```
Background: #1e1e1e (near black)
Foreground: #d4d4d4 (light gray)
Selection:  #264f78 (blue)
```

**Light theme:**
```
Background: #ffffff (white)
Foreground: #333333 (dark gray)
Selection:  #add6ff (light blue)
```

### Color as Information, Not Decoration

**Good uses of color:**
- Syntax highlighting
- Status indicators (success/error/warning)
- Diff visualization (added/removed/changed)
- Selection highlighting

**Bad uses:**
- Decorative borders
- Rainbow output without meaning
- Color for emphasis without semantic value

---

## Help System Design

### The Hierarchy of Help

```
┌─────────────────────────────────────────┐
│ Level 1: Inline Context Hints           │
│ (shown at bottom of screen)             │
├─────────────────────────────────────────┤
│ Level 2: On-Demand Help (press ?)       │
│ (full-screen or modal help overlay)     │
├─────────────────────────────────────────┤
│ Level 3: Documentation                  │
│ (man pages, README, wiki)               │
└─────────────────────────────────────────┘
```

### Level 1: Inline Hints

Always show available actions in current context:

```
┌────────────────────────────────────────────────────┐
│                                                    │
│  [Content area]                                    │
│                                                    │
├────────────────────────────────────────────────────┤
│ ↑↓ Navigate │ Enter Select │ Esc Back │ ? Help     │
└────────────────────────────────────────────────────┘
```

**Rules:**
- Show only keys relevant to current mode/panel
- Update dynamically as context changes
- Use consistent positioning (bottom is standard)
- Short, scannable descriptions

### Level 2: On-Demand Help

Triggered by `?` or `F1`:

```
┌────────────────────────────────────────────────────┐
│                    HELP                           │
├────────────────────────────────────────────────────┤
│  NAVIGATION                                       │
│  ─────────────                                   │
│  j/k        Move down/up                         │
│  gg/G       Jump to top/bottom                   │
│  /          Search forward                       │
│  n/N        Next/previous match                   │
│                                                    │
│  ACTIONS                                          │
│  ─────────                                       │
│  Enter     Select / Confirm                       │
│  Space     Toggle selection                        │
│  d         Delete selected                        │
│  e         Edit selected                          │
│                                                    │
│  GENERAL                                          │
│  ────────                                         │
│  ?         Show this help                        │
│  q         Quit                                  │
└────────────────────────────────────────────────────┘
```

### Level 3: Full Documentation

- README with quick start
- Man pages for CLI tools
- Wiki for detailed guides
- Examples section

### Help Design Principles

1. **Discoverable** — Always accessible via `?` or `F1`
2. **Contextual** — Shows only relevant keys
3. **Minimal** — Short descriptions, not essays
4. **Scannable** — Grouped by category, bold key names
5. **Actionable** — Shows what keys DO, not just names

---

## Command Palette Design

### What is a Command Palette?

A searchable, keyboard-driven interface for executing commands, navigating, or searching. popularized by VS Code, Slack, and modern applications.

### Design Principles

**Trigger:**
- `:` — Commands
- `/` — Search
- `Ctrl+P` — Quick open (VS Code style)
- `Ctrl+K` — Command palette (Slack style)

**Interface:**
```
┌─────────────────────────────────────────┐
│ > ▌                                      │
├─────────────────────────────────────────┤
│ ► Switch to dark theme                   │
│   Switch to light theme                  │
│   Open settings                          │
│   Run formatter                          │
└─────────────────────────────────────────┘
```

### Fuzzy Matching

Command palettes use fuzzy matching for flexible input:

| Typed | Matches |
|-------|---------|
| `sw` | Switch, Show, Search within... |
| `sdt` | Switch to dark theme |
| `dth` | dark theme, default theme |

### Prefix Patterns

Common conventions:
- `>` — Commands only
- `@` — Symbols/users
- `#` — Search content
- `:` — Go to line

### Keyboard Navigation

```
↑/↓ or Ctrl+J/K    Navigate options
Enter              Execute selected
Tab                Autocomplete
Esc                Close
Ctrl+C             Clear input
```

---

## Architecture and Patterns

### The Elm Architecture (TEA)

Popular for TUI applications (Bubbletea, tui-realm):

```
┌─────────┐    Action     ┌─────────┐    State    ┌─────────┐
│   User  │ ──────────▶  │  Update │ ──────────▶  │   View  │
│         │              │         │              │         │
└─────────┘              └─────────┘              └─────────┘
                              │
                              ▼
                        ┌─────────┐
                        │  Model  │
                        │ (State) │
                        └─────────┘
```

**Components:**
- **Model** — Application state
- **Message** — User actions/intents
- **Update** — Pure function: `(model, msg) → model`
- **View** — Render state to terminal

### State Machines

Model complex UI states as state machines:

```
┌─────────┐    start     ┌─────────┐    complete     ┌─────────┐
│  IDLE   │ ──────────▶ │ RUNNING │ ──────────────▶│ SUCCESS │
└─────────┘              └─────────┘                └─────────┘
      ▲                       │                           │
      │         error          │                           │
      └───────────────────────┘                           │
                                                          │
                                                          ▼
                                                    ┌─────────┐
                                                    │  ERROR  │
                                                    └─────────┘
```

### Component-Based Design

Break UI into reusable components:

```
App
├── Header
│   ├── Title
│   └── StatusBar
├── MainPanel
│   ├── ListView
│   │   └── ListItem (×n)
│   ├── DetailView
│   └── ActionBar
└── Footer
    └── HelpHint
```

### Event Handling

```rust
// Pseudocode for event loop
loop {
    match event::read() {
        Key('q') => break,
        Key('j') => state.move_down(),
        Key('k') => state.move_up(),
        Key('\n') => state.select(),
        Key('?') => state.show_help(),
        Resize(w, h) => state.resize(w, h),
        _ => {} // Ignore unknown events
    }
    terminal.draw(|f| view(f, &state));
}
```

---

## Performance Principles

### Instant Response

The #1 rule of TUI design:

> "We should stop babying our users and try to raise beginners and the less technical to the bar of modern day power users."

**Techniques:**
- Process input immediately
- Async I/O for long operations
- Show progress, don't block
- Cache expensive computations

### Rendering Efficiency

**Dirty rendering** — Only redraw changed areas:
```rust
if state.is_dirty() {
    terminal.draw(|f| render(f, &state))?;
    state.clear_dirty();
}
```

**Batched updates** — Coalesce rapid changes:
```rust
// Instead of 60 updates/second, batch into 30
let update = ticker.select().throttle(Duration::from_millis(33));
```

### Async Operations

Never block the main loop:

```rust
// Good: Non-blocking async operation
tokio::spawn(async {
    let result = api_call().await;
    dispatch(Action::ApiComplete(result));
});

// Show loading indicator immediately
dispatch(Action::StartLoading);
```

### Memory Considerations

- Lazy load large datasets
- Virtualize long lists (render only visible items)
- Limit history size
- Clear caches when not needed

---

## Resources and References

### Articles
- [Learning From Terminals to Design the Future of User Interfaces](https://brandur.org/interfaces) — Brandur
- [UI Density](https://mattstromawn.com/writing/ui-density/) — Matt Ström
- [Designing Command Palettes](https://solomon.io/designing-command-palettes/) — Sam Solomon
- [A designer's guide to loving the terminal](https://www.alexchantastic.com/designers-guide-to-the-terminal) — Alex Chan

### Popular TUI Applications (for inspiration)
- [lazygit](https://github.com/jesseduffield/lazygit) — Git TUI
- [ranger](https://github.com/ranger/ranger) — File manager
- [htop](https://github.com/htop-dev/htop) — Process manager
- [fzf](https://github.com/junegunn/fzf) — Fuzzy finder
- [tmux](https://github.com/tmux/tmux) — Terminal multiplexer
- [aws-tui](https://aws-tui.dev/) — AWS console TUI

### TUI Frameworks
- **Rust:** [Ratatui](https://ratatui.rs/) — Fork of tui-rs
- **Go:** [Bubbletea](https://github.com/charmbracelet/bubbletea) — Elm-inspired
- **C++:** [FTXUI](https://github.com/ArthurSonzogni/FTXUI) — Functional TUI
- **Python:** [Textual](https://github.com/Textualize/textual) — Modern TUI framework

### Design Tools
- [TUIStudio](https://tui.studio/) — Visual TUI editor
- [tldr](https://tldr.sh/) — Simplified man pages

---

## Summary: The TUI Design Manifesto

1. **Speed is sacred** — Every millisecond matters
2. **Keyboard first** — Mouse is optional, not required
3. **Consistency is kindness** — Same keys, same behavior
4. **Help is always available** — Inline hints, on-demand help, docs
5. **Escape is safety** — Every state has an exit
6. **Color is information** — Semantic, not decorative
7. **Density is a choice** — Match to user needs
8. **Composability wins** — Output should flow to other tools

> "We should build networked applications that cache content and make network fetches asynchronously to remote APIs so that humans aren't waiting for data to come back over the wire while they're working."
