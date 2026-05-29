use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style as SyntectStyle, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;
use once_cell::sync::Lazy;

use super::wrap::wrap_text_preserving_newlines;

static SYNTAX_SET: Lazy<SyntaxSet> = Lazy::new(|| {
    SyntaxSet::load_defaults_newlines()
});

static THEME_SET: Lazy<ThemeSet> = Lazy::new(|| {
    ThemeSet::load_defaults()
});

// ============================================================================
// Markdown Rendering
// ============================================================================

/// Render text that may contain markdown, while preserving line breaks.
pub fn render_text_content(text: &str, width: usize, base_style: Style) -> Vec<Line<'static>> {
    let mut result = Vec::new();
    let mut in_code_block = false;
    let mut code_lang = String::new();
    let mut code_lines = Vec::new();
    let mut table_rows: Vec<String> = Vec::new();

    for line in text.split('\n') {
        process_content_line(
            line,
            &mut result,
            &mut in_code_block,
            &mut code_lang,
            &mut code_lines,
            &mut table_rows,
            width,
            base_style,
        );
    }

    // Flush pending content
    if !table_rows.is_empty() {
        flush_table_rows(&mut result, &table_rows, width, base_style);
    }
    if in_code_block && !code_lines.is_empty() {
        flush_code_block(&mut result, &code_lines, &code_lang);
    }

    result
}

fn process_content_line(
    line: &str,
    result: &mut Vec<Line<'static>>,
    in_code_block: &mut bool,
    code_lang: &mut String,
    code_lines: &mut Vec<String>,
    table_rows: &mut Vec<String>,
    width: usize,
    base_style: Style,
) {
    let trimmed = line.trim();

    // Code block handling
    if trimmed.starts_with("```") {
        flush_table_rows(result, table_rows, width, base_style);
        table_rows.clear();

        if *in_code_block {
            flush_code_block(result, code_lines, code_lang);
            code_lang.clear();
            *in_code_block = false;
        } else {
            *code_lang = trimmed[3..].trim().to_string();
            *in_code_block = true;
        }
        return;
    }

    if *in_code_block {
        code_lines.push(line.to_string());
        return;
    }

    // Table handling
    if trimmed.starts_with('|') && trimmed.ends_with('|') && trimmed.contains("|") {
        table_rows.push(trimmed.to_string());
        return;
    }

    // Flush pending table
    if !table_rows.is_empty() {
        flush_table_rows(result, table_rows, width, base_style);
        table_rows.clear();
    }

    // Process regular content line
    render_content_line(trimmed, line, result, width, base_style);
}

fn render_content_line(
    trimmed: &str,
    raw_line: &str,
    result: &mut Vec<Line<'static>>,
    width: usize,
    base_style: Style,
) {
    // Empty line = paragraph break
    if trimmed.is_empty() {
        result.push(Line::raw("").style(base_style));
        return;
    }

    // Horizontal rule
    if trimmed.starts_with("---") || trimmed.starts_with("***") {
        result.push(Line::raw("─".repeat(width)).style(base_style));
        return;
    }

    // Headers
    if let Some(header_text) = trimmed.strip_prefix("# ") {
        result.push(Line::raw(header_text.to_string()).style(base_style.add_modifier(Modifier::BOLD)));
        result.push(Line::raw("").style(base_style));
        return;
    }
    if let Some(header_text) = trimmed.strip_prefix("## ") {
        result.push(Line::raw(header_text.to_string()).style(base_style.add_modifier(Modifier::BOLD)));
        result.push(Line::raw("").style(base_style));
        return;
    }

    // List items
    if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
        let content = &trimmed[2..];
        result.push(Line::raw(format!("• {}", content)).style(base_style));
        result.push(Line::raw("").style(base_style));
        return;
    }

    // Regular text
    let fixed_line = fix_text_spacing(raw_line);
    render_plain_line(result, &fixed_line, width, base_style);
}

fn flush_table_rows(result: &mut Vec<Line<'static>>, table_rows: &[String], width: usize, base_style: Style) {
    for table_line in render_markdown_table(table_rows, width) {
        result.push(Line::raw(table_line).style(base_style));
    }
}

fn flush_code_block(result: &mut Vec<Line<'static>>, code_lines: &[String], code_lang: &str) {
    let code_text = code_lines.join("\n");
    let highlighted = highlight_code_block_ratatui(code_lang, &code_text);
    result.extend(highlighted);
}

fn render_plain_line(result: &mut Vec<Line<'static>>, line: &str, width: usize, base_style: Style) {
    if line.len() <= width {
        result.push(Line::raw(line.to_string()).style(base_style));
    } else {
        for wrapped in wrap_text_preserving_newlines(line, width) {
            result.push(Line::raw(wrapped).style(base_style));
        }
    }
}

/// Add space between sentences when they run together and sanitize problematic characters
fn fix_text_spacing(text: &str) -> String {
    let mut result = text.to_string();

    result = result.replace('ð', " ").replace('Ð', " ");

    result = regex::Regex::new(r"([.!?;:,])([A-Za-z])")
        .unwrap()
        .replace_all(&result, "$1 $2")
        .to_string();

    result = regex::Regex::new(r"([\)\]}])([A-Za-z])")
        .unwrap()
        .replace_all(&result, "$1 $2")
        .to_string();

    result = regex::Regex::new(r"([a-z])([A-Z])")
        .unwrap()
        .replace_all(&result, "$1 $2")
        .to_string();

    result = regex::Regex::new(r"(\d)([A-Za-z])")
        .unwrap()
        .replace_all(&result, "$1 $2")
        .to_string();

    result = regex::Regex::new(r"\bindscripts\b")
        .unwrap()
        .replace_all(&result, "in scripts")
        .to_string();

    result = regex::Regex::new(r"\bpantry,andscripts\b")
        .unwrap()
        .replace_all(&result, "pantry, and scripts")
        .to_string();

    result
}

/// Render a markdown table with box drawing characters
fn render_markdown_table(rows: &[String], width: usize) -> Vec<String> {
    if rows.is_empty() {
        return Vec::new();
    }

    let parsed_rows = parse_table_rows(rows);
    if parsed_rows.is_empty() {
        return rows.to_vec();
    }

    let col_widths = calculate_column_widths(&parsed_rows, width);
    render_table_with_widths(&parsed_rows, &col_widths)
}

fn parse_table_rows(rows: &[String]) -> Vec<Vec<String>> {
    rows.iter()
        .map(|row| {
            row.split('|')
                .skip(1)
                .filter(|s| !s.is_empty())
                .map(|s| s.trim().to_string())
                .collect()
        })
        .collect()
}

fn calculate_column_widths(parsed_rows: &[Vec<String>], width: usize) -> Vec<usize> {
    let col_count = parsed_rows.iter().map(|r| r.len()).max().unwrap_or(0);
    if col_count == 0 {
        return Vec::new();
    }

    let mut col_widths = vec![0usize; col_count];
    for row in parsed_rows {
        for (i, cell) in row.iter().enumerate() {
            if i < col_count {
                col_widths[i] = col_widths[i].max(cell.len());
            }
        }
    }

    let cell_width = ((width.saturating_sub(col_count + 1)) / col_count).max(10);
    for w in &mut col_widths {
        *w = (*w).min(cell_width);
    }

    col_widths
}

fn render_table_with_widths(parsed_rows: &[Vec<String>], col_widths: &[usize]) -> Vec<String> {
    let mut result = Vec::new();

    let v_join = |sep: &str| -> String {
        col_widths
            .iter()
            .map(|w| "─".repeat(*w + 2))
            .collect::<Vec<_>>()
            .join(sep)
    };

    result.push(format!("┌{}┐", v_join("┬")));

    for (i, row) in parsed_rows.iter().enumerate() {
        let cells: Vec<String> = row
            .iter()
            .enumerate()
            .map(|(j, cell)| {
                let w = *col_widths.get(j).unwrap_or(&10);
                format!(" {:w$} ", cell, w = w)
            })
            .collect();

        result.push(format!("│{}│", cells.join("│")));

        if i == 0 {
            result.push(format!("├{}┤", v_join("┼")));
        }
    }

    result.push(format!("└{}┘", v_join("┴")));

    result
}

// ============================================================================
// Syntax Highlighting
// ============================================================================

fn syntect_style_to_ratatui(style: SyntectStyle) -> Style {
    let fg = ratatui::style::Color::Rgb(style.foreground.r, style.foreground.g, style.foreground.b);
    let mut ratatui_style = Style::default().fg(fg);
    if style.font_style.contains(syntect::highlighting::FontStyle::BOLD) {
        ratatui_style = ratatui_style.add_modifier(Modifier::BOLD);
    }
    if style.font_style.contains(syntect::highlighting::FontStyle::ITALIC) {
        ratatui_style = ratatui_style.add_modifier(Modifier::ITALIC);
    }
    if style.font_style.contains(syntect::highlighting::FontStyle::UNDERLINE) {
        ratatui_style = ratatui_style.add_modifier(Modifier::UNDERLINED);
    }
    ratatui_style
}

/// Highlight code block and return ratatui Lines with proper styling
pub fn highlight_code_block_ratatui(lang: &str, code: &str) -> Vec<Line<'static>> {
    let syntax = SYNTAX_SET.find_syntax_by_token(lang)
        .or_else(|| SYNTAX_SET.find_syntax_by_extension(lang))
        .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text());

    let theme = &THEME_SET.themes["base16-ocean.dark"];
    let mut highlighter = HighlightLines::new(syntax, theme);

    let mut highlighted_lines = Vec::new();
    for line in LinesWithEndings::from(code) {
        let ranges: Vec<(SyntectStyle, &str)> = highlighter.highlight_line(line, &SYNTAX_SET).unwrap();
        let mut spans = Vec::new();
        for (style, text) in ranges {
            let ratatui_style = syntect_style_to_ratatui(style);
            spans.push(Span::styled(text.to_string(), ratatui_style));
        }
        highlighted_lines.push(Line::from(spans));
    }

    highlighted_lines
}
