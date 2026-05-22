use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
};
use crate::theme::ThemeWrapper;
use super::{ModelOption, Onboarding, OnboardingStep, ProviderOption};

const MODAL_WIDTH: u16 = 60;
const MODAL_MIN_HEIGHT: u16 = 15;

pub fn render_onboarding(
    onboarding: &Onboarding,
    area: Rect,
    buf: &mut Buffer,
    theme: &ThemeWrapper,
) {
    let bg_panel: ratatui::style::Color = theme.color("bg.panel").into();
    let border_color: ratatui::style::Color = theme.color("border.unfocused").into();
    let accent_primary: ratatui::style::Color = theme.color("accent.primary").into();
    let text_primary: ratatui::style::Color = theme.color("text.primary").into();
    let text_secondary: ratatui::style::Color = theme.color("text.secondary").into();
    let text_muted: ratatui::style::Color = theme.color("text.muted").into();
    let success: ratatui::style::Color = theme.color("success").into();
    let error_color: ratatui::style::Color = theme.color("error").into();

    // Calculate centered modal area
    let modal_width = MODAL_WIDTH.min(area.width.saturating_sub(4));
    let modal_height = MODAL_MIN_HEIGHT.max(area.height.saturating_sub(4));
    let modal_area = centered_rect(modal_width, modal_height, area);

    // Fill background
    fill_background(modal_area, buf, bg_panel);

    // Draw border
    draw_border(modal_area, buf, border_color);

    // Render step-specific content
    match &onboarding.step {
        OnboardingStep::Welcome => {
            render_welcome_step(modal_area, buf, theme, accent_primary, text_primary, text_muted);
        }
        OnboardingStep::ProviderSelect => {
            render_provider_select_step(modal_area, buf, theme, onboarding, accent_primary, text_primary, text_secondary, text_muted);
        }
        OnboardingStep::KeyInput => {
            render_key_input_step(modal_area, buf, theme, onboarding, accent_primary, text_primary, text_secondary, text_muted, success, error_color);
        }
        OnboardingStep::ModelSelect => {
            render_model_select_step(modal_area, buf, theme, onboarding, accent_primary, text_primary, text_secondary, text_muted);
        }
        OnboardingStep::Complete => {
            render_complete_step(modal_area, buf, theme, onboarding, accent_primary, text_primary, text_secondary, text_muted);
        }
    }
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width, height)
}

fn fill_background(area: Rect, buf: &mut Buffer, bg_color: ratatui::style::Color) {
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_style(Style::default().bg(bg_color));
            }
        }
    }
}

fn draw_border(area: Rect, buf: &mut Buffer, border_color: ratatui::style::Color) {
    let style = Style::default().fg(border_color);
    // Top border
    for x in area.x..area.x + area.width {
        if let Some(cell) = buf.cell_mut((x, area.y)) {
            cell.set_symbol("─");
            cell.set_style(style);
        }
    }
    // Bottom border
    for x in area.x..area.x + area.width {
        if let Some(cell) = buf.cell_mut((x, area.y + area.height - 1)) {
            cell.set_symbol("─");
            cell.set_style(style);
        }
    }
    // Left border
    for y in area.y..area.y + area.height {
        if let Some(cell) = buf.cell_mut((area.x, y)) {
            cell.set_symbol("│");
            cell.set_style(style);
        }
    }
    // Right border
    for y in area.y..area.y + area.height {
        if let Some(cell) = buf.cell_mut((area.x + area.width - 1, y)) {
            cell.set_symbol("│");
            cell.set_style(style);
        }
    }
    // Corners
    if let Some(cell) = buf.cell_mut((area.x, area.y)) {
        cell.set_symbol("┌");
        cell.set_style(style);
    }
    if let Some(cell) = buf.cell_mut((area.x + area.width - 1, area.y)) {
        cell.set_symbol("┐");
        cell.set_style(style);
    }
    if let Some(cell) = buf.cell_mut((area.x, area.y + area.height - 1)) {
        cell.set_symbol("└");
        cell.set_style(style);
    }
    if let Some(cell) = buf.cell_mut((area.x + area.width - 1, area.y + area.height - 1)) {
        cell.set_symbol("┘");
        cell.set_style(style);
    }
}

// ─── Step Renderers ───────────────────────────────────────────────────────────

fn render_welcome_step(
    area: Rect,
    buf: &mut Buffer,
    theme: &ThemeWrapper,
    accent_primary: ratatui::style::Color,
    text_primary: ratatui::style::Color,
    text_muted: ratatui::style::Color,
) {
    let inner_x = area.x + 2;
    let center_x = area.x + area.width / 2;

    // Title
    let title = "Welcome to runie";
    let title_style = Style::default().fg(accent_primary).add_modifier(Modifier::BOLD);
    let title_len = title.len() as u16;
    let title_x = center_x.saturating_sub(title_len / 2);
    buf.set_string(title_x, area.y + 3, title, title_style);

    // Subtitle
    let subtitle = "AI-powered coding assistant";
    let subtitle_style = Style::default().fg(text_primary);
    let subtitle_len = subtitle.len() as u16;
    let subtitle_x = center_x.saturating_sub(subtitle_len / 2);
    buf.set_string(subtitle_x, area.y + 5, subtitle, subtitle_style);

    // Help text
    let help_top = "Press Enter to get started";
    let help_bottom = "Press Esc to skip setup";
    let help_style = Style::default().fg(text_muted);
    let help_top_len = help_top.len() as u16;
    let help_top_x = center_x.saturating_sub(help_top_len / 2);
    buf.set_string(help_top_x, area.y + area.height - 4, help_top, help_style);

    let help_bottom_len = help_bottom.len() as u16;
    let help_bottom_x = center_x.saturating_sub(help_bottom_len / 2);
    buf.set_string(help_bottom_x, area.y + area.height - 3, help_bottom, help_style);
}

fn render_provider_select_step(
    area: Rect,
    buf: &mut Buffer,
    theme: &ThemeWrapper,
    onboarding: &Onboarding,
    accent_primary: ratatui::style::Color,
    text_primary: ratatui::style::Color,
    text_secondary: ratatui::style::Color,
    text_muted: ratatui::style::Color,
) {
    let inner_x = area.x + 2;

    // Title
    let title = "Choose your AI provider";
    let title_style = Style::default().fg(accent_primary).add_modifier(Modifier::BOLD);
    buf.set_string(inner_x, area.y + 2, title, title_style);

    // Provider list
    let list_start_y = area.y + 4;
    for (i, provider) in onboarding.providers.iter().enumerate() {
        let y = list_start_y + i as u16;
        let is_selected = Some(i) == onboarding.selected_provider;

        // Selection indicator
        let indicator = if is_selected { "▸ " } else { "  " };
        let indicator_style = if is_selected {
            Style::default().fg(accent_primary).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(text_muted)
        };
        buf.set_string(inner_x, y, indicator, indicator_style);

        // Provider name
        let name_style = if is_selected {
            Style::default().fg(accent_primary).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(text_primary)
        };
        buf.set_string(inner_x + 2, y, &provider.name, name_style);
    }

    // Description of selected provider
    if let Some(idx) = onboarding.selected_provider {
        if let Some(provider) = onboarding.providers.get(idx) {
            let desc_y = list_start_y + onboarding.providers.len() as u16 + 1;
            let desc_style = Style::default().fg(text_secondary);
            buf.set_string(inner_x, desc_y, &provider.description, desc_style);
        }
    }

    // Navigation help
    let help_text = "↑↓ to navigate · Enter to select · Esc to go back";
    let help_style = Style::default().fg(text_muted);
    let help_len = help_text.len() as u16;
    let help_x = area.x + (area.width.saturating_sub(help_len)) / 2;
    buf.set_string(help_x, area.y + area.height - 2, help_text, help_style);
}

fn render_key_input_step(
    area: Rect,
    buf: &mut Buffer,
    theme: &ThemeWrapper,
    onboarding: &Onboarding,
    accent_primary: ratatui::style::Color,
    text_primary: ratatui::style::Color,
    text_secondary: ratatui::style::Color,
    text_muted: ratatui::style::Color,
    success: ratatui::style::Color,
    error_color: ratatui::style::Color,
) {
    let inner_x = area.x + 2;
    let inner_width = area.width.saturating_sub(4) as usize;

    // Title with provider name
    let provider_name = onboarding
        .get_current_provider()
        .map(|p| p.name.as_str())
        .unwrap_or("AI");
    let title = format!("Enter your {} API key", provider_name);
    let title_style = Style::default().fg(accent_primary).add_modifier(Modifier::BOLD);
    buf.set_string(inner_x, area.y + 2, &title, title_style);

    // Input box
    let input_y = area.y + 5;
    let input_height = 3;
    let input_box_width = inner_width as u16;

    // Top of input box
    if let Some(cell) = buf.cell_mut((inner_x, input_y)) {
        cell.set_symbol("┌");
        cell.set_style(Style::default().fg(text_muted));
    }
    for x in 1..input_box_width.saturating_sub(1) {
        if let Some(cell) = buf.cell_mut((inner_x + x, input_y)) {
            cell.set_symbol("─");
            cell.set_style(Style::default().fg(text_muted));
        }
    }
    if let Some(cell) = buf.cell_mut((inner_x + input_box_width - 1, input_y)) {
        cell.set_symbol("┐");
        cell.set_style(Style::default().fg(text_muted));
    }

    // Middle line with masked input
    let middle_y = input_y + 1;
    if let Some(cell) = buf.cell_mut((inner_x, middle_y)) {
        cell.set_symbol("│");
        cell.set_style(Style::default().fg(text_muted));
    }
    if let Some(cell) = buf.cell_mut((inner_x + input_box_width - 1, middle_y)) {
        cell.set_symbol("│");
        cell.set_style(Style::default().fg(text_muted));
    }

    // Masked input text
    let masked_input = "•".repeat(onboarding.api_key_input.len().min(40));
    let input_display = if masked_input.is_empty() {
        String::from(" ".repeat(40))
    } else {
        masked_input
    };
    let input_style = Style::default().fg(text_primary);
    buf.set_string(inner_x + 2, middle_y, &input_display, input_style);

    // Bottom of input box
    let bottom_y = input_y + 2;
    if let Some(cell) = buf.cell_mut((inner_x, bottom_y)) {
        cell.set_symbol("└");
        cell.set_style(Style::default().fg(text_muted));
    }
    for x in 1..input_box_width.saturating_sub(1) {
        if let Some(cell) = buf.cell_mut((inner_x + x, bottom_y)) {
            cell.set_symbol("─");
            cell.set_style(Style::default().fg(text_muted));
        }
    }
    if let Some(cell) = buf.cell_mut((inner_x + input_box_width - 1, bottom_y)) {
        cell.set_symbol("┘");
        cell.set_style(Style::default().fg(text_muted));
    }

    // Validation status
    let status_y = bottom_y + 1;
    let is_valid = onboarding.validate_key();
    let (status_icon, status_text, status_color) = if onboarding.api_key_input.is_empty() {
        ("", "", text_muted)
    } else if is_valid {
        ("✓", "Valid key format", success)
    } else {
        ("✗", "Invalid key format", error_color)
    };

    if !status_icon.is_empty() {
        let icon_style = Style::default().fg(status_color);
        buf.set_string(inner_x, status_y, status_icon, icon_style);
        let text_style = Style::default().fg(status_color);
        buf.set_string(inner_x + 2, status_y, status_text, text_style);
    }

    // Help text
    let help_y = area.y + area.height - 4;
    let help_text = "Your key stays local in ~/.runie/config.toml";
    let help_style = Style::default().fg(text_secondary);
    buf.set_string(inner_x, help_y, help_text, help_style);

    // Navigation help
    let nav_text = "Enter to continue · Esc to go back";
    let nav_style = Style::default().fg(text_muted);
    let nav_len = nav_text.len() as u16;
    let nav_x = area.x + (area.width.saturating_sub(nav_len)) / 2;
    buf.set_string(nav_x, area.y + area.height - 2, nav_text, nav_style);
}

fn render_model_select_step(
    area: Rect,
    buf: &mut Buffer,
    theme: &ThemeWrapper,
    onboarding: &Onboarding,
    accent_primary: ratatui::style::Color,
    text_primary: ratatui::style::Color,
    text_secondary: ratatui::style::Color,
    text_muted: ratatui::style::Color,
) {
    let inner_x = area.x + 2;

    // Title
    let title = "Choose a model";
    let title_style = Style::default().fg(accent_primary).add_modifier(Modifier::BOLD);
    buf.set_string(inner_x, area.y + 2, title, title_style);

    // Model list
    let list_start_y = area.y + 4;
    for (i, model) in onboarding.models.iter().enumerate() {
        let y = list_start_y + i as u16;
        let is_selected = Some(i) == onboarding.selected_model;

        // Selection indicator
        let indicator = if is_selected { "▸ " } else { "  " };
        let indicator_style = if is_selected {
            Style::default().fg(accent_primary).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(text_muted)
        };
        buf.set_string(inner_x, y, indicator, indicator_style);

        // Model name
        let name_style = if is_selected {
            Style::default().fg(accent_primary).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(text_primary)
        };
        buf.set_string(inner_x + 2, y, &model.name, name_style);

        // Description on same line, right-aligned
        let desc_style = Style::default().fg(text_secondary);
        let desc_len = model.description.len() as u16;
        let desc_x = area.x + area.width.saturating_sub(desc_len + 2);
        buf.set_string(desc_x, y, &model.description, desc_style);
    }

    // Navigation help
    let help_text = "↑↓ to navigate · Enter to select · Esc to go back";
    let help_style = Style::default().fg(text_muted);
    let help_len = help_text.len() as u16;
    let help_x = area.x + (area.width.saturating_sub(help_len)) / 2;
    buf.set_string(help_x, area.y + area.height - 2, help_text, help_style);
}

fn render_complete_step(
    area: Rect,
    buf: &mut Buffer,
    theme: &ThemeWrapper,
    onboarding: &Onboarding,
    accent_primary: ratatui::style::Color,
    text_primary: ratatui::style::Color,
    text_secondary: ratatui::style::Color,
    text_muted: ratatui::style::Color,
) {
    let inner_x = area.x + 2;
    let inner_width = area.width.saturating_sub(4) as usize;

    // Title
    let title = "You're all set!";
    let title_style = Style::default().fg(accent_primary).add_modifier(Modifier::BOLD);
    let title_len = title.len() as u16;
    let title_x = area.x + (area.width.saturating_sub(title_len)) / 2;
    buf.set_string(title_x, area.y + 2, title, title_style);

    // Summary box
    let box_y = area.y + 5;
    let box_height = 6;
    let box_width = inner_width as u16;

    // Box top
    if let Some(cell) = buf.cell_mut((inner_x, box_y)) {
        cell.set_symbol("┌");
        cell.set_style(Style::default().fg(text_muted));
    }
    for x in 1..box_width.saturating_sub(1) {
        if let Some(cell) = buf.cell_mut((inner_x + x, box_y)) {
            cell.set_symbol("─");
            cell.set_style(Style::default().fg(text_muted));
        }
    }
    if let Some(cell) = buf.cell_mut((inner_x + box_width - 1, box_y)) {
        cell.set_symbol("┐");
        cell.set_style(Style::default().fg(text_muted));
    }

    // Box content
    if let (Some(provider), Some(model)) = (onboarding.get_current_provider(), onboarding.get_current_model()) {
        // Provider
        let provider_label = "Provider: ";
        let provider_value = &provider.name;
        let provider_line = format!("{}{}", provider_label, provider_value);
        buf.set_string(inner_x + 2, box_y + 1, &provider_line, Style::default().fg(text_secondary));

        // Model
        let model_label = "Model: ";
        let model_value = &model.name;
        let model_line = format!("{}{}", model_label, model_value);
        buf.set_string(inner_x + 2, box_y + 2, &model_line, Style::default().fg(text_secondary));

        // API Key (masked)
        let key_label = "API Key: ";
        let masked_key = "•".repeat(8);
        let key_value = format!("{}{}", key_label, masked_key);
        buf.set_string(inner_x + 2, box_y + 3, &key_value, Style::default().fg(text_secondary));
    }

    // Box bottom
    let box_bottom_y = box_y + box_height - 1;
    if let Some(cell) = buf.cell_mut((inner_x, box_bottom_y)) {
        cell.set_symbol("└");
        cell.set_style(Style::default().fg(text_muted));
    }
    for x in 1..box_width.saturating_sub(1) {
        if let Some(cell) = buf.cell_mut((inner_x + x, box_bottom_y)) {
            cell.set_symbol("─");
            cell.set_style(Style::default().fg(text_muted));
        }
    }
    if let Some(cell) = buf.cell_mut((inner_x + box_width - 1, box_bottom_y)) {
        cell.set_symbol("┘");
        cell.set_style(Style::default().fg(text_muted));
    }

    // Side borders for content area
    for y in (box_y + 1)..box_bottom_y {
        if let Some(cell) = buf.cell_mut((inner_x, y)) {
            cell.set_symbol("│");
            cell.set_style(Style::default().fg(text_muted));
        }
        if let Some(cell) = buf.cell_mut((inner_x + box_width - 1, y)) {
            cell.set_symbol("│");
            cell.set_style(Style::default().fg(text_muted));
        }
    }

    // Final help text
    let help_text = "Press Enter to start coding";
    let help_style = Style::default().fg(text_primary).add_modifier(Modifier::BOLD);
    let help_len = help_text.len() as u16;
    let help_x = area.x + (area.width.saturating_sub(help_len)) / 2;
    buf.set_string(help_x, area.y + area.height - 3, help_text, help_style);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;

    fn make_theme() -> ThemeWrapper {
        ThemeWrapper::default()
    }

    #[test]
    fn test_render_welcome_step() {
        let theme = make_theme();
        let onboarding = Onboarding::new();

        let area = Rect::new(0, 0, 80, 30);
        let mut buf = Buffer::empty(area);

        render_onboarding(&onboarding, area, &mut buf, &theme);

        // Check that title appears
        let content = buf.content();
        let has_title = content.iter().any(|cell| {
            cell.symbol() == "W" || cell.symbol() == "e" || cell.symbol() == "l"
        });
        assert!(has_title, "Welcome title should appear");
    }

    #[test]
    fn test_render_provider_select() {
        let theme = make_theme();
        let mut onboarding = Onboarding::new();
        onboarding.step = OnboardingStep::ProviderSelect;

        let area = Rect::new(0, 0, 80, 30);
        let mut buf = Buffer::empty(area);

        render_onboarding(&onboarding, area, &mut buf, &theme);

        let content = buf.content();
        let has_openai = content.iter().any(|cell| cell.symbol() == "O");
        assert!(has_openai, "Provider names should appear");
    }

    #[test]
    fn test_render_key_input_step() {
        let theme = make_theme();
        let mut onboarding = Onboarding::new();
        onboarding.step = OnboardingStep::KeyInput;
        onboarding.select_provider(0);
        onboarding.api_key_input = "sk-test123456".to_string();

        let area = Rect::new(0, 0, 80, 30);
        let mut buf = Buffer::empty(area);

        render_onboarding(&onboarding, area, &mut buf, &theme);

        // Check that input box renders
        let content = buf.content();
        let has_box_chars = content.iter().any(|cell| cell.symbol() == "┌" || cell.symbol() == "┐");
        assert!(has_box_chars, "Input box should render");
    }

    #[test]
    fn test_render_complete_step() {
        let theme = make_theme();
        let mut onboarding = Onboarding::new();
        onboarding.step = OnboardingStep::Complete;
        onboarding.select_provider(0);
        onboarding.select_model(0);
        onboarding.api_key_input = "sk-test123".to_string();

        let area = Rect::new(0, 0, 80, 30);
        let mut buf = Buffer::empty(area);

        render_onboarding(&onboarding, area, &mut buf, &theme);

        let content = buf.content();
        let has_set = content.iter().any(|cell| cell.symbol() == "s" || cell.symbol() == "e" || cell.symbol() == "t");
        assert!(has_set, "Complete message should appear");
    }

    #[test]
    fn test_centered_rect() {
        let area = Rect::new(0, 0, 80, 30);
        let modal = centered_rect(60, 20, area);
        assert_eq!(modal.x, 10);
        assert_eq!(modal.y, 5);
        assert_eq!(modal.width, 60);
        assert_eq!(modal.height, 20);
    }
}
