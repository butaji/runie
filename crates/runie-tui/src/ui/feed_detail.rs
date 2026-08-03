//! Fullscreen framed feed element detail view.
//!
//! Opened from the feed when the user presses Enter on any feed element in
//! vim nav mode (Grok-style). Renders the element's full content in a bordered
//! overlay with a title bar (element kind label), scrollable body, and footer
//! hint.

use ratatui::{
    layout::{Margin, Rect},
    style::{Modifier, Style},
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

    // Reserve Grok-style selection chrome above the shortcut footer.
    let selection_chrome = detail.visual_anchor.is_some() && inner.height >= 4;
    let reserved = if selection_chrome { 3 } else { 1 };
    let body_area = Rect { x: inner.x, y: inner.y, width: inner.width, height: inner.height - reserved };
    let footer_area = Rect { x: inner.x, y: inner.y + inner.height - 1, width: inner.width, height: 1 };

    render_body(f, detail, body_area);
    if selection_chrome {
        let status_area = Rect { x: inner.x, y: inner.y + inner.height - 3, width: inner.width, height: 1 };
        let divider_area = Rect { x: inner.x, y: inner.y + inner.height - 2, width: inner.width, height: 1 };
        render_selection_status(f, status_area, detail);
        f.render_widget(
            Paragraph::new("─".repeat(divider_area.width as usize)),
            divider_area,
        );
    }
    render_footer(f, footer_area, detail);
}

fn render_selection_status(f: &mut Frame, area: Rect, detail: &runie_core::model::feed_detail::FeedElementDetail) {
    let anchor = detail.visual_anchor.unwrap_or(detail.scroll);
    let (start, end) = if anchor <= detail.scroll {
        (anchor, detail.scroll)
    } else {
        (detail.scroll, anchor)
    };
    let text = format!("Selected: {} line(s)", end - start + 1);
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(text, crate::theme::style_hint()))),
        area,
    );
}

fn render_body(f: &mut Frame, detail: &runie_core::model::FeedElementDetail, area: Rect) {
    let content_width = area.width.saturating_sub(2).max(1);
    let body = if detail.search_filter && !detail.search_query.is_empty() {
        detail
            .body_text()
            .lines()
            .enumerate()
            .filter(|(index, _)| detail.search_matches.contains(index))
            .map(|(_, line)| line.to_owned())
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        detail.body_text()
    };

    // Wrap text at content width, preserving newlines
    let lines: Vec<Line<'static>> = if detail.wrap {
        wrap_text_lines(&body, content_width)
    } else {
        body.lines()
            .map(|line| Line::from(line.to_owned()))
            .collect()
    };

    let max_scroll = lines.len().saturating_sub(area.height as usize);
    let offset = detail.scroll.min(max_scroll);
    let mut visible: Vec<Line<'static>> = lines
        .into_iter()
        .skip(offset)
        .take(area.height as usize)
        .collect();

    if let Some(anchor) = detail.visual_anchor {
        let (start, end) = if anchor <= detail.scroll {
            (anchor, detail.scroll)
        } else {
            (detail.scroll, anchor)
        };
        let selection_bg = crate::theme::color_bg_panel();
        for (index, line) in visible.iter_mut().enumerate() {
            let source_line = offset + index;
            if (start..=end).contains(&source_line) {
                line.style = Style::new().bg(selection_bg);
            }
        }
    }

    if !detail.search_filter && !detail.search_query.is_empty() {
        let match_style = Style::new()
            .fg(crate::theme::color_bg())
            .bg(crate::theme::color_accent())
            .add_modifier(Modifier::BOLD);
        for line in &mut visible {
            let text = line.to_string();
            *line = highlight_matches(&text, &detail.search_query, match_style);
        }
    }

    let margin = Margin::new(1, 0);
    let padded = area.inner(margin);

    let text = if visible.is_empty() {
        Text::from(Line::from(""))
    } else {
        Text::from(visible)
    };
    f.render_widget(Paragraph::new(text), padded);
}

fn highlight_matches(text: &str, query: &str, match_style: Style) -> Line<'static> {
    if query.is_empty() {
        return Line::from(text.to_owned());
    }
    let haystack = text.to_lowercase();
    let needle = query.to_lowercase();
    let Some(start) = haystack.find(&needle) else {
        return Line::from(text.to_owned());
    };
    let end = start + needle.len();
    Line::from(vec![
        Span::raw(text[..start].to_owned()),
        Span::styled(text[start..end].to_owned(), match_style),
        Span::raw(text[end..].to_owned()),
    ])
}

fn render_footer(f: &mut Frame, area: Rect, detail: &runie_core::model::FeedElementDetail) {
    let text = if detail.search_editing {
        let mode = if detail.search_filter {
            "Filter"
        } else {
            "Search"
        };
        format!(
            "{mode}: {} │ Enter:accept │ Esc:cancel",
            detail.search_query
        )
    } else if !detail.search_query.is_empty() {
        let mode = if detail.search_filter {
            "filter"
        } else {
            "search"
        };
        format!(
            "{mode}: {} │ n/N:next/prev │ q/Esc:back",
            detail.search_query
        )
    } else {
        if detail.visual_anchor.is_some() {
            format!(
                "v:clear │ y:copy │ w:{}",
                if detail.wrap { "nowrap" } else { "wrap" }
            )
        } else {
            format!(
                "q/Esc:back │ /:search │ f:filter │ v:select │ w:{} │ ↑/↓:scroll",
                if detail.wrap { "wrap" } else { "nowrap" }
            )
        }
    };
    let spans = vec![Span::styled(text, crate::theme::style_hint())];
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
            search_query: String::new(),
            search_editing: false,
            search_filter: false,
            search_matches: Vec::new(),
            search_current: 0,
            visual_anchor: None,
            wrap: true,
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
            search_query: String::new(),
            search_editing: false,
            search_filter: false,
            search_matches: Vec::new(),
            search_current: 0,
            visual_anchor: None,
            wrap: true,
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
    fn renders_visual_selection_status_and_divider() {
        let detail = FeedElementDetail {
            element_index: 0,
            scroll: 2,
            kind: FeedElementKind::Thought { content: "one\ntwo\nthree\nfour".to_string() },
            search_query: String::new(),
            search_editing: false,
            search_filter: false,
            search_matches: Vec::new(),
            search_current: 0,
            visual_anchor: Some(0),
            wrap: false,
        };
        let snap = make_snap(Some(detail));
        let backend = ratatui::backend::TestBackend::new(80, 10);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| render_feed_detail(f, &snap, f.area()))
            .unwrap();
        let text = buffer_string(&terminal);
        assert!(
            text.contains("Selected: 3 line(s)"),
            "selection status missing: {text}"
        );
        assert!(
            text.contains("────────────────"),
            "selection divider missing: {text}"
        );
        let selected_bg = crate::theme::color_bg_panel();
        let has_selected_background = (0..terminal.backend().buffer().area().height).any(|y| {
            (0..terminal.backend().buffer().area().width).any(|x| terminal.backend().buffer()[(x, y)].bg == selected_bg)
        });
        assert!(
            has_selected_background,
            "selected line must use theme background"
        );
    }

    #[test]
    fn highlights_case_insensitive_search_match() {
        let line = highlight_matches("Prefix Needle suffix", "needle", Style::default());
        assert_eq!(line.spans.len(), 3);
        assert_eq!(line.spans[1].content, "Needle");
    }

    #[test]
    fn renders_tool_running_with_name() {
        let detail = FeedElementDetail {
            element_index: 0,
            scroll: 0,
            kind: FeedElementKind::ToolRunning { name: "Read".to_string(), args: "path/to/file".to_string() },
            search_query: String::new(),
            search_editing: false,
            search_filter: false,
            search_matches: Vec::new(),
            search_current: 0,
            visual_anchor: None,
            wrap: true,
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
