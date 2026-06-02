use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Widget,
};

use crate::glyphs;
use crate::theme::ThemeColors;

/// GlobalTagsViewModel holds tags to display between feed and input.
#[derive(Debug, Clone)]
pub struct GlobalTagsViewModel {
    pub left: Option<String>,
    pub right: String,
}

impl Default for GlobalTagsViewModel {
    fn default() -> Self {
        Self {
            left: None,
            right: String::new(),
        }
    }
}

impl GlobalTagsViewModel {
    /// Create idle state: empty (no turn info shown when idle)
    pub fn idle() -> Self {
        Self {
            left: None,
            right: String::new(),
        }
    }
}

fn build_turn_info(turn_duration: Option<u64>, turn_tokens: Option<usize>, turn_tool_calls: Option<usize>) -> String {
    let duration = match turn_duration {
        Some(d) => d,
        None => return String::new(),
    };
    
    let elapsed_str = format_duration(duration);
    let mut parts = vec![format!("turn: {}", elapsed_str)];
    
    if let Some(tools) = turn_tool_calls {
        if tools > 0 {
            parts.push(format!("{}tc", tools));
        }
    }
    
    if let Some(tokens) = turn_tokens {
        parts.push(format!("\u{21E3}{}", format_token_count(tokens)));
    }
    
    format!("[{}]", parts.join(", "))
}

fn format_duration(duration: u64) -> String {
    if duration < 60 {
        format!("{}s", duration)
    } else if duration < 3600 {
        format!("{}m {:02}s", duration / 60, duration % 60)
    } else {
        format!("{}h {:02}m", duration / 3600, (duration % 3600) / 60)
    }
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

impl GlobalTagsViewModel {
    /// Create running state with animated spinner.
    /// Format: "{spinner} {status} · Ctrl+Enter:interject   [turn: {time}, {tools}tc, ⇣{tokens}]"
    pub fn running(spinner: char, status: &str, time: &str, _tokens: u64, _turn_duration: Option<u64>, turn_tokens: Option<usize>, turn_tool_calls: Option<usize>) -> Self {
        let mut parts = vec![format!("turn: {}", time)];
        if let Some(tools) = turn_tool_calls {
            if tools > 0 {
                parts.push(format!("{}tc", tools));
            }
        }
        if let Some(t) = turn_tokens {
            parts.push(format!("\u{21E3}{}", format_token_count(t)));
        }
        let right = format!("[{}]", parts.join(", "));
        Self {
            left: Some(format!("{} {} · Ctrl+Enter:interject", spinner, status)),
            right,
        }
    }

    /// Create error state: "Error" (no spinner, no turn info)
    pub fn error(status: &str) -> Self {
        Self {
            left: None,
            right: format!("[{}]", status),
        }
    }
}

impl Widget for GlobalTagsViewModel {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Default colors for backward compatibility (Widget trait doesn't support colors)
        let bg_base: ratatui::style::Color = ratatui::style::Color::Reset;
        let text_dim: ratatui::style::Color = ratatui::style::Color::DarkGray;
        let accent: ratatui::style::Color = ratatui::style::Color::Blue;
        render_global_tags_impl(&self, area, buf, bg_base, text_dim, accent);
    }
}

pub fn render_global_tags(vm: &GlobalTagsViewModel, area: Rect, buf: &mut Buffer, colors: &ThemeColors) {
    render_global_tags_impl(vm, area, buf, colors.bg_base, colors.text_dim, colors.accent_primary);
}

fn render_global_tags_impl(
    vm: &GlobalTagsViewModel,
    area: Rect,
    buf: &mut Buffer,
    bg_base: ratatui::style::Color,
    text_dim: ratatui::style::Color,
    accent: ratatui::style::Color,
) {
    // Fill background at correct coordinates
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_style(Style::default().bg(bg_base));
            }
        }
    }

    let right_len = vm.right.len();

    // If nothing to show, return early
    if vm.right.is_empty() && vm.left.is_none() {
        return;
    }

    // Render left part — align to inner left edge (account for border at x=0)
    let x = area.x + 1;
    if let Some(ref left) = vm.left {
        let left_style = Style::default()
            .fg(accent);
        let span = Span::styled(left.clone(), left_style);
        buf.set_line(x, area.y, &Line::from(span), left.len() as u16);
    }

    // Render right part — right-aligned to inner right edge (account for border)
    let right_x = area.x + (area.width.saturating_sub(right_len as u16 + 1));
    let right_style = Style::default()
        .fg(text_dim);
    let span = Span::styled(vm.right.clone(), right_style);
    buf.set_line(right_x, area.y, &Line::from(span), right_len as u16);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_global_tags_renders_at_area_y_not_zero() {
        let vm = GlobalTagsViewModel::idle();
        let area = Rect::new(5, 10, 80, 1);
        let mut buf = Buffer::empty(Rect::new(0, 0, 100, 20));

        Widget::render(vm, area, &mut buf);

        let any_content_at_y0: bool = (0..80).any(|x| {
            buf.cell((x, 0)).map(|c| c.symbol() != " ").unwrap_or(false)
        });
        assert!(!any_content_at_y0, "y=0 should be empty when area.y=10");
    }

    #[test]
    fn test_global_tags_running_state_position() {
        let vm = GlobalTagsViewModel::running('⠋', "thinking", "5s", 200, None, None, None);
        let area = Rect::new(0, 15, 80, 1);
        let mut buf = Buffer::empty(Rect::new(0, 0, 80, 20));

        Widget::render(vm, area, &mut buf);

        let cell_at_y15 = buf.cell((1, 15));
        let cell_at_y0 = buf.cell((1, 0));

        assert!(
            cell_at_y15.is_some() && cell_at_y15.unwrap().symbol() == "⠋",
            "Expected spinner at y=15, found: {:?}",
            cell_at_y15.map(|c| c.symbol())
        );
        assert!(
            cell_at_y0.is_none() || cell_at_y0.unwrap().symbol() == " ",
            "y=0 should be empty, found: {:?}",
            cell_at_y0.map(|c| c.symbol())
        );
    }
}
