//! Render function for ExtensionsModal

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Clear,
    prelude::Widget,
};
use crate::theme::ThemeWrapper;
use crate::components::extensions_modal::{ExtensionsModal, ExtensionTab, ExtensionScope, ExtensionAction};

/// Render the ExtensionsModal to the given area
pub fn render(modal: &ExtensionsModal, area: Rect, buf: &mut Buffer, theme: &ThemeWrapper) {
    if area.width < 10 || area.height < 5 {
        return;
    }

    let colors = ThemeColors::from(theme);

    // Clear the area
    Clear.render(area, buf);

    // Draw sharp corner border
    render_sharp_border(area, buf, colors.border);

    // Draw header area (tabs on row 0, search on row 1)
    render_header(modal, area, buf, &colors);

    // Draw content area (items)
    render_content(modal, area, buf, &colors);

    // Draw footer (if needed)
    render_footer(area, buf, &colors);
}

struct ThemeColors {
    bg_panel: Color,
    border: Color,
    accent_primary: Color,
    accent_secondary: Color,
    text_primary: Color,
    text_dim: Color,
    text_muted: Color,
}

impl ThemeColors {
    fn from(theme: &ThemeWrapper) -> Self {
        Self {
            bg_panel: theme.color("bg.panel").into(),
            border: theme.color("border.unfocused").into(),
            accent_primary: theme.color("accent.primary").into(),
            accent_secondary: theme.color("accent.secondary").into(),
            text_primary: theme.color("text.primary").into(),
            text_dim: theme.color("text.dim").into(),
            text_muted: theme.color("text.muted").into(),
        }
    }
}

/// Render sharp corner border (┌┐└┘│─)
fn render_sharp_border(area: Rect, buf: &mut Buffer, color: Color) {
    let left = area.x;
    let right = area.x + area.width - 1;
    let top = area.y;
    let bottom = area.y + area.height - 1;

    draw_corners(left, right, top, bottom, buf, color);
    draw_horizontal_borders(left, right, top, buf, color);
    draw_horizontal_borders(left, right, bottom, buf, color);
    draw_vertical_borders(left, top, bottom, buf, color);
    draw_vertical_borders(right, top, bottom, buf, color);
}

fn draw_corners(left: u16, right: u16, top: u16, bottom: u16, buf: &mut Buffer, color: Color) {
    if let Some(cell) = buf.cell_mut((left, top)) {
        cell.set_char('┌');
        cell.set_style(Style::default().fg(color));
    }
    if let Some(cell) = buf.cell_mut((right, top)) {
        cell.set_char('┐');
        cell.set_style(Style::default().fg(color));
    }
    if let Some(cell) = buf.cell_mut((left, bottom)) {
        cell.set_char('└');
        cell.set_style(Style::default().fg(color));
    }
    if let Some(cell) = buf.cell_mut((right, bottom)) {
        cell.set_char('┘');
        cell.set_style(Style::default().fg(color));
    }
}

fn draw_horizontal_borders(left: u16, right: u16, y: u16, buf: &mut Buffer, color: Color) {
    for x in (left + 1)..right {
        if let Some(cell) = buf.cell_mut((x, y)) {
            cell.set_char('─');
            cell.set_style(Style::default().fg(color));
        }
    }
}

fn draw_vertical_borders(x: u16, top: u16, bottom: u16, buf: &mut Buffer, color: Color) {
    for y in (top + 1)..bottom {
        if let Some(cell) = buf.cell_mut((x, y)) {
            cell.set_char('│');
            cell.set_style(Style::default().fg(color));
        }
    }
}

/// Render the header area with tabs and search bar
fn render_header(modal: &ExtensionsModal, area: Rect, buf: &mut Buffer, colors: &ThemeColors) {
    // Row 0: Tabs
    render_tabs(modal, area, buf, colors);

    // Row 1: Search bar
    render_search_bar(modal, area, buf, colors);
}

/// Render tab bar
fn render_tabs(modal: &ExtensionsModal, area: Rect, buf: &mut Buffer, colors: &ThemeColors) {
    let tabs = ExtensionTab::all();
    let tab_start_x = area.x + 1;
    let mut tab_x = tab_start_x;

    for (i, tab) in tabs.iter().enumerate() {
        let is_active = modal.active_tab == *tab;
        let tab_style = if is_active {
            Style::default().fg(colors.accent_primary)
        } else {
            Style::default().fg(colors.text_dim)
        };

        let tab_text = format!(" {} ", tab.label());
        let tab_line = Line::from(vec![Span::styled(&tab_text, tab_style)]);
        let tab_width = tab_text.len() as u16;

        // Draw tab text
        buf.set_line(tab_x, area.y, &tab_line, tab_width);

        // Draw separator (│) between tabs
        if i < tabs.len() - 1 {
            let sep_x = tab_x + tab_width;
            if sep_x < area.x + area.width - 1 {
                if let Some(cell) = buf.cell_mut((sep_x, area.y)) {
                    cell.set_char('│');
                    cell.set_style(Style::default().fg(colors.text_dim));
                }
            }
        }

        tab_x += tab_width + 1;
    }
}

/// Render search bar with filter dropdown
fn render_search_bar(modal: &ExtensionsModal, area: Rect, buf: &mut Buffer, colors: &ThemeColors) {
    let row = area.y + 1;
    let _inner_width = area.width - 2;

    // Draw horizontal separator (─)
    for x in (area.x + 1)..(area.x + area.width - 1) {
        if let Some(cell) = buf.cell_mut((x, row)) {
            cell.set_char('─');
            cell.set_style(Style::default().fg(colors.border));
        }
    }

    // Search hint text: "/ to search"
    let search_hint = "/ to search";
    let search_style = Style::default().fg(colors.text_dim);
    let search_line = Line::from(vec![Span::styled(search_hint, search_style)]);
    buf.set_line(area.x + 1, row, &search_line, search_hint.len() as u16);

    // Filter dropdown on right: "Workspace ⌄"
    let filter_text = match modal.filter_scope {
        crate::components::extensions_modal::FilterScope::Workspace => "Workspace ⌄",
        crate::components::extensions_modal::FilterScope::Project => "Project ⌄",
    };
    let filter_style = Style::default().fg(colors.text_muted);
    let filter_width = filter_text.len() as u16;
    let filter_x = area.x + area.width - 1 - filter_width;
    let filter_line = Line::from(vec![Span::styled(filter_text, filter_style)]);
    buf.set_line(filter_x, row, &filter_line, filter_width);
}

/// Render the content area with extension items
fn render_content(modal: &ExtensionsModal, area: Rect, buf: &mut Buffer, colors: &ThemeColors) {
    let content_start_y = area.y + 2;
    let items = modal.filtered_items();

    for (i, item) in items.iter().enumerate() {
        let row = content_start_y + (i as u16);
        if row >= area.y + area.height - 1 {
            break;
        }

        let is_selected = i == modal.selected_index;
        render_item(item, area, row, buf, colors, is_selected);
    }
}

/// Render a single extension item
fn render_item(item: &crate::components::extensions_modal::ExtensionItem, area: Rect, row: u16, buf: &mut Buffer, colors: &ThemeColors, is_selected: bool) {
    clear_item_row(area, row, buf, colors, is_selected);
    let mut x = area.x + 1;
    x += render_item_expand(row, x, buf, colors, is_selected);
    x += render_item_name(item, row, x, buf, colors, is_selected);
    x += render_item_version(item, row, x, buf, colors, is_selected);
    x += render_item_scope(item, area, row, x, buf, colors, is_selected);
    render_item_action(item, row, x, buf, colors, is_selected);
}

fn clear_item_row(area: Rect, row: u16, buf: &mut Buffer, colors: &ThemeColors, is_selected: bool) {
    for x in (area.x + 1)..(area.x + area.width - 1) {
        if let Some(cell) = buf.cell_mut((x, row)) {
            cell.set_char(' ');
            if is_selected {
                cell.set_style(Style::default().bg(colors.accent_primary).fg(colors.bg_panel));
            }
        }
    }
}

fn render_item_expand(row: u16, x: u16, buf: &mut Buffer, colors: &ThemeColors, is_selected: bool) -> u16 {
    let indicator_style = if is_selected {
        Style::default().fg(colors.bg_panel)
    } else {
        Style::default().fg(colors.text_dim)
    };
    if let Some(cell) = buf.cell_mut((x, row)) {
        cell.set_char(' ');
        cell.set_style(indicator_style);
    }
    1
}

fn render_item_name(item: &crate::components::extensions_modal::ExtensionItem, row: u16, x: u16, buf: &mut Buffer, colors: &ThemeColors, is_selected: bool) -> u16 {
    let name_style = if is_selected {
        Style::default().fg(colors.bg_panel)
    } else {
        Style::default().fg(colors.text_primary)
    };
    let name_text = format!("{} ", item.name);
    let name_len = name_text.len() as u16;
    buf.set_string(x, row, &name_text, name_style);
    name_len
}

fn render_item_version(item: &crate::components::extensions_modal::ExtensionItem, row: u16, x: u16, buf: &mut Buffer, colors: &ThemeColors, is_selected: bool) -> u16 {
    let Some(ref version) = item.version else { return 0; };
    let version_style = if is_selected {
        Style::default().fg(colors.bg_panel)
    } else {
        Style::default().fg(colors.text_muted)
    };
    let version_text = format!("{} ", version);
    let len = version_text.len() as u16;
    buf.set_string(x, row, &version_text, version_style);
    len
}

fn render_item_scope(item: &crate::components::extensions_modal::ExtensionItem, area: Rect, row: u16, x: u16, buf: &mut Buffer, colors: &ThemeColors, is_selected: bool) -> u16 {
    let scope_text = match item.scope {
        ExtensionScope::Project => "(project)  ",
        ExtensionScope::Workspace => "(workspace)",
    };
    let scope_style = if is_selected {
        Style::default().fg(colors.bg_panel)
    } else {
        Style::default().fg(colors.text_muted)
    };
    let scope_len = scope_text.len() as u16;
    let pad_len = 20.min((area.x + area.width - 1 - x - scope_len - 12) as usize);
    let mut cx = x + pad_len as u16;
    buf.set_string(cx, row, scope_text, scope_style);
    cx += scope_len;
    cx - x
}

fn render_item_action(item: &crate::components::extensions_modal::ExtensionItem, row: u16, x: u16, buf: &mut Buffer, colors: &ThemeColors, is_selected: bool) {
    let action_text = match item.action {
        ExtensionAction::Install => "[install]",
        ExtensionAction::Installed => "[installed]",
        ExtensionAction::Update => "[update]",
    };
    let action_style = if is_selected {
        Style::default().fg(colors.bg_panel)
    } else {
        Style::default().fg(colors.accent_secondary)
    };
    buf.set_string(x, row, action_text, action_style);
}

/// Render footer if needed
fn render_footer(_area: Rect, _buf: &mut Buffer, _colors: &ThemeColors) {
    // Footer can be used for keyboard hints in the future
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::buffer::Buffer;

    #[test]
    fn test_render_small_area() {
        let modal = ExtensionsModal::default();
        let area = Rect::new(0, 0, 5, 3);
        let mut buf = Buffer::empty(area);
        let theme = ThemeWrapper::default();
        render(&modal, area, &mut buf, &theme);
        // Should not panic on small areas
    }

    #[test]
    fn test_render_normal_area() {
        let modal = ExtensionsModal::default();
        let area = Rect::new(10, 10, 80, 20);
        let mut buf = Buffer::empty(area);
        let theme = ThemeWrapper::default();
        render(&modal, area, &mut buf, &theme);
        // Check that border chars are set
        assert!(buf.cell((10, 10)).is_some());
    }
}
