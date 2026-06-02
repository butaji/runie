//! StatusBar rendering functions.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
};
use crate::theme::ThemeColors;
use crate::tui::state::TuiMode;
use crate::tui::view_models::{McpStatus, StatusBarViewModel};
use crate::style::format::STATUS_SEPARATOR;

use super::StatusItem;

pub fn render_ref(vm: &StatusBarViewModel, area: Rect, buf: &mut Buffer, colors: &ThemeColors) {
    // Hide status bar on home screen
    if matches!(vm.mode, TuiMode::HomeScreen) {
        return;
    }

    let text_tertiary = colors.text_dim;
    let text_secondary = colors.text_secondary;
    let bg = colors.bg_base;

    fill_status_background(area, buf, bg);

    let hotkeys = vm.hotkeys();
    let left_end = render_hotkey_items(area, buf, &hotkeys, text_tertiary);

    // During onboarding, only show hotkeys - hide model/token/cost info
    if !matches!(vm.mode, TuiMode::Onboarding) {
        render_ref_center(area, buf, left_end, text_secondary, vm);
    }

    // Render MCP status on the right side
    render_mcp_status(&vm.mcp_status, area, buf, colors);
}

fn fill_status_background(area: Rect, buf: &mut Buffer, bg: ratatui::style::Color) {
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_style(Style::default().bg(bg));
            }
        }
    }
}

fn render_hotkey_items(area: Rect, buf: &mut Buffer, hotkeys: &[StatusItem], text_tertiary: ratatui::style::Color) -> u16 {
    let mut x = area.x;
    let mut first = true;

    for item in hotkeys {
        if !first {
            let sep = Span::styled(STATUS_SEPARATOR, Style::default().fg(text_tertiary));
            let line = Line::from(sep);
            buf.set_line(x, area.y, &line, 5);
            x += 5;
        }
        first = false;

        // Grok-style: key:description without space separator
        let full_text = format!("{}:{}", item.key, item.description);
        let line = Line::raw(&full_text).style(Style::default().fg(text_tertiary));
        let width = full_text.len() as u16;
        buf.set_line(x, area.y, &line, width);
        x += width;
    }
    x
}

/// Renders center text only if it fits without overlapping left side
fn render_ref_center(area: Rect, buf: &mut Buffer, left_end: u16, text_secondary: ratatui::style::Color, vm: &StatusBarViewModel) {
    let Some(center_text) = vm.center_text() else { return };
    let center_width = center_text.chars().count() as u16;
    let min_padding = 2u16;

    let min_center_x = left_end + min_padding;
    let ideal_center_x = area.x + (area.width.saturating_sub(center_width)) / 2;

    let center_x = if ideal_center_x >= min_center_x {
        ideal_center_x
    } else {
        return; // Not enough space on left, skip center
    };

    if center_x + center_width <= area.x + area.width {
        let line = Line::raw(center_text).style(Style::default().fg(text_secondary));
        buf.set_line(center_x, area.y, &line, center_width);
    }
}

fn render_mcp_status(mcp: &McpStatus, area: Rect, buf: &mut Buffer, colors: &ThemeColors) {
    let text = match mcp {
        McpStatus::Connected(n) if *n > 0 => {
            format!("⚡ {} MCP servers", n)
        }
        McpStatus::Unavailable(n) if *n > 0 => {
            format!("⛔ {} MCP servers unavailable", n)
        }
        _ => return,
    };

    let line_width = text.chars().count() as u16;
    let x = area.x + area.width.saturating_sub(line_width + 1);
    let line = Line::styled(text, Style::default().fg(colors.text_dim));
    buf.set_line(x, area.y, &line, line_width);
}
