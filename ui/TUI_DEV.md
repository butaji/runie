# TUI Development Guide

## Quick Start

```bash
# Install bacon for auto-rebuild
cargo install bacon

# Start watching for changes
bacon

# Or run specific jobs:
bacon ui        # Fast type-check of runie-tui crate
bacon test      # Run grok_parity_test
bacon run       # Run the actual TUI binary
bacon parity    # Run grok_parity_test in release mode
```

## Development Workflow

### 1. Fast Iteration (paint.rs DSL changes)

```bash
bacon ui
```
This type-checks just the runie-tui crate. Sub-second feedback on save.

### 2. Visual Testing (component rendering)

```bash
bacon parity
```
Runs all 47+ component-level visual assertions. ~5s on incremental build.

### 3. Live TUI Testing

```bash
bacon run
```
Runs the actual TUI binary with hot-reload on save.

### 4. DSL Spec Development

Edit dspec JSON files and preview:

```bash
./scripts/watch-dspec.sh ui/dumps/scenarios/01_welcome_screen.dspec.json
```

Or compare against reference:

```bash
cargo run -p runie-tui --bin runie-dspec \
  ui/dumps/scenarios/01_welcome_screen.dspec.json \
  --diff ui/dumps/grok/01_welcome_screen.txt
```

## Paint DSL

The `paint` module provides a declarative Node tree for UI rendering:

```rust
use crate::paint::{paint, row, col, text, Node, StyleRef};

// Build a node tree
let node = col(vec![
    row(vec![
        Node::T(text("branch").accent()),
        Node::T(text(" ")),
        Node::T(text("path").secondary()),
        Node::Fill,
        Node::T(text("│ 20K / 512K │").dim()),
    ])
    .fill_trailing()
    .build(),
    Node::Blank { bg: StyleRef::BgBase },
]);

// Render to buffer
paint(&node, area, buf, &theme);
```

### Node Types

- `text("...")` - Single styled text run
- `row(vec![...])` - Horizontal layout
- `col(vec![...])` - Vertical layout  
- `border(node)` - Box border around child
- `pad(node)` - Add padding
- `Node::Fill` - Consume remaining space
- `Node::Blank` - Empty space

### Text Styles

```rust
text("...").primary()   // text_primary color
text("...").secondary() // text_secondary color
text("...").dim()       // text_dim with DIM modifier
text("...").muted()     // text_muted color
text("...").accent()    // accent_primary color
text("...").wrap()      // Multi-line wrapping
text("...").right_inset(n) // Right-aligned with n-cell gap
```

### Row Options

```rust
row(vec![...])
    .gap(1)           // Space between children
    .fill_trailing()   // Fill background to edge
    .bg(StyleRef::BgBase) // Background color
```

## Parity Testing

Run the comprehensive Grok parity test suite:

```bash
cargo run -p runie-tui --bin grok_parity_test --release
```

Tests cover:
- Header Bar: idle, streaming, token formats
- Welcome Menu: items, hints, tip text
- User Message: exact indent, timestamp position
- Thinking Block: collapsed, expanded states
- Tool Status: running, complete, error states
- Status Bar: idle, running states
- Input Border: normal, plan, always-approve modes

## Dspec Format

Dspec is a JSON DSL for describing TUI screens:

```json
{
  "width": 78,
  "height": 24,
  "root": {
    "type": "col",
    "children": [
      { "type": "blank" },
      {
        "type": "row",
        "fill_trailing": true,
        "children": [
          { "type": "text", "content": " branch", "style": "accent" },
          { "type": "fill" },
          { "type": "text", "content": "│ 20K / 512K │", "style": "dim" }
        ]
      }
    ]
  }
}
```

### Supported Types

- `text` - Styled text with content, style, right_inset
- `row` - Horizontal layout with gap, fill_trailing
- `col` - Vertical layout with gap
- `border` - Box border with optional title
- `pad` - Padding with top, right, bottom, left
- `fill` - Consume remaining space
- `blank` - Empty space with style
- `right_gap` - Gap on right side
- `HFill` - Fill + right-aligned text
- `If` - Conditional rendering
