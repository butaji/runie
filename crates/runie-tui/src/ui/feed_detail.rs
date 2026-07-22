//! Fullscreen framed feed element detail view.
//!
//! Opened from the feed when the user presses Enter on any feed element in
//! vim nav mode (Grok-style). Renders the element's full content in a bordered
//! overlay with a title bar (element kind label), scrollable body, and footer
//! hint.

use ratatui::{
    layout::{Margin, Rect},
    text::{Line, Span, Text},
    widgets::Paragraph,
    Frame,
};
use runie_core::Snapshot;

/// Render the feed element detail overlay fullscreen over the feed area.
pub fn render_feed_detail(f: &mut Frame, snap: &Snapshot, _area: Rect) {
    let Some(detail) = snap.feed_element_detail.as_ref() else {
        return;
    };

    let title = format!(" {} ", detail.kind.label());

    // Use setup_panel to render the panel with consistent styling
    let inner = crate::popups::panel::setup_panel(f, _area, &title);

    if inner.height < 3 {
        return;
    }

    // Body area is the full inner rect; footer is the last row
    let body_area = Rect { x: inner.x, y: inner.y, width: inner.width, height: inner.height - 1 };
    let footer_area = Rect { x: inner.x, y: inner.y + inner.height - 1, width: inner.width, height: 1 };

    render_body(f, detail, body_area);
    render_footer(f, footer_area);
}

fn render_body(f: &mut Frame, detail: &runie_core::model::FeedElementDetail, area: Rect) {
    let content_width = area.width.saturating_sub(2).max(1);
    let body = detail.body_text();

    // Wrap text at content width, preserving newlines
    let lines: Vec<Line<'static>> = wrap_text_lines(&body, content_width);

    let max_scroll = lines.len().saturating_sub(area.height as usize);
    let offset = detail.scroll.min(max_scroll);
    let visible: Vec<Line<'static>> = lines
        .into_iter()
        .skip(offset)
        .take(area.height as usize)
        .collect();

    let margin = Margin::new(1, 0);
    let padded = area.inner(margin);

    let text = if visible.is_empty() {
        Text::from(Line::from(""))
    } else {
        Text::from(visible)
    };
    f.render_widget(Paragraph::new(text), padded);
}

fn render_footer(f: &mut Frame, area: Rect) {
    let spans = vec![
        Span::styled("q/Esc", crate::theme::style_hint_key()),
        Span::styled(":back", crate::theme::style_hint()),
        Span::styled(" │ ", crate::theme::style_hint()),
        Span::styled("↑/↓", crate::theme::style_hint_key()),
        Span::styled(":scroll", crate::theme::style_hint()),
    ];
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

/// Wrap text at the given width, preserving existing newlines.
fn wrap_text_lines(text: &str, width: u16) -> Vec<Line<'static>> {
    let width = width as usize;
    let mut result = Vec::new();

    for paragraph in text.split('\n') {
        if paragraph.is_empty() {
            result.push(Line::from(""));
            continue;
        }

        // Simple word-wrap: split by whitespace, accumulate until exceeding width
        let mut current = String::new();
        for word in paragraph.split_whitespace() {
            let with_word = if current.is_empty() {
                word.to_string()
            } else {
                format!("{} {}", current, word)
            };
            if with_word.len() > width && !current.is_empty() {
                result.push(Line::from(current.clone()));
                current = word.to_string();
            } else {
                current = with_word;
            }
        }
        if !current.is_empty() {
            result.push(Line::from(current));
        }
    }

    if result.is_empty() {
        result.push(Line::from(""));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use runie_core::model::feed_detail::{FeedElementDetail, FeedElementKind};

    fn buffer_string(terminal: &ratatui::Terminal<ratatui::backend::TestBackend>) -> String {
        let buf = terminal.backend().buffer();
        (0..buf.area().height)
            .map(|y| {
                (0..buf.area().width)
                    .map(|x| buf[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn make_snap(detail: Option<FeedElementDetail>) -> Snapshot {
        Snapshot { feed_element_detail: detail, ..Default::default() }
    }

    #[test]
    fn renders_title_for_user_input_element() {
        let detail = FeedElementDetail {
            element_index: 0,
            scroll: 0,
            kind: FeedElementKind::UserInput { content: "Hello world".to_string() },
        };
        let snap = make_snap(Some(detail));
        let backend = ratatui::backend::TestBackend::new(80, 10);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| render_feed_detail(f, &snap, f.area()))
            .unwrap();

        let text = buffer_string(&terminal);
        assert!(
            text.contains("User Input"),
            "title must show element kind: {text}"
        );
        assert!(
            text.contains("Hello world"),
            "body must render content: {text}"
        );
    }

    #[test]
    fn renders_footer_hint_bar() {
        let detail = FeedElementDetail {
            element_index: 0,
            scroll: 0,
            kind: FeedElementKind::Thought { content: "thinking...".to_string() },
        };
        let snap = make_snap(Some(detail));
        let backend = ratatui::backend::TestBackend::new(80, 8);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| render_feed_detail(f, &snap, f.area()))
            .unwrap();

        let text = buffer_string(&terminal);
        assert!(text.contains("q/Esc"), "footer must show back hint: {text}");
        assert!(
            text.contains("scroll"),
            "footer must show scroll hint: {text}"
        );
    }

    #[test]
    fn renders_tool_running_with_name() {
        let detail = FeedElementDetail {
            element_index: 0,
            scroll: 0,
            kind: FeedElementKind::ToolRunning { name: "Read".to_string(), args: "path/to/file".to_string() },
        };
        let snap = make_snap(Some(detail));
        let backend = ratatui::backend::TestBackend::new(80, 12);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| render_feed_detail(f, &snap, f.area()))
            .unwrap();

        let text = buffer_string(&terminal);
        assert!(
            text.contains("Tool Running"),
            "title must show Tool Running: {text}"
        );
        assert!(text.contains("Read"), "body must show tool name: {text}");
        assert!(
            text.contains("path/to/file"),
            "body must show tool args: {text}"
        );
    }
}
