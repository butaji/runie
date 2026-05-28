use std::collections::HashMap;
use std::fmt::Write;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Gauge, Paragraph, Widget},
};
use syntect::easy::HighlightLines;
use syntect::parsing::SyntaxSet;
use syntect::highlighting::{ThemeSet, Style as SyntectStyle};
use syntect::util::LinesWithEndings;
use once_cell::sync::Lazy;
use crate::theme::ThemeWrapper;
use crate::tui::state::AnimationState;
use super::types::{MessageItem, PlanStatus};

static SYNTAX_SET: Lazy<SyntaxSet> = Lazy::new(|| {
    SyntaxSet::load_defaults_newlines()
});

static THEME_SET: Lazy<ThemeSet> = Lazy::new(|| {
    ThemeSet::load_defaults()
});

/// Cache for wrap_text results to avoid recomputing every frame.
/// Key is (text, width) -> value is Vec<String> of wrapped lines.
#[derive(Clone)]
pub struct WrapCache {
    cache: HashMap<(String, usize), Vec<String>>,
    access_order: Vec<(String, usize)>,
    max_size: usize,
}

impl Default for WrapCache {
    fn default() -> Self {
        Self::new()
    }
}

impl WrapCache {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            access_order: Vec::new(),
            max_size: 100,
        }
    }

    /// Get wrapped text from cache or compute and store it.
    pub fn get_wrapped(&mut self, text: &str, width: usize) -> Vec<String> {
        let key = (text.to_string(), width);
        if let Some(cached) = self.cache.get(&key) {
            // Move to end (most recently used)
            if let Some(pos) = self.access_order.iter().position(|k| *k == key) {
                self.access_order.remove(pos);
                self.access_order.push(key);
            }
            return cached.clone();
        }

        // Evict if at capacity
        if self.cache.len() >= self.max_size {
            if let Some(oldest) = self.access_order.first().cloned() {
                self.cache.remove(&oldest);
                self.access_order.remove(0);
            }
        }

        let wrapped = wrap_text_preserving_newlines(text, width);
        self.cache.insert(key.clone(), wrapped.clone());
        self.access_order.push(key);
        wrapped
    }

    /// Clear the cache (call when messages change).
    pub fn clear(&mut self) {
        self.cache.clear();
        self.access_order.clear();
    }
}

/// Wrap text into lines respecting word boundaries
pub fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![String::new()];
    }
    let mut lines = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        if current.len() + word.len() + 1 > width {
            if !current.is_empty() {
                lines.push(current.clone());
                current.clear();
            }
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }

    if !current.is_empty() {
        lines.push(current);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

/// Wrap text while preserving newlines from source.
/// Pi-style: split on \n first, then wrap each line separately.
pub fn wrap_text_preserving_newlines(text: &str, width: usize) -> Vec<String> {
    let mut result = Vec::new();

    for line in text.split('\n') {
        let trimmed = line.trim_end();

        if trimmed.is_empty() {
            // Empty line = paragraph break
            result.push(String::new());
            continue;
        }

        // Wrap this line if too long
        if trimmed.len() <= width {
            result.push(trimmed.to_string());
        } else {
            result.extend(wrap_single_line(trimmed, width));
        }
    }

    result
}

/// Wrap a single line (no newlines) to width
fn wrap_single_line(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        if current.is_empty() {
            current.push_str(word);
        } else if current.len() + 1 + word.len() <= width {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(current);
            current = word.to_string();
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }

    lines
}

/// Render text that may contain markdown, while preserving line breaks.
fn render_text_content(text: &str, width: usize, base_style: Style) -> Vec<Line<'static>> {
    let mut result = Vec::new();
    let mut in_code_block = false;
    let mut code_lang = String::new();
    let mut code_lines = Vec::new();
    let mut table_rows: Vec<String> = Vec::new();

    // Split by actual newlines FIRST - never lose them
    for line in text.split('\n') {
        let trimmed = line.trim();

        // Code block handling
        if trimmed.starts_with("```") {
            // Flush any pending table before code block
            if !table_rows.is_empty() {
                for table_line in render_markdown_table(&table_rows, width) {
                    result.push(Line::raw(table_line).style(base_style));
                }
                table_rows.clear();
            }

            if in_code_block {
                // End code block - highlight and add
                let code_text = code_lines.join("\n");
                let highlighted = highlight_code_block_ratatui(&code_lang, &code_text);
                for hl_line in highlighted {
                    result.push(hl_line);
                }
                code_lines.clear();
                code_lang.clear();
                in_code_block = false;
            } else {
                code_lang = trimmed[3..].trim().to_string();
                in_code_block = true;
            }
            continue;
        }

        if in_code_block {
            code_lines.push(line.to_string());
            continue;
        }

        // Check for markdown table row
        if trimmed.starts_with('|') && trimmed.ends_with('|') && trimmed.contains("|") {
            table_rows.push(trimmed.to_string());
            continue;
        }

        // Flush pending table if we hit non-table content
        if !table_rows.is_empty() {
            for table_line in render_markdown_table(&table_rows, width) {
                result.push(Line::raw(table_line).style(base_style));
            }
            table_rows.clear();
        }

        // Empty line = paragraph break
        if trimmed.is_empty() {
            result.push(Line::raw("").style(base_style));
            continue;
        }

        // Check for horizontal rule
        if trimmed.starts_with("---") || trimmed.starts_with("***") {
            result.push(Line::raw("─".repeat(width)).style(base_style));
            continue;
        }

        // Headers
        if let Some(header_text) = trimmed.strip_prefix("# ") {
            result.push(Line::raw(header_text.to_string()).style(base_style.add_modifier(ratatui::style::Modifier::BOLD)));
            result.push(Line::raw("").style(base_style));
            continue;
        }
        if let Some(header_text) = trimmed.strip_prefix("## ") {
            result.push(Line::raw(header_text.to_string()).style(base_style.add_modifier(ratatui::style::Modifier::BOLD)));
            result.push(Line::raw("").style(base_style));
            continue;
        }

        // List items with spacing
        if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            let content = &trimmed[2..];
            result.push(Line::raw(format!("• {}", content)).style(base_style));
            result.push(Line::raw("").style(base_style)); // blank line after
            continue;
        }

        // Fix text spacing: add space between sentences that run together
        let fixed_line = fix_text_spacing(line);

        // Regular text - wrap to width
        if fixed_line.len() <= width {
            result.push(Line::raw(fixed_line).style(base_style));
        } else {
            for wrapped in wrap_text_preserving_newlines(&fixed_line, width) {
                result.push(Line::raw(wrapped).style(base_style));
            }
        }
    }

    // Flush any pending table
    if !table_rows.is_empty() {
        for table_line in render_markdown_table(&table_rows, width) {
            result.push(Line::raw(table_line).style(base_style));
        }
    }

    // Handle unclosed code block
    if in_code_block && !code_lines.is_empty() {
        let code_text = code_lines.join("\n");
        let highlighted = highlight_code_block_ratatui(&code_lang, &code_text);
        for hl_line in highlighted {
            result.push(hl_line);
        }
    }

    result
}

/// Add space between sentences when they run together and sanitize problematic characters
fn fix_text_spacing(text: &str) -> String {
    let mut result = text.to_string();

    // Replace problematic Unicode characters
    result = result.replace('ð', " ").replace('Ð', " ");

    // Space after punctuation before letter (.,!?:;) followed by uppercase or lowercase
    result = regex::Regex::new(r"([.!?;:,])([A-Za-z])")
        .unwrap()
        .replace_all(&result, "$1 $2")
        .to_string();

    // Space after closing paren/bracket before letter
    result = regex::Regex::new(r"([\)\]}])([A-Za-z])")
        .unwrap()
        .replace_all(&result, "$1 $2")
        .to_string();

    // Space between camelCase words
    result = regex::Regex::new(r"([a-z])([A-Z])")
        .unwrap()
        .replace_all(&result, "$1 $2")
        .to_string();

    // Space between number and letter
    result = regex::Regex::new(r"(\d)([A-Za-z])")
        .unwrap()
        .replace_all(&result, "$1 $2")
        .to_string();

    // Fix common missing spaces
    result = regex::Regex::new(r"\bindscripts\b")
        .unwrap()
        .replace_all(&result, "and scripts")
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

    let parsed_rows: Vec<Vec<String>> = rows.iter()
        .map(|row| {
            row.split('|')
                .skip(1)
                .filter(|s| !s.is_empty())
                .map(|s| s.trim().to_string())
                .collect()
        })
        .collect();

    if parsed_rows.is_empty() {
        return rows.to_vec();
    }

    let col_count = parsed_rows.iter().map(|r| r.len()).max().unwrap_or(0);
    if col_count == 0 {
        return rows.to_vec();
    }

    let mut col_widths = vec![0usize; col_count];
    for row in &parsed_rows {
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

    let mut result = Vec::new();

    let v_join = |sep: &str| -> String {
        col_widths.iter()
            .map(|w| "─".repeat(*w + 2))
            .collect::<Vec<_>>()
            .join(sep)
    };

    result.push(format!("┌{}┐", v_join("┬")));

    for (i, row) in parsed_rows.iter().enumerate() {
        let cells: Vec<String> = row.iter().enumerate()
            .map(|(j, cell)| {
                let w = col_widths.get(j).copied().unwrap_or(10);
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

/// Convert syntect style to ratatui style
fn syntect_style_to_ratatui(style: SyntectStyle) -> Style {
    let fg = ratatui::style::Color::Rgb(style.foreground.r, style.foreground.g, style.foreground.b);
    let bg = ratatui::style::Color::Rgb(style.background.r, style.background.g, style.background.b);
    let mut ratatui_style = Style::default().fg(fg).bg(bg);
    if style.font_style.contains(syntect::highlighting::FontStyle::BOLD) {
        ratatui_style = ratatui_style.add_modifier(ratatui::style::Modifier::BOLD);
    }
    if style.font_style.contains(syntect::highlighting::FontStyle::ITALIC) {
        ratatui_style = ratatui_style.add_modifier(ratatui::style::Modifier::ITALIC);
    }
    if style.font_style.contains(syntect::highlighting::FontStyle::UNDERLINE) {
        ratatui_style = ratatui_style.add_modifier(ratatui::style::Modifier::UNDERLINED);
    }
    ratatui_style
}

/// Highlight code block and return ratatui Lines with proper styling
fn highlight_code_block_ratatui(lang: &str, code: &str) -> Vec<Line<'static>> {
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

pub fn should_show_cursor(
    animation: &AnimationState,
    agent_running: bool,
    absolute_idx: usize,
    total_messages: usize,
    msg: &MessageItem,
) -> bool {
    animation.streaming_cursor_visible
        && agent_running
        && absolute_idx == total_messages.saturating_sub(1)
        && matches!(msg, MessageItem::Assistant { .. })
}

/// Find the index of the most recent message that needs a spinner.
pub fn find_most_recent_spinner_index(messages: &[MessageItem]) -> Option<usize> {
    messages.iter().enumerate().rev().find(|(_, msg)| {
        matches!(msg,
            MessageItem::Thought { .. }
            | MessageItem::ToolRunning { .. }
            | MessageItem::PlanStep { status: PlanStatus::Active, .. }
            | MessageItem::Rewind { .. }
        )
    }).map(|(i, _)| i)
}

pub fn get_msg_type(msg: &MessageItem) -> &'static str {
    match msg {
        MessageItem::User { .. } => "user",
        MessageItem::Assistant { .. } => "assistant",
        MessageItem::Thought { .. } => "thought",
        MessageItem::Separator { .. } => "separator",
        MessageItem::ToolCall { .. } => "tool",
        MessageItem::Edit { .. } => "edit",
        MessageItem::System { .. } => "system",
        MessageItem::Error { .. } => "error", // P2-1: Structured error type
        MessageItem::ToolRunning { .. } => "tool_running",
        MessageItem::ToolComplete { .. } => "tool_complete",
        MessageItem::PlanStep { .. } => "plan_step",
        MessageItem::Interrupt { .. } => "interrupt",
        MessageItem::Rewind { .. } => "rewind",
    }
}

pub fn render_single_msg(
    msg: &MessageItem,
    area: Rect,
    row: u16,
    margin_x: u16,
    text_x: u16,
    max_rows: u16,
    buf: &mut Buffer,
    theme: &ThemeWrapper,
    accent_primary: ratatui::style::Color,
    text_secondary: ratatui::style::Color,
    text_muted: ratatui::style::Color,
    text_dim: ratatui::style::Color,
    success: ratatui::style::Color,
    error: ratatui::style::Color,
    code_path: ratatui::style::Color,
    spinner: char,
    cursor_visible: bool,
    show_spinner: bool,
    rewind_spinner: char,
    animation: &AnimationState,
    wrap_cache: &mut WrapCache,
    agent_running: bool,
) -> u16 {
    match msg {
        MessageItem::User { text, .. } => {
            render_user_msg(text, area, row, margin_x, text_x, max_rows, buf, theme, accent_primary, wrap_cache)
        }
        MessageItem::Assistant { text, .. } => {
            render_assistant_msg(text, area, row, margin_x, text_x, max_rows, buf, text_secondary, text_muted, cursor_visible, wrap_cache, agent_running, spinner)
        }
        MessageItem::Thought { duration_secs } => {
            render_thought_msg(*duration_secs, area, row, margin_x, text_x, buf, text_muted, spinner, show_spinner)
        }
        MessageItem::Separator { elapsed_secs, tool_calls, tokens_used } => {
            render_separator(*elapsed_secs, *tool_calls, *tokens_used, area, row, margin_x, buf, text_dim)
        }
        MessageItem::ToolCall { name, args, result, is_error } => {
            render_tool_call_msg(name, args, result.as_deref(), *is_error, area, row, margin_x, text_x, max_rows, buf, text_secondary, text_muted, success, error)
        }
        MessageItem::Edit { filename, diff: _ } => {
            render_edit_msg(filename, area, row, margin_x, text_x, buf, text_secondary, code_path)
        }
        MessageItem::System { text } => {
            render_system_msg(text, area, row, margin_x, text_x, buf, text_muted, error)
        }
        // P2-1: Render structured error messages with [!] icon and recovery hint
        MessageItem::Error { message, recoverable } => {
            render_error_msg(message, *recoverable, area, row, margin_x, text_x, buf, error, text_muted)
        }
        MessageItem::ToolRunning { name, args, duration_ms } => {
            render_tool_running_msg(name, args, *duration_ms, area, row, margin_x, text_x, buf, text_secondary, spinner, show_spinner)
        }
        MessageItem::ToolComplete { name, result, lines } => {
            render_tool_complete_msg(name, result, lines.as_ref(), area, row, margin_x, text_x, buf, success, text_muted)
        }
        MessageItem::PlanStep { step, text, status } => {
            render_plan_step_msg(*step, text, status, area, row, margin_x, text_x, buf, text_dim, text_secondary, spinner, show_spinner)
        }
        MessageItem::Interrupt => {
            render_interrupt_msg(area, row, margin_x, text_x, buf, error, text_dim, animation)
        }
        MessageItem::Rewind { steps } => {
            render_rewind_msg(*steps, area, row, margin_x, text_x, buf, text_muted, rewind_spinner, show_spinner)
        }
    }
}

fn render_user_msg(
    text: &str,
    area: Rect,
    row: u16,
    margin_x: u16,
    text_x: u16,
    max_rows: u16,
    buf: &mut Buffer,
    theme: &ThemeWrapper,
    accent_primary: ratatui::style::Color,
    wrap_cache: &mut WrapCache,
) -> u16 {
    let text_primary: ratatui::style::Color = theme.color("text.primary").into();
    let bg_panel: ratatui::style::Color = theme.color("bg.panel").into();

    let wrapped = wrap_cache.get_wrapped(text, (area.width as usize).saturating_sub(8));
    let msg_height = wrapped.len() as u16;
    let total_height = msg_height + 2;

    draw_user_panel_bg(area, row, margin_x, total_height, buf, bg_panel);
    if let Some(cell) = buf.cell_mut((margin_x, area.y + row + 1)) {
        cell.set_char('❯');
        cell.set_style(Style::default().fg(accent_primary).bg(bg_panel));
    }
    draw_user_text_lines(&wrapped, row, text_x, max_rows, area, buf, text_primary, bg_panel);

    total_height
}

fn draw_user_panel_bg(area: Rect, row: u16, margin_x: u16, total_height: u16, buf: &mut Buffer, bg_panel: ratatui::style::Color) {
    let panel_start_y = area.y + row;
    let panel_start_x = margin_x - 1;
    let panel_width = area.width - 2;
    for r in 0..total_height {
        if panel_start_y + r >= area.y + area.height { break; }
        for x in 0..panel_width {
            if panel_start_x + x < area.x + area.width {
                if let Some(cell) = buf.cell_mut((panel_start_x + x, panel_start_y + r)) {
                    cell.set_style(Style::default().bg(bg_panel));
                }
            }
        }
    }
}

fn draw_user_text_lines(wrapped: &[String], row: u16, text_x: u16, max_rows: u16, area: Rect, buf: &mut Buffer, text_primary: ratatui::style::Color, bg_panel: ratatui::style::Color) {
    for (i, line_text) in wrapped.iter().enumerate() {
        if row + 1 + i as u16 >= max_rows { break; }
        let line = Line::raw(line_text).style(Style::default().fg(text_primary).bg(bg_panel));
        buf.set_line(text_x, area.y + row + 1 + i as u16, &line, area.width - 6);
    }
}

/// Extracts <think>...</think> think blocks from text and returns (main_text, think_blocks).
/// DeepSeek models use these for internal reasoning.
pub fn extract_think_blocks(text: &str) -> (String, Vec<String>) {
    let mut main_text = String::with_capacity(text.len());
    let mut think_blocks = Vec::new();
    let mut last_end = 0;

    let bytes = text.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        // Check for <think>
        if bytes[i..].starts_with(b"<think>") {
            let start = i;
            // Find </think> after <think>
            let mut j = i + 7; // skip <think>
            let mut found = false;
            while j < bytes.len() {
                if bytes[j..].starts_with(b"</think>") {
                    // Found end
                    let block_start = i + 7; // after <think>
                    let block_end = j;
                    i = j + 8; // after </think>
                    found = true;
                    // Append text before this block to main_text
                    main_text.push_str(&text[last_end..start]);
                    // Extract the think block content (without tags)
                    let think_content = text[block_start..block_end].trim();
                    if !think_content.is_empty() {
                        think_blocks.push(think_content.to_string());
                    }
                    last_end = i;
                    break;
                }
                j += 1;
            }
            if !found {
                // No closing tag, keep rest as-is
                break;
            }
        } else {
            i += 1;
        }
    }

    // Append remaining text after last processed block
    if last_end < text.len() {
        main_text.push_str(&text[last_end..]);
    }

    (main_text, think_blocks)
}

/// Strips <think>...</think> think blocks from text (DeepSeek models use these).
pub fn strip_think_tags(text: &str) -> String {
    extract_think_blocks(text).0
}


/// Render a single think block as a box with border
fn render_think_block_box(think_content: &str, area: Rect, row: u16, margin_x: u16, text_muted: ratatui::style::Color, wrap_cache: &mut WrapCache, buf: &mut Buffer) -> u16 {
    let inner_width = (area.width - margin_x + area.x - 6) as usize;
    let title = " Thinking ";
    let border_width = inner_width + 4;

    // Title line: ┌─ Thinking ─────────┐
    let title_line = format!("┌{}{}┐", title, "─".repeat(border_width.saturating_sub(title.len() + 2)));
    if row >= area.height { return 0; }
    let line = Line::raw(title_line).style(Style::default().fg(text_muted));
    buf.set_line(margin_x, area.y + row, &line, area.width - margin_x + area.x - 2);
    let mut rendered = 1u16;

    // Content lines
    let wrapped = wrap_cache.get_wrapped(think_content, inner_width);
    for line_text in wrapped {
        let line_y = row + rendered;
        if line_y >= area.height { break; }
        let content_line = format!("│  {} │", line_text);
        let line = Line::raw(content_line).style(Style::default().fg(text_muted));
        buf.set_line(margin_x, area.y + line_y, &line, area.width - margin_x + area.x - 2);
        rendered += 1;
    }

    // Bottom border: └────────────────────┘
    if row + rendered < area.height {
        let bottom_line = format!("└{}┘", "─".repeat(border_width));
        let line = Line::raw(bottom_line).style(Style::default().fg(text_muted));
        buf.set_line(margin_x, area.y + row + rendered, &line, area.width - margin_x + area.x - 2);
        rendered += 1;
    }

    rendered
}

fn render_assistant_msg(text: &str, area: Rect, row: u16, margin_x: u16, _text_x: u16, max_rows: u16, buf: &mut Buffer, text_secondary: ratatui::style::Color, text_muted: ratatui::style::Color, cursor_visible: bool, wrap_cache: &mut WrapCache, agent_running: bool, spinner: char) -> u16 {
    let (stripped, think_blocks) = extract_think_blocks(text);

    // If both stripped and think_blocks are empty, show placeholder
    if stripped.trim().is_empty() && think_blocks.is_empty() {
        let content = if agent_running {
            format!("{} Thinking...", spinner)
        } else {
            "·".to_string()
        };
        let para = Paragraph::new(Line::raw(content).style(Style::default().fg(text_muted)))
            .style(Style::default().fg(text_muted));
        let para_area = Rect::new(margin_x, area.y + row, area.width - margin_x + area.x - 2, 1);
        para.render(para_area, buf);
        return 1;
    }

    let width = (area.width - margin_x + area.x - 2) as usize;
    let mut rendered = 0u16;

    // Render think blocks first
    for think in &think_blocks {
        if row + rendered >= max_rows { break; }
        let block_rows = render_think_block_box(think, area, row + rendered, margin_x, text_muted, wrap_cache, buf);
        rendered += block_rows;
        // Add a blank line after think block
        if row + rendered < max_rows {
            let line = Line::raw("").style(Style::default().fg(text_secondary));
            buf.set_line(margin_x, area.y + row + rendered, &line, area.width - margin_x + area.x - 2);
            rendered += 1;
        }
    }

    // If no main text content, we're done
    if stripped.trim().is_empty() {
        return rendered;
    }

    // Render main text content
    let base_style = Style::default().fg(text_secondary);
    let markdown_lines = render_text_content(&stripped, width, base_style);

    for (i, line) in markdown_lines.iter().enumerate() {
        let line_y = row + rendered + i as u16;
        if line_y >= max_rows { break; }
        buf.set_line(margin_x, area.y + line_y, line, area.width - margin_x + area.x - 2);
    }
    let text_rows = markdown_lines.len() as u16;
    rendered += text_rows;

    if cursor_visible && rendered > 0 {
        let cursor_y = area.y + row + rendered - 1;
        let last_line_text = markdown_lines.last().map(|l| l.to_string()).unwrap_or_default();
        let cursor_x = margin_x + (last_line_text.len() as u16).min(area.width - margin_x + area.x - 3);
        if cursor_x < area.x + area.width - 1 {
            if let Some(cell) = buf.cell_mut((cursor_x, cursor_y)) {
                cell.set_char('▊');
                cell.set_style(Style::default().fg(text_secondary));
            }
        }
    }
    rendered
}

fn render_thought_msg(duration_secs: f32, area: Rect, row: u16, margin_x: u16, text_x: u16, buf: &mut Buffer, text_muted: ratatui::style::Color, spinner: char, show_spinner: bool) -> u16 {
    if let Some(cell) = buf.cell_mut((margin_x, area.y + row)) {
        cell.set_char('◆');
        cell.set_style(Style::default().fg(text_muted));
    }
    let mut thought_text = String::with_capacity(32);
    write!(thought_text, "Thought for {:.1}s", duration_secs).ok();
    if show_spinner {
        write!(thought_text, " {}", spinner).ok();
    }
    let para = Paragraph::new(Line::raw(thought_text).style(Style::default().fg(text_muted)))
        .style(Style::default().fg(text_muted));
    let para_area = Rect::new(text_x, area.y + row, area.width - text_x + area.x - 4, 1);
    para.render(para_area, buf);
    1
}

fn render_separator(elapsed_secs: u64, tool_calls: usize, tokens_used: Option<usize>, area: Rect, row: u16, margin_x: u16, buf: &mut Buffer, text_dim: ratatui::style::Color) -> u16 {
    let mut parts = Vec::new();

    // Format elapsed
    let elapsed_str = if elapsed_secs < 60 {
        format!("{}s", elapsed_secs)
    } else if elapsed_secs < 3600 {
        format!("{}m {:02}s", elapsed_secs / 60, elapsed_secs % 60)
    } else {
        format!("{}h {:02}m", elapsed_secs / 3600, (elapsed_secs % 3600) / 60)
    };

    parts.push(format!("Worked for {}", elapsed_str));

    if tool_calls > 0 {
        parts.push(format!("{} tool call{}", tool_calls, if tool_calls == 1 { "" } else { "s" }));
    }

    if let Some(tokens) = tokens_used {
        parts.push(format!("{} tokens", format_token_count(tokens)));
    }

    let label = parts.join(" • ");
    let total_len = label.len() + 2; // "─ " + " ─"

    let content_width = (area.width - margin_x * 2) as usize;
    if total_len >= content_width {
        let line = Line::raw(label).style(Style::default().fg(text_dim));
        buf.set_line(margin_x, area.y + row, &line, area.width);
    } else {
        let padding = content_width - total_len;
        let left_pad = padding / 2;
        let right_pad = padding - left_pad;
        let line_text = format!("{}{} {}{}",
            "─".repeat(left_pad),
            "─",
            label,
            "─".repeat(right_pad)
        );
        let line = Line::raw(line_text).style(Style::default().fg(text_dim));
        buf.set_line(margin_x, area.y + row, &line, area.width);
    }
    1
}

fn format_token_count(tokens: usize) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}K", tokens as f64 / 1_000.0)
    } else {
        tokens.to_string()
    }
}

fn render_system_msg(text: &str, area: Rect, row: u16, margin_x: u16, text_x: u16, buf: &mut Buffer, text_muted: ratatui::style::Color, error: ratatui::style::Color) -> u16 {
    let is_error = text.starts_with("Error:");
    let color = if is_error { error } else { text_muted };
    if let Some(cell) = buf.cell_mut((margin_x, area.y + row)) {
        cell.set_char('◆');
        cell.set_style(Style::default().fg(color));
    }
    let para = Paragraph::new(Line::raw(text).style(Style::default().fg(color)))
        .style(Style::default().fg(color));
    let para_area = Rect::new(text_x, area.y + row, area.width - text_x + area.x - 4, 1);
    para.render(para_area, buf);
    1
}

// P2-1: Render structured error messages with [!] icon and recovery hint
fn render_error_msg(message: &str, _recoverable: bool, area: Rect, row: u16, margin_x: u16, text_x: u16, buf: &mut Buffer, error: ratatui::style::Color, _text_muted: ratatui::style::Color) -> u16 {
    // Draw [!] icon
    if let Some(cell) = buf.cell_mut((margin_x, area.y + row)) {
        cell.set_char('!');
        cell.set_style(Style::default().fg(error).add_modifier(ratatui::style::Modifier::BOLD));
    }

    // Draw error message using Paragraph
    let para = Paragraph::new(Line::raw(message).style(Style::default().fg(error)))
        .style(Style::default().fg(error));
    let para_area = Rect::new(text_x, area.y + row, area.width - text_x + area.x - 4, 1);
    para.render(para_area, buf);

    // P0 FIX: Removed misleading "(press Enter to retry)" hint - Enter submits new message, doesn't retry
    1
}

fn render_tool_call_msg(
    name: &str,
    args: &str,
    result: Option<&str>,
    is_error: bool,
    area: Rect,
    row: u16,
    margin_x: u16,
    text_x: u16,
    max_rows: u16,
    buf: &mut Buffer,
    text_secondary: ratatui::style::Color,
    text_muted: ratatui::style::Color,
    success: ratatui::style::Color,
    error: ratatui::style::Color,
) -> u16 {
    draw_tool_header(margin_x, text_x, area, row, buf, text_muted, text_secondary, name, args);
    let mut rendered = 1;
    if let Some(result_text) = result {
        rendered += draw_tool_result(result_text, is_error, area, row + 1, text_x, max_rows, buf, text_muted, success, error);
    }
    rendered
}

/// Format tool arguments in compact form for single-line display
fn format_tool_args_compact(args: &str) -> String {
    if args.is_empty() {
        return String::new();
    }

    // Try to extract primary argument
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(args) {
        if let serde_json::Value::Object(map) = json {
            // For single-arg tools, show just the value
            if map.len() == 1 {
                if let Some((_, value)) = map.iter().next() {
                    return match value {
                        serde_json::Value::String(s) => s.clone(),
                        other => other.to_string(),
                    };
                }
            }
            // For multi-arg, show first arg or summary
            let parts: Vec<String> = map.iter()
                .take(2)
                .map(|(k, v)| format!("{}={}", k, v.to_string().trim_matches('"')))
                .collect();
            if map.len() > 2 {
                format!("{}, ...", parts.join(", "))
            } else {
                parts.join(", ")
            }
        } else {
            args.to_string()
        }
    } else {
        args.trim().to_string()
    }
}

fn draw_tool_header(margin_x: u16, text_x: u16, area: Rect, row: u16, buf: &mut Buffer, _text_muted: ratatui::style::Color, text_secondary: ratatui::style::Color, name: &str, args: &str) {
    // Codex-style: ● name · args (compact inline format)
    if let Some(cell) = buf.cell_mut((margin_x, area.y + row)) {
        cell.set_char('●');
        cell.set_style(Style::default().fg(text_secondary));
    }

    // Build compact inline format: name · args
    let compact_args = format_tool_args_compact(args);
    let header_text = if compact_args.is_empty() {
        name.to_string()
    } else {
        format!("{} · {}", name, compact_args)
    };

    let line = Line::raw(header_text).style(Style::default().fg(text_secondary));
    buf.set_line(text_x, area.y + row, &line, area.width - 4);
}

fn draw_tool_result(result_text: &str, is_error: bool, area: Rect, row: u16, text_x: u16, max_rows: u16, buf: &mut Buffer, text_muted: ratatui::style::Color, _success: ratatui::style::Color, error: ratatui::style::Color) -> u16 {
    // Codex-style tree result: filter empty lines and render with └ prefix
    let result_lines: Vec<&str> = result_text.split('\n').filter(|l| !l.is_empty()).collect();
    if result_lines.is_empty() {
        return 0;
    }

    let mut rendered = 0u16;
    let prefix = if is_error { "  └✗ " } else { "  └ " };

    for (idx, line_text) in result_lines.iter().enumerate() {
        let result_y = row + idx as u16;
        if result_y >= max_rows { break; }

        // First line gets tree prefix, subsequent lines get indent only
        if idx == 0 {
            let prefix_text = format!("{}{}", prefix, line_text);
            let line = Line::raw(prefix_text).style(Style::default().fg(if is_error { error } else { text_muted }));
            buf.set_line(text_x, area.y + result_y, &line, area.width.saturating_sub(text_x));
        } else {
            let indented_text = format!("    {}", line_text);
            let line = Line::raw(indented_text).style(Style::default().fg(text_muted));
            buf.set_line(text_x, area.y + result_y, &line, area.width.saturating_sub(text_x));
        }
        rendered += 1;
    }

    rendered
}

fn render_edit_msg(filename: &str, area: Rect, row: u16, margin_x: u16, text_x: u16, buf: &mut Buffer, text_secondary: ratatui::style::Color, code_path: ratatui::style::Color) -> u16 {
    if let Some(cell) = buf.cell_mut((margin_x, area.y + row)) {
        cell.set_char('◆');
        cell.set_style(Style::default().fg(text_secondary));
    }
    let edit_label = "Edit ";
    let filename_only = std::path::Path::new(filename).file_name().and_then(|n| n.to_str()).unwrap_or(filename);
    let edit_len = edit_label.len() as u16;
    for (i, ch) in edit_label.chars().enumerate() {
        if let Some(cell) = buf.cell_mut((text_x + i as u16, area.y + row)) {
            cell.set_char(ch);
            cell.set_style(Style::default().fg(text_secondary));
        }
    }
    for (i, ch) in filename_only.chars().enumerate() {
        let x_pos = text_x + edit_len + i as u16;
        if x_pos < area.x + area.width {
            if let Some(cell) = buf.cell_mut((x_pos, area.y + row)) {
                cell.set_char(ch);
                cell.set_style(Style::default().fg(code_path));
            }
        }
    }
    1
}

fn render_tool_running_msg(name: &str, args: &str, duration_ms: u64, area: Rect, row: u16, margin_x: u16, text_x: u16, buf: &mut Buffer, text_secondary: ratatui::style::Color, spinner: char, show_spinner: bool) -> u16 {
    if let Some(cell) = buf.cell_mut((margin_x, area.y + row)) {
        cell.set_char('●');
        cell.set_style(Style::default().fg(text_secondary));
    }
    let mut header = String::with_capacity(64);
    write!(header, "{} {}", name, args).ok();
    if show_spinner {
        write!(header, " {}", spinner).ok();
    }
    let line = Line::raw(header).style(Style::default().fg(text_secondary));
    buf.set_line(text_x, area.y + row, &line, area.width - 4);
    if duration_ms > 1000 {
        let bar_y = row + 1;
        let bar_x = text_x;
        let bar_width = 10u16;
        let ratio = duration_ms.min(10000) as f64 / 10000.0;
        let gauge_area = Rect::new(bar_x + 1, area.y + bar_y, bar_width, 1);
        Gauge::default()
            .ratio(ratio)
            .label("")
            .style(Style::default().fg(text_secondary))
            .render(gauge_area, buf);
        return 2;
    }
    1
}

fn render_tool_complete_msg(name: &str, result: &str, lines: Option<&usize>, area: Rect, row: u16, margin_x: u16, text_x: u16, buf: &mut Buffer, success: ratatui::style::Color, text_muted: ratatui::style::Color) -> u16 {
    if let Some(cell) = buf.cell_mut((margin_x, area.y + row)) {
        cell.set_char('✓');
        cell.set_style(Style::default().fg(success));
    }
    let mut text = String::with_capacity(64);
    write!(text, "{} {}", name, result).ok();
    if let Some(l) = lines {
        write!(text, " ({} lines)", l).ok();
    }
    let line = Line::raw(text).style(Style::default().fg(text_muted));
    buf.set_line(text_x, area.y + row, &line, area.width - 4);
    1
}

fn render_plan_step_msg(step: usize, text: &str, status: &PlanStatus, area: Rect, row: u16, margin_x: u16, text_x: u16, buf: &mut Buffer, text_dim: ratatui::style::Color, text_secondary: ratatui::style::Color, spinner: char, show_spinner: bool) -> u16 {
    match status {
        PlanStatus::Pending => {
            if let Some(cell) = buf.cell_mut((margin_x, area.y + row)) {
                cell.set_char('▸');
                cell.set_style(Style::default().fg(text_dim));
            }
            let mut line_text = String::with_capacity(32);
            write!(line_text, "{}. {} (pending)", step, text).ok();
            let line = Line::raw(line_text).style(Style::default().fg(text_dim));
            buf.set_line(text_x, area.y + row, &line, area.width - 4);
        }
        PlanStatus::Active => {
            if let Some(cell) = buf.cell_mut((margin_x, area.y + row)) {
                cell.set_char('│');
                cell.set_style(Style::default().fg(text_secondary));
            }
            if let Some(cell) = buf.cell_mut((margin_x + 1, area.y + row)) {
                cell.set_char('●');
                cell.set_style(Style::default().fg(text_secondary));
            }
            let pulse_char = if spinner == '⠋' || spinner == '⠹' || spinner == '⠴' || spinner == '⠧' || spinner == '⠏' { '▐' } else { ' ' };
            if pulse_char == '▐' {
                if let Some(cell) = buf.cell_mut((area.x + area.width - 1, area.y + row)) {
                    cell.set_char('▐');
                    cell.set_style(Style::default().fg(text_secondary));
                }
            }
            let mut line_text = String::with_capacity(32);
            write!(line_text, "{}. {}", step, text).ok();
            if show_spinner {
                write!(line_text, " {}", spinner).ok();
            }
            let line = Line::raw(line_text).style(Style::default().fg(text_secondary));
            buf.set_line(text_x + 1, area.y + row, &line, area.width - 5);
        }
        PlanStatus::Complete => {
            if let Some(cell) = buf.cell_mut((margin_x, area.y + row)) {
                cell.set_char('✓');
                cell.set_style(Style::default().fg(text_secondary));
            }
            let mut line_text = String::with_capacity(32);
            write!(line_text, "{}. {}", step, text).ok();
            let line = Line::raw(line_text).style(Style::default().fg(text_secondary));
            buf.set_line(text_x, area.y + row, &line, area.width - 4);
        }
    }
    1
}

fn render_interrupt_msg(area: Rect, row: u16, margin_x: u16, text_x: u16, buf: &mut Buffer, error: ratatui::style::Color, text_dim: ratatui::style::Color, animation: &AnimationState) -> u16 {
    if let Some(cell) = buf.cell_mut((margin_x, area.y + row)) {
        cell.set_char('✗');
        cell.set_style(Style::default().fg(error));
    }
    let style = if let Some(start) = animation.interrupt_fade_start {
        let elapsed = start.elapsed().as_millis() as f32;
        let fade_ms = 500.0;
        if elapsed >= fade_ms {
            Style::default().fg(text_dim)
        } else {
            Style::default().fg(error)
        }
    } else {
        Style::default().fg(error)
    };
    let line = Line::raw("Interrupted").style(style);
    buf.set_line(text_x, area.y + row, &line, area.width - 4);
    1
}

fn render_rewind_msg(steps: usize, area: Rect, row: u16, margin_x: u16, text_x: u16, buf: &mut Buffer, text_muted: ratatui::style::Color, spinner: char, show_spinner: bool) -> u16 {
    if let Some(cell) = buf.cell_mut((margin_x, area.y + row)) {
        cell.set_char('↺');
        cell.set_style(Style::default().fg(text_muted));
    }
    let mut text = String::with_capacity(32);
    write!(text, "Rewinding...").ok();
    if show_spinner {
        write!(text, " {}", spinner).ok();
    }
    write!(text, " ({} steps)", steps).ok();
    let line = Line::raw(text).style(Style::default().fg(text_muted));
    buf.set_line(text_x, area.y + row, &line, area.width - 4);
    1
}

/// Render the empty-state welcome message in the chat feed.
/// Called when `messages.is_empty()` and `!agent_running`.
pub fn render_empty_state(
    area: Rect,
    buf: &mut Buffer,
    text_muted: ratatui::style::Color,
    text_dim: ratatui::style::Color,
    text_x: u16,
) {
    let center_y = area.height / 2;
    let available_width = area.width - text_x + area.x;

    // Title line
    let title = Paragraph::new(Line::raw("runie").style(Style::default().fg(text_dim).add_modifier(ratatui::style::Modifier::BOLD)))
        .style(Style::default().fg(text_dim));
    title.render(Rect::new(text_x, center_y.saturating_sub(3), available_width, 1), buf);

    // Tagline
    let tagline = Paragraph::new(Line::raw("Your coding companion").style(Style::default().fg(text_muted)))
        .style(Style::default().fg(text_muted));
    tagline.render(Rect::new(text_x, center_y.saturating_sub(2), available_width, 1), buf);

    // Primary CTA
    let cta = Paragraph::new(Line::raw("Type a message and press Enter to start").style(Style::default().fg(text_muted)))
        .style(Style::default().fg(text_muted));
    cta.render(Rect::new(text_x, center_y, available_width, 1), buf);

    // Secondary hints
    let hint1 = Paragraph::new(Line::raw("Press ^k for commands · ^b for sidebar · ^q to quit").style(Style::default().fg(text_dim)))
        .style(Style::default().fg(text_dim));
    hint1.render(Rect::new(text_x, center_y.saturating_add(1), available_width, 1), buf);
}
