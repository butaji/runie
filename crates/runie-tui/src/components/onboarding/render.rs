use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    prelude::Widget,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
};
use crate::components::panel::Panel;
use crate::theme::ThemeWrapper;
use super::{Onboarding, OnboardingStep};

// ─── Main Render Entry ─────────────────────────────────────────────────────────

pub fn render_onboarding(onboarding: &Onboarding, area: Rect, buf: &mut Buffer, theme: &ThemeWrapper) {
    match &onboarding.step {
        OnboardingStep::Welcome => render_welcome(area, buf, theme),
        OnboardingStep::ProviderSelect => render_provider_select(area, buf, theme, onboarding),
        OnboardingStep::KeyInput => render_key_input(area, buf, theme, onboarding),
        OnboardingStep::ModelSelect => render_model_select(area, buf, theme, onboarding),
        OnboardingStep::Complete => render_complete(area, buf, theme, onboarding),
    }
}

fn theme_color(key: &str, theme: &ThemeWrapper) -> Color {
    theme.color(key).into()
}

fn centered_rect(area: Rect, width: u16, height: u16) -> Rect {
    let x = area.x.saturating_add(area.width.saturating_sub(width) / 2);
    let y = area.y.saturating_add(area.height.saturating_sub(height) / 2);
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}

// ─── Welcome Step ─────────────────────────────────────────────────────────────

fn render_welcome(area: Rect, buf: &mut Buffer, theme: &ThemeWrapper) {
    let inner_h = 12; // logo(7) + gap(1) + title(1) + sub(1) + gap(1) + button(1) + gap(1) + hint(1)
    let dialog_h = inner_h + 4;
    let dialog_w = 56;
    let accent = theme_color("accent.primary", theme);
    let border_unfocused = theme_color("border.unfocused", theme);

    let dialog_area = centered_rect(area, dialog_w, dialog_h);

    Panel::new()
        .border_gradient(border_unfocused, accent)
        .title_color(border_unfocused)
        .title_center()
        .render(dialog_area, buf, |inner, buf| {
            let accent = theme_color("accent.primary", theme);
            let center_x = inner.x + inner.width / 2;
            let start_y = inner.y;

            let logo_y = start_y;
            render_logo(Rect::new(center_x - 3, logo_y, 7, 7), buf, accent);

            let title_y = logo_y + 7 + 1;
            render_title(Rect::new(inner.x, title_y, inner.width, 1), buf, "runie", theme);

            let sub_y = title_y + 1;
            render_subtitle(Rect::new(inner.x, sub_y, inner.width, 1), buf, "AI coding assistant", theme);

            let btn_y = sub_y + 2;
            let btn_w = 14;
            let btn_x = center_x.saturating_sub(btn_w / 2);
            render_button(Rect::new(btn_x, btn_y, btn_w, 1), buf, "Get started", 'g', theme);

            let hint_y = btn_y + 2;
            render_hint(Rect::new(inner.x, hint_y, inner.width, 1), buf, "Press Enter to start", theme);
        });
}

// ─── Provider Select Step ────────────────────────────────────────────────────

fn render_provider_select(area: Rect, buf: &mut Buffer, theme: &ThemeWrapper, onboarding: &Onboarding) {
    let list_h = onboarding.providers.len() as u16;
    let inner_h = 5 + 2 + 1 + 2 + list_h; // logo(5) + gap(2) + title(1) + gap(2) + list(n)
    let dialog_h = inner_h + 4;
    let dialog_w = 56;
    let accent = theme_color("accent.primary", theme);
    let border_unfocused = theme_color("border.unfocused", theme);

    let dialog_area = centered_rect(area, dialog_w, dialog_h);

    Panel::new()
        .border_gradient(border_unfocused, accent)
        .title_color(border_unfocused)
        .title_center()
        .render(dialog_area, buf, |inner, buf| {
            let accent = theme_color("accent.primary", theme);
            let center_x = inner.x + inner.width / 2;
            let start_y = inner.y;

            render_small_logo(Rect::new(center_x - 2, start_y, 5, 5), buf, accent);

            let title_y = start_y + 5 + 2;
            render_title_left(Rect::new(inner.x, title_y, inner.width, 1), buf, "Choose provider", theme);

            let list_y = title_y + 1 + 2;
            let items: Vec<&str> = onboarding.providers.iter().map(|p| p.name.as_str()).collect();
            render_list(Rect::new(inner.x, list_y, inner.width, list_h), buf, &items, onboarding.selected_item, theme);
        });
}

// ─── Key Input Step ───────────────────────────────────────────────────────────

fn render_key_input(area: Rect, buf: &mut Buffer, theme: &ThemeWrapper, onboarding: &Onboarding) {
    let inner_h = 1 + 1 + 1 + 1 + 3 + 1; // title(1) + gap(1) + input(1) + underline(1) + gap(3) + hint(1)
    let dialog_h = inner_h + 4;
    let dialog_w = 56;
    let accent = theme_color("accent.primary", theme);
    let border_unfocused = theme_color("border.unfocused", theme);

    let dialog_area = centered_rect(area, dialog_w, dialog_h);

    Panel::new()
        .border_gradient(border_unfocused, accent)
        .title_color(border_unfocused)
        .title_center()
        .render(dialog_area, buf, |inner, buf| {
            let text_primary = theme_color("text.primary", theme);
            let text_muted = theme_color("text.muted", theme);
            let success = theme_color("success", theme);
            let error_color = theme_color("error", theme);
            let center_x = inner.x + inner.width / 2;
            let start_y = inner.y;

            let provider_name = onboarding.get_current_provider().map(|p| p.name.as_str()).unwrap_or("AI");
            let title_y = start_y;
            render_title(Rect::new(inner.x, title_y, inner.width, 1), buf, &format!("Enter {} API key", provider_name), theme);

            let input_y = title_y + 2;
            let masked = "•".repeat(onboarding.api_key_input.chars().count().min(35));
            let display = if masked.is_empty() { String::from(" ") } else { masked };
            let input_x = center_x.saturating_sub(20);
            let input_area = Rect::new(input_x, input_y, 40, 1);
            Paragraph::new(display.as_str())
                .style(Style::default().fg(text_primary))
                .render(input_area, buf);

            let ul_y = input_y + 1;
            Paragraph::new("─".repeat(40))
                .style(Style::default().fg(text_muted))
                .render(Rect::new(input_x, ul_y, 40, 1), buf);

            let status_y = input_y + 3;
            if !onboarding.api_key_input.is_empty() {
                let (icon, status_text, status_color) = if onboarding.validate_key() {
                    ("✓", "Valid", success)
                } else {
                    ("✗", "Invalid", error_color)
                };
                Paragraph::new(format!("{} {}", icon, status_text))
                    .style(Style::default().fg(status_color))
                    .render(Rect::new(input_x, status_y, 20, 1), buf);
            }

            let hint_y = status_y + 2;
            render_hint(Rect::new(inner.x, hint_y, inner.width, 1), buf, "Your key stays local", theme);
        });
}

// ─── Model Select Step ────────────────────────────────────────────────────────

fn render_model_select(area: Rect, buf: &mut Buffer, theme: &ThemeWrapper, onboarding: &Onboarding) {
    let list_h = onboarding.models.len() as u16;
    let inner_h = 1 + 2 + list_h; // title(1) + gap(2) + list(n)
    let dialog_h = inner_h + 4;
    let dialog_w = 56;
    let accent = theme_color("accent.primary", theme);
    let border_unfocused = theme_color("border.unfocused", theme);

    let dialog_area = centered_rect(area, dialog_w, dialog_h);

    Panel::new()
        .border_gradient(border_unfocused, accent)
        .title_color(border_unfocused)
        .title_center()
        .render(dialog_area, buf, |inner, buf| {
            let start_y = inner.y;
            let title_y = start_y;
            render_title_left(Rect::new(inner.x, title_y, inner.width, 1), buf, "Choose model", theme);

            let list_y = title_y + 1 + 2;
            let items: Vec<String> = onboarding.models.iter().map(|m| m.name.clone()).collect();
            let descriptions: Vec<String> = onboarding.models.iter().map(|m| m.description.clone()).collect();
            render_list_with_desc(Rect::new(inner.x, list_y, inner.width, list_h), buf, &items, &descriptions, onboarding.selected_item, theme);
        });
}

// ─── Complete Step ───────────────────────────────────────────────────────────

fn render_complete(area: Rect, buf: &mut Buffer, theme: &ThemeWrapper, onboarding: &Onboarding) {
    let inner_h = 1 + 1 + 1 + 1; // title(1) + check(1) + summary(1) + hint(1) + gaps
    let dialog_h = inner_h + 4;
    let dialog_w = 56;
    let accent = theme_color("accent.primary", theme);
    let border_unfocused = theme_color("border.unfocused", theme);

    let dialog_area = centered_rect(area, dialog_w, dialog_h);

    Panel::new()
        .border_gradient(border_unfocused, accent)
        .title_color(border_unfocused)
        .title_center()
        .render(dialog_area, buf, |inner, buf| {
            let accent = theme_color("accent.primary", theme);
            let success = theme_color("success", theme);
            let start_y = inner.y;

            render_title(Rect::new(inner.x, start_y, inner.width, 1), buf, "Ready to code", theme);

            let check_y = start_y + 2;
            Paragraph::new("✓")
                .style(Style::default().fg(success).add_modifier(Modifier::BOLD))
                .alignment(Alignment::Center)
                .render(Rect::new(inner.x, check_y, inner.width, 1), buf);

            if let (Some(provider), Some(model)) = (onboarding.get_current_provider(), onboarding.get_current_model()) {
                let summary_y = check_y + 2;
                render_subtitle(Rect::new(inner.x, summary_y, inner.width, 1), buf, &format!("Using {} · {}", provider.name, model.name), theme);
            }

            let hint_y = start_y + 6;
            Paragraph::new("Enter to start")
                .style(Style::default().fg(accent))
                .alignment(Alignment::Center)
                .render(Rect::new(inner.x, hint_y, inner.width, 1), buf);
        });
}

// ─── Reusable Element Renderers ───────────────────────────────────────────────

fn render_logo(area: Rect, buf: &mut Buffer, color: Color) {
    let style = Style::default().fg(color);
    let dot = "⠿";
    let pattern: [[u8; 7]; 7] = [
        [0, 1, 1, 1, 1, 1, 0],
        [1, 0, 0, 0, 0, 0, 1],
        [1, 0, 1, 1, 1, 0, 1],
        [1, 0, 1, 0, 1, 0, 1],
        [1, 0, 1, 1, 1, 0, 1],
        [1, 0, 0, 0, 0, 0, 1],
        [0, 1, 1, 1, 1, 1, 0],
    ];
    for (row, row_data) in pattern.iter().enumerate() {
        let row_y = area.y + row as u16;
        for (col, &val) in row_data.iter().enumerate() {
            if val == 1 {
                let col_x = area.x + col as u16;
                if col_x < buf.area.width && row_y < buf.area.height {
                    if let Some(cell) = buf.cell_mut((col_x, row_y)) {
                        cell.set_char(dot.chars().next().unwrap_or('⠿'));
                        cell.set_style(style);
                    }
                }
            }
        }
    }
}

fn render_small_logo(area: Rect, buf: &mut Buffer, color: Color) {
    let style = Style::default().fg(color);
    let dot = "⠿";
    let pattern: [[u8; 5]; 5] = [
        [0, 1, 1, 1, 0],
        [1, 0, 0, 0, 1],
        [1, 0, 1, 0, 1],
        [1, 0, 0, 0, 1],
        [0, 1, 1, 1, 0],
    ];
    for (row, row_data) in pattern.iter().enumerate() {
        let row_y = area.y + row as u16;
        for (col, &val) in row_data.iter().enumerate() {
            if val == 1 {
                let col_x = area.x + col as u16;
                if col_x < buf.area.width && row_y < buf.area.height {
                    if let Some(cell) = buf.cell_mut((col_x, row_y)) {
                        cell.set_char(dot.chars().next().unwrap_or('⠿'));
                        cell.set_style(style);
                    }
                }
            }
        }
    }
}

fn render_title(area: Rect, buf: &mut Buffer, text: &str, theme: &ThemeWrapper) {
    let accent = theme_color("accent.primary", theme);
    Paragraph::new(text)
        .style(Style::default().fg(accent).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .render(area, buf);
}

fn render_title_left(area: Rect, buf: &mut Buffer, text: &str, theme: &ThemeWrapper) {
    let text_primary = theme_color("text.primary", theme);
    Paragraph::new(text)
        .style(Style::default().fg(text_primary).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Left)
        .render(area, buf);
}

fn render_subtitle(area: Rect, buf: &mut Buffer, text: &str, theme: &ThemeWrapper) {
    let text_muted = theme_color("text.muted", theme);
    Paragraph::new(text)
        .style(Style::default().fg(text_muted))
        .alignment(Alignment::Center)
        .render(area, buf);
}

fn render_button(area: Rect, buf: &mut Buffer, text: &str, shortcut: char, theme: &ThemeWrapper) {
    let accent = theme_color("accent.primary", theme);
    let bg_base = theme_color("bg.base", theme);
    let spans: Vec<Span> = text.chars().map(|ch| {
        if ch.to_lowercase().next() == Some(shortcut) {
            Span::styled(ch.to_string(), Style::default().fg(bg_base).bg(accent).add_modifier(Modifier::UNDERLINED))
        } else {
            Span::styled(ch.to_string(), Style::default().fg(bg_base).bg(accent))
        }
    }).collect();
    let line = Line::from(spans);
    Paragraph::new(Text::from(vec![line]))
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(accent).bg(accent)))
        .style(Style::default().bg(accent))
        .render(area, buf);
}

fn render_hint(area: Rect, buf: &mut Buffer, text: &str, theme: &ThemeWrapper) {
    let text_muted = theme_color("text.muted", theme);
    Paragraph::new(text)
        .style(Style::default().fg(text_muted))
        .alignment(Alignment::Center)
        .render(area, buf);
}

fn render_list(area: Rect, buf: &mut Buffer, items: &[&str], selected_idx: usize, theme: &ThemeWrapper) {
    let accent = theme_color("accent.primary", theme);
    let text_primary = theme_color("text.primary", theme);
    let text_muted = theme_color("text.muted", theme);
    for (i, &item) in items.iter().enumerate() {
        let row_y = area.y + i as u16;
        let is_selected = i == selected_idx;
        let indicator = if is_selected { "▸ " } else { "  " };
        let indicator_style = if is_selected { Style::default().fg(accent) } else { Style::default().fg(text_muted) };
        let name_style = if is_selected { Style::default().fg(accent).add_modifier(Modifier::BOLD) } else { Style::default().fg(text_primary) };
        let line = Line::from(vec![Span::styled(indicator, indicator_style), Span::styled(item, name_style)]);
        Paragraph::new(line).render(Rect::new(area.x, row_y, area.width, 1), buf);
    }
}

fn render_list_with_desc(area: Rect, buf: &mut Buffer, items: &[String], descriptions: &[String], selected_idx: usize, theme: &ThemeWrapper) {
    let accent = theme_color("accent.primary", theme);
    let text_primary = theme_color("text.primary", theme);
    let text_muted = theme_color("text.muted", theme);
    let text_secondary = theme_color("text.secondary", theme);
    let max_desc_len = 25;
    for (i, (item, desc)) in items.iter().zip(descriptions.iter()).enumerate() {
        let row_y = area.y + i as u16;
        let is_selected = i == selected_idx;
        let indicator = if is_selected { "▸ " } else { "  " };
        let indicator_style = if is_selected { Style::default().fg(accent) } else { Style::default().fg(text_muted) };
        let name_style = if is_selected { Style::default().fg(accent).add_modifier(Modifier::BOLD) } else { Style::default().fg(text_primary) };
        let desc_truncated = if desc.chars().count() > max_desc_len {
            let truncated: String = desc.chars().take(max_desc_len - 3).collect();
            format!("{}...", truncated)
        } else {
            desc.clone()
        };
        let desc_style = if is_selected { Style::default().fg(accent) } else { Style::default().fg(text_secondary) };
        let line = Line::from(vec![Span::styled(indicator, indicator_style), Span::styled(item.as_str(), name_style), Span::raw(" "), Span::styled(&desc_truncated, desc_style)]);
        Paragraph::new(line).render(Rect::new(area.x, row_y, area.width, 1), buf);
    }
}

// Alias for render_ref pattern compatibility
pub use render_onboarding as render_ref;

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;

    fn make_theme() -> ThemeWrapper {
        ThemeWrapper::default()
    }

    #[test]
    fn test_welcome_step_renders() {
        let theme = make_theme();
        let onboarding = Onboarding::new();
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        render_ref(&onboarding, area, &mut buf, &theme);
        let content = buf.content();
        assert!(content.iter().any(|c| c.symbol() == "r" || c.symbol() == "u"));
    }

    #[test]
    fn test_provider_select_renders() {
        let theme = make_theme();
        let mut onboarding = Onboarding::new();
        onboarding.step = OnboardingStep::ProviderSelect;
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        render_ref(&onboarding, area, &mut buf, &theme);
        let content = buf.content();
        assert!(content.iter().any(|c| c.symbol() == "O" || c.symbol() == "p"));
    }

    #[test]
    fn test_key_input_renders() {
        let theme = make_theme();
        let mut onboarding = Onboarding::new();
        onboarding.step = OnboardingStep::KeyInput;
        onboarding.select_provider(0);
        onboarding.api_key_input = "sk-test".to_string();
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        render_ref(&onboarding, area, &mut buf, &theme);
        let content = buf.content();
        assert!(content.iter().any(|c| c.symbol() == "•"));
    }

    #[test]
    fn test_model_select_renders() {
        let theme = make_theme();
        let mut onboarding = Onboarding::new();
        onboarding.step = OnboardingStep::ModelSelect;
        onboarding.select_provider(0);
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        render_ref(&onboarding, area, &mut buf, &theme);
        let content = buf.content();
        assert!(content.iter().any(|c| c.symbol() == "G" || c.symbol() == "P"));
    }

    #[test]
    fn test_complete_step_renders() {
        let theme = make_theme();
        let mut onboarding = Onboarding::new();
        onboarding.step = OnboardingStep::Complete;
        onboarding.select_provider(0);
        onboarding.select_model(0);
        onboarding.api_key_input = "sk-test".to_string();
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        render_ref(&onboarding, area, &mut buf, &theme);
        let content = buf.content();
        assert!(content.iter().any(|c| c.symbol() == "R" || c.symbol() == "e"));
    }
}