use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Style, Modifier},
};
use crate::theme::ThemeWrapper;
use super::{CommandPalette, PaletteItem, PaletteStep};

pub fn render(palette: &CommandPalette, area: Rect, buf: &mut Buffer, theme: &ThemeWrapper) {
    let bg_panel: ratatui::style::Color = theme.color("bg.panel").into();
    let accent_primary: ratatui::style::Color = theme.color("accent.primary").into();
    let accent_secondary: ratatui::style::Color = theme.color("accent.secondary").into();
    let text_primary: ratatui::style::Color = theme.color("text.primary").into();
    let text_muted: ratatui::style::Color = theme.color("text.muted").into();
    let text_secondary: ratatui::style::Color = theme.color("text.secondary").into();
    let border_unfocused: ratatui::style::Color = theme.color("border.unfocused").into();

    clear_background(area, buf, bg_panel);
    draw_border(area, buf, border_unfocused);
    draw_title(area, buf, accent_primary, text_muted);
    draw_panes(palette, area, buf, theme, accent_secondary, text_secondary);
    draw_input_area(palette, area, buf, accent_primary, text_primary, text_muted, text_secondary);
}

fn clear_background(area: Rect, buf: &mut Buffer, bg_panel: ratatui::style::Color) {
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_style(Style::default().bg(bg_panel));
            }
        }
    }
}

fn draw_border(area: Rect, buf: &mut Buffer, border_unfocused: ratatui::style::Color) {
    for x in area.x..area.x + area.width {
        if let Some(cell) = buf.cell_mut((x, area.y)) {
            cell.set_char('─');
            cell.set_style(Style::default().fg(border_unfocused));
        }
    }
    for x in area.x..area.x + area.width {
        if let Some(cell) = buf.cell_mut((x, area.y + area.height - 1)) {
            cell.set_char('─');
            cell.set_style(Style::default().fg(border_unfocused));
        }
    }
    for y in area.y..area.y + area.height {
        if let Some(cell) = buf.cell_mut((area.x, y)) {
            cell.set_char('│');
            cell.set_style(Style::default().fg(border_unfocused));
        }
    }
    for y in area.y..area.y + area.height {
        if let Some(cell) = buf.cell_mut((area.x + area.width - 1, y)) {
            cell.set_char('│');
            cell.set_style(Style::default().fg(border_unfocused));
        }
    }
    if let Some(cell) = buf.cell_mut((area.x, area.y)) {
        cell.set_char('╭');
        cell.set_style(Style::default().fg(border_unfocused));
    }
    if let Some(cell) = buf.cell_mut((area.x + area.width - 1, area.y)) {
        cell.set_char('╮');
        cell.set_style(Style::default().fg(border_unfocused));
    }
    if let Some(cell) = buf.cell_mut((area.x, area.y + area.height - 1)) {
        cell.set_char('╰');
        cell.set_style(Style::default().fg(border_unfocused));
    }
    if let Some(cell) = buf.cell_mut((area.x + area.width - 1, area.y + area.height - 1)) {
        cell.set_char('╯');
        cell.set_style(Style::default().fg(border_unfocused));
    }
}

fn draw_title(area: Rect, buf: &mut Buffer, accent_primary: ratatui::style::Color, text_muted: ratatui::style::Color) {
    let title = " Command Palette ";
    let title_style = Style::default().fg(accent_primary).add_modifier(Modifier::BOLD);
    for (i, ch) in title.chars().enumerate() {
        let x = area.x + 1 + i as u16;
        if x < area.x + area.width - 1 {
            if let Some(cell) = buf.cell_mut((x, area.y)) {
                cell.set_char(ch);
                cell.set_style(title_style);
            }
        }
    }

    let close_hint = " [Esc] ";
    let close_start = area.x + area.width - 1 - close_hint.len() as u16;
    for (i, ch) in close_hint.chars().enumerate() {
        let x = close_start + i as u16;
        if x > area.x && x < area.x + area.width - 1 {
            if let Some(cell) = buf.cell_mut((x, area.y)) {
                cell.set_char(ch);
                cell.set_style(Style::default().fg(text_muted));
            }
        }
    }
}

fn draw_panes(
    palette: &CommandPalette,
    area: Rect,
    buf: &mut Buffer,
    theme: &ThemeWrapper,
    accent_secondary: ratatui::style::Color,
    text_secondary: ratatui::style::Color,
) {
    let (pane_w, pane_h, pane_y, object_x, action_x, arg_x) = compute_pane_layout(area);
    let inner_y = area.y + 1;

    draw_pane_headers(object_x, action_x, arg_x, inner_y, palette, buf, accent_secondary, text_secondary);
    draw_vertical_separators(pane_y, pane_h, action_x, arg_x, area, buf, text_secondary);

    let object_area = Rect::new(object_x, pane_y, pane_w.saturating_sub(1), pane_h);
    let action_area = Rect::new(action_x, pane_y, pane_w.saturating_sub(1), pane_h);
    let arg_area = Rect::new(arg_x, pane_y, pane_w.saturating_sub(1), pane_h);

    render_panes_by_step(palette, area, buf, theme, object_area, action_area, arg_area);
}

fn compute_pane_layout(area: Rect) -> (u16, u16, u16, u16, u16, u16) {
    let inner_x = area.x + 1;
    let inner_y = area.y + 1;
    let inner_w = area.width.saturating_sub(2);
    let inner_h = area.height.saturating_sub(2);

    let pane_w = inner_w / 3;
    let pane_h = inner_h.saturating_sub(4);
    let pane_y = inner_y + 1;

    let object_x = inner_x;
    let action_x = inner_x + pane_w;
    let arg_x = inner_x + pane_w * 2;
    (pane_w, pane_h, pane_y, object_x, action_x, arg_x)
}

fn render_panes_by_step(
    palette: &CommandPalette,
    _area: Rect,
    buf: &mut Buffer,
    theme: &ThemeWrapper,
    object_area: Rect,
    action_area: Rect,
    arg_area: Rect,
) {
    match palette.step {
        PaletteStep::Object => {
            render_pane_items(object_area, buf, theme, palette, &palette.filtered_objects, &palette.objects, PaletteStep::Object);
        }
        PaletteStep::Action => {
            render_pane_items(object_area, buf, theme, palette, &palette.filtered_objects, &palette.objects, PaletteStep::Object);
            render_pane_items(action_area, buf, theme, palette, &palette.filtered_actions, &palette.actions, PaletteStep::Action);
        }
        PaletteStep::Arguments => {
            render_pane_items(object_area, buf, theme, palette, &palette.filtered_objects, &palette.objects, PaletteStep::Object);
            render_pane_items(action_area, buf, theme, palette, &palette.filtered_actions, &palette.actions, PaletteStep::Action);
            render_arguments_pane(arg_area, buf, theme, palette);
        }
    }
}

fn draw_pane_headers(
    object_x: u16,
    action_x: u16,
    arg_x: u16,
    inner_y: u16,
    palette: &CommandPalette,
    buf: &mut Buffer,
    accent_secondary: ratatui::style::Color,
    text_secondary: ratatui::style::Color,
) {
    let obj_header = " OBJECT ";
    let act_header = " ACTION ";
    let arg_header = " ARGS ";

    let obj_style = if palette.step == PaletteStep::Object {
        Style::default().fg(accent_secondary).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(text_secondary)
    };
    let act_style = if palette.step == PaletteStep::Action {
        Style::default().fg(accent_secondary).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(text_secondary)
    };
    let arg_style = if palette.step == PaletteStep::Arguments {
        Style::default().fg(accent_secondary).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(text_secondary)
    };

    buf.set_string(object_x, inner_y, obj_header, obj_style);
    buf.set_string(action_x, inner_y, act_header, act_style);
    buf.set_string(arg_x, inner_y, arg_header, arg_style);
}

fn draw_vertical_separators(
    pane_y: u16,
    pane_h: u16,
    action_x: u16,
    arg_x: u16,
    area: Rect,
    buf: &mut Buffer,
    text_secondary: ratatui::style::Color,
) {
    for y in pane_y..pane_y + pane_h {
        let sep1_x = action_x - 1;
        if sep1_x > area.x && sep1_x < area.x + area.width - 1 {
            if let Some(cell) = buf.cell_mut((sep1_x, y)) {
                cell.set_char('│');
                cell.set_style(Style::default().fg(text_secondary));
            }
        }
        let sep2_x = arg_x - 1;
        if sep2_x > area.x && sep2_x < area.x + area.width - 1 {
            if let Some(cell) = buf.cell_mut((sep2_x, y)) {
                cell.set_char('│');
                cell.set_style(Style::default().fg(text_secondary));
            }
        }
    }
}

pub fn render_pane_items(
    area: Rect,
    buf: &mut Buffer,
    theme: &ThemeWrapper,
    palette: &CommandPalette,
    indices: &[usize],
    items: &[PaletteItem],
    pane_step: PaletteStep,
) {
    let text_primary: ratatui::style::Color = theme.color("text.primary").into();
    let text_muted: ratatui::style::Color = theme.color("text.muted").into();
    let accent_secondary: ratatui::style::Color = theme.color("accent.secondary").into();

    let visible_items = area.height as usize;
    let start_idx = if palette.selected >= visible_items {
        palette.selected - visible_items + 1
    } else {
        0
    };

    for i in 0..visible_items {
        render_single_pane_item(area, buf, palette, indices, items, pane_step.clone(), text_primary, text_muted, accent_secondary, start_idx, i);
    }
}

fn render_single_pane_item(
    area: Rect,
    buf: &mut Buffer,
    palette: &CommandPalette,
    indices: &[usize],
    items: &[PaletteItem],
    pane_step: PaletteStep,
    text_primary: ratatui::style::Color,
    text_muted: ratatui::style::Color,
    accent_secondary: ratatui::style::Color,
    start_idx: usize,
    i: usize,
) {
    let item_idx = start_idx + i;
    if item_idx >= indices.len() {
        return;
    }
    let global_idx = indices[item_idx];
    let item = &items[global_idx];
    let is_selected = item_idx == palette.selected;
    let is_active_pane = is_pane_active(pane_step, palette.step.clone());

    let y = area.y + i as u16;
    let icon = if is_selected && is_active_pane { "▸" } else { " " };
    let label_style = compute_label_style(is_selected, is_active_pane, accent_secondary, text_primary, text_muted);

    buf.set_string(area.x + 1, y, icon, label_style);
    buf.set_string(area.x + 3, y, &item.label, label_style);
}

fn is_pane_active(pane_step: PaletteStep, current_step: PaletteStep) -> bool {
    match pane_step {
        PaletteStep::Object => current_step == PaletteStep::Object,
        PaletteStep::Action => current_step == PaletteStep::Action,
        PaletteStep::Arguments => false,
    }
}

fn compute_label_style(
    is_selected: bool,
    is_active_pane: bool,
    accent_secondary: ratatui::style::Color,
    text_primary: ratatui::style::Color,
    text_muted: ratatui::style::Color,
) -> Style {
    match (is_selected, is_active_pane) {
        (true, true) => Style::default().fg(accent_secondary).add_modifier(Modifier::BOLD),
        (true, false) => Style::default().fg(text_primary),
        (false, _) => Style::default().fg(text_muted),
    }
}

fn render_arguments_pane(area: Rect, buf: &mut Buffer, theme: &ThemeWrapper, palette: &CommandPalette) {
    let text_primary: ratatui::style::Color = theme.color("text.primary").into();
    let text_muted: ratatui::style::Color = theme.color("text.muted").into();

    if let (Some(obj), Some(action)) = (&palette.selected_object, &palette.selected_action) {
        let context = format!("{} → {}", obj.label, action.label);
        let context_style = Style::default().fg(text_primary);
        buf.set_string(area.x + 1, area.y, &context, context_style);

        let hint = match action.id.as_str() {
            "read" | "edit" | "write" | "delete" | "load" => "filename or path",
            "switch" => "model name",
            "save" => "session name",
            "start" | "stop" | "view" | "configure" => "agent name",
            _ => "value",
        };
        let hint_style = Style::default().fg(text_muted);
        buf.set_string(area.x + 1, area.y + 2, hint, hint_style);
    }
}

fn draw_input_area(
    palette: &CommandPalette,
    area: Rect,
    buf: &mut Buffer,
    accent_primary: ratatui::style::Color,
    text_primary: ratatui::style::Color,
    text_muted: ratatui::style::Color,
    text_secondary: ratatui::style::Color,
) {
    let inner_x = area.x + 1;
    draw_query_input(inner_x, area, buf, palette, accent_primary, text_primary, text_muted);
    draw_argument_input(inner_x, area, buf, palette, text_primary, text_muted, text_secondary);
    draw_navigation_hint(inner_x, area, buf, text_muted);
}

fn draw_query_input(
    inner_x: u16,
    area: Rect,
    buf: &mut Buffer,
    palette: &CommandPalette,
    accent_primary: ratatui::style::Color,
    text_primary: ratatui::style::Color,
    text_muted: ratatui::style::Color,
) {
    let input_y = area.y + area.height - 3;
    buf.set_string(inner_x, input_y, "▸ ", Style::default().fg(accent_primary));

    let query_text = if palette.query.is_empty() { "type to filter..." } else { &palette.query };
    let query_style = if palette.query.is_empty() {
        Style::default().fg(text_muted)
    } else {
        Style::default().fg(text_primary)
    };
    buf.set_string(inner_x + 2, input_y, query_text, query_style);
}

fn draw_argument_input(
    inner_x: u16,
    area: Rect,
    buf: &mut Buffer,
    palette: &CommandPalette,
    text_primary: ratatui::style::Color,
    text_muted: ratatui::style::Color,
    text_secondary: ratatui::style::Color,
) {
    if palette.step != PaletteStep::Arguments {
        return;
    }
    let arg_y = area.y + area.height - 4;
    buf.set_string(inner_x, arg_y, "value: ", Style::default().fg(text_secondary));
    let arg_text = if palette.argument_input.is_empty() { "enter value..." } else { &palette.argument_input };
    let arg_style = if palette.argument_input.is_empty() {
        Style::default().fg(text_muted)
    } else {
        Style::default().fg(text_primary)
    };
    buf.set_string(inner_x + 7, arg_y, arg_text, arg_style);
}

fn draw_navigation_hint(inner_x: u16, area: Rect, buf: &mut Buffer, text_muted: ratatui::style::Color) {
    let instr_y = area.y + area.height - 2;
    buf.set_string(inner_x, instr_y, "[↑↓] navigate  [Enter] select  [Esc] close", Style::default().fg(text_muted));
}
