use ratatui::{buffer::Buffer, layout::Rect, style::{Color, Modifier, Style}};
use crate::components::DialogFrame;
use crate::glyphs;
use crate::style::selection;
use crate::style::box_chars;
use crate::theme::ThemeWrapper;
use super::{CommandPalette, PaletteCommandDef};

pub fn render(palette: &CommandPalette, area: Rect, buf: &mut Buffer, theme: &ThemeWrapper) {
    let accent_primary: Color = theme.color("accent.primary").into();
    let accent_secondary: Color = theme.color("accent.secondary").into();
    let text_primary: Color = theme.color("text.primary").into();
    let text_muted: Color = theme.color("text.muted").into();
    let text_secondary: Color = theme.color("text.secondary").into();
    let dialog_w = area.width.saturating_sub(2);
    let dialog_h = area.height.saturating_sub(2);
    DialogFrame::new(dialog_w, dialog_h).title("Commands").show_close_hint()
        .render(area, buf, theme, |inner, buf| {
            if palette.is_argument_mode {
                render_argument_mode(palette, inner, buf, accent_primary, text_primary, text_muted, text_secondary);
            } else {
                render_command_list(palette, inner, buf, accent_secondary, text_primary, text_muted, text_secondary);
            }
        });
}

fn render_command_list(palette: &CommandPalette, area: Rect, buf: &mut Buffer, accent_secondary: Color, text_primary: Color, text_muted: Color, text_secondary: Color) {
    let inner_x = area.x + 1;
    let inner_y = area.y + 1;
    let inner_w = area.width.saturating_sub(2);
    buf.set_string(inner_x, inner_y, crate::glyphs::CHEVRON_WITH_SPACE, Style::default().fg(accent_secondary));

    // Determine the prompt text: show hint, or "No matching commands" when filtered list is empty
    let prompt = if palette.filtered_commands.is_empty() {
        "no matches — clear filter to see all"
    } else {
        "type to search..."
    };
    let query_style = if palette.filtered_commands.is_empty() { Style::default().fg(text_muted) } else { Style::default().fg(text_primary) };
    buf.set_string(inner_x + 2, inner_y, prompt, query_style);
    let sep_y = inner_y + 1;
    draw_separator(inner_x, sep_y, inner_w, buf, text_muted);
    let list_y = sep_y + 1;
    // No footer hint - status bar already shows hotkeys
    let list_h = area.height.saturating_sub(4);
    let visible_items = list_h as usize;
    let commands = &palette.filtered_commands;
    let all_commands = palette.all_commands();
    let mut rendered = 0;
    let max_y = area.y + area.height - 1;
    for i in 0..visible_items {
        if i >= commands.len() { break; }
        let global_idx = commands[i];
        let cmd = &all_commands[global_idx];
        let y = list_y + rendered as u16;
        if y >= max_y { break; }
        let is_selected = i == palette.selected;
        render_command_row(cmd, y, inner_x, inner_w, is_selected, text_primary, text_muted, text_secondary, buf);
        rendered += 1;
    }
    // Grok spec: in-box footer hint strip ↑/↓ nav | Enter select | Esc close
    // Rendered 1 row above the bottom border, with a "─" divider above it.
    if area.height >= 4 {
        let footer_y = area.y + area.height - 2;
        draw_separator(inner_x, footer_y, inner_w, buf, text_muted);
        let footer_text = "↑/↓ nav  |  Enter select  |  Esc close";
        buf.set_string(inner_x + 1, footer_y + 1, footer_text, Style::default().fg(text_muted));
    }
}

fn render_command_row(cmd: &PaletteCommandDef, y: u16, x: u16, max_w: u16, is_selected: bool, text_primary: Color, text_muted: Color, text_secondary: Color, buf: &mut Buffer) {
    let indicator = if is_selected { selection::SELECTED.to_string() } else { selection::UNSELECTED.to_string() };
    let indicator_style = if is_selected { Style::default().fg(text_primary).add_modifier(Modifier::BOLD) } else { Style::default().fg(text_muted) };
    buf.set_string(x + 1, y, &indicator, indicator_style);
    let label_x = x + 3;
    let label_style = if is_selected { Style::default().fg(text_primary).add_modifier(Modifier::BOLD) } else { Style::default().fg(text_secondary) };
    buf.set_string(label_x, y, &cmd.label, label_style);
    if !cmd.aliases.is_empty() {
        let alias_text = format!(" · {}", cmd.aliases[0]);
        let alias_x = label_x + cmd.label.len() as u16;
        buf.set_string(alias_x, y, &alias_text, Style::default().fg(text_muted));
    }
    if let Some(ref kb) = cmd.keybinding {
        let kb_len = kb.len() as u16;
        let kb_x = x + max_w - 1 - kb_len;
        buf.set_string(kb_x, y, kb, Style::default().fg(text_muted));
    }
}

fn render_argument_mode(palette: &CommandPalette, area: Rect, buf: &mut Buffer, accent_primary: Color, text_primary: Color, text_muted: Color, _text_secondary: Color) {
    let inner_x = area.x + 1;
    let inner_y = area.y + 1;
    let inner_w = area.width.saturating_sub(2);
    let cmd_label = if let Some(cmd_def) = palette.selected_command(0) { cmd_def.label.clone() } else { "Arguments".to_string() };
    buf.set_string(inner_x, inner_y, &cmd_label, Style::default().fg(text_primary).add_modifier(Modifier::BOLD));
    let hint = if let Some(cmd_def) = palette.selected_command(0) { &cmd_def.arg_hint } else { "type value" };
    buf.set_string(inner_x, inner_y + 1, hint, Style::default().fg(text_muted));
    let sep_y = inner_y + 2;
    draw_separator(inner_x, sep_y, inner_w, buf, text_muted);
    let input_y = inner_y + 3;
    buf.set_string(inner_x, input_y, &glyphs::CHEVRON.to_string(), Style::default().fg(accent_primary));
    let arg_value = &palette.argument_input;
    let display_value = if arg_value.is_empty() { "type value..." } else { arg_value };
    buf.set_string(inner_x + 2, input_y, display_value, Style::default().fg(text_primary));
    let sep2_y = input_y + 2;
    draw_separator(inner_x, sep2_y, inner_w, buf, text_muted);
}

fn draw_separator(x: u16, y: u16, max_w: u16, buf: &mut Buffer, color: Color) {
    let line: String = box_chars::H.to_string().repeat(max_w as usize);
    buf.set_string(x, y, &line, Style::default().fg(color));
}
