# Tidy TUI Design System

## Philosophy

Tidy is a dark, industrial, data-dense terminal interface for AI agent orchestration. It balances the visual density of system monitors (btop) with the refined minimalism of modern TUIs (Crush), using careful contrast hierarchies and selective accent colors to guide attention.

The interface reads like a dashboard when you need overview, and fades into the background when you need focus. Everything serves the conversation between human and agent.

---

## Color System

All colors are provided by the [Opaline](https://docs.rs/opaline) theme system. The default theme is `silkcircuit_neon`. Colors are accessed via semantic tokens, never hardcoded.

### Base Palette

| Token | Hex | Usage |
|-------|-----|-------|
| `bg.base` | `#121218` | Main background — near-black with slight warmth |
| `bg.panel` | `#181820` | Panel/card backgrounds — barely lighter than base |
| `bg.code` | `#1e1e28` | Code block backgrounds |
| `bg.highlight` | `#37324b` | Hover/selection backgrounds |
| `bg.selection` | `#3c3c50` | Active selection |

### Text Hierarchy

| Token | Hex | Usage |
|-------|-----|-------|
| `text.primary` | `#f8f8f2` | Primary text — bright but not white |
| `text.secondary` | `#bcbcca` | Body text — comfortable reading |
| `text.muted` | `#82879f` | Labels, hints, disabled state |
| `text.dim` | `#6e7daf` | Subtle borders, separators |

### Accent Colors

| Token | Hex | Usage |
|-------|-----|-------|
| `accent.primary` | `#e135ff` | **Purple** — primary brand color, active states, chevrons |
| `accent.secondary` | `#80ffea` | **Cyan** — secondary actions, running states, focus rings |
| `accent.tertiary` | `#ff6ac1` | **Coral/Pink** — special highlights, user input |

### Semantic Colors

| Token | Hex | Usage |
|-------|-----|-------|
| `success` | `#50fa7b` | Success, passing tests, completed agents |
| `warning` | `#f1fa8c` | Warnings, pending states, attention needed |
| `error` | `#ff6363` | Errors, failures, destructive actions |
| `info` | `#80ffea` | Information, running processes, active tools |

### Borders

| Token | Hex | Usage |
|-------|-----|-------|
| `border.focused` | `#80ffea` | Focused/selected borders |
| `border.unfocused` | `#82879f` | Inactive borders, separators |

### Contrast Rules

- **Low contrast default**: Most text uses `text.secondary` (`#bcbcca`) on `bg.base` (`#121218`) — ratio ~5:1, comfortable but not harsh
- **High contrast for importance**: Critical elements use `text.primary` (`#f8f8f2`) — ratio ~9:1
- **Accent for recognition**: `accent.primary` (purple) creates immediate visual landmarks for active elements, input prompts, and the agent identity
- **Muted for background**: `text.dim` (`#6e7daf`) for borders, separators, inactive elements

---

## Visual Language

### Glyphs & Symbols

The interface uses a carefully curated set of Unicode symbols to create a distinctive visual language:

| Symbol | Usage |
|--------|-------|
| `❯` | Input prompt chevron — primary accent color |
| `▌` | Bookmark/attention marker — accent color, left edge |
| `▸` | List item indicator — muted, expands to accent when selected |
| `●` | Running/active state — cyan accent |
| `✓` | Completed/success — green |
| `✗` | Failed/error — red |
| `○` | Waiting/idle — muted |
| `│` | Thin separator — unfocused border color |
| `─` | Horizontal rule — unfocused border color |
| `╭╮╰╯` | Rounded corners for boxes — unfocused border color |
| `░` | Shadow/fade effect — dim |
| `▒` | Modal backdrop dim — dim with background |

### No Traditional Borders

Panels and containers use:
- **Background color shifts** (`bg.base` → `bg.panel`) to define areas
- **Single accent bars** (`▌`) for focus indicators
- **Subtle separators** (`─`, `│`) only when needed for scanning
- **Never** heavy box borders around the main content area

### Shadows

Modals and floating panels cast a subtle shadow:
- Right edge: `░` characters, 1 cell offset
- Bottom edge: `░` characters, 1 cell offset
- Corner: `▒` character for depth
- Color: `text.dim` on `bg.base`

This creates a lifted effect without terminal transparency support.

---

## Layout System

### Zones

```
┌─────────────────────────────────────────────┐
│ TopBar (repo · branch · status)             │  height: 1
├──────────────────────────────────────────────┤
│                                              │
│  Center Feed      │  Right Sidebar          │
│  (Messages +      │  (Agents as cards)      │
│   Events)         │                         │
│                   │  ╭─ coder ───────────╮  │
│                   │  │ ● editing files  │  │
│                   │  │ claude-4 · 45s   │  │
│                   │  ╰──────────────────╯  │
│                   │                         │
│                   │  ╭─ test ────────────╮  │
│                   │  │ ✓ running tests  │  │
│                   │  │ gpt-4 · 12s      │  │
│                   │  ╰──────────────────╯  │
│                                              │
├──────────────────────────────────────────────┤
│ InputBar                                    │  dynamic height
├──────────────────────────────────────────────┤
│ StatusBar (^b sidebar · ^k cmd)             │  height: 1
└──────────────────────────────────────────────┘
```

### Margins & Spacing

- **Screen margin**: 2 chars horizontal, 1 char vertical
- **Between zones**: 0 lines (zones touch, separated by background color)
- **Inside cards**: 1 char padding
- **Between cards**: 1 blank line
- **Input bar**: grows with content, minimum 3 lines (border + 1 line + border)

### Responsive Behavior

- Terminal < 60 cols: Sidebar auto-hides, full-width feed
- Terminal < 40 cols: Status bar abbreviates, minimal mode
- Terminal < 30 cols: Error message, terminal too small

---

## Components

### Input Bar

The input bar is the primary interaction surface. It should feel premium and responsive.

**Structure:**
```
╭───────────────────────────────────────────╮
│❯ user input here                          │
╰───────────────────────── model: claude-4 ─╯
```

**Design:**
- Rounded corners (`╭╮╰╯`)
- `❯` prompt in `accent.primary` (purple)
- Hardware cursor as thin bar (`SteadyBar`) — terminal-native, no color control
- Info text at bottom-right corner (model name, status)
- Info spaced with single spaces: `─ text ─`
- Multi-line support with `Shift+Enter` or `Ctrl+J`
- Each logical line = 1 visual line (no wrapping)

### Message Feed

The center feed mixes different message types into a single chronological stream.

**Message Types:**

| Type | Visual |
|------|--------|
| **User** | Right-aligned, `accent.tertiary` (coral), subtle background |
| **Assistant** | Left-aligned, `text.primary`, full width |
| **Tool Call** | Collapsible, `text.muted` header, `▼` expand icon |
| **Tool Result** | Collapsible, success/error icon, code block if needed |
| **Thought** | Italic, `text.dim`, shows duration |
| **System** | Centered, `warning` or `info` color, subtle |

**Code Blocks:**
- Background: `bg.code`
- Line numbers: `text.muted`
- Syntax highlighting: Opaline `code.*` tokens
- No language badge (cleaner)

### Agent Cards (Sidebar)

Agents render as bordered cards, not a flat list.

```
╭─ agent-name ────────────────╮
│ ● current task description  │
│ model-name · elapsed-time  │
╰─────────────────────────────╯
```

**States:**
- **Running**: `●` in `accent.secondary` (cyan), pulsing indicator
- **Completed**: `✓` in `success` (green)
- **Failed**: `✗` in `error` (red)
- **Waiting**: `○` in `text.muted`

### Modals

Permission modal, command palette, and dialogs share a consistent style:

```
╭─ Title ────────────────────── [Esc] ─╮
│                                      │
│  Content here                        │
│                                      │
│  [Y] Confirm  [N] Cancel            │
╰──────────────────────────────────────╯
```

**Design:**
- Rounded corners, `border.unfocused` color
- Title in `accent.primary` with bold
- `[Esc]` hint on top-right
- Shadow offset 1 cell right, 1 cell down
- Backdrop dimmed to `bg.base`

### Command Palette

Object/Action/Arguments flow with 3-pane layout:

```
╭─ Command Palette ──────────── [Esc] ─╮
│ OBJECT     ACTION      ARGS          │
│ ▸ File...                             │
│   Agent...                            │
│   Model...                            │
│                                      │
│ ▸ type to filter...                   │
│ [↑↓] navigate  [Enter] select        │
╰──────────────────────────────────────╯
```

**Design:**
- Active pane header in `accent.secondary` (cyan)
- Selected item with `▸` indicator
- Query input with `❯` prompt

---

## Animations & Motion

### Principles

- **Subtle**: Motion should be felt, not noticed
- **Functional**: Animations convey state changes, not decoration
- **Terminal-friendly**: No smooth transitions (terminals are discrete), use stepped frames

### Specific Animations

| Animation | Implementation |
|-----------|----------------|
| **Running indicator** | `●` cycles through `○◐◑●` every 500ms (stepped, not smooth) |
| **Progress bars** | Filled blocks (`█`) with gradient from `accent.primary` to `accent.secondary` |
| **Typing indicator** | Three dots that cycle `·` `··` `···` |
| **Sidebar toggle** | Instant show/hide (terminals can't animate width) |
| **Modal appear** | Shadow draws first, then content (2-frame sequence) |
| **Scroll** | Instant jump with optional `↑`/`↓` indicators at edges |

### Progress Indicators

For long-running operations:
- **Determinate**: `███████░░░ 70%` with gradient colors
- **Indeterminate**: `◐◑◒◓` spinning in `accent.secondary`
- **Agent thinking**: Duration counter that updates every 100ms: `thinking... 1.2s`

---

## Typography

### Fonts (Terminal-Dependent)

- **Monospace required** — all rendering is grid-based
- **Recommended**: JetBrains Mono, Fira Code, or any font with good Unicode box-drawing support
- **Nerd Fonts optional** — for extra glyph variety, but not required

### Text Styles

| Purpose | Style |
|---------|-------|
| Headers | Bold, `accent.primary` |
| Body | Normal, `text.secondary` |
| Code | Normal, `bg.code` background, syntax-colored |
| Labels | Normal, `text.muted` |
| Active | Bold, `accent.secondary` |
| Error | Normal, `error` |

---

## Interaction Patterns

### Keyboard-First

All actions accessible via keyboard. Mouse is optional.

| Key | Action |
|-----|--------|
| `Enter` | Send message / Confirm |
| `Shift+Enter` | New line in input |
| `Esc` | Close modal / Cancel |
| `Ctrl+B` | Toggle sidebar |
| `Ctrl+K` / `Ctrl+P` | Command palette |
| `Ctrl+C` / `Ctrl+Q` | Quit |
| `↑/↓` | Navigate lists |
| `Tab` | Next field |

### Bash-Style Input

Full readline-style editing in input bar:
- `Ctrl+A` / `Ctrl+E` — Start/end of line
- `Ctrl+W` — Delete word backward
- `Ctrl+U` / `Ctrl+K` — Delete to start/end
- `Ctrl+B` / `Ctrl+F` — Back/forward char
- `Ctrl+N` / `Ctrl+P` — Next/prev line (in multi-line)

---

## Theme System

All visual properties are tokenized through Opaline:

```rust
// Getting a color
theme.color("text.primary")     // -> OpalineColor
theme.color("accent.secondary") // -> OpalineColor

// Converting to ratatui Color
let fg: ratatui::style::Color = theme.color("text.primary").into();
```

### Adding New Themes

1. Create a `.toml` theme file in Opaline format
2. Register in the theme discovery system
3. All components automatically use the new palette

No component should ever hardcode a color. All visual decisions are made through semantic tokens.

---

## Implementation Notes

### Rendering Order

1. Clear frame with `bg.base`
2. Draw top bar
3. Draw content area (message list + sidebar)
4. Draw input bar
5. Draw status bar
6. Draw modal shadow (if any)
7. Draw modal content (if any)
8. Position hardware cursor

### Performance

- Re-render entire frame on every tick (terminal constraint)
- Minimize allocations in render loop
- Use `Buffer::empty()` for off-screen compositing
- Cache theme color lookups (they're cheap but not free)

### Testing

All components must have unit tests for:
- Render output verification
- State machine transitions
- Keyboard input handling
- Edge cases (empty, overflow, wrap)

---

## Future Considerations

- **Split panes**: Horizontal splits for diff/code view (btop-style)
- **Mouse support**: Click-to-select, scroll, resize handles
- **Images**: Sixel/kitty graphics for rich content (if terminal supports)
- **More themes**: Light mode, high-contrast mode, colorblind-friendly
- **Animated transitions**: If we ever get a terminal with compositing

---

## References

- **btop** — Visual density, color-coded data, gradient bars
- **Crush** — Dark theme, thick accent bars, block glyphs, minimal chrome
- **Opaline** — Token-based theming system, semantic color naming
- **Ratatui** — Rust TUI framework, buffer-based rendering
- **Crossterm** — Cross-platform terminal control, input handling
