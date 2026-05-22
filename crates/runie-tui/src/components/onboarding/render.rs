use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
};
use crate::theme::ThemeWrapper;
use super::{Onboarding, OnboardingStep};

// ─── Main Render Entry ─────────────────────────────────────────────────────────

pub fn render_onboarding(onboarding: &Onboarding, area: Rect, buf: &mut Buffer, theme: &ThemeWrapper) {
    let bg_panel: ratatui::style::Color = theme.color("bg.panel").into();

    fill_background(area, buf, bg_panel);

    match &onboarding.step {
        OnboardingStep::Welcome => render_welcome(area, buf, theme),
        OnboardingStep::ProviderSelect => render_provider_select(area, buf, theme, onboarding),
        OnboardingStep::KeyInput => render_key_input(area, buf, theme, onboarding),
        OnboardingStep::ModelSelect => render_model_select(area, buf, theme, onboarding),
        OnboardingStep::Complete => render_complete(area, buf, theme, onboarding),
    }
}

// ─── Background ───────────────────────────────────────────────────────────────

fn fill_background(area: Rect, buf: &mut Buffer, bg_color: ratatui::style::Color) {
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_style(Style::default().bg(bg_color));
            }
        }
    }
}

// ─── Welcome Step ─────────────────────────────────────────────────────────────

fn render_welcome(area: Rect, buf: &mut Buffer, theme: &ThemeWrapper) {
    let accent: ratatui::style::Color = theme.color("accent.primary").into();
    let text_primary: ratatui::style::Color = theme.color("text.primary").into();
    let text_muted: ratatui::style::Color = theme.color("text.muted").into();

    let center_x = area.x + area.width / 2;
    let center_y = area.y + area.height / 2;

    // Logo (Braille dots forming circle)
    let logo_y = center_y - 5;
    render_dotted_logo(center_x, logo_y, buf, accent);

    // Title: "runie"
    let title = "runie";
    let title_style = Style::default().fg(accent).add_modifier(Modifier::BOLD);
    let title_x = center_x.saturating_sub(3);
    buf.set_string(title_x, logo_y + 6, title, title_style);

    // Subtitle
    let subtitle = "AI coding assistant";
    let subtitle_style = Style::default().fg(text_muted);
    let subtitle_x = center_x.saturating_sub(subtitle.len() as u16 / 2);
    buf.set_string(subtitle_x, logo_y + 8, subtitle, subtitle_style);

    // Get started hint
    let hint = "Get started  Enter";
    let hint_style = Style::default().fg(text_primary);
    let hint_x = center_x.saturating_sub(8);
    buf.set_string(hint_x, area.y + area.height - 4, hint, hint_style);
}

fn render_dotted_logo(cx: u16, y: u16, buf: &mut Buffer, color: ratatui::style::Color) {
    let style = Style::default().fg(color);
    let dot = "⠿";

    // Circle of dots (simplified 7x7 pattern)
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
                buf.set_string(col_x, row_y, dot, style);
            }
        }
    }
}

// ─── Provider Select Step ────────────────────────────────────────────────────

fn render_provider_select(area: Rect, buf: &mut Buffer, theme: &ThemeWrapper, onboarding: &Onboarding) {
    let accent: ratatui::style::Color = theme.color("accent.primary").into();
    let text_primary: ratatui::style::Color = theme.color("text.primary").into();
    let text_muted: ratatui::style::Color = theme.color("text.muted").into();

    let center_x = area.x + area.width / 2;
    let inner_x = center_x.saturating_sub(12);

    // Small logo
    let logo_y = area.y + 2;
    render_small_logo(center_x, logo_y, buf, accent);

    // Title
    let title = "Choose provider";
    let title_style = Style::default().fg(text_primary).add_modifier(Modifier::BOLD);
    buf.set_string(inner_x, logo_y + 4, title, title_style);

    // Provider list
    let list_y = logo_y + 6;
    for (i, provider) in onboarding.providers.iter().enumerate() {
        let row_y = list_y + i as u16;
        let is_selected = Some(i) == onboarding.selected_provider;

        let name_style = if is_selected {
            Style::default().fg(accent).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(text_primary)
        };

        buf.set_string(inner_x, row_y, &provider.name, name_style);

        // Hotkey hint right-aligned
        let hotkey = "enter";
        let hotkey_style = if is_selected {
            Style::default().fg(accent)
        } else {
            Style::default().fg(text_muted)
        };
        let hotkey_x = area.x + area.width.saturating_sub(6);
        buf.set_string(hotkey_x, row_y, hotkey, hotkey_style);
    }

    // Navigation hints are shown in the status bar only
}

fn render_small_logo(cx: u16, y: u16, buf: &mut Buffer, color: ratatui::style::Color) {
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
                buf.set_string(col_x, row_y, dot, style);
            }
        }
    }
}

// ─── Key Input Step ───────────────────────────────────────────────────────────

fn render_key_input(area: Rect, buf: &mut Buffer, theme: &ThemeWrapper, onboarding: &Onboarding) {
    let text_primary: ratatui::style::Color = theme.color("text.primary").into();
    let text_muted: ratatui::style::Color = theme.color("text.muted").into();
    let text_secondary: ratatui::style::Color = theme.color("text.secondary").into();
    let success: ratatui::style::Color = theme.color("success").into();
    let error_color: ratatui::style::Color = theme.color("error").into();

    let center_x = area.x + area.width / 2;

    // Provider name
    let provider_name = onboarding
        .get_current_provider()
        .map(|p| p.name.as_str())
        .unwrap_or("AI");
    let title = format!("Enter {} API key", provider_name);
    let title_style = Style::default().fg(text_primary).add_modifier(Modifier::BOLD);
    let title_x = center_x.saturating_sub(title.len() as u16 / 2);
    buf.set_string(title_x, area.y + 4, &title, title_style);

    // Input line (minimal: just an underline)
    let input_y = area.y + 7;
    let input_style = Style::default().fg(text_primary);
    let masked = "•".repeat(onboarding.api_key_input.len().min(35));
    let display = if masked.is_empty() { String::from(" ") } else { masked };
    let input_x = center_x.saturating_sub(20);
    buf.set_string(input_x, input_y, &display, input_style);

    // Underline
    let underline_len = 40;
    let underline_x = input_x;
    for i in 0..underline_len {
        if let Some(cell) = buf.cell_mut((underline_x + i, input_y + 1)) {
            cell.set_char('─');
            cell.set_style(Style::default().fg(text_muted));
        }
    }

    // Validation indicator
    let is_valid = onboarding.validate_key();
    let (icon, status_text, status_color) = if onboarding.api_key_input.is_empty() {
        ("", "", text_muted)
    } else if is_valid {
        ("✓", "Valid", success)
    } else {
        ("✗", "Invalid", error_color)
    };

    let status_y = input_y + 3;
    if !icon.is_empty() {
        let icon_style = Style::default().fg(status_color);
        buf.set_string(input_x, status_y, icon, icon_style);

        let text_style = Style::default().fg(status_color);
        buf.set_string(input_x + 2, status_y, status_text, text_style);
    }

    // Privacy hint
    let hint = "Your key stays local";
    let hint_style = Style::default().fg(text_secondary);
    let hint_x = center_x.saturating_sub(hint.len() as u16 / 2);
    buf.set_string(hint_x, status_y + 2, hint, hint_style);


}

// ─── Model Select Step ────────────────────────────────────────────────────────

fn render_model_select(area: Rect, buf: &mut Buffer, theme: &ThemeWrapper, onboarding: &Onboarding) {
    let accent: ratatui::style::Color = theme.color("accent.primary").into();
    let text_primary: ratatui::style::Color = theme.color("text.primary").into();
    let text_muted: ratatui::style::Color = theme.color("text.muted").into();
    let text_secondary: ratatui::style::Color = theme.color("text.secondary").into();

    let center_x = area.x + area.width / 2;
    let inner_x = center_x.saturating_sub(14);

    // Title
    let title = "Choose model";
    let title_style = Style::default().fg(text_primary).add_modifier(Modifier::BOLD);
    buf.set_string(inner_x, area.y + 3, title, title_style);

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
        buf.set_string(inner_x, row_y, &model.name, name_style);

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
        let desc_x = inner_x + 18;
        buf.set_string(desc_x, row_y, &desc, desc_style);

        // Hotkey
        let hotkey = "enter";
        let hotkey_style = if is_selected {
            Style::default().fg(accent)
        } else {
            Style::default().fg(text_muted)
        };
        let hotkey_x = area.x + area.width.saturating_sub(6);
        buf.set_string(hotkey_x, row_y, hotkey, hotkey_style);
    }


}

// ─── Complete Step ───────────────────────────────────────────────────────────

fn render_complete(area: Rect, buf: &mut Buffer, theme: &ThemeWrapper, onboarding: &Onboarding) {
    let accent: ratatui::style::Color = theme.color("accent.primary").into();
    let text_primary: ratatui::style::Color = theme.color("text.primary").into();
    let text_muted: ratatui::style::Color = theme.color("text.muted").into();
    let success: ratatui::style::Color = theme.color("success").into();

    let center_x = area.x + area.width / 2;
    let center_y = area.y + area.height / 2;

    // Title
    let title = "Ready to code";
    let title_style = Style::default().fg(text_primary).add_modifier(Modifier::BOLD);
    let title_x = center_x.saturating_sub(title.len() as u16 / 2);
    buf.set_string(title_x, center_y - 4, title, title_style);

    // Checkmark
    let checkmark = "✓";
    let check_style = Style::default().fg(success).add_modifier(Modifier::BOLD);
    buf.set_string(center_x, center_y - 2, checkmark, check_style);

    // Summary line
    if let (Some(provider), Some(model)) = (onboarding.get_current_provider(), onboarding.get_current_model()) {
        let summary = format!("Using {} · {}", provider.name, model.name);
        let summary_style = Style::default().fg(text_muted);
        let summary_x = center_x.saturating_sub(summary.len() as u16 / 2);
        buf.set_string(summary_x, center_y, &summary, summary_style);
    }

    // Start hint
    let hint = "Enter to start";
    let hint_style = Style::default().fg(accent);
    let hint_x = center_x.saturating_sub(hint.len() as u16 / 2);
    buf.set_string(hint_x, center_y + 4, hint, hint_style);
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
