use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};
use crate::theme::ThemeWrapper;
use super::{Onboarding, OnboardingStep};

// ─── Main Render Entry ─────────────────────────────────────────────────────────

pub fn render_onboarding(onboarding: &Onboarding, area: Rect, buf: &mut Buffer, theme: &ThemeWrapper) {
    // Note: Background clearing is NOT done here — the caller (render_onboarding_mode)
    // handles it before rendering the status bar, to avoid wiping it

    match &onboarding.step {
        OnboardingStep::Welcome => render_welcome(area, buf, theme),
        OnboardingStep::ProviderSelect => render_provider_select(area, buf, theme, onboarding),
        OnboardingStep::KeyInput => render_key_input(area, buf, theme, onboarding),
        OnboardingStep::ModelSelect => render_model_select(area, buf, theme, onboarding),
        OnboardingStep::Complete => render_complete(area, buf, theme, onboarding),
    }
}

// ─── Welcome Step ─────────────────────────────────────────────────────────────

fn render_welcome(area: Rect, buf: &mut Buffer, theme: &ThemeWrapper) {
    let accent: Color = theme.color("accent.primary").into();
    let text_muted: Color = theme.color("text.muted").into();
    let bg_base: Color = theme.color("bg.base").into();
    let fg_dark = bg_base;
    let button_bg = accent;
    let button_fg = fg_dark;

    let center_x = area.x + area.width / 2;
    let center_y = area.y + area.height / 2;

    // Logo (Braille dots forming circle) – left side
    let logo_y = center_y - 3;
    render_dotted_logo(center_x.saturating_sub(8), logo_y, buf, accent);

    // Buttons to the right of logo – stacked vertically
    let button_x = center_x + 4;
    let button_y = logo_y;
    render_button_widget(buf, button_x, button_y, "Get started", 'g', button_bg, button_fg);
    render_button_widget(buf, button_x, button_y + 2, "Enter", 'e', button_bg, button_fg);

    // Title and subtitle below logo
    let title_y = logo_y + 7;
    let title = Paragraph::new("runie")
        .style(Style::default().fg(accent).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center);
    title.render(Rect::new(area.x, title_y, area.width, 1), buf);

    let subtitle_y = title_y + 1;
    let subtitle = Paragraph::new("AI coding assistant")
        .style(Style::default().fg(text_muted))
        .alignment(Alignment::Center);
    subtitle.render(Rect::new(area.x, subtitle_y, area.width, 1), buf);
}

fn render_dotted_logo(cx: u16, y: u16, buf: &mut Buffer, color: Color) {
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
        let row_y = y + row as u16;
        for (col, &val) in row_data.iter().enumerate() {
            if val == 1 {
                let col_x = cx.saturating_sub(3) + col as u16;
                if col_x < buf.area.width && row_y < buf.area.height {
                    buf.cell_mut((col_x, row_y)).unwrap().set_char(dot.chars().next().unwrap());
                    buf.cell_mut((col_x, row_y)).unwrap().set_style(style);
                }
            }
        }
    }
}

/// Render a button as a native ratatui Paragraph widget
fn render_button_widget(
    buf: &mut Buffer,
    x: u16,
    y: u16,
    text: &str,
    shortcut: char,
    bg: Color,
    fg: Color,
) {
    let spans: Vec<Span> = text
        .chars()
        .map(|ch| {
            if ch.to_lowercase().next() == Some(shortcut) {
                Span::styled(ch.to_string(), Style::default().fg(fg).bg(bg).add_modifier(Modifier::UNDERLINED))
            } else {
                Span::styled(ch.to_string(), Style::default().fg(fg).bg(bg))
            }
        })
        .collect();

    let line = Line::from(spans);
    let text_widget = Text::from(vec![line]);
    let para = Paragraph::new(text_widget)
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(bg).bg(bg)))
        .style(Style::default().bg(bg));

    let button_area = Rect::new(x, y, text.len() as u16 + 2, 1);
    para.render(button_area, buf);
}

// ─── Provider Select Step ────────────────────────────────────────────────────

fn render_provider_select(area: Rect, buf: &mut Buffer, theme: &ThemeWrapper, onboarding: &Onboarding) {
    let accent: Color = theme.color("accent.primary").into();
    let text_primary: Color = theme.color("text.primary").into();
    let text_muted: Color = theme.color("text.muted").into();

    let center_x = area.x + area.width / 2;
    let inner_x = center_x.saturating_sub(14);

    // Vertically center all content
    let logo_h = 5;
    let gap_after_logo = 2;
    let title_h = 1;
    let gap_after_title = 2;
    let list_h = onboarding.providers.len() as u16;
    let total_h = logo_h + gap_after_logo + title_h + gap_after_title + list_h;
    let start_y = area.y + (area.height.saturating_sub(total_h)) / 2;

    // Small logo (centered)
    let logo_y = start_y;
    render_small_logo(center_x, logo_y, buf, accent);

    // Title
    let title_y = logo_y + logo_h + gap_after_logo;
    let title = Paragraph::new("Choose provider")
        .style(Style::default().fg(text_primary).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Left);
    title.render(Rect::new(inner_x, title_y, 30, 1), buf);

    // Provider list
    let list_y = title_y + title_h + gap_after_title;
    for (i, provider) in onboarding.providers.iter().enumerate() {
        let row_y = list_y + i as u16;
        let is_selected = Some(i) == onboarding.selected_provider;

        // Selection indicator
        let indicator = if is_selected { "▸ " } else { "  " };
        let indicator_style = if is_selected {
            Style::default().fg(accent)
        } else {
            Style::default().fg(text_muted)
        };
        let indicator_span = Span::styled(indicator, indicator_style);

        // Provider name
        let name_style = if is_selected {
            Style::default().fg(accent).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(text_primary)
        };
        let name_span = Span::styled(&provider.name, name_style);

        let line = Line::from(vec![indicator_span, name_span]);
        let para = Paragraph::new(line);
        para.render(Rect::new(inner_x, row_y, area.width.saturating_sub(inner_x), 1), buf);
    }
}

fn render_small_logo(cx: u16, y: u16, buf: &mut Buffer, color: Color) {
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
        let row_y = y + row as u16;
        for (col, &val) in row_data.iter().enumerate() {
            if val == 1 {
                let col_x = cx.saturating_sub(2) + col as u16;
                if col_x < buf.area.width && row_y < buf.area.height {
                    buf.cell_mut((col_x, row_y)).unwrap().set_char(dot.chars().next().unwrap());
                    buf.cell_mut((col_x, row_y)).unwrap().set_style(style);
                }
            }
        }
    }
}

// ─── Key Input Step ───────────────────────────────────────────────────────────

fn render_key_input(area: Rect, buf: &mut Buffer, theme: &ThemeWrapper, onboarding: &Onboarding) {
    let text_primary: Color = theme.color("text.primary").into();
    let text_muted: Color = theme.color("text.muted").into();
    let text_secondary: Color = theme.color("text.secondary").into();
    let success: Color = theme.color("success").into();
    let error_color: Color = theme.color("error").into();

    let center_x = area.x + area.width / 2;

    // Provider name
    let provider_name = onboarding
        .get_current_provider()
        .map(|p| p.name.as_str())
        .unwrap_or("AI");
    let title_text = format!("Enter {} API key", provider_name);
    let title = Paragraph::new(title_text)
        .style(Style::default().fg(text_primary).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center);
    title.render(Rect::new(area.x, area.y + 4, area.width, 1), buf);

    // Input line (minimal: just an underline)
    let input_y = area.y + 7;
    let masked = "•".repeat(onboarding.api_key_input.len().min(35));
    let display = if masked.is_empty() { String::from(" ") } else { masked };
    let input_x = center_x.saturating_sub(20);
    let input = Paragraph::new(display)
        .style(Style::default().fg(text_primary));
    input.render(Rect::new(input_x, input_y, 40, 1), buf);

    // Underline using native widget
    let underline = Paragraph::new("─".repeat(40))
        .style(Style::default().fg(text_muted));
    underline.render(Rect::new(input_x, input_y + 1, 40, 1), buf);

    // Validation indicator
    let is_valid = onboarding.validate_key();
    let status_y = input_y + 3;
    if !onboarding.api_key_input.is_empty() {
        let (icon, status_text, status_color) = if is_valid {
            ("✓", "Valid", success)
        } else {
            ("✗", "Invalid", error_color)
        };
        let status = Paragraph::new(format!("{} {}", icon, status_text))
            .style(Style::default().fg(status_color));
        status.render(Rect::new(input_x, status_y, 20, 1), buf);
    }

    // Privacy hint
    let hint = Paragraph::new("Your key stays local")
        .style(Style::default().fg(text_secondary))
        .alignment(Alignment::Center);
    hint.render(Rect::new(area.x, status_y + 2, area.width, 1), buf);
}

// ─── Model Select Step ────────────────────────────────────────────────────────

fn render_model_select(area: Rect, buf: &mut Buffer, theme: &ThemeWrapper, onboarding: &Onboarding) {
    let accent: Color = theme.color("accent.primary").into();
    let text_primary: Color = theme.color("text.primary").into();
    let text_muted: Color = theme.color("text.muted").into();
    let text_secondary: Color = theme.color("text.secondary").into();

    let center_x = area.x + area.width / 2;
    let inner_x = center_x.saturating_sub(14);

    // Title
    let title = Paragraph::new("Choose model")
        .style(Style::default().fg(text_primary).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Left);
    title.render(Rect::new(inner_x, area.y + 3, 30, 1), buf);

    // Model list
    let list_y = area.y + 5;
    for (i, model) in onboarding.models.iter().enumerate() {
        let row_y = list_y + i as u16;
        let is_selected = Some(i) == onboarding.selected_model;

        // Model name
        let name_style = if is_selected {
            Style::default().fg(accent).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(text_primary)
        };
        let name_span = Span::styled(&model.name, name_style);

        // Description (truncated if needed)
        let max_desc_len = 25;
        let desc = if model.description.len() > max_desc_len {
            format!("{}...", &model.description[..max_desc_len - 3])
        } else {
            model.description.clone()
        };
        let desc_style = if is_selected {
            Style::default().fg(accent)
        } else {
            Style::default().fg(text_secondary)
        };
        let desc_span = Span::styled(desc, desc_style);

        let line = Line::from(vec![name_span, Span::raw(" "), desc_span]);
        let para = Paragraph::new(line);
        para.render(Rect::new(inner_x, row_y, area.width.saturating_sub(inner_x), 1), buf);
    }
}

// ─── Complete Step ───────────────────────────────────────────────────────────

fn render_complete(area: Rect, buf: &mut Buffer, theme: &ThemeWrapper, onboarding: &Onboarding) {
    let accent: Color = theme.color("accent.primary").into();
    let text_primary: Color = theme.color("text.primary").into();
    let text_muted: Color = theme.color("text.muted").into();
    let success: Color = theme.color("success").into();

    let center_y = area.y + area.height / 2;

    // Title
    let title = Paragraph::new("Ready to code")
        .style(Style::default().fg(text_primary).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center);
    title.render(Rect::new(area.x, center_y - 4, area.width, 1), buf);

    // Checkmark
    let checkmark = Paragraph::new("✓")
        .style(Style::default().fg(success).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center);
    checkmark.render(Rect::new(area.x, center_y - 2, area.width, 1), buf);

    // Summary line
    if let (Some(provider), Some(model)) = (onboarding.get_current_provider(), onboarding.get_current_model()) {
        let summary = format!("Using {} · {}", provider.name, model.name);
        let summary_widget = Paragraph::new(summary)
            .style(Style::default().fg(text_muted))
            .alignment(Alignment::Center);
        summary_widget.render(Rect::new(area.x, center_y, area.width, 1), buf);
    }

    // Start hint
    let hint = Paragraph::new("Enter to start")
        .style(Style::default().fg(accent))
        .alignment(Alignment::Center);
    hint.render(Rect::new(area.x, center_y + 4, area.width, 1), buf);
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