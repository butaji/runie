# tui-pantry Analysis

**Repo**: taho-inc/tui-pantry | **Version**: 0.4.0 | **Stars**: 22 | **License**: Apache-2.0 / MIT

---

## 1. What tui-pantry Provides

**tui-pantry is NOT a widget library.** It's a **component-driven development harness** for Ratatui (the tui crate successor) — equivalent to Storybook for React. It lets you build, preview, and iterate on terminal widgets in isolation, outside your application, with zero application dependencies.

### Core Abstractions

| Concept | Purpose |
|---------|---------|
| **Ingredient** | A single "story" — one specific widget configuration with mock data. Implements the `Ingredient` trait. |
| **PropInfo** | Documents a widget's configurable surface (`name`, `ty`, `description`). |
| **Tab** | Top-level category: Widgets, Panes, Views, Styles |
| **Section** | Optional grouping above widget level (e.g., "Charts", "Layout") |
| **Group** | Widget name shown as collapsible tree parent (e.g., "Gauge") |
| **Variant** | Specific configuration under a group (e.g., "Low (green)", "High (red)") |

### Features

- **Interactive TUI browser** — navigate ingredients, switch tabs, toggle dark/light theme, cycle color depth (24-bit → 256 → 16 → 8 → mono), cycle preview backgrounds
- **Headless dump mode** (`cargo pantry dump`) — render ingredients to ANSI escape sequences on stdout for CI/testing
- **List mode** (`cargo pantry list`) — print all `group/variant` pairs
- **`pantry.toml` driven** — colors, typography, ingredient discovery all from TOML config
- **`pantry_ingredients!` proc macro** — reads `pantry.toml` at compile time, generates ingredient aggregation code, tracks file for rebuild-on-change
- **Stylesheet system** — color palettes and typography defined in TOML, rendered as swatches in the Styles tab
- **`render_centered` layout helper** — centers a widget on one or both axes
- **Mouse + keyboard input forwarding** — interactive ingredients receive events when preview pane has focus
- **`Pane` struct** — chrome wrapper (border + title) around an ingredient

### Example Pantry Widgets (from `examples/example-pantry/src/widgets/`)

Shows patterns for: `Gauge`, `Sparkline`, `BarChart`, `Chart`, `Table`, `List`, `Paragraph`, `Block`, `Scrollbar`, `Canvas`, `StatusBadge`, `KeyValue`, `LogStream`, `EmptyState`, `TruncatedText`, `Tabs`, `Logo`

---

## 2. Key Types and Traits

### `Ingredient` Trait (core unit of display)

```rust
pub trait Ingredient: Send {
    // Required
    fn group(&self) -> &str;        // "Node Table"
    fn name(&self) -> &str;         // "Default"
    fn source(&self) -> &str;       // "my_crate::widgets::node_table"
    fn render(&self, area: Rect, buf: &mut Buffer);

    // Optional (with defaults)
    fn tab(&self) -> &str { "Widgets" }           // "Widgets" | "Panes" | "Views" | "Styles"
    fn section(&self) -> Option<&str> { None }    // "Charts", "Layout", etc.
    fn description(&self) -> &str { "" }
    fn props(&self) -> &[PropInfo] { &[] }
    fn interactive(&self) -> bool { false }
    fn handle_key(&mut self, code: KeyCode) -> bool { false }
    fn handle_mouse(&mut self, event: MouseEvent, area: Rect) -> bool { false }
    fn animated(&self) -> bool { false }
}
```

### `PropInfo`

```rust
pub struct PropInfo {
    pub name: &'static str,        // "ratio"
    pub ty: &'static str,          // "f64"
    pub description: &'static str, // "Fill from 0.0 to 1.0"
}
```

### `Pane<'a>`

Chrome wrapper widget:
```rust
pub struct Pane<'a> {
    title: &'a str,
    ingredient: &'a dyn Ingredient,
    focused: bool,
    theme: &'a PantryTheme,
}
```

### `PantryTheme` / `ThemePair`

Catppuccin Mocha (dark) + Latte (light) themes with per-field TOML overrides:
```rust
pub struct PantryTheme {
    pub accent: Color, pub panel_bg: Color, pub cursor_bg: Color,
    pub border: Color, pub border_dim: Color, pub text: Color,
    pub text_dim: Color, pub doc_accent: Color, pub doc_text: Color,
    pub doc_type: Color, pub indicator: Color, pub dark: bool,
}
```

### `PreviewBackgrounds`

Named preview surface colors from TOML for testing widgets on different backgrounds.

### `render_centered` Helper

```rust
pub fn render_centered(
    widget: impl Widget,
    width: Option<Constraint>,   // None = fill
    height: Option<Constraint>,  // None = fill
    area: Rect,
    buf: &mut Buffer,
)
```

### `is_click` Helper

```rust
pub fn is_click(event: &MouseEvent) -> bool
```

---

## 3. Comparison with anvil-tui

| Aspect | anvil-tui | tui-pantry |
|--------|-----------|------------|
| **Purpose** | Production TUI widget rendering | Development harness for isolated widget preview |
| **Backend** | Custom minimal (no ratatui) | Ratatui 0.30 |
| **Theme system** | `Theme` + `ColorPalette` structs with hardcoded RGB tuples | `PantryTheme` with Catppuccin defaults + TOML override |
| **Style system** | Custom `Style` struct with builder pattern, ANSI rendering | Delegates to Ratatui's `Style` |
| **Layout** | Custom `Rect`, `Constraint`, `Layout` | Delegates to Ratatui's layout + `render_centered` helper |
| **Buffer** | Custom `Buffer` + `Cell` | Delegates to Ratatui's `Buffer` |
| **Components** | `Overlay`, `CodeBlock`, `Collapsible`, `AgentList`, `TopBar`, `MessageList`, `InputBar`, `StatusBar` | N/A (ingredient wrappers around Ratatui stock widgets + custom examples) |
| **Configuration** | Hardcoded theme values | `pantry.toml` with `[config]`, `[pantry.dark]`, `[pantry.light]`, `[colors.<family>]`, `[typography]` |
| **Proc macros** | None | `pantry_ingredients!` for ingredient discovery |
| **Interactive preview** | N/A | Full keyboard/mouse handling with `handle_key`, `handle_mouse` |
| **Color depth emulation** | None | Cycles through 24-bit → 256 → 16 → 8 → mono |
| **Headless CI mode** | None | `cargo pantry dump` → ANSI stdout |

**Fundamental difference**: anvil-tui is a self-contained TUI rendering crate. tui-pantry is a development tool that wraps Ratatui to provide isolated preview of Ratatui widgets. They don't overlap in purpose.

---

## 4. What We Can Borrow / Integrate

### High Value

1. **`render_centered` layout helper** — simple but useful utility we lack. Trivially copy-pasteable into our `layout.rs`.

2. **TOML-driven theme palette** — our `ColorPalette` is hardcoded. A `pantry.toml` approach with color families, scale strips (100–900 numeric keys), and hex/named color parsing is a proven pattern for design token management. The `stylesheet.rs` parsing code (especially `parse_color`) is worth studying.

3. **`PropInfo` + `props()` documentation pattern** — our components have no self-describing property surface. Adding `props()` to document what our widgets accept helps discoverability and future tooling.

4. **Color depth emulation logic** (`color_depth.rs`) — useful for testing how our ANSI rendering holds up at lower color depths. ~350 lines of code.

5. **The `Ingredient` trait pattern for our widget docs** — we could adopt similar `group`/`name`/`source`/`description`/`props()` metadata to document our widget variants systematically.

6. **Theme toggle at runtime** — our theme is locked dark. The `t` key to swap dark/light in tui-pantry is a UX pattern worth adopting.

### Low/N/A Value

- **tui-pantry doesn't provide widgets** — it provides a harness. It won't give us new UI components.
- **Its component examples** (EmptyState, LogStream, etc.) are simple demonstration code, not production-quality.
- **`pantry_ingredients!` macro** — specific to ratatui's `Ingredient` trait, not directly applicable.
- **The full pantry browser** — a separate binary/harness, not something to integrate into anvil itself.

---

## 5. Source Code Structure

```
tui-pantry/
├── Cargo.toml              # workspace root; members: tui-pantry, tui-pantry-macros, example-pantry
├── src/
│   ├── lib.rs              # re-exports, Ingredient trait, PropInfo, run! macro, is_click, render_centered
│   ├── app.rs              # App state machine, runs the pantry TUI
│   ├── ui.rs               # All terminal UI rendering (sidebar, preview pane, tabs, etc.)
│   ├── nav.rs              # Navigation tree, keyboard navigation logic
│   ├── ingredient.rs        # Ingredient trait object handling
│   ├── pane.rs             # Pane chrome widget (border + title)
│   ├── theme.rs            # PantryTheme, ThemePair, PreviewBackgrounds
│   ├── stylesheet.rs       # TOML parsing for colors/typography
│   ├── color_depth.rs      # Color depth emulation (24-bit → 256 → 16 → 8 → mono)
│   ├── dump.rs             # Headless dump/list mode
│   ├── layout.rs           # render_centered helper
│   └── bin/cargo-pantry.rs # cargo-pantry binary
├── crates/
│   └── tui-pantry-macros/
│       └── src/lib.rs      # pantry_ingredients! proc macro
├── examples/
│   └── example-pantry/     # Reference pantry with ratatui stock widgets
│       └── src/
│           ├── widgets/    # 19 widget examples (gauge, table, chart, etc.)
│           ├── panes/      # Composed sections
│           ├── views/      # Full-page layouts
│           └── styles/     # Color palette + typography
└── scaffold/               # Templates for cargo pantry init
```

**Module size estimates** (from file sizes):
- `nav.rs`: ~26KB — largest, navigation tree + keyboard handling
- `ui.rs`: ~28KB — full rendering
- `stylesheet.rs`: ~19KB — TOML parsing
- `app.rs`: ~14KB — app state machine
- `color_depth.rs`: ~11KB — color depth emulation

---

## 6. Dependencies

### `tui-pantry` (main crate)

| Dependency | Version | Purpose |
|------------|---------|---------|
| **ratatui** | 0.30 | Backend widget rendering |
| **toml** | 0.9 (features: display, preserve_order) | Parsing `pantry.toml` |
| **tui-pantry-macros** | 0.4.0 (path) | `pantry_ingredients!` proc macro |

### `tui-pantry-macros` (proc macro crate)

| Dependency | Purpose |
|------------|---------|
| **quote** | Token stream generation |
| **proc-macro2** | Token parsing |

No heavy dependencies. Zero-runtime-footprint for the macro itself.

---

## Summary

**tui-pantry is a Storybook equivalent for Ratatui.** It provides:
- An `Ingredient` trait — the core abstraction for a widget "story"
- A TOML-driven stylesheet system with color palettes and typography
- An interactive TUI browser for navigating/previewing widgets
- Headless CI modes (`dump`, `list`)
- Color depth emulation and preview backgrounds
- Catppuccin Mocha/Latte theme system with runtime dark/light toggle

**It does NOT provide widgets** — it's a development harness, not a widget library.

**For anvil-tui**, the most actionable takeaways are:
1. Copy `render_centered` into our layout module
2. Study `stylesheet.rs` for TOML color/typography parsing patterns if we want a config-file-driven theme
3. Consider adopting `PropInfo`/`props()` documentation for our widget variants
4. The color depth emulation in `color_depth.rs` is useful testing infrastructure
5. Our fundamental rendering model (custom Style/Buffer/Layout) vs Ratatui are incompatible — we would need to port to Ratatui to use tui-pantry directly, which is a major undertaking
