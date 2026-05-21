# Ratatui 0.30.0 API Research

## Overview

Ratatui is a Rust library for building terminal user interfaces (TUIs). It provides a set of widgets and utilities to create complex Rust TUIs. The library is based on **immediate rendering with intermediate buffers** — for each frame, apps must render all widgets that are part of the UI.

**Key Resources:**
- [Website](https://ratatui.rs/)
- [API Docs](https://docs.rs/ratatui/0.30.0)
- [Examples](https://github.com/ratatui/ratatui/tree/main/examples)
- [Tutorials](https://ratatui.rs/tutorials/)

## 1. Core Types

### Terminal

`Terminal<B: Backend>` is the main interface for rendering frames to the terminal.

```rust
// Initialization (simple approach - recommended)
ratatui::run(|terminal| {
    loop {
        terminal.draw(|frame| {
            // render widgets
        })?;
        if event::read()?.is_key_press() {
            break Ok(());
        }
    }
})?
```

```rust
// Manual initialization
let mut terminal = ratatui::init()?;
// ... app logic
ratatui::restore()?;
```

### Frame

`Frame` provides access to the terminal area and renders widgets. Obtained in the `terminal.draw()` closure.

```rust
terminal.draw(|frame| {
    // Get the full area
    let area = frame.area();
    
    // Render a widget
    frame.render_widget(my_widget, area);
    
    // Render with state (for StatefulWidget)
    frame.render_stateful_widget(list, area, &mut list_state);
});
```

### Backend

Trait for terminal backend implementations. Built-in backends:
- `CrosstermBackend` (default, crossterm 0.29)
- `TermionBackend` (termion)
- `TermwizBackend` (termwiz)

### Buffer

2D grid of cells representing what will be rendered.

```rust
pub struct Buffer {
    pub area: Rect,
    pub content: Vec<Cell>,
}

impl Buffer {
    pub fn empty(area: Rect) -> Buffer;
    pub fn set_string(x, y, string, style);
    pub fn set_style(area, style);
    // ...
}
```

### Cell

Single terminal cell containing a character and style.

```rust
pub struct Cell {
    pub symbol: String,
    pub fg: Color,
    pub bg: Color,
    pub modifier: Modifier,
}
```

### Rect

Represents a rectangular area in the terminal.

```rust
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl Rect {
    pub fn new(x, y, width, height) -> Self;
    pub fn area(&self) -> u16;
    pub fn left(), right(), top(), bottom();
    pub fn inner(&self, margin: u16) -> Rect;  // Shrink by margin
}
```

---

## 2. Widget Trait

The core trait for all UI components.

```rust
pub trait Widget {
    fn render(self, area: Rect, buf: &mut Buffer);
}
```

**Key patterns:**

### Recommended: Reference-based Widget (0.26+)
```rust
struct MyWidget { content: String }

impl Widget for &MyWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Line::raw(&self.content).render(area, buf);
    }
}

// Usage - widget can be reused across frames
let widget = MyWidget { content: "Hello".to_string() };
frame.render_widget(&widget, area);
```

### Consuming Widget (legacy)
```rust
impl Widget for MyWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Line::raw(&self.content).render(area, buf);
    }
}

// Usage - consumed on each render
frame.render_widget(MyWidget { content: "Hello".to_string() }, area);
```

### StatefulWidget

For widgets that need to maintain state between renders.

```rust
pub trait StatefulWidget {
    type State;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State);
}

// Example: List with selection
let mut list_state = ListState::default();
frame.render_stateful_widget(list, area, &mut list_state);
```

---

## 3. Layout

`Layout` splits terminal space into rectangular areas using constraints.

### Basic Usage

```rust
use ratatui::layout::{Constraint, Direction, Layout};

// Vertical layout with 3 sections
let vertical = Layout::vertical([
    Constraint::Length(1),    // Fixed 1 row (e.g., header)
    Constraint::Min(0),      // Remaining space
    Constraint::Length(1),   // Fixed 1 row (e.g., footer)
]);

let [header, main, footer] = vertical.areas(frame.area());

// Horizontal layout within main area
let horizontal = Layout::horizontal([
    Constraint::Percentage(30),
    Constraint::Percentage(70),
]);
let [left, right] = horizontal.areas(main);
```

### Constraint Types

```rust
enum Constraint {
    Length(u16),        // Fixed number of cells
    Percentage(u16),     // Percentage of available space (0-100)
    Ratio(u32, u32),    // Ratio (e.g., Ratio(1, 3))
    Min(u16),            // Minimum with flexible remainder
    Max(u16),            // Maximum
    Fill(u16),           // Fill remaining space (u16 is weight)
}
```

### Direction

```rust
enum Direction {
    Horizontal,
    Vertical,
}
```

### Layout Methods

```rust
impl Layout {
    pub fn new(direction: Direction, constraints: Vec<Constraint>) -> Self;
    pub fn vertical(constraints) -> Self;
    pub fn horizontal(constraints) -> Self;
    pub fn split(&self, area: Rect) -> Rc<[Rect]>;  // Runtime
    pub fn areas<const N: usize>(&self, area: Rect) -> [Rect; N];  // Compile-time
    pub fn margin(self, u16) -> Self;
    pub fn spacing(self, u16) -> Self;
    pub fn flex(self, Flex) -> Self;
}

enum Flex {
    Legacy,       // Last item stretches (traditional behavior)
    Start,        // Align to start
    Center,       // Align to center
    End,          // Align to end
    SpaceBetween, // Equal space between items
    SpaceAround,  // Equal space around items
    SpaceEvenly,  // Equal space everywhere
}
```

### Flex Example

```rust
// Items centered with equal spacing
let layout = Layout::horizontal([
    Constraint::Length(10),
    Constraint::Length(10),
    Constraint::Length(10),
])
.flex(Flex::SpaceEvenly);
```

---

## 4. Creating Custom Widgets

### Complete Example: Custom Block Widget

```rust
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    widgets::Widget,
};

pub struct BorderedBlock {
    title: String,
    style: Style,
}

impl BorderedBlock {
    pub fn new(title: &str) -> Self {
        Self {
            title: title.to_string(),
            style: Style::default(),
        }
    }
    
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
}

// Recommended: implement for reference
impl Widget for &BorderedBlock {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Fill background
        buf.set_style(area, self.style);
        
        // Draw borders
        for x in area.x..area.x + area.width {
            // Top border
            buf[(x, area.y)].set_symbol("─");
            // Bottom border
            buf[(x, area.y + area.height - 1)].set_symbol("─");
        }
        for y in area.y..area.y + area.height {
            // Left border
            buf[(area.x, y)].set_symbol("│");
            // Right border
            buf[(area.x + area.width - 1, y)].set_symbol("│");
        }
        
        // Corners
        buf[(area.x, area.y)].set_symbol("┌");
        buf[(area.x + area.width - 1, area.y)].set_symbol("┐");
        buf[(area.x, area.y + area.height - 1)].set_symbol("└");
        buf[(area.x + area.width - 1, area.y + area.height - 1)].set_symbol("┘");
        
        // Draw title if space permits
        if area.width > 2 {
            let title_len = self.title.len().min(area.width as usize - 2);
            buf.set_string(
                area.x + 1,
                area.y,
                &self.title[..title_len],
                Style::default(),
            );
        }
    }
}
```

### Using the Widget

```rust
let block = BorderedBlock::new("Title").style(Style::default().fg(Color::Cyan));
frame.render_widget(&block, area);
```

### StatefulWidget Example: Counter

```rust
use ratatui::{buffer::Buffer, layout::Rect, widgets::StatefulWidget, style::{Color, Style}};

pub struct Counter {
    label: String,
}

pub struct CounterState {
    pub count: u32,
}

impl StatefulWidget for Counter {
    type State = CounterState;
    
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let text = format!("{}: {}", self.label, state.count);
        buf.set_string(area.x, area.y, &text, Style::default().fg(Color::Yellow));
    }
}

// Usage
let mut counter_state = CounterState { count: 0 };
let counter = Counter { label: "Count".to_string() };
frame.render_stateful_widget(counter, area, &mut counter_state);
```

---

## 5. Event Loop

Ratatui does **not** include input handling. Use backend library directly (e.g., crossterm).

### Crossterm Event Handling

```rust
use crossterm::event::{self, Event, KeyCode, KeyEventKind};

fn main() -> std::io::Result<()> {
    ratatui::run(|mut terminal| {
        loop {
            terminal.draw(|frame| {
                frame.render_widget(Paragraph::new("Press 'q' to quit"), frame.area());
            })?;
            
            // Handle events
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => break Ok(()),
                        KeyCode::Enter => { /* handle enter */ }
                        _ => {}
                    }
                }
            }
        }
    })?
}
```

### Event Types (crossterm)

```rust
enum Event {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize(u16, u16),  // Columns, Rows
    FocusGained,
    FocusLost,
}
```

### KeyCode Variants

```rust
KeyCode::Char('c')
KeyCode::Enter
KeyCode::Tab
KeyCode::Backspace
KeyCode::Esc
KeyCode::Left, Right, Up, Down
KeyCode::Home, End
KeyCode::PageUp, PageDown
KeyCode::F1, F2, ... F12
KeyCode::Delete
KeyCode::Insert
```

---

## 6. Built-in Widgets

All built-in widgets are in `ratatui::widgets`.

### Block

Borders, titles, and padding around content.

```rust
use ratatui::widgets::Block;
use ratatui::style::BorderType;

// Basic block
let block = Block::bordered()
    .title("Header")
    .title_alignment(Alignment::Center)
    .borders(Borders::ALL | Borders::LEFT);

// Different border styles
Block::bordered()
    .border_type(BorderType::Rounded)  // Or: Plain, Round, Thick
```

### Paragraph

Multi-line text with wrapping.

```rust
use ratatui::widgets::Paragraph;
use ratatui::text::{Line, Text};
use ratatui::style::Stylize;

// Simple
Paragraph::new("Hello world")

// Multi-line with wrapping
Paragraph::new("Long text...").wrap(Wrap { trim: true })

// Styled text
Paragraph::new(Text::from(vec![
    Line::from(vec![
        Span::raw("Normal "),
        Span::styled("Bold ", Style::new().bold()),
        Span::styled("Red ", Color::Red),
    ])
]))

// Alignment
Paragraph::new("Text").alignment(Alignment::Center)
```

### List

Scrollable list with optional selection.

```rust
use ratatui::widgets::{List, ListItem, ListState};

// Items
let items = vec![
    ListItem::new("Item 1"),
    ListItem::new("Item 2"),
    ListItem::new("Item 3"),
];

let list = List::new(items)
    .block(Block::bordered().title("List"))
    .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
    .highlight_symbol(">> ");

let mut state = ListState::default();
state.select(Some(0));  // Select first item

frame.render_stateful_widget(list, area, &mut state);
```

### Table

Grid of columns and rows.

```rust
use ratatui::widgets::{Table, Row, Cell};

let rows = vec![
    Row::new(vec!["Col1", "Col2", "Col3"]),
    Row::new(vec!["a", "b", "c"]).style(Style::new().fg(Color::Red)),
];

let table = Table::new(rows)
    .block(Block::bordered())
    .widths([10, 10, 10])
    .column_spacing(1);

frame.render_widget(table, area);
```

### Gauge

Progress indicator.

```rust
use ratatui::widgets::Gauge;

Gauge::new(0.75, "Progress")
    .label("75%")
    .gauge_style(Style::new().green())
```

### Tabs

Horizontal tab bar.

```rust
use ratatui::widgets::Tabs;

let titles = vec!["Tab1", "Tab2", "Tab3"];
let tabs = Tabs::new(titles)
    .block(Block::bordered())
    .select(0);

frame.render_widget(tabs, area);
```

### Sparkline

Compact data visualization.

```rust
use ratatui::widgets::Sparkline;

Sparkline::new(vec![1, 2, 3, 4, 5, 4, 3, 2, 1])
```

### Chart

Line/scatter plot (requires `all-widgets` or `widget-chart` feature).

### Canvas

Low-level drawing with ASCII art.

---

## 7. Styling

### Style Struct

```rust
use ratatui::style::{Color, Modifier, Style};

Style::default()
    .fg(Color::White)           // Foreground color
    .bg(Color::Black)           // Background color
    .add_modifier(Modifier::BOLD | Modifier::ITALIC)
    .remove_modifier(Modifier::DIM)
```

### Color Types

```rust
enum Color {
    // Named colors
    Black, Red, Green, Yellow, Blue, Magenta, Cyan, White,
    DarkGray, LightRed, LightGreen, LightYellow, LightBlue,
    LightMagenta, LightCyan, LightWhite,
    
    // Indexed (256-color palette)
    Indexed(u8),
    
    // True color (RGB)
    Rgb(u8, u8, u8),
    
    // Reset to terminal default
    Reset,
}
```

### Stylize Trait (Shorthand)

```rust
use ratatui::style::Stylize;

// Shorthand methods
"text".red()                    // fg = Red
"text".on_blue()                // bg = Blue
"text".bold().italic()
"text". underlined()

// Combining
Paragraph::new("Hello").red().on_white().bold()
```

### Modifier Flags

```rust
enum Modifier {
    BOLD,
    DIM,
    ITALIC,
    UNDERLINED,
    REVERSED,
    HIDDEN,
    CROSSED_OUT,
    SLOW_BLINK,
    RAPID_BLINK,
}
```

### Styled Text Types

```rust
use ratatui::text::{Line, Span, Text};

// Span - single styled segment
Span::raw("raw text")
Span::styled("styled", Style::new().red().bold())

// Line - row of spans
Line::from(vec![
    Span::raw("Hello "),
    Span::styled("World", Style::new().green()),
])

// Text - multiple lines
Text::from(vec![
    Line::from("Line 1"),
    Line::from("Line 2"),
])
```

---

## 8. Rendering to Buffer

### Buffer Methods

```rust
impl Buffer {
    // Create buffers
    pub fn empty(area: Rect) -> Buffer;
    pub fn filled(area: Rect, cell: Cell) -> Buffer;
    
    // Write to buffer
    pub fn set_string(x, y, string, style);
    pub fn set_stringn(x, y, string, max_width, style) -> (u16, u16);
    pub fn set_line(x, y, line, max_width) -> (u16, u16);
    pub fn set_span(x, y, span, max_width) -> (u16, u16);
    pub fn set_style(area, style);
    
    // Indexing
    pub fn cell((x, y)) -> Option<&Cell>;      // Safe accessor
    pub fn cell_mut((x, y)) -> Option<&mut Cell>;
    buf[(x, y)];                                // Index trait
}
```

### Cell Operations

```rust
cell.set_symbol("A");
cell.set_style(Style::new().fg(Color::Red));
cell.symbol();   // Get character
cell.style();    // Get style
```

### Frame Rendering

```rust
frame.render_widget(widget, area);              // Stateless
frame.render_stateful_widget(widget, area, state);  // With state

// Render widget at offset
frame.render_widget(widget, area.offset((x, y)));

// Render widget with style override
frame.render_widget(widget.style(style), area);
```

### Diff-Based Rendering (Under the Hood)

Ratatui uses diff-based rendering:
1. Previous and current buffers are compared
2. Only changed cells are written to terminal
3. Uses `Buffer::diff()` to compute minimal updates

---

## Complete Example: Simple TUI App

```rust
use ratatui::{
    DefaultTerminal,
    Frame,
    layout::{Constraint, Layout},
    style::Stylize,
    widgets::{Block, Paragraph},
};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};

fn main() -> std::io::Result<()> {
    ratatui::run(app)?;
    Ok(())
}

fn app(terminal: &mut DefaultTerminal) -> std::io::Result<()> {
    loop {
        terminal.draw(render)?;
        
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break Ok(()),
                    _ => {}
                }
            }
        }
    }
}

fn render(frame: &mut Frame) {
    let area = frame.area();
    
    // Vertical layout
    let layout = Layout::vertical([
        Constraint::Length(3),   // Header
        Constraint::Min(0),      // Content
        Constraint::Length(1),   // Footer
    ]);
    let [header, content, footer] = layout.areas(area);
    
    // Header
    let header_block = Block::bordered()
        .title(" My App ".cyan().bold())
        .title_alignment(ratatui::layout::Alignment::Center);
    frame.render_widget(header_block, header);
    
    // Content
    let paragraph = Paragraph::new(
        "Welcome to Ratatui!\nPress 'q' to quit."
    )
    .centered();
    frame.render_widget(paragraph, content);
    
    // Footer
    let footer_text = Paragraph::new("Press 'q' to exit".dim());
    frame.render_widget(footer_text, footer);
}
```

---

## Comparison: anvil-tui vs Ratatui

The `anvil-tui` crate in this workspace uses **custom abstractions** that mirror ratatui's API but are simplified implementations:

| Concept | anvil-tui | Ratatui |
|---------|------------|---------|
| Buffer | Custom `Buffer` struct | `ratatui::buffer::Buffer` |
| Layout | Custom `Layout`, `Constraint`, `Direction` | `ratatui::layout::Layout`, etc. |
| Styling | Custom `Style` with RGB tuples | Full `Style` with `Color`, `Modifier`, `Stylize` trait |
| Widget rendering | Custom `render(&mut self, buf, area, theme)` | `Widget::render(self, area, buf)` |
| Text | Custom `StyledText`, `AnsiRenderer` | `Span`, `Line`, `Text`, built-in ANSI rendering |

The anvil-tui approach predates or simplifies ratatui for specific needs. For new projects, using ratatui directly is recommended.

---

## Key Files in This Workspace

- `/crates/anvil-tui/src/lib.rs` - Public re-exports
- `/crates/anvil-tui/src/tui.rs` - Main TUI orchestration
- `/crates/anvil-tui/src/layout.rs` - Simplified layout types
- `/crates/anvil-tui/src/buffer.rs` - Simplified buffer implementation
- `/crates/anvil-tui/src/style.rs` - Simplified styling
- `/crates/anvil-tui/src/events.rs` - Event re-exports from crossterm
- `/crates/anvil-tui/src/components/*.rs` - UI components (TopBar, MessageList, etc.)

---

## Further Reading

- [Hello Ratatui Tutorial](https://ratatui.rs/tutorials/hello-ratatui/)
- [Counter App Tutorial](https://ratatui.rs/tutorials/counter-app/)
- [Layout Documentation](https://ratatui.rs/concepts/layout/)
- [Widget Documentation](https://ratatui.rs/concepts/widgets/)
- [Custom Widget Recipe](https://ratatui.rs/recipes/widgets/custom/)
- [Examples](https://github.com/ratatui/ratatui/tree/main/examples)
