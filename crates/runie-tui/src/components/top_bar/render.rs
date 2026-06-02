use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
};
use crate::components::top_bar::{calculate_pct, draw_gauge, format_context_window, format_token_count, TopBarViewModel, helpers::build_left_spans};
use crate::theme::ThemeColors;
use crate::glyphs::spinner_frame;

pub fn render_top_bar(
    vm: &TopBarViewModel,
    area: Rect,
    buf: &mut Buffer,
    colors: &ThemeColors,
) {
    let bg = colors.bg_base;
    let x = area.x + 1;
    let bright = colors.text_dim;
    let dim = colors.text_dim;
    let dim_style = Style::default().fg(dim).add_modifier(Modifier::DIM);

    // Build left spans with explicit bg so text cells don't show as black
    let mut left_parts = vec![Span::styled(" ", Style::default().bg(bg))];

    // Add spinner when agent is running
    if vm.agent_running {
        let spinner_char = spinner_frame(vm.braille_frame);
        left_parts.push(Span::styled(spinner_char.to_string(), Style::default().fg(bright).bg(bg)));
        left_parts.push(Span::styled(" ", Style::default().bg(bg)));
    }

    left_parts.extend(build_left_spans(vm, bright, dim, &dim_style, bg));
    if left_parts.len() > 1 {
        buf.set_line(x, area.y, &Line::from(left_parts), area.width.saturating_sub(2));
    }

    let pct = calculate_pct(vm);
    let window_str = format_context_window(vm.context_window);
    let tokens_str = format_token_count(vm.estimated_tokens);
    let text = format!("{} / {} {:.0}%", tokens_str, window_str, pct);
    let text_len = text.len() as u16;
    let gauge_width = 3u16;
    let right_x = area.x + area.width.saturating_sub(text_len + gauge_width + 1);

    if right_x > x {
        buf.set_line(
            right_x,
            area.y,
            &Line::from(vec![Span::styled(text, Style::default().fg(bright).bg(bg))]),
            text_len,
        );
        draw_gauge(
            Rect::new(right_x + text_len, area.y, gauge_width, 1),
            pct,
            buf,
            colors.text_dim,
            colors.text_secondary,
            colors.bg_panel,
        );
    }

    // Fill entire area background unconditionally — gaps between left/right text
    // must match the theme bg, never show terminal's default (black)
    for y in area.y..area.y + area.height {
        for x_cell in area.x..area.x + area.width {
            if let Some(cell) = buf.cell_mut((x_cell, y)) {
                let mut style = cell.style();
                style = style.bg(bg);
                cell.set_style(style);
            }
        }
    }
}
