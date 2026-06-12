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
        let mut s = resolve_base_style(self, theme);
        if matches!(self, StyleRef::Dim) {
            s = s.add_modifier(Modifier::DIM);
        }
        s
    }
}

fn resolve_base_style(style: StyleRef, theme: &ThemeColors) -> Style {
    match style {
        StyleRef::Primary => Style::default().fg(theme.text_primary),
        StyleRef::Secondary => Style::default().fg(theme.text_secondary),
        StyleRef::Dim => Style::default().fg(theme.text_dim),
        StyleRef::Muted => Style::default().fg(theme.text_muted),
        StyleRef::Accent => Style::default().fg(theme.accent_primary),
        StyleRef::BgPanel => Style::default().bg(theme.bg_panel),
        StyleRef::BgBase => Style::default().bg(theme.bg_base),
        StyleRef::Custom(s) => s,
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
    /// A horizontal fill with a right-aligned text. Fills remaining
    /// space with bg, then places `text` at the right edge with `n`
    /// cells of gap. This is the most common pattern in TUI rows
    /// (e.g. top bar with right-aligned chip).
    HFillText { text: Text, gap: u16 },
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
mod paint_builders;
pub use paint_builders::{row, col, border, pad};

// ─── Paint (the render function) ──────────────────────────────────────

/// Paint `node` into `buf` within `area`. Returns the area that was
/// used (useful for parents that want to know how much space a child
/// consumed). Returns `area` unchanged if the node is empty.
pub fn paint(node: &Node, area: Rect, buf: &mut Buffer, theme: &ThemeColors) {
    if area.width == 0 || area.height == 0 { return; }
    paint_node(node, area, buf, theme);
}

fn paint_node(node: &Node, area: Rect, buf: &mut Buffer, theme: &ThemeColors) {
    paint_node_variant(node, area, buf, theme);
}

fn paint_node_variant(node: &Node, area: Rect, buf: &mut Buffer, theme: &ThemeColors) {
    paint_node_simple(node, area, buf, theme);
}

fn paint_node_simple(node: &Node, area: Rect, buf: &mut Buffer, theme: &ThemeColors) {
    if let Node::T(t) = node { paint_text(t, area, buf, theme); return; }
    if let Node::Blank { bg } = node { paint_blank(area, buf, bg.resolve(theme)); return; }
    if let Node::Fill = node { return; }
    if let Node::RightGap { .. } = node { return; }
    paint_node_container(node, area, buf, theme);
}

fn paint_node_container(node: &Node, area: Rect, buf: &mut Buffer, theme: &ThemeColors) {
    if let Node::Row(r) = node { paint_row(r, area, buf, theme); return; }
    if let Node::Col(c) = node { paint_col(c, area, buf, theme); return; }
    if let Node::Border(b) = node { paint_border(b, area, buf, theme); return; }
    if let Node::Pad(p) = node { paint_pad(p, area, buf, theme); return; }
    if let Node::HFillText { text, gap } = node { paint_hfill_text(text, *gap, area, buf, theme); return; }
    paint_node_conditional(node, area, buf, theme);
}

fn paint_node_conditional(node: &Node, area: Rect, buf: &mut Buffer, theme: &ThemeColors) {
    if let Node::If { cond: true, then } = node { paint(then, area, buf, theme); return; }
    if let Node::If { cond: false, .. } = node { paint_blank(area, buf, StyleRef::BgBase.resolve(theme)); return; }
    if let Node::List(ns) = node { paint_list(ns, area, buf, theme); }
}

fn paint_hfill_text(text: &Text, gap: u16, area: Rect, buf: &mut Buffer, theme: &ThemeColors) {
    let text_w = text.width() as u16;
    let total = text_w + gap;
    if total <= area.width {
        let fill_w = area.width - total;
        if fill_w > 0 {
            let fill_area = Rect { x: area.x, y: area.y, width: fill_w, height: area.height };
            paint_blank(fill_area, buf, StyleRef::BgBase.resolve(theme));
        }
        let text_area = Rect { x: area.x + fill_w + gap, y: area.y, width: text_w, height: area.height };
        paint_text(text, text_area, buf, theme);
    } else {
        paint_text(text, area, buf, theme);
    }
}

fn paint_list(ns: &[Node], area: Rect, buf: &mut Buffer, theme: &ThemeColors) {
    let r = Row { children: ns.to_vec(), gap: 0, bg: StyleRef::BgBase, fill_trailing: false };
    paint_row(&r, area, buf, theme);
}

fn paint_text(t: &Text, area: Rect, buf: &mut Buffer, theme: &ThemeColors) {
    if area.width == 0 || area.height == 0 { return; }
    let style = t.style.resolve(theme);
    if t.wrap {
        paint_text_wrapped(t, area, style, buf);
    } else {
        paint_text_single(t, area, style, buf);
    }
}

fn paint_text_wrapped(t: &Text, area: Rect, style: Style, buf: &mut Buffer) {
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
}

fn paint_text_single(t: &Text, area: Rect, style: Style, buf: &mut Buffer) {
    let text_w = if t.right_inset > 0 {
        t.content.chars().count().min(area.width.saturating_sub(t.right_inset) as usize)
    } else {
        t.content.chars().count().min(area.width as usize)
    };
    let x = area.x;
    let clipped: String = t.content.chars().take(text_w).collect();
    for (i, c) in clipped.chars().enumerate() {
        if let Some(cell) = buf.cell_mut((x + i as u16, area.y)) {
            cell.set_char(c).set_style(style);
        }
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
    paint_blank(area, buf, r.bg.resolve(theme));
    if area.width == 0 || area.height == 0 { return; }

    let (fixed_width, n_fills) = measure_row_children(r, area.width);
    let gaps = r.children.len().saturating_sub(1) as u32 * r.gap as u32;
    let free = (area.width as u32).saturating_sub(fixed_width + gaps);
    let fill_w = if n_fills > 0 { free / n_fills as u32 } else { 0 };

    let (last_x_end, x) = paint_row_children(r, area, fill_w, buf, theme);

    if r.fill_trailing && last_x_end < area.x.saturating_add(area.width) {
        let rem = Rect {
            x: last_x_end,
            y: area.y,
            width: area.x.saturating_add(area.width).saturating_sub(last_x_end),
            height: area.height,
        };
        paint_blank(rem, buf, r.bg.resolve(theme));
    }
}

fn measure_row_children(r: &Row, area_width: u16) -> (u32, u32) {
    let mut fixed_width: u32 = 0;
    let mut n_fills = 0u32;
    for c in &r.children {
        match c {
            Node::Fill => n_fills += 1,
            Node::T(t) => fixed_width += t.width() as u32,
            Node::RightGap { n } => fixed_width += *n as u32,
            _ => fixed_width += area_width as u32,
        }
    }
    (fixed_width, n_fills)
}

fn paint_row_children(
    r: &Row,
    area: Rect,
    fill_w: u32,
    buf: &mut Buffer,
    theme: &ThemeColors,
) -> (u16, u16) {
    let mut x = area.x;
    let mut last_x_end = area.x;

    for (i, c) in r.children.iter().enumerate() {
        if i > 0 && r.gap > 0 { x += r.gap; }
        let child_w = get_child_width(c, fill_w);
        let child_area = calc_child_area(c, area, x, child_w);
        paint(c, child_area, buf, theme);
        x = advance_x(c, x, fill_w, child_w);
        last_x_end = x;
    }
    (last_x_end, x)
}

fn get_child_width(c: &Node, fill_w: u32) -> u16 {
    match c {
        Node::Fill => fill_w as u16,
        Node::T(t) => t.width(),
        Node::RightGap { n } => *n,
        _ => 0,
    }
}

fn calc_child_area(c: &Node, area: Rect, x: u16, child_w: u16) -> Rect {
    let needs_full = matches!(c, Node::T(t) if t.right_inset > 0);
    if needs_full {
        Rect { x, y: area.y, width: area.x.saturating_add(area.width).saturating_sub(x), height: area.height }
    } else if matches!(c, Node::Fill) || matches!(c, Node::T(_)) {
        Rect { x, y: area.y, width: child_w.min(area.x.saturating_add(area.width).saturating_sub(x)), height: area.height }
    } else {
        let rem = area.x.saturating_add(area.width).saturating_sub(x);
        Rect { x, y: area.y, width: rem, height: area.height }
    }
}

fn advance_x(c: &Node, x: u16, fill_w: u32, child_w: u16) -> u16 {
    if matches!(c, Node::Fill) { x + fill_w as u16 } else { x + child_w }
}

fn paint_col(c: &Col, area: Rect, buf: &mut Buffer, theme: &ThemeColors) {
    paint_blank(area, buf, c.bg.resolve(theme));
    if area.width == 0 || area.height == 0 { return; }
    let (fixed_h, n_fills) = measure_col_children(c);
    let gaps = c.children.len().saturating_sub(1) as u32 * c.gap as u32;
    let free = (area.height as u32).saturating_sub(fixed_h + gaps);
    let fill_h = if n_fills > 0 { free / n_fills as u32 } else { 0 };
    paint_col_children(c, area, fill_h, buf, theme);
}

fn measure_col_children(c: &Col) -> (u32, u32) {
    let mut fixed_h: u32 = 0;
    let mut n_fills = 0u32;
    for child in &c.children {
        if matches!(child, Node::Fill) { n_fills += 1; } else { fixed_h += 1; }
    }
    (fixed_h, n_fills)
}

fn paint_col_children(c: &Col, area: Rect, fill_h: u32, buf: &mut Buffer, theme: &ThemeColors) {
    let mut y = area.y;
    for (i, child) in c.children.iter().enumerate() {
        if i > 0 && c.gap > 0 { y += c.gap; }
        let h = if matches!(child, Node::Fill) { fill_h as u16 } else { 1u16 };
        let remaining_height = area.y.saturating_add(area.height).saturating_sub(y);
        let ca = Rect { x: area.x, y, width: area.width, height: h.min(remaining_height) };
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

    paint_border_horizontal(area, style, buf, RL::from(Span::styled(
        make_border_line(area.width, ROUNDED.top_left, ROUNDED.top_right, ROUNDED.horizontal_top),
        style,
    )));
    paint_border_horizontal(
        Rect { x: area.x, y: area.y + area.height - 1, width: area.width, height: 1 },
        style, buf,
        RL::from(Span::styled(
            make_border_line(area.width, ROUNDED.bottom_left, ROUNDED.bottom_right, ROUNDED.horizontal_bottom),
            style,
        )),
    );
    paint_border_vertical_sides(area, style, buf);

    if let Some(t) = &b.title {
        let t_str = format!(" {} ", t.content);
        let t_style = t.style.resolve(theme);
        let t_line = RL::from(Span::styled(t_str, t_style));
        buf.set_line(area.x + 2, area.y, &t_line, (area.width - 4).min(t.content.chars().count() as u16 + 2));
    }

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

fn make_border_line(width: u16, left: &str, right: &str, middle: &str) -> String {
    let mut s = String::with_capacity(width as usize);
    s.push_str(left);
    for _ in 0..(width - 2) { s.push_str(middle); }
    s.push_str(right);
    s
}

fn paint_border_horizontal(area: Rect, _style: Style, buf: &mut Buffer, line: Line) {
    buf.set_line(area.x, area.y, &line, area.width);
}

fn paint_border_vertical_sides(area: Rect, style: Style, buf: &mut Buffer) {
    use ratatui::symbols::border::ROUNDED;
    use ratatui::text::Line as RL;
    for y in (area.y + 1)..(area.y + area.height - 1) {
        let left_line = RL::from(Span::styled(ROUNDED.vertical_left.to_string(), style));
        buf.set_line(area.x, y, &left_line, 1);
        let right_line = RL::from(Span::styled(ROUNDED.vertical_right.to_string(), style));
        buf.set_line(area.x + area.width - 1, y, &right_line, 1);
    }
}

// Tests moved to paint_tests.rs
