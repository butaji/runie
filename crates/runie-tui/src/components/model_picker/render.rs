use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};
use crate::components::panel::Panel;
use crate::theme::ThemeWrapper;
use super::ModelPicker;

fn theme_color(key: &str, theme: &ThemeWrapper) -> Color {
    theme.color(key).into()
}

pub fn render(picker: &ModelPicker, area: Rect, buf: &mut Buffer, theme: &ThemeWrapper) {
    if area.width < 10 || area.height < 5 {
        return;
    }
    let text_muted = theme_color("text.muted", theme);
    let visible = picker.visible_providers();
    if visible.is_empty() {
        render_empty(area, buf, theme, text_muted);
        return;
    }
    let (p_idx, m_idx) = picker.selected;
    let (dialog_area, inner) = calc_layout(area, picker, &visible);
    Panel::new()
        .border_gradient(theme_color("accent.primary", theme), text_muted)
        .render(dialog_area, buf, |inner, buf| {
            render_title(inner, buf, theme);
            let y_offset = render_providers(p_idx, m_idx, &visible, picker, inner, buf, theme);
            if picker.show_details {
                render_details(p_idx, m_idx, &visible, inner, buf, theme, y_offset);
            }
            render_hint(inner, picker, buf, theme);
        });
}

fn calc_layout<'a>(area: Rect, picker: &ModelPicker, visible: &[&'a super::ProviderGroup]) -> (Rect, Rect) {
    let total_models: usize = visible.iter().map(|p| p.models.len()).sum();
    let header_lines = visible.len();
    let total_lines = header_lines + total_models;
    let dialog_h = (total_lines as u16 + 4).min(area.height - 2);
    let dialog_w = if picker.show_details { 70 } else { 50 }.min(area.width - 4);
    let x = area.x + (area.width.saturating_sub(dialog_w)) / 2;
    let y = area.y + (area.height.saturating_sub(dialog_h)) / 2;
    let dialog_area = Rect::new(x, y, dialog_w, dialog_h);
    let inner = Rect::new(x + 2, y + 1, dialog_w - 4, dialog_h - 2);
    (dialog_area, inner)
}

fn render_title(inner: Rect, buf: &mut Buffer, theme: &ThemeWrapper) {
    let text = Line::from(vec![Span::styled(
        "select model",
        Style::default().fg(theme_color("accent.primary", theme)).add_modifier(Modifier::BOLD),
    )]);
    Paragraph::new(text).render(Rect::new(inner.x, inner.y, inner.width, 1), buf);
}

fn render_providers<'a>(p_idx: usize, m_idx: usize, visible: &[&'a super::ProviderGroup], picker: &ModelPicker, inner: Rect, buf: &mut Buffer, theme: &ThemeWrapper) -> u16 {
    let text_muted = theme_color("text.muted", theme);
    let text_primary = theme_color("text.primary", theme);
    let text_dim = theme_color("text.dim", theme);
    let accent = theme_color("accent.primary", theme);
    let accent_secondary = theme_color("accent.secondary", theme);
    let mut y = inner.y + 2;
    for (provider_display_idx, provider) in visible.iter().enumerate() {
        y = render_provider_header(provider_display_idx, p_idx, provider, inner.x, y, inner.width, accent, text_muted, buf);
        for (model_display_idx, model) in provider.models.iter().enumerate() {
            y = render_model_item(provider_display_idx, model_display_idx, p_idx, m_idx, provider, model, picker, inner.x, y, inner.width, accent, text_primary, text_dim, accent_secondary, buf);
        }
    }
    y
}

fn render_provider_header(provider_display_idx: usize, p_idx: usize, provider: &super::ProviderGroup, x: u16, y: u16, width: u16, accent: Color, text_muted: Color, buf: &mut Buffer) -> u16 {
    let arrow = if provider_display_idx == p_idx { "▶" } else { " " };
    let header_style = if provider_display_idx == p_idx {
        Style::default().fg(accent).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(text_muted)
    };
    let header_line = Line::from(vec![
        Span::styled("  ", Style::default()),
        Span::styled(arrow, header_style),
        Span::styled("  ", Style::default()),
        Span::styled(&provider.provider_name, header_style),
    ]);
    Paragraph::new(header_line).render(Rect::new(x, y, width.min(40), 1), buf);
    y + 1
}

fn render_model_item(provider_display_idx: usize, model_display_idx: usize, p_idx: usize, m_idx: usize, provider: &super::ProviderGroup, model: &super::ModelInfo, picker: &ModelPicker, x: u16, y: u16, width: u16, accent: Color, text_primary: Color, text_dim: Color, accent_secondary: Color, buf: &mut Buffer) -> u16 {
    let is_selected = provider_display_idx == p_idx && model_display_idx == m_idx;
    let is_current = picker.is_current(&provider.provider_id, &model.id);
    let mut line_parts = vec![Span::styled("    ", Style::default())];
    let sel_char = if is_selected { "▸" } else { " " };
    let sel_style = if is_selected { Style::default().fg(accent) } else { Style::default().fg(text_dim) };
    line_parts.push(Span::styled(sel_char, sel_style));
    let name_style = if is_selected {
        Style::default().fg(text_primary).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(text_primary)
    };
    line_parts.push(Span::styled(format!(" {}", model.name), name_style));
    if model.is_recommended {
        line_parts.push(Span::styled(" ★", Style::default().fg(accent_secondary)));
    }
    if is_current {
        line_parts.push(Span::styled(" ← current", Style::default().fg(text_dim)));
    }
    let line = Line::from(line_parts);
    Paragraph::new(line).render(Rect::new(x, y, width.min(40), 1), buf);
    y + 1
}

fn render_details(p_idx: usize, m_idx: usize, visible: &[&super::ProviderGroup], inner: Rect, buf: &mut Buffer, theme: &ThemeWrapper, y_offset: u16) {
    let text_muted = theme_color("text.muted", theme);
    let text_dim = theme_color("text.dim", theme);
    let text_primary = theme_color("text.primary", theme);
    let details_y = inner.y + inner.height.saturating_sub(6);
    if details_y > y_offset + 1 {
        let sep_text = "─".repeat(inner.width as usize);
        let sep = Line::from(vec![Span::styled(&sep_text, Style::default().fg(text_dim))]);
        Paragraph::new(sep).render(Rect::new(inner.x, y_offset, inner.width, 1), buf);
        let y = y_offset + 1;
        if let Some(provider) = visible.get(p_idx) {
            if let Some(model) = provider.models.get(m_idx) {
                let detail_line = Line::from(vec![
                    Span::styled(&provider.provider_name, Style::default().fg(text_muted)),
                    Span::styled(" / ", Style::default().fg(text_dim)),
                    Span::styled(&model.name, Style::default().fg(text_primary)),
                ]);
                Paragraph::new(detail_line).render(Rect::new(inner.x, y, inner.width, 1), buf);
                let desc_line = Line::from(vec![Span::styled(&model.description, Style::default().fg(text_dim))]);
                Paragraph::new(desc_line).render(Rect::new(inner.x, y + 1, inner.width, 1), buf);
                let id_line = Line::from(vec![Span::styled("id: ", Style::default().fg(text_dim)), Span::styled(&model.id, Style::default().fg(text_muted))]);
                Paragraph::new(id_line).render(Rect::new(inner.x, y + 2, inner.width, 1), buf);
            }
        }
    }
}

fn render_hint(inner: Rect, picker: &ModelPicker, buf: &mut Buffer, theme: &ThemeWrapper) {
    let text_dim = theme_color("text.dim", theme);
    let hint_text = if picker.filter.is_empty() {
        "[d] details  [ Esc ] close"
    } else {
        "[d] details  [ Esc ] close  [ / ] filter"
    };
    let hint_line = Line::from(vec![Span::styled(hint_text, Style::default().fg(text_dim))]);
    let hint_y = inner.y + inner.height.saturating_sub(1);
    Paragraph::new(hint_line).render(Rect::new(inner.x, hint_y, inner.width, 1), buf);
}

fn render_empty(area: Rect, buf: &mut Buffer, theme: &ThemeWrapper, text_muted: Color) {
    let dialog_w = 30.min(area.width - 4);
    let dialog_h = 6;
    let x = area.x + (area.width.saturating_sub(dialog_w)) / 2;
    let y = area.y + (area.height.saturating_sub(dialog_h)) / 2;
    let dialog_area = Rect::new(x, y, dialog_w, dialog_h);
    let accent = theme_color("accent.primary", theme);
    Panel::new().border_gradient(accent, text_muted).render(dialog_area, buf, |_inner, _buf| {});
    let msg = "no models match filter";
    let line = Line::from(vec![Span::styled(msg, Style::default().fg(text_muted))]);
    let line_w = msg.len() as u16;
    let line_x = area.x + (area.width.saturating_sub(line_w)) / 2;
    let line_y = area.y + area.height / 2;
    buf.set_line(line_x, line_y, &line, line_w);
}
