//! Scrollback widget: append-only transcript with autoscroll.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line as RatLine, Span};
use ratatui::widgets::{Paragraph, Widget, Wrap};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    User,
    Assistant,
    Reasoning,
    Tool,
    ToolResult,
    ToolOutput,
    System,
    Activity,
}

impl LineKind {
    pub fn style(self) -> Style {
        match self {
            LineKind::User => Style::default(),
            // Grok uses a vertical transcript bar for assistant/reasoning
            // blocks; body text stays primary rather than green.
            LineKind::Assistant => Style::default(),
            LineKind::Reasoning => Style::default().add_modifier(Modifier::DIM | Modifier::ITALIC),
            LineKind::Tool => Style::default(),
            LineKind::ToolResult => Style::default(),
            LineKind::ToolOutput => Style::default(),
            LineKind::System => Style::default().add_modifier(Modifier::DIM),
            LineKind::Activity => Style::default(),
        }
    }

    pub fn prefix(self) -> &'static str {
        match self {
            // Grok reserves a three-column transcript gutter before user
            // content: the cursor is at column 5 in the 80-column frame.
            LineKind::User => "   ❯ ",
            LineKind::Assistant => "   ┃ ",
            LineKind::Reasoning => "   ┃ ",
            LineKind::Tool => "   ◆ ",
            LineKind::ToolResult => "     ↳ ",
            // Structured Grok tools render terminal output directly below the
            // tool header, with a two-column indentation and no result arrow.
            LineKind::ToolOutput => "  ",
            LineKind::System => "   * ",
            LineKind::Activity => "❙  ",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Line {
    pub kind: LineKind,
    pub text: String,
}

impl Line {
    pub fn new(kind: LineKind, text: impl Into<String>) -> Self {
        Self {
            kind,
            text: text.into(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Scrollback {
    lines: Vec<Line>,
    autoscroll: bool,
    scroll_offset: usize,
    reasoning_expanded: bool,
}

impl Scrollback {
    pub fn new() -> Self {
        Self {
            lines: Vec::new(),
            autoscroll: true,
            scroll_offset: 0,
            reasoning_expanded: false,
        }
    }

    pub fn append(&mut self, line: Line) {
        self.lines.push(line);
        if self.autoscroll {
            // Hold offset so the tail is in view after the next render
            // (the actual clamp happens in `render` once we know area height).
            self.scroll_offset = self.lines.len();
        }
    }

    pub fn clear(&mut self) {
        self.lines.clear();
        self.scroll_offset = 0;
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// Borrow the lines (for tests).
    pub fn lines(&self) -> &[Line] {
        &self.lines
    }

    /// Set Grok-compatible reasoning display mode. Collapsed mode renders
    /// only `Thought`; expanded mode renders the captured reasoning body.
    pub fn set_reasoning_expanded(&mut self, expanded: bool) {
        self.reasoning_expanded = expanded;
    }

    pub fn reasoning_expanded(&self) -> bool {
        self.reasoning_expanded
    }

    /// Find the index of the first line whose `text` contains the needle.
    pub fn find_first_containing(&self, needle: &str) -> Option<usize> {
        self.lines.iter().position(|l| l.text.contains(needle))
    }

    /// Find all line indices whose `text` contains the needle.
    pub fn find_all_containing(&self, needle: &str) -> Vec<usize> {
        self.lines
            .iter()
            .enumerate()
            .filter_map(|(i, l)| {
                if l.text.contains(needle) {
                    Some(i)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Mutable reference to the last line of `kind`, if any.
    pub fn last_mut_by_kind(&mut self, kind: LineKind) -> Option<&mut Line> {
        self.lines.iter_mut().rev().find(|l| l.kind == kind)
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        // Wrap-aware: each Line is one logical row that may wrap to multiple
        // physical rows. We approximate by giving each line 1 "slot" plus
        // overflow based on text length and area width.
        let width = area.width as usize;
        let mut physical_rows: Vec<(LineKind, String)> = Vec::new();
        for line in &self.lines {
            let reasoning_collapsed = line.kind == LineKind::Reasoning && !self.reasoning_expanded;
            let source_text = if reasoning_collapsed {
                "Thought"
            } else {
                line.text.as_str()
            };
            let prefix = line.kind.prefix();
            for (part_index, part) in source_text.split('\n').enumerate() {
                let prefixed = if part.is_empty() {
                    String::new()
                } else if part_index == 0 {
                    format!("{prefix}{part}")
                } else {
                    format!("{}{}", " ".repeat(prefix.chars().count()), part)
                };
                if width == 0 || prefixed.chars().count() <= width {
                    physical_rows.push((line.kind, prefixed));
                } else {
                    // Wrap long logical lines by character count.
                    let mut chars: Vec<char> = prefixed.chars().collect();
                    while !chars.is_empty() {
                        let take = width.min(chars.len());
                        let head: String = chars.drain(..take).collect();
                        physical_rows.push((line.kind, head));
                    }
                }
            }
        }

        let total = physical_rows.len();
        let visible = area.height as usize;
        // Clamp scroll_offset so the tail is visible.
        if total > visible {
            let max_offset = total - visible;
            if self.scroll_offset > max_offset {
                self.scroll_offset = max_offset;
            }
            if self.scroll_offset == 0 && self.autoscroll {
                self.scroll_offset = max_offset;
            }
        } else {
            self.scroll_offset = 0;
        }

        let start = self.scroll_offset;
        let end = (start + visible).min(total);

        if start >= end {
            // Nothing to render. Avoid passing an empty slice to ratatui's
            // Paragraph/Line, which can panic on some versions.
            return;
        }

        for (row, (kind, text)) in physical_rows[start..end].iter().enumerate() {
            let line = styled_line(*kind, text);
            Paragraph::new(line).wrap(Wrap { trim: false }).render(
                Rect {
                    x: area.x,
                    y: area.y + row as u16,
                    width: area.width,
                    height: 1,
                },
                buf,
            );
        }
    }
}

/// Render the small CommonMark subset that is visible in Grok's normal
/// transcript: headings, bullets, inline emphasis, and inline code. Keeping
/// this at the widget boundary means replay events remain the single source
/// of truth and no test needs a terminal process.
fn styled_line(kind: LineKind, text: &str) -> RatLine<'static> {
    if kind == LineKind::User {
        let pointer = text.find('❯');
        if let Some(pointer) = pointer {
            let body_start = pointer + '❯'.len_utf8();
            return RatLine::from(vec![
                Span::styled(
                    text[..body_start].to_owned(),
                    Style::default().fg(Color::Rgb(122, 162, 247)),
                ),
                Span::styled(text[body_start..].to_owned(), kind.style()),
            ]);
        }
    }
    if kind == LineKind::Tool {
        let Some(header_start) = text.find("◆ ") else {
            return RatLine::from(text.to_owned()).style(kind.style());
        };
        let split = header_start + "◆ ".len();
        let (prefix, body) = text.split_at(split);
        let name_end = body.find(|c: char| c.is_whitespace()).unwrap_or(body.len());
        let name = &body[..name_end];
        let rest = &body[name_end..];
        return RatLine::from(vec![
            Span::styled(prefix.to_owned(), kind.style()),
            Span::styled(name.to_owned(), kind.style().add_modifier(Modifier::BOLD)),
            Span::styled(rest.to_owned(), kind.style()),
        ]);
    }
    if kind != LineKind::Assistant {
        return RatLine::from(text.to_owned()).style(kind.style());
    }
    let (prefix, body) = text.split_at(
        text.find(|c: char| !c.is_whitespace())
            .unwrap_or(text.len()),
    );
    let mut spans = vec![Span::styled(prefix.to_owned(), kind.style())];
    let Some(body) = body.strip_prefix("┃ ") else {
        return RatLine::from(text.to_owned()).style(kind.style());
    };
    spans.push(Span::styled("┃ ".to_owned(), kind.style()));
    let mut body_spans = markdown_spans(body, kind.style());
    spans.append(&mut body_spans);
    RatLine::from(spans)
}

fn markdown_spans(text: &str, base: Style) -> Vec<Span<'static>> {
    let heading = text.strip_prefix("# ").or_else(|| text.strip_prefix("## "));
    if let Some(title) = heading {
        return vec![Span::styled(
            title.to_owned(),
            base.add_modifier(Modifier::BOLD),
        )];
    }
    let (bullet, content) = text
        .strip_prefix("- ")
        .map(|rest| ("• ", rest))
        .or_else(|| text.strip_prefix("* ").map(|rest| ("• ", rest)))
        .unwrap_or(("", text));
    let mut spans = vec![Span::styled(bullet.to_owned(), base)];
    let mut rest = content;
    while let Some(start) = rest.find("**") {
        if start > 0 {
            spans.push(Span::styled(rest[..start].to_owned(), base));
        }
        let after = &rest[start + 2..];
        let Some(end) = after.find("**") else {
            spans.push(Span::styled(rest[start..].to_owned(), base));
            return spans;
        };
        spans.push(Span::styled(
            after[..end].to_owned(),
            base.add_modifier(Modifier::BOLD),
        ));
        rest = &after[end + 2..];
    }
    if !rest.is_empty() {
        spans.push(Span::styled(rest.to_owned(), base));
    }
    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_and_len() {
        let mut s = Scrollback::new();
        s.append(Line::new(LineKind::User, "hi"));
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn clear_empties() {
        let mut s = Scrollback::new();
        s.append(Line::new(LineKind::User, "hi"));
        s.clear();
        assert!(s.is_empty());
    }

    #[test]
    fn find_first_containing() {
        let mut s = Scrollback::new();
        s.append(Line::new(LineKind::Assistant, "Hello world"));
        s.append(Line::new(LineKind::Assistant, "Goodbye world"));
        assert_eq!(s.find_first_containing("world"), Some(0));
    }

    #[test]
    fn visual_styles_are_stable() {
        assert_eq!(LineKind::User.style().fg, None);
        assert!(!LineKind::User.style().add_modifier.contains(Modifier::BOLD));
        assert_eq!(LineKind::Assistant.style().fg, None);
        assert_eq!(LineKind::Tool.style().fg, None);
        assert_eq!(LineKind::ToolResult.style().fg, None);
        assert!(LineKind::System
            .style()
            .add_modifier
            .contains(Modifier::DIM));
    }

    #[test]
    fn structured_tool_header_bolds_only_the_action_name() {
        let rendered = styled_line(LineKind::Tool, "   ◆ List .");
        assert!(rendered.spans[1]
            .style
            .add_modifier
            .contains(Modifier::BOLD));
        assert!(!rendered.spans[2]
            .style
            .add_modifier
            .contains(Modifier::BOLD));
    }

    #[test]
    fn embedded_newlines_render_as_separate_rows() {
        let mut scrollback = Scrollback::new();
        scrollback.append(Line::new(LineKind::Assistant, "first\nsecond"));
        let mut buffer = Buffer::empty(Rect::new(0, 0, 30, 4));
        scrollback.render(Rect::new(0, 0, 30, 4), &mut buffer);
        assert_eq!(buffer.cell((3, 0)).expect("first row").symbol(), "┃");
        assert_eq!(buffer.cell((3, 1)).expect("second row").symbol(), " ");
        let rendered: String = (0..30)
            .flat_map(|y| (0..30).map(move |x| (x, y)))
            .filter_map(|(x, y)| buffer.cell((x, y)))
            .map(|cell| cell.symbol().to_string())
            .collect();
        assert!(rendered.contains("second"));
    }

    #[test]
    fn assistant_markdown_preserves_grok_bullets_and_bold_spans() {
        let mut scrollback = Scrollback::new();
        scrollback.append(Line::new(LineKind::Assistant, "# runie"));
        scrollback.append(Line::new(LineKind::Assistant, "- **fast** replay"));
        let mut buffer = Buffer::empty(Rect::new(0, 0, 30, 3));
        scrollback.render(Rect::new(0, 0, 30, 3), &mut buffer);
        assert_eq!(buffer.cell((3, 0)).expect("heading prefix").symbol(), "┃");
        assert!(buffer
            .cell((5, 0))
            .expect("heading")
            .modifier
            .contains(Modifier::BOLD));
        assert_eq!(buffer.cell((5, 1)).expect("bullet prefix").symbol(), "•");
        assert!(buffer
            .cell((7, 1))
            .expect("bold word")
            .modifier
            .contains(Modifier::BOLD));
    }

    #[test]
    fn grok_user_feed_cursor_is_at_column_five() {
        let mut scrollback = Scrollback::new();
        scrollback.append(Line::new(LineKind::User, "Please list files"));
        let mut buffer = Buffer::empty(Rect::new(2, 0, 76, 1));
        scrollback.render(Rect::new(2, 0, 76, 1), &mut buffer);
        assert_eq!(buffer.cell((2, 0)).expect("gutter").symbol(), " ");
        assert_eq!(buffer.cell((5, 0)).expect("Grok user cursor").symbol(), "❯");
        assert_eq!(
            buffer.cell((7, 0)).expect("first user letter").symbol(),
            "P"
        );
    }

    #[test]
    fn grok_user_pointer_uses_blue_accent_without_bold_body() {
        let rendered = styled_line(LineKind::User, "   ❯ hello");
        assert_eq!(rendered.spans[0].style.fg, Some(Color::Rgb(122, 162, 247)));
        assert!(!rendered.spans[0]
            .style
            .add_modifier
            .contains(Modifier::BOLD));
        assert!(!rendered.spans[1]
            .style
            .add_modifier
            .contains(Modifier::BOLD));
    }

    #[test]
    fn reasoning_uses_grok_dim_italic_transcript_style() {
        let style = LineKind::Reasoning.style();
        assert!(style.add_modifier.contains(Modifier::DIM));
        assert!(style.add_modifier.contains(Modifier::ITALIC));
        assert_eq!(LineKind::Reasoning.prefix(), "   ┃ ");
    }

    #[test]
    fn reasoning_fold_has_deterministic_collapsed_and_expanded_cells() {
        let mut scrollback = Scrollback::new();
        scrollback.append(Line::new(LineKind::Reasoning, "checking the request"));
        let mut collapsed = Buffer::empty(Rect::new(0, 0, 40, 1));
        scrollback.render(Rect::new(0, 0, 40, 1), &mut collapsed);
        assert_eq!(
            collapsed.cell((3, 0)).expect("collapsed gutter").symbol(),
            "┃"
        );
        assert_eq!(
            collapsed.cell((5, 0)).expect("collapsed label").symbol(),
            "T"
        );

        scrollback.set_reasoning_expanded(true);
        let mut expanded = Buffer::empty(Rect::new(0, 0, 40, 1));
        scrollback.render(Rect::new(0, 0, 40, 1), &mut expanded);
        assert_eq!(expanded.cell((5, 0)).expect("expanded body").symbol(), "c");
        assert!(expanded
            .cell((5, 0))
            .expect("expanded style")
            .modifier
            .contains(Modifier::DIM));
        assert!(expanded
            .cell((5, 0))
            .expect("expanded style")
            .modifier
            .contains(Modifier::ITALIC));
    }
}
