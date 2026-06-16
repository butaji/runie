//! Word-wrapping helpers for message rendering.
//!
//! The implementation lives in `runie_core::layout` so that core scroll math
//! and the TUI renderer share the exact same wrapping rules.

use crate::markdown::MdSpan;

pub(crate) use runie_core::layout::word_wrap;

/// Wrap a sequence of styled spans into rows, preserving styles at word-wrap
/// boundaries. This lets the TUI render the unified markdown AST directly
/// without re-parsing inline markdown for each wrapped chunk.
pub(crate) fn wrap_styled_spans(
    spans: &[MdSpan],
    first_width: u16,
    rest_width: u16,
) -> Vec<Vec<MdSpan>> {
    let rows = split_spans_by_newline(spans);
    let mut out = Vec::new();
    let mut width = first_width;
    for row in rows {
        if row.is_empty() {
            out.push(Vec::new());
        } else {
            out.extend(wrap_span_row(&row, width));
        }
        width = rest_width;
    }
    out
}

fn split_spans_by_newline(spans: &[MdSpan]) -> Vec<Vec<MdSpan>> {
    let mut rows: Vec<Vec<MdSpan>> = vec![Vec::new()];
    for span in spans {
        if span.content.contains('\n') {
            for (i, part) in span.content.split('\n').enumerate() {
                if i > 0 {
                    rows.push(Vec::new());
                }
                if !part.is_empty() {
                    rows.last_mut().unwrap().push(MdSpan {
                        content: part.to_string(),
                        style: span.style,
                    });
                }
            }
        } else {
            rows.last_mut().unwrap().push(span.clone());
        }
    }
    rows
}

fn wrap_span_row(spans: &[MdSpan], width: u16) -> Vec<Vec<MdSpan>> {
    let text: String = spans.iter().map(|s| s.content.as_str()).collect();
    if text.is_empty() {
        return vec![Vec::new()];
    }
    let wrapped = word_wrap(&text, width, width);
    let runs = style_runs(spans);
    let mut out = Vec::new();
    let mut cursor = 0;
    for line in wrapped {
        let start = text[cursor..].find(&line).unwrap_or(0) + cursor;
        let end = start + line.len();
        out.push(spans_for_range(start, end, &text, &runs));
        cursor = end;
    }
    out
}

fn style_runs(spans: &[MdSpan]) -> Vec<(usize, usize, ratatui::style::Style)> {
    let mut runs = Vec::new();
    let mut offset = 0;
    for span in spans {
        let end = offset + span.content.len();
        runs.push((offset, end, span.style));
        offset = end;
    }
    runs
}

fn spans_for_range(
    start: usize,
    end: usize,
    text: &str,
    runs: &[(usize, usize, ratatui::style::Style)],
) -> Vec<MdSpan> {
    let mut spans = Vec::new();
    for (rs, re, style) in runs {
        let s = start.max(*rs);
        let e = end.min(*re);
        if s < e {
            push_span(&mut spans, &text[s..e], *style);
        }
    }
    spans
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
    spans.push(MdSpan {
        content: text.to_string(),
        style,
    });
}
