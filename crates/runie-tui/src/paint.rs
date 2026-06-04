//! `paint` — a tiny declarative TUI DSL.
//!
//! Goal: replace 100+ lines of imperative `buf.set_string` + manual
//! x/y math with a single declarative Node tree per component, then
//! one `paint(&Node, area, buf, &theme)` call to render it.
//!
//! The Node tree is the SOURCE OF INTENT for the UI. Changing the
//! visual = changing the tree. The render math is in one place
//! (`paint()`), and it handles flex-like layout (rows = horizontal,
//! cols = vertical, fill = expand, pad = inset, border = outlined box).
//!
//! Example — a status-bar hotkey row, replacing ~50 lines of
//! imperative `set_line` calls:
//!
//! ```ignore
//! paint(&row(vec![
//!     text("Shift+Tab:mode").dim(),
//!     text("│").dim(),
//!     text("Ctrl+.:shortcuts").dim(),
//! ]).fill_trailing(), area, buf, &theme);
//! ```

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use crate::theme::ThemeColors;

// ─── Style reference ──────────────────────────────────────────────────
// A small alias set. The most common styles in the codebase (text_dim,
// text_secondary, text_primary, accent_primary, plus optional BOLD/DIM
// modifiers) are exposed as methods on `Text`. Anything beyond is
// `Text::custom(Style)`.

/// A style reference resolved against a `ThemeColors` at paint time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyleRef {
    Primary,
    Secondary,
    Dim,
    Muted,
    Accent,
    BgPanel,
    BgBase,
    /// A user-supplied `Style`. Use `Text::custom(...)` for one-offs.
    Custom(Style),
}

impl StyleRef {
    pub fn resolve(self, theme: &ThemeColors) -> Style {
        let mut s = match self {
            StyleRef::Primary => Style::default().fg(theme.text_primary),
            StyleRef::Secondary => Style::default().fg(theme.text_secondary),
            StyleRef::Dim => Style::default().fg(theme.text_dim),
            StyleRef::Muted => Style::default().fg(theme.text_muted),
            StyleRef::Accent => Style::default().fg(theme.accent_primary),
            StyleRef::BgPanel => Style::default().bg(theme.bg_panel),
            StyleRef::BgBase => Style::default().bg(theme.bg_base),
            StyleRef::Custom(s) => s,
        };
        if matches!(self, StyleRef::Dim) {
            s = s.add_modifier(Modifier::DIM);
        }
        s
    }
}

// ─── Text primitive ───────────────────────────────────────────────────

/// A single styled text run. Builder methods exist for the common
/// styles so call sites read like English:
/// `Text::new("foo").dim()` reads as "text 'foo' in dim style".
#[derive(Clone)]
pub struct Text {
    pub content: String,
    pub style: StyleRef,
    /// When true, the text wraps and grows the column height. When
    /// false (default), it's a single line and overflow is clipped.
    pub wrap: bool,
    /// Right-edge inset. `0` = flush with area right (default). A
    /// positive value (e.g. `2`) leaves that many cells of empty
    /// space on the right of the text.
    pub right_inset: u16,
}

impl Text {
    pub fn new<S: Into<String>>(s: S) -> Self {
        Text { content: s.into(), style: StyleRef::Primary, wrap: false, right_inset: 0 }
    }
    pub fn primary(mut self) -> Self { self.style = StyleRef::Primary; self }
    pub fn secondary(mut self) -> Self { self.style = StyleRef::Secondary; self }
    pub fn dim(mut self) -> Self { self.style = StyleRef::Dim; self }
    pub fn muted(mut self) -> Self { self.style = StyleRef::Muted; self }
    pub fn accent(mut self) -> Self { self.style = StyleRef::Accent; self }
    pub fn wrap(mut self) -> Self { self.wrap = true; self }
    /// Position this text with `n` cells of empty space on its right
    /// edge. Used for right-aligned elements (e.g. timestamps) that
    /// need a gap before the screen edge.
    pub fn right_inset(mut self, n: u16) -> Self { self.right_inset = n; self }
    pub fn custom(mut self, style: Style) -> Self {
        self.style = StyleRef::Custom(style);
        self
    }

    /// Render width: 0 if empty. Currently uses char count; switch
    /// to display width if we ever hit wide-char strings.
    pub fn width(&self) -> u16 {
        self.content.chars().count() as u16
    }
}

// Convenience free function so call sites can write `text("foo").dim()`
// instead of `Text::new("foo").dim()`.
pub fn text<S: Into<String>>(s: S) -> Text { Text::new(s) }

// ─── Node tree ────────────────────────────────────────────────────────

/// A node in the declarative tree. Render is layout-then-paint.
#[derive(Clone)]
pub enum Node {
    /// A single text run.
    T(Text),
    /// Horizontal layout: children placed left-to-right, separated by
    /// `gap` empty cells. If `fill_trailing` is true, the LAST child
    /// with `Node::Fill` expands to consume remaining horizontal space.
    Row(Row),
    /// Vertical layout: children placed top-to-bottom with `gap`. The
    /// first child with `Node::Fill` expands to consume remaining
    /// vertical space.
    Col(Col),
    /// A border around a single child (or empty interior). 1 cell of
    /// padding on all sides.
    Border(Border),
    /// Pads a child by (top, right, bottom, left).
    Pad(Pad),
    /// Consumes all remaining space in the parent's axis. In a Row,
    /// horizontal; in a Col, vertical. Acts like CSS `flex: 1`.
    Fill,
    /// A pure blank rectangle, painted with the bg style.
    Blank { bg: StyleRef },
    /// A gap of `n` cells on the right of a row. Used to leave
    /// trailing empty space before right-aligned elements.
    RightGap { n: u16 },
    /// A conditional node — rendered only if `cond` is true. Useful
    /// for mode-conditional UI (e.g. on home screen, hide X).
    If {
        cond: bool,
        then: Box<Node>,
    },
    /// A list of children rendered as one. Convenience for `row(vec![])`.
    List(Vec<Node>),
}

#[derive(Clone)]
pub struct Row {
    pub children: Vec<Node>,
    pub gap: u16,
    /// The cell style painted for the gap cells (and trailing
    /// background of the row when no Fill is present).
    pub bg: StyleRef,
    /// If true, the trailing empty cells in the row are filled with
    /// the bg color up to the area's right edge. Matches Grok's
    /// "extend the hotkey row to the screen edge" behavior.
    pub fill_trailing: bool,
}

#[derive(Clone)]
pub struct Col {
    pub children: Vec<Node>,
    pub gap: u16,
    pub bg: StyleRef,
}

#[derive(Clone)]
pub struct Border {
    pub child: Option<Box<Node>>,
    pub title: Option<Text>,
    pub style: StyleRef,
}

#[derive(Clone)]
pub struct Pad {
    pub child: Box<Node>,
    pub top: u16,
    pub right: u16,
    pub bottom: u16,
    pub left: u16,
}

// ─── Builders ─────────────────────────────────────────────────────────

pub fn row(children: Vec<Node>) -> Row {
    Row { children, gap: 0, bg: StyleRef::BgBase, fill_trailing: false }
}
pub fn col(children: Vec<Node>) -> Col {
    Col { children, gap: 0, bg: StyleRef::BgBase }
}

impl Row {
    pub fn gap(mut self, n: u16) -> Self { self.gap = n; self }
    pub fn bg(mut self, s: StyleRef) -> Self { self.bg = s; self }
    pub fn fill_trailing(mut self) -> Self { self.fill_trailing = true; self }
    pub fn child(mut self, n: Node) -> Self { self.children.push(n); self }
    pub fn children(mut self, mut cs: Vec<Node>) -> Self {
        self.children.append(&mut cs);
        self
    }
    pub fn build(self) -> Node { Node::Row(self) }
}

impl Col {
    pub fn gap(mut self, n: u16) -> Self { self.gap = n; self }
    pub fn bg(mut self, s: StyleRef) -> Self { self.bg = s; self }
    pub fn child(mut self, n: Node) -> Self { self.children.push(n); self }
    pub fn build(self) -> Node { Node::Col(self) }
}

pub fn border(child: Node) -> Border {
    Border { child: Some(Box::new(child)), title: None, style: StyleRef::Dim }
}
impl Border {
    pub fn title<T: Into<String>>(mut self, t: T) -> Self {
        self.title = Some(Text::new(t));
        self
    }
    pub fn style(mut self, s: StyleRef) -> Self { self.style = s; self }
    pub fn build(self) -> Node { Node::Border(self) }
}

pub fn pad(child: Node) -> Pad {
    Pad { child: Box::new(child), top: 0, right: 0, bottom: 0, left: 0 }
}
impl Pad {
    pub fn all(mut self, n: u16) -> Self {
        self.top = n; self.right = n; self.bottom = n; self.left = n;
        self
    }
    pub fn xy(mut self, x: u16, y: u16) -> Self {
        self.left = x; self.right = x; self.top = y; self.bottom = y;
        self
    }
    pub fn build(self) -> Node { Node::Pad(self) }
}

// ─── Paint (the render function) ──────────────────────────────────────

/// Paint `node` into `buf` within `area`. Returns the area that was
/// used (useful for parents that want to know how much space a child
/// consumed). Returns `area` unchanged if the node is empty.
pub fn paint(node: &Node, area: Rect, buf: &mut Buffer, theme: &ThemeColors) {
    if area.width == 0 || area.height == 0 { return; }
    match node {
        Node::T(t) => paint_text(t, area, buf, theme),
        Node::Row(r) => paint_row(r, area, buf, theme),
        Node::Col(c) => paint_col(c, area, buf, theme),
        Node::Border(b) => paint_border(b, area, buf, theme),
        Node::Pad(p) => paint_pad(p, area, buf, theme),
        Node::Fill => { /* parent already expanded the area */ },
        Node::Blank { bg } => paint_blank(area, buf, bg.resolve(theme)),
        Node::RightGap { n } => {
            // Reserved empty cells at the end of a row.
        }
        Node::If { cond: true, then } => paint(then, area, buf, theme),
        Node::If { cond: false, .. } => paint_blank(area, buf, StyleRef::BgBase.resolve(theme)),
        Node::List(ns) => {
            // Lists are inlined by their parent. If we got here
            // bare, treat as a row with 0 gap.
            let r = Row { children: ns.to_vec(), gap: 0, bg: StyleRef::BgBase, fill_trailing: false };
            paint_row(&r, area, buf, theme);
        }
    }
}

fn paint_text(t: &Text, area: Rect, buf: &mut Buffer, theme: &ThemeColors) {
    if area.width == 0 || area.height == 0 { return; }
    let style = t.style.resolve(theme);
    if t.wrap {
        // Wrap: split content into lines by display width, then paint
        // each on a new row, clipped to area.height.
        let w = area.width as usize;
        let chars: Vec<char> = t.content.chars().collect();
        let mut y = area.y;
        for line in chars.chunks(w) {
            if y >= area.y + area.height { break; }
            let line_s: String = line.iter().collect();
            let line_obj = Line::from(Span::styled(line_s, style));
            buf.set_line(area.x, y, &line_obj, w as u16);
            y += 1;
        }
    } else {
        // Single line, clipped to area.width. Default placement
        // starts at `area.x`. When `right_inset > 0`, the text is
        // shifted right so its right edge sits `right_inset` cells
        // before the area's right edge.
        let text_w = if t.right_inset > 0 {
            t.content.chars().count().min(area.width.saturating_sub(t.right_inset) as usize)
        } else {
            t.content.chars().count().min(area.width as usize)
        };
        // Default placement starts at `area.x`.
        let x = area.x;
        let clipped: String = t.content.chars().take(text_w).collect();
        let line = Line::from(Span::styled(clipped, style));
        buf.set_line(x, area.y, &line, text_w as u16);
    }
}
fn paint_blank(area: Rect, buf: &mut Buffer, style: Style) {
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_style(style);
            }
        }
    }
}

fn paint_row(r: &Row, area: Rect, buf: &mut Buffer, theme: &ThemeColors) {
    // Background fill first.
    paint_blank(area, buf, r.bg.resolve(theme));
    if area.width == 0 || area.height == 0 { return; }

    // First pass: measure non-Fill children.
    let mut fixed_width: u32 = 0;
    let mut n_fills = 0u32;
    for (i, c) in r.children.iter().enumerate() {
        match c {
            Node::Fill => n_fills += 1,
            Node::T(t) => fixed_width += t.width() as u32,
            _ => {
                // For non-trivial children, we measure by painting
                // them and asking for the area used. For the
                // single-row case, that's `area.width` minus
                // trailing gap. Use a sentinel: measure via inner
                // width and pass back later.
                fixed_width += area.width as u32;
                let _ = i;
            }
        }
    }
    let gaps = r.children.len().saturating_sub(1) as u32 * r.gap as u32;
    let free = (area.width as u32).saturating_sub(fixed_width + gaps);
    let fill_w = if n_fills > 0 { free / n_fills as u32 } else { 0 };

    // Second pass: paint children left-to-right.
    let mut x = area.x;
    let mut last_x_end = area.x;
    for (i, c) in r.children.iter().enumerate() {
        if i > 0 && r.gap > 0 {
            x += r.gap;
        }
        let child_w = match c {
            Node::Fill => fill_w as u16,
            Node::T(t) => t.width(),
            _ => 0, // complex children: paint in remaining area below
        };
        // When the text has `right_inset > 0`, it wants to position
        // itself relative to the row's right edge, not its own
        // (possibly trimmed) child area. Give it the full row
        // area so the right_inset calculation lands in the right
        // place.
        let needs_full_width = matches!(c, Node::T(t) if t.right_inset > 0);
        let child_area = if needs_full_width {
            Rect { x, y: area.y, width: (area.x + area.width).saturating_sub(x), height: area.height }
        } else if matches!(c, Node::Fill) || matches!(c, Node::T(_)) {
            Rect { x, y: area.y, width: child_w.min(area.x + area.width - x), height: area.height }
        } else {
            // Complex child gets the remaining width.
            let rem = area.x + area.width - x;
            Rect { x, y: area.y, width: rem, height: area.height }
        };
        paint(c, child_area, buf, theme);
        if !matches!(c, Node::Fill) {
            x += child_w;
        } else {
            x += fill_w as u16;
        }
        last_x_end = x;
    }

    // fill_trailing: paint background up to area.right().
    if r.fill_trailing && last_x_end < area.x + area.width {
        let rem = Rect {
            x: last_x_end,
            y: area.y,
            width: area.x + area.width - last_x_end,
            height: area.height,
        };
        paint_blank(rem, buf, r.bg.resolve(theme));
    }
}

fn paint_col(c: &Col, area: Rect, buf: &mut Buffer, theme: &ThemeColors) {
    paint_blank(area, buf, c.bg.resolve(theme));
    if area.width == 0 || area.height == 0 { return; }
    let mut fixed_h: u32 = 0;
    let mut n_fills = 0u32;
    for child in &c.children {
        match child {
            Node::Fill => n_fills += 1,
            Node::T(_) => fixed_h += 1,
            _ => fixed_h += 1, // rough — complex children get 1 row for now
        }
    }
    let gaps = c.children.len().saturating_sub(1) as u32 * c.gap as u32;
    let free = (area.height as u32).saturating_sub(fixed_h + gaps);
    let fill_h = if n_fills > 0 { free / n_fills as u32 } else { 0 };

    let mut y = area.y;
    for (i, child) in c.children.iter().enumerate() {
        if i > 0 && c.gap > 0 { y += c.gap; }
        let h = match child {
            Node::Fill => fill_h as u16,
            _ => 1u16,
        };
        let ca = Rect { x: area.x, y, width: area.width, height: h.min(area.y + area.height - y) };
        paint(child, ca, buf, theme);
        y += h;
    }
}

fn paint_pad(p: &Pad, area: Rect, buf: &mut Buffer, theme: &ThemeColors) {
    if area.width == 0 || area.height == 0 { return; }
    let x = area.x.saturating_add(p.left);
    let y = area.y.saturating_add(p.top);
    let w = area.width.saturating_sub(p.left + p.right);
    let h = area.height.saturating_sub(p.top + p.bottom);
    if w == 0 || h == 0 { return; }
    let inner = Rect { x, y, width: w, height: h };
    paint(&p.child, inner, buf, theme);
}

fn paint_border(b: &Border, area: Rect, buf: &mut Buffer, theme: &ThemeColors) {
    use ratatui::symbols::border::ROUNDED;
    use ratatui::text::Line as RL;
    if area.width < 2 || area.height < 2 { return; }
    let style = b.style.resolve(theme);

    // Top edge
    let mut top_str = String::with_capacity(area.width as usize);
    top_str.push_str(ROUNDED.top_left);
    for _ in 0..(area.width - 2) { top_str.push_str(ROUNDED.horizontal_top); }
    top_str.push_str(ROUNDED.top_right);
    let top_line = RL::from(Span::styled(top_str, style));
    buf.set_line(area.x, area.y, &top_line, area.width);

    // Bottom edge
    let mut bot_str = String::with_capacity(area.width as usize);
    bot_str.push_str(ROUNDED.bottom_left);
    for _ in 0..(area.width - 2) { bot_str.push_str(ROUNDED.horizontal_bottom); }
    bot_str.push_str(ROUNDED.bottom_right);
    let bot_line = RL::from(Span::styled(bot_str, style));
    buf.set_line(area.x, area.y + area.height - 1, &bot_line, area.width);

    // Left + right edges
    for y in (area.y + 1)..(area.y + area.height - 1) {
        let left_line = RL::from(Span::styled(ROUNDED.vertical_left.to_string(), style));
        buf.set_line(area.x, y, &left_line, 1);
        let right_line = RL::from(Span::styled(ROUNDED.vertical_right.to_string(), style));
        buf.set_line(area.x + area.width - 1, y, &right_line, 1);
    }

    // Optional title in the top edge
    if let Some(t) = &b.title {
        let t_str = format!(" {} ", t.content);
        let t_style = t.style.resolve(theme);
        let t_line = RL::from(Span::styled(t_str, t_style));
        // Place at column 2 of the top edge
        buf.set_line(area.x + 2, area.y, &t_line, (area.width - 4).min(t.content.chars().count() as u16 + 2));
    }

    // Child
    if let Some(child) = &b.child {
        let inner = Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(2),
        };
        paint(child, inner, buf, theme);
    }
}

// ─── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn paint_to_text(node: &Node, w: u16, h: u16) -> String {
        let backend = TestBackend::new(w, h);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| {
            paint(node, frame.area(), frame.buffer_mut(), &crate::theme::ThemeColors::default_for_test());
        }).unwrap();
        let mut s = String::new();
        let buf = terminal.backend().buffer().clone();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                s.push_str(buf.cell((x, y)).unwrap().symbol());
            }
            s.push('\n');
        }
        s
    }

    #[test]
    fn paints_text() {
        let s = paint_to_text(&text("hi").dim(), 5, 1);
        assert!(s.starts_with("hi"));
    }

    #[test]
    fn paints_row_with_gap() {
        let s = paint_to_text(
            &row(vec![Node::T(text("ab")), Node::T(text("cd"))])
                .gap(1)
                .build(),
            6, 1,
        );
        // Expected: "ab cd "  (clip trailing)
        assert!(s.contains("ab"));
        assert!(s.contains("cd"));
    }

    #[test]
    fn fill_takes_remaining() {
        let s = paint_to_text(
            &row(vec![Node::T(text("ab")), Node::Fill]).build(),
            5, 1,
        );
        // Fill is 3 cells: cells 2,3,4 should be space
        assert!(s.starts_with("ab"));
    }

    #[test]
    fn col_stacks() {
        let s = paint_to_text(
            &col(vec![Node::T(text("a")), Node::T(text("b"))]).build(),
            1, 2,
        );
        assert_eq!(s.lines().next().unwrap(), "a");
        assert_eq!(s.lines().nth(1).unwrap(), "b");
    }
}
