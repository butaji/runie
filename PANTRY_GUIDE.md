# Tidy Pantry — Widget Development Guide

The **Pantry** is a development environment for designing and testing Tidy TUI widgets in isolation. Think of it as a **Storybook for terminal UIs** — you build widgets with mock data, preview them instantly, and iterate without running the full application.

---

## What You Can Do

| Action | Command | What Happens |
|--------|---------|--------------|
| **Preview widgets** | `cargo run --bin preview -p anvil-pantry` | ANSI-colored widget dump to terminal |
| **Interactive browser** | `cd pantry && cargo run` | Navigate widgets with arrow keys, switch themes |
| **List widgets** | `cargo run --bin cargo-pantry -p anvil-cli -- list` | Print all registered ingredients |
| **Headless test** | `cargo run --bin dump -p anvil-pantry` | Plain text layout (for CI) |
| **Add new widget** | Edit `pantry/src/main.rs` | Create new `Ingredient`, register it |

---

## Current Widgets

### 1. TopBar
**Group**: `TopBar` | **Variant**: `Default`

Shows branch/path on left, stats on right.
```
main src/components                           4 ✓  4.56%
```

**Props**:
- `branch` (String) — Git branch name
- `path` (String) — Current directory
- `checks_passed` (Option<usize>) — Check count
- `percentage` (Option<f32>) — Completion %

### 2. MessageList
**Group**: `MessageList` | **Variant**: `With Messages`

Scrollable conversation history.
```
❯ Edit the copy on this page

◆ Thought for 2.5s

◆ Edit frontend/apps/website/src/app/(main)/cli/page.tsx
```

### 3. InputBar
**Group**: `InputBar` | **Variant**: `Default`

Bottom input field with prompt and model info.
```
┌──────────────────────────────────────────────────────────────┐
│ ❯ /btw                         grok-build-latest · always...│
└──────────────────────────────────────────────────────────────┘
```

### 4. Overlay
**Group**: `Overlay` | **Variant**: `Skills`

Modal popup with tabs and lists.
```
┌─ Skills [×] ─────────────────────────────────────────────────┐
│ Hooks   Plugins   Marketplace   Skills   MCP Servers         │
│                                                              │
│   ▸ rust-check    (local)                                    │
│   ▸ gcloud-auth   (local)                                    │
│   ▸ code-review   (local)                                    │
└──────────────────────────────────────────────────────────────┘
```

### 5. StatusBar
**Group**: `StatusBar` | **Variant**: `Chat Mode`

Key binding hints at the bottom.
```
Enter send | Shift-Tab normal | ^h home | ^q quit
```

---

## Quick Start

### Preview All Widgets (Colored)

```bash
cd /Users/admin/Code/GitHub/anvil
cargo run --bin preview -p anvil-pantry
```

Shows a full layout with all widgets rendered with ANSI colors. Press `Enter` to exit.

### Interactive Widget Browser

```bash
cd /Users/admin/Code/GitHub/anvil/pantry
cargo run
```

**Controls**:
- `↑/↓` — Navigate widgets in sidebar
- `Tab` — Switch between widget tabs (Widgets, Panes, Views, Styles)
- `t` — Toggle dark/light theme
- `c` — Cycle color depth (24-bit → 256 → 16 → 8 → mono)
- `b` — Cycle preview background
- `q` — Quit

### List Available Widgets

```bash
cargo run --bin cargo-pantry -p anvil-cli -- list
```

Output:
```
Available widgets:
  TopBar::Default
  MessageList::With Messages
  InputBar::Default
  Overlay::Skills
  StatusBar::Chat Mode
```

---

## Adding a New Widget

### Step 1: Create the Ingredient

Add a new struct to `pantry/src/main.rs`:

```rust
/// CodeBlock widget as a Ratatui Ingredient
pub struct CodeBlockIngredient;

impl Ingredient for CodeBlockIngredient {
    fn group(&self) -> &str {
        "CodeBlock"  // Widget category
    }

    fn name(&self) -> &str {
        "With Syntax Highlight"  // Specific variant
    }

    fn source(&self) -> &str {
        "anvil_tui::components::code_block::CodeBlock"
    }

    fn render(&self, area: Rect, buf: &mut Buffer) {
        // Your widget rendering code here
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Code")
            .style(Style::default().bg(Color::Rgb(30, 30, 30)));
        
        let inner = block.inner(area);
        block.render(area, buf);
        
        // Render code lines with syntax highlighting
        let line = Line::from(vec![
            Span::styled("  1  ", Style::default().fg(Color::Rgb(128, 128, 128))),
            Span::styled("fn main() {", Style::default().fg(Color::Rgb(224, 224, 224))),
        ]);
        Paragraph::new(line).render(inner, buf);
    }

    fn props(&self) -> &[PropInfo] {
        &[
            PropInfo {
                name: "lines",
                ty: "Vec<CodeLine>",
                description: "Lines of code to display",
            },
            PropInfo {
                name: "start_line",
                ty: "usize",
                description: "Starting line number",
            },
        ]
    }
}
```

### Step 2: Register It

Add to the `ingredients()` function:

```rust
pub fn ingredients() -> Vec<Box<dyn Ingredient>> {
    vec![
        Box::new(TopBarIngredient::new()),
        Box::new(MessageListIngredient),
        Box::new(InputBarIngredient),
        Box::new(OverlayIngredient),
        Box::new(StatusBarIngredient),
        Box::new(CodeBlockIngredient),  // ← Add here
    ]
}
```

### Step 3: Run

```bash
cd /Users/admin/Code/GitHub/anvil/pantry
cargo run
```

Your new widget appears in the sidebar under `CodeBlock` group.

---

## Creating Multiple Variants

One group can have multiple variants:

```rust
pub struct CodeBlockEmpty;
pub struct CodeBlockWithDiff;
pub struct CodeBlockLongFile;

impl Ingredient for CodeBlockEmpty {
    fn group(&self) -> &str { "CodeBlock" }
    fn name(&self) -> &str { "Empty" }
    // ...
}

impl Ingredient for CodeBlockWithDiff {
    fn group(&self) -> &str { "CodeBlock" }
    fn name(&self) -> &str { "With Diff" }
    // Shows added/removed lines...
}
```

Both appear under `CodeBlock` in the browser.

---

## Interactive Widgets

Add keyboard/mouse handling:

```rust
impl Ingredient for MyInteractiveWidget {
    fn interactive(&self) -> bool { true }

    fn handle_key(&mut self, code: KeyCode) -> bool {
        match code {
            KeyCode::Char('j') => { self.scroll_down(); true }
            KeyCode::Char('k') => { self.scroll_up(); true }
            _ => false,
        }
    }

    fn handle_mouse(&mut self, event: MouseEvent, area: Rect) -> bool {
        if is_click(&event) && area.contains(event.column, event.row) {
            self.clicked = true;
            true
        } else {
            false
        }
    }
}
```

---

## Theming

### Theme Configuration (`pantry/pantry.toml`)

```toml
[pantry.dark]
accent = "#5ccfe6"         # Cyan headings
panel_bg = "#252525"       # Panel background
text = "#e0e0e0"           # Primary text
text_dim = "#808080"       # Secondary text
border = "#404040"         # Borders
dark = true

[pantry.light]
accent = "#007acc"
panel_bg = "#f5f5f5"
text = "#333333"
text_dim = "#666666"
border = "#d0d0d0"
dark = false
```

**Runtime**: Press `t` in the browser to toggle dark/light.

---

## Workflow: Design → Preview → Integrate

### 1. Design in Pantry

```bash
cd pantry
# Edit src/main.rs — add your widget
cargo run  # Preview interactively
```

### 2. Test at Different Color Depths

Press `c` to cycle:
- TrueColor (24-bit) → 256 colors → 16 colors → 8 colors → Monochrome

Ensures your widget looks good on all terminals.

### 3. Copy to Production

Once satisfied, port the rendering logic to `crates/anvil-tui/src/components/` using the custom buffer API.

**Pantry (ratatui)**:
```rust
fn render(&self, area: Rect, buf: &mut Buffer) {
    Paragraph::new("Hello").render(area, buf);
}
```

**Production (anvil-tui)**:
```rust
fn render(&self, buf: &mut Buffer, area: Rect, theme: &Theme) {
    buf.set_str(area.x, area.y, "Hello", Style::new().fg(theme.palette.fg));
}
```

---

## Tips

**Use `cargo watch` for auto-reload**:
```bash
cargo install cargo-watch
cd pantry && cargo watch -x "run --bin preview"
```

**Test specific widget**:
```bash
# Only render the CodeBlock
cargo run --bin dump -p anvil-pantry | grep -A 20 "CodeBlock"
```

**Screenshot for PRs**:
```bash
# Run preview, take terminal screenshot
# The ANSI output renders correctly in:
# - iTerm2 / Terminal.app
# - VS Code integrated terminal
# - GitHub Actions (with proper terminal emulation)
```

---

## Architecture

```
pantry/
├── pantry.toml          # Theme configuration
├── src/
│   ├── main.rs          # Ingredient definitions + registration
│   └── bin/
│       ├── preview.rs   # ANSI-colored dump (for visual checking)
│       └── dump.rs      # Plain text dump (for CI/testing)
└── Cargo.toml           # ratatui + tui-pantry deps
```

**Pantry** uses `ratatui` (higher-level widget framework) for rapid iteration.
**Production** (`anvil-tui`) uses custom buffer for performance and control.

Both share the same design tokens (colors from screenshots), so widgets look identical.

---

## Summary

| Task | Command |
|------|---------|
| Preview widgets | `cargo run --bin preview -p anvil-pantry` |
| Interactive browser | `cd pantry && cargo run` |
| List widgets | `cargo run --bin cargo-pantry -p anvil-cli -- list` |
| Add widget | Edit `pantry/src/main.rs` + register in `ingredients()` |
| Toggle theme | Press `t` in browser |
| Test color depth | Press `c` in browser |
| Headless test | `cargo run --bin dump -p anvil-pantry` |
