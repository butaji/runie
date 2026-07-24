//! Markdown rendering for agent messages.
//!
//! Uses `tui_markdown` for all markdown parsing and styling. This replaces
//! the former custom inline AST (`MdInline`) with the library's implementation.

use std::ops::Range;

use linkify::LinkFinder;
use ratatui::style::Color;
use ratatui::text::Span;
use ratatui::text::Text;

/// Parsed inline markdown span for styling.
#[derive(Debug, Clone, PartialEq)]
pub struct MdSpan {
    pub content: String,
    pub style: ratatui::style::Style,
}

/// Strip backtick markers from inline code, replacing them with the code content.
/// Also strips markdown link markers `[text](url)` → `text` and records link
/// ranges for OSC-8 hyperlink injection.
/// Returns the stripped text and a list of (byte_range_in_stripped_text, url).
fn strip_markdown_markers(text: &str) -> (String, Vec<(Range<usize>, String)>) {
    let mut result = String::with_capacity(text.len());
    let mut link_ranges: Vec<(Range<usize>, String)> = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        let b = bytes[i];

        // Inline code: `code` — strip backticks, keep content.
        if b == b'`' {
            i += 1;
            let code_start = result.len();
            while i < bytes.len() && bytes[i] != b'`' {
                result.push(bytes[i] as char);
                i += 1;
            }
            if i < bytes.len() && bytes[i] == b'`' {
                i += 1;
            }
            let _ = code_start; // suppress unused warning; kept for clarity
            continue;
        }

        // Link: [text](url) — strip markers, record range + url.
        if b == b'[' {
            i += 1;
            let link_text_start = result.len();
            // Collect link text until ].
            while i < bytes.len() && bytes[i] != b']' {
                result.push(bytes[i] as char);
                i += 1;
            }
            if i < bytes.len() && bytes[i] == b']' {
                i += 1;
            }
            let link_text_end = result.len();

            // Expect (url).
            if i < bytes.len() && bytes[i] == b'(' {
                i += 1;
                let url_start = i;
                while i < bytes.len() && bytes[i] != b')' {
                    i += 1;
                }
                if i < bytes.len() && bytes[i] == b')' {
                    let url = String::from_utf8_lossy(&bytes[url_start..i]).to_string();
                    link_ranges.push((link_text_start..link_text_end, url));
                    i += 1;
                }
            }
            continue;
        }

        result.push(b as char);
        i += 1;
    }

    (result, link_ranges)
}

/// Parse markdown text and style it using `tui_markdown`, then apply a base
/// foreground color to all spans.
///
/// Handles literal newlines by splitting the text first, styling each line
/// with `tui_markdown`, and recombining with explicit `\n` spans. This
/// preserves the same line-boundary behavior as the former `parse_inline_spans`
/// approach where `SoftBreak` spans were inserted for each `\n`.
///
/// For grok parity:
/// - Inline code gets bold styling (no background)
/// - Bold/italic markers are stripped (tui_markdown handles styling)
/// - Links show link text with OSC-8 hyperlink styling
pub fn apply_color_to_inlines(text: &str, base_color: Color) -> Vec<MdSpan> {
    // Keep inline code backticks so tui_markdown detects them and applies
    // its default background styling; fix_inline_code_spans converts bg→bold.
    // Keep markdown links so tui_markdown parses them correctly.
    let text = text;

    // Strip heading markers (e.g. "# ", "## ") from the text string before
    // tui_markdown parses it. This avoids the problem where merge_adjacent_spans
    // collapses heading-prefix spans (bold, same style as text) into one span,
    // making post-parse stripping impossible.
    let text = strip_heading_prefixes(&text);

    // Split by explicit newlines first. Each segment is styled independently with
    // tui_markdown, then rejoined with `\n` spans so downstream wrapping
    // (wrap_styled_spans::split_spans_by_newline) sees the same structure as
    // the old SoftBreak-span approach.
    let mut result = Vec::new();
    let mut is_first = true;

    for segment in text.split_inclusive('\n') {
        if !is_first {
            // Insert explicit newline span between segments
            push_span(&mut result, "\n", ratatui::style::Style::default());
        }
        if !segment.is_empty() && segment != "\n" {
            let parsed: Text<'_> = tui_markdown::from_str(segment.trim_end_matches('\n'));
            let raw = text_to_md_spans(&parsed);
            let colored = override_base_color(raw, base_color);
            // Post-process to strip background from code spans and ensure bold.
            let processed = fix_inline_code_spans(colored);
            result.extend(processed);
        }
        is_first = false;
    }

    // Detect plain URLs using linkify and apply OSC-8 hyperlinks.
    result = apply_osc8_from_plain_urls(result, &text);

    result
}

/// Detect plain URLs in `text` using linkify and apply OSC-8 hyperlink sequences
/// to the corresponding spans in `spans`.
///
/// This mirrors Grok's `url_scan.rs` approach: scan the source text for bare URLs,
/// then project those positions into the already-parsed spans.
fn apply_osc8_from_plain_urls(spans: Vec<MdSpan>, text: &str) -> Vec<MdSpan> {
    // Find all plain URLs in the stripped text.
    let mut finder = LinkFinder::new();
    finder.kinds(&[linkify::LinkKind::Url]);

    let links: Vec<(Range<usize>, String)> = finder
        .links(text)
        .map(|link| {
            let start = link.start();
            let end = link.end();
            let url = link.as_str().to_string();
            (start..end, url)
        })
        .collect();

    if links.is_empty() {
        return spans;
    }

    apply_osc8_hyperlinks(spans, links)
}
fn apply_osc8_hyperlinks(spans: Vec<MdSpan>, links: Vec<(std::ops::Range<usize>, String)>) -> Vec<MdSpan> {
    if links.is_empty() {
        return spans;
    }

    let mut result = Vec::new();

    for span in spans {
        let content = &span.content;
        let mut pos = 0;

        // Find all links that overlap with this span.
        let mut overlapping: Vec<(usize, &std::ops::Range<usize>, &String)> = Vec::new();
        for (range, url) in &links {
            // Check if range overlaps with current span content.
            if range.start < pos + content.len() && range.end > pos {
                overlapping.push((pos, range, url));
            }
        }

        if overlapping.is_empty() {
            result.push(span);
            continue;
        }

        // Sort by start position.
        overlapping.sort_by_key(|(p, r, _)| *p);

        let mut current_pos = 0;
        for (_span_start, range, url) in overlapping {
            // Adjust range to be relative to this span.
            let rel_start = range.start.saturating_sub(pos);
            let rel_end = (range.end - pos).min(content.len());

            if rel_start > current_pos {
                // Text before the link.
                result.push(MdSpan {
                    content: content[current_pos..rel_start].to_string(),
                    style: span.style,
                });
            }

            // The link text with OSC-8 hyperlink.
            // OSC-8 format: ESC ] 8 ; ; URL ESC \  text  ESC ] 8 ; ; ESC \
            let link_text = &content[rel_start..rel_end];
            let osc8_start = format!("\x1b]8;;{}\x1b\\", url);
            let osc8_end = "\x1b]8;;\x1b\\";
            let linked_content = format!("{}{}{}", osc8_start, link_text, osc8_end);

            result.push(MdSpan {
                content: linked_content,
                style: span.style.underlined(),
            });

            current_pos = rel_end;
        }

        // Remaining text after last link.
        if current_pos < content.len() {
            result.push(MdSpan {
                content: content[current_pos..].to_string(),
                style: span.style,
            });
        }
    }

    result
}

/// Post-process spans to fix inline code styling: strip backticks from content,
/// remove background, ensure bold.
fn fix_inline_code_spans(spans: Vec<MdSpan>) -> Vec<MdSpan> {
    spans
        .into_iter()
        .map(|s| {
            // Check if this span has a background color (likely inline code).
            if s.style.bg.is_some() {
                // Strip leading/trailing backticks from the content.
                let content = s.content.strip_prefix('`').unwrap_or(&s.content);
                let content = content.strip_suffix('`').unwrap_or(content);
                // Remove background by resetting to default (Color::Reset), keep bold modifier.
                let new_style = s
                    .style
                    .bg(Color::Reset)
                    .add_modifier(ratatui::style::Modifier::BOLD);
                MdSpan { content: content.to_string(), style: new_style }
            } else {
                s
            }
        })
        .collect()
}

/// Strip leading markdown heading markers ("# ", "## ", etc.) from a text string.
/// This is called BEFORE tui_markdown parses the text, so the markers are gone
/// before merge_adjacent_spans can collapse them into the content.
fn strip_heading_prefixes(text: &str) -> String {
    let trimmed = text.trim_start();
    if !trimmed.starts_with('#') {
        return text.to_string();
    }
    // Count leading '#' characters.
    let hash_count = trimmed.chars().take_while(|&c| c == '#').count();
    // After hashes, expect a space.
    let after_hashes = &trimmed[hash_count..];
    if after_hashes.starts_with(' ') {
        // Strip "# " (or "## ", etc.) — keep everything after the space.
        let remaining = after_hashes[1..].trim_start();
        // Preserve original leading whitespace if there was any.
        let leading_ws = &text[..text.len() - text.trim_start().len()];
        format!("{}{}", leading_ws, remaining)
    } else {
        text.to_string()
    }
}

/// Parse markdown text into styled spans using `tui_markdown`.
pub fn parse_inline_markdown(text: &str) -> Vec<MdSpan> {
    let text: Text<'_> = tui_markdown::from_str(text);
    text_to_md_spans(&text)
}

/// Parse markdown text with a custom base foreground color.
pub fn parse_inline_markdown_with_color(text: &str, base_color: Color) -> Vec<MdSpan> {
    apply_color_to_inlines(text, base_color)
}

/// Override the base foreground color on all spans while preserving modifiers
/// (bold, italic, etc.) and background colors.
fn override_base_color(spans: Vec<MdSpan>, base_color: Color) -> Vec<MdSpan> {
    spans
        .into_iter()
        .map(|s| {
            let modifier = s.style.add_modifier;
            let bg = s.style.bg;
            let mut new_style = ratatui::style::Style::default().fg(base_color);
            if modifier != Default::default() {
                new_style = new_style.add_modifier(modifier);
            }
            if let Some(b) = bg {
                new_style = new_style.bg(b);
            }
            MdSpan { content: s.content, style: new_style }
        })
        .collect()
}

fn push_span(spans: &mut Vec<MdSpan>, text: &str, style: ratatui::style::Style) {
    if text.is_empty() {
        return;
    }
    if let Some(last) = spans.last_mut() {
        if last.style == style {
            last.content.push_str(text);
            return;
        }
    }
    spans.push(MdSpan { content: text.to_owned(), style });
}

/// Convert tui_markdown's Text output to our MdSpan format.
fn text_to_md_spans(text: &Text<'_>) -> Vec<MdSpan> {
    let mut result = Vec::new();
    for line in text.lines.iter() {
        if !result.is_empty() {
            // Add newline between lines
            push_span(&mut result, "\n", ratatui::style::Style::default());
        }
        for span in line.spans.iter() {
            result.push(MdSpan { content: span.content.to_string(), style: span.style });
        }
    }
    // Merge adjacent spans with the same style
    merge_adjacent_spans(result)
}

/// Merge adjacent spans with the same style to reduce span count.
fn merge_adjacent_spans(spans: Vec<MdSpan>) -> Vec<MdSpan> {
    if spans.is_empty() {
        return spans;
    }
    let mut result = Vec::new();
    let mut current = spans[0].clone();

    for span in spans.iter().skip(1) {
        if current.style == span.style {
            current.content.push_str(&span.content);
        } else {
            result.push(current);
            current = span.clone();
        }
    }
    result.push(current);
    result
}

/// Convert `MdSpan` slices to ratatui `Span`s.
pub fn md_to_spans(md_spans: &[MdSpan]) -> Vec<Span<'static>> {
    md_spans
        .iter()
        .map(|s| Span::styled(s.content.clone(), s.style))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::text::Line;
    use ratatui::widgets::Paragraph;
    use ratatui::Terminal;

    #[test]
    fn styled_spans_preserved() {
        // Parse markdown text with inline styles and verify spans are extracted correctly.
        // Uses tui_markdown internally (via apply_color_to_inlines).
        let spans = apply_color_to_inlines("plain **bold** *italic* `code`", Color::White);

        let has_bold = spans
            .iter()
            .any(|s| s.content == "bold" && s.style.add_modifier(ratatui::style::Modifier::BOLD) == s.style);
        let has_italic = spans
            .iter()
            .any(|s| s.content == "italic" && s.style.add_modifier(ratatui::style::Modifier::ITALIC) == s.style);
        // tui_markdown renders inline code with a leading space; we check for that span
        // and verify it has the base color applied (tui_markdown does not give code a bg).
        let has_code = spans.iter().any(|s| s.content == " code" && s.style.fg == Some(Color::White));
        assert!(has_bold, "missing bold span");
        assert!(has_italic, "missing italic span");
        assert!(has_code, "missing code span");

        let backend = TestBackend::new(50, 3);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                let line = Line::from(md_to_spans(&spans));
                f.render_widget(Paragraph::new(line), f.area());
            })
            .unwrap();
        let buf = terminal.backend().buffer();
        let first_line = buf.content.chunks(50).next().unwrap();
        let text: String = first_line.iter().map(|c| c.symbol()).collect();
        assert!(
            text.contains("plain bold italic code"),
            "rendered text missing: {text}"
        );
    }

    #[test]
    fn parse_inline_markdown_uses_tui_markdown() {
        // Test that parse_inline_markdown produces styled spans via tui_markdown.
        let result = parse_inline_markdown("This is **bold** and *italic*.");
        assert!(!result.is_empty());
        let has_bold = result
            .iter()
            .any(|s| s.content == "bold" && s.style.add_modifier(ratatui::style::Modifier::BOLD) == s.style);
        assert!(has_bold, "bold span should have BOLD modifier");
    }

    #[test]
    fn parse_inline_markdown_with_color_applies_base_color() {
        let custom_color = Color::Red;
        let result = parse_inline_markdown_with_color("Plain text.", custom_color);
        assert!(!result.is_empty());
        // All spans should have the base color applied.
        for span in &result {
            assert!(
                span.style.fg.is_some(),
                "span should have foreground color set"
            );
        }
    }

    #[test]
    fn apply_color_to_inlines_uses_tui_markdown() {
        // Verify apply_color_to_inlines uses tui_markdown by checking that it
        // correctly parses bold/italic from raw markdown text.
        let spans = apply_color_to_inlines("**strong** and *emphasis*", Color::Green);
        let has_strong = spans
            .iter()
            .any(|s| s.content == "strong" && s.style.add_modifier(ratatui::style::Modifier::BOLD) == s.style);
        let has_emphasis = spans
            .iter()
            .any(|s| s.content == "emphasis" && s.style.add_modifier(ratatui::style::Modifier::ITALIC) == s.style);
        assert!(has_strong, "missing strong span via tui_markdown");
        assert!(has_emphasis, "missing emphasis span via tui_markdown");
    }

    #[test]
    fn apply_color_to_inlines_h1_has_teal_color() {
        // H1 headings are passed as "# Heading One" (with prefix) to tui_markdown
        // so it detects the heading level. The base teal color should be preserved.
        // The "# " prefix must be stripped from the output spans.
        let teal = Color::Rgb(115, 218, 202);
        let spans = apply_color_to_inlines("# Heading One", teal);
        assert!(!spans.is_empty(), "should have at least one span");
        // All spans should have the teal foreground color.
        for span in &spans {
            assert!(
                span.style.fg.is_some(),
                "H1 span should have foreground color set: {:?}",
                span.style
            );
            assert_eq!(
                span.style.fg, Some(teal),
                "H1 span should have teal color, got: {:?}",
                span.style.fg
            );
        }
        // The "# " marker must NOT appear in any span content.
        let combined: String = spans.iter().map(|s| s.content.as_str()).collect();
        assert!(
            !combined.contains('#'),
            "H1 marker '#' should be stripped from output, got: {:?}",
            combined
        );
    }

    #[test]
    fn md_to_spans_preserves_fg_color() {
        // Verify md_to_spans preserves the foreground color.
        let teal = Color::Rgb(115, 218, 202);
        let md_spans = apply_color_to_inlines("Heading One", teal);
        let spans = md_to_spans(&md_spans);
        assert!(!spans.is_empty(), "should have at least one span");
        for span in &spans {
            assert!(
                span.style.fg.is_some(),
                "Span from md_to_spans should have fg color: {:?}",
                span.style
            );
            assert_eq!(
                span.style.fg, Some(teal),
                "Span should have teal color through md_to_spans"
            );
        }
    }

    #[test]
    fn h1_heading_with_prefix_produces_teal_spans() {
        // This simulates what render_agent_heading_block does:
        // 1. inlines_to_text produces "Heading One"
        // 2. Prepend "# " → "# Heading One"
        // 3. apply_color_to_inlines strips the prefix and styles with teal
        use ratatui::style::Color;
        let teal = Color::Rgb(115, 218, 202);

        // Simulate the heading text as constructed in render_agent_heading_block
        let text = "# Heading One";
        let spans = apply_color_to_inlines(text, teal);

        // Verify spans exist and have teal foreground
        assert!(!spans.is_empty(), "should produce spans for H1");

        // Check that at least one span has teal foreground
        let has_teal = spans.iter().any(|s| s.style.fg == Some(teal));
        assert!(has_teal, "H1 should have teal fg color. Spans: {:?}", spans);

        // Verify no spans have Reset foreground (color was applied)
        for span in &spans {
            assert!(
                span.style.fg.is_some(),
                "span {:?} should have fg color applied, not Reset",
                span.content
            );
        }
    }
}
