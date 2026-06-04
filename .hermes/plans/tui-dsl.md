# TUI DSL — Plan

## Goal
Make TUI development at "edit, save, see" speeds.
A real DSL: declarative node trees, not manual `set_string` calls.

## Current pain
- 5200+ lines of hand-rolled `buf.set_string` / `set_line` / `set_style`
  across `crates/runie-tui/src/components/*/render*.rs`
- Each component is 100-400 lines of imperative buffer math
- Layout changes require understanding cell math, not intent
- Iterating on a UI element: edit Rust → ~1.5s cargo build → render

## DSL design — `crates/runie-tui/src/paint/`

A tiny Rust crate (or module) with:

```rust
pub enum Node {
    Text(Span),           // a single text run
    Row(RowSpec),         // horizontal container, gaps, fill semantics
    Col(ColSpec),         // vertical container
    Box(BoxSpec),         // border + padding wrapper
    Fill,                 // expand to consume remaining space
    Pad(PadSpec),         // padding-only wrapper
    Cond { cond: bool, then: Box<Node> },
}

pub struct Span { text: String, style: StyleRef }
pub enum StyleRef { Plain, Dim, Accent, Muted, Bg(String), Fg(String) }
```

The renderer is one function:

```rust
pub fn paint(node: &Node, area: Rect, buf: &mut Buffer, theme: &Theme)
```

It does the layout math (flex-like: row lays out children left-to-right with gaps, col lays out top-to-bottom, fill absorbs remaining space) and writes to the buffer.

## Component conversion strategy
Pick the simplest, most-bang-for-buck component first:
**Status bar / hotkey bar** in `crates/runie-tui/src/components/status_bar/`.
Currently 338 lines of layout math. Should be ~30 lines with the DSL.

Then:
- Plan modal (381 lines, very structured)
- Settings modal (140 lines)
- Shortcuts panel (246 lines)

## Speed win
After DSL exists, scenarios can declare their **expected** layout in a
small DSL script (not the dump file). The replay tool runs the renderer
on both the actual TUI state AND the DSL spec, and diffs the buffers.
This makes the spec the single source of truth for the "Grok looks
like this" assertion — no need to capture a frozen dump, no need to
maintain a `.txt` for parity.

## Iteration path (short, each as a separate commit)

1. **paint crate skeleton** — `Node`, `Span`, `paint()` function with
   Row/Col/Text/Fill. ~150 lines. Test it paints a known simple layout.
2. **status_bar migrated** — replace `render_ref` with a DSL
   declaration. The paint function does all the work.
3. **plan_modal migrated** — validates Box/Pad.
4. **DSpec scenario format** — `.dspec` files (TOML-like) that
   describe a screen, runnable through a new `bin/runie-dspec` tool.
5. **03_chat_simple via dspec** — declare the entire chat screen
   in a dspec; replace the .txt dump.

## Out of scope (for now)
- Visual layout editor
- Auto-generate components from a sketch
- Live reloading Rust code (cargo hot is too slow anyway)
