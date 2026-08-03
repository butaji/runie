//! Code block rendering.

use ratatui::{
    style::Style,
    text::{Line, Span},
};

use crate::syntax::highlight_code;
use crate::theme::{code_header_label, style_code_header, style_feed_timestamp, GLYPH_AGENT, GLYPH_INDENT};
use unicode_width::UnicodeWidthStr;

pub(super) fn render_code_header(lang: &str, is_first: bool, content_width: u16, ts_str: &str) -> Line<'static> {
    let prefix = if is_first { GLYPH_AGENT } else { GLYPH_INDENT };
    let label = code_header_label(prefix, lang);
    let mut spans = vec![Span::styled(label.clone(), style_code_header())];
    if is_first && content_width > 0 {
        let text_len = UnicodeWidthStr::width(label.as_str()) as u16;
        let ts_width = UnicodeWidthStr::width(ts_str) as u16 + 1;
        let padding = content_width
            .saturating_sub(text_len)
            .saturating_sub(ts_width);
        if padding > 0 {
            spans.push(Span::raw(" ".repeat(padding as usize)));
        }
        spans.push(Span::styled(format!(" {}", ts_str), style_feed_timestamp()));
    }
    Line::from(spans).style(style_code_header())
}

pub(super) fn render_code_block_lines(content: &str, lang: &str) -> Vec<Line<'static>> {
    let highlighted = highlight_code(content, lang);
    highlighted
        .into_iter()
        .map(|tokens| {
            let mut spans = vec![Span::raw(GLYPH_INDENT.to_owned())];
            for token in tokens {
                spans.push(Span::styled(token.content, token.style));
            }
            Line::from(spans)
        })
        .collect()
}

/// Render Mermaid source as a bounded, deterministic text preview.
///
/// A terminal graphics backend is optional, so the source must remain useful
/// everywhere. Keeping this preview separate from syntax highlighting also
/// gives a future graphics backend a stable replacement point.
pub(super) fn render_mermaid_fallback(content: &str) -> Vec<Line<'static>> {
    let style = crate::theme::style_code_header();
    let mut lines = vec![Line::from(vec![
        Span::styled(crate::theme::GLYPH_INDENT, style),
        Span::styled("[Mermaid diagram]", style),
    ])];
    for source_line in content.lines().take(12) {
        let art = mermaid_flow_art(source_line);
        lines.push(Line::from(vec![
            Span::raw(crate::theme::GLYPH_INDENT),
            Span::raw("  "),
            Span::raw(art),
        ]));
    }
    if content.lines().count() > 12 {
        lines.push(Line::from(vec![
            Span::raw(crate::theme::GLYPH_INDENT),
            Span::styled(
                "  … diagram preview truncated",
                crate::theme::style_feed_timestamp(),
            ),
        ]));
    }
    lines
}

/// Render the small, deterministic subset of Mermaid flow syntax used by the
/// feed preview. Unknown syntax remains source text, while directed edges
/// become visible Unicode flow arrows instead of raw `-->` markup.
fn mermaid_flow_art(source: &str) -> String {
    if let Some((left, right)) = source.split_once("-->") {
        let left = left.trim();
        let right = right.trim();
        if !left.is_empty() && !right.is_empty() {
            return format!("{left}  ▼  {right}");
        }
    }
    source.to_owned()
}

/// Render Grok's source-preserving Mermaid affordance row. Rendering/opening
/// an image is intentionally deferred; the row is still useful in every
/// terminal and provides a stable hook for future mouse routing.
pub(super) fn render_mermaid_affordance(width: u16) -> Line<'static> {
    let mut spans = vec![Span::styled("◇ mermaid", Style::default().fg(crate::theme::color_dim()))];
    let mut used = UnicodeWidthStr::width("◇ mermaid");
    for button in ["[Open Image]", "[Copy Image Path]", "[Copy Source]"] {
        let button_width = 3 + UnicodeWidthStr::width(button);
        if used + button_width > width as usize {
            break;
        }
        spans.push(Span::styled(
            format!("   {button}"),
            crate::theme::style_hint(),
        ));
        used += button_width;
    }
    Line::from(spans)
}

pub(super) fn is_mermaid_lang(info: &str) -> bool {
    info.split_whitespace()
        .next()
        .is_some_and(|token| token.eq_ignore_ascii_case("mermaid") || token.eq_ignore_ascii_case("mmd"))
}

#[cfg(test)]
mod tests {
    use super::{is_mermaid_lang, render_mermaid_affordance, render_mermaid_fallback};

    #[test]
    fn mermaid_preview_is_bounded_and_has_text_fallback() {
        let source = (0..20)
            .map(|i| format!("node{i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let lines = render_mermaid_fallback(&source);
        assert_eq!(
            lines.len(),
            14,
            "header + 12 source rows + truncation marker"
        );
        let rendered = lines
            .iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(rendered.contains("[Mermaid diagram]"));
        assert!(rendered.contains("node0"));
        assert!(rendered.contains("diagram preview truncated"));
        assert!(!rendered.contains("node19"));
    }

    #[test]
    fn mermaid_affordance_has_grok_actions() {
        let rendered = render_mermaid_affordance(100).to_string();
        assert!(rendered.contains("◇ mermaid"));
        assert!(rendered.contains("[Open Image]"));
        assert!(rendered.contains("[Copy Image Path]"));
        assert!(rendered.contains("[Copy Source]"));
    }

    #[test]
    fn mermaid_affordance_does_not_clip_buttons_on_narrow_rows() {
        let rendered = render_mermaid_affordance(20).to_string();
        assert!(rendered.contains("◇ mermaid"));
        assert!(!rendered.contains("[Copy Image Path"));
        assert!(!rendered.contains("[Copy Source"));
    }

    #[test]
    fn mermaid_flow_edges_render_as_unicode_arrows() {
        let rendered = render_mermaid_fallback("flowchart TD\n  A --> B\n  B --> C")
            .iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(rendered.contains("A  ▼  B"));
        assert!(rendered.contains("B  ▼  C"));
        assert!(!rendered.contains("A --> B"));
    }

    #[test]
    fn mermaid_language_matches_first_info_token_only() {
        assert!(is_mermaid_lang("Mermaid theme=base"));
        assert!(is_mermaid_lang("mmd"));
        assert!(!is_mermaid_lang("mermaidx"));
        assert!(!is_mermaid_lang("rust"));
    }
}
