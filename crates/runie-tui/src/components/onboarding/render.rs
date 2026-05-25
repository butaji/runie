use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    prelude::Widget,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
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
    let text_muted = theme_color("text.muted", theme);
    let text_primary = theme_color("text.primary", theme);
    let accent = theme_color("accent.primary", theme);

    let dialog_area = centered_rect(area, 40, 12);

    Panel::new()
        .border_gradient(accent, text_muted)
        .render(dialog_area, buf, |inner, buf| {
            let inner = Rect::new(inner.x + 2, inner.y + 1, inner.width.saturating_sub(4), inner.height.saturating_sub(2));
            let title_y = inner.y;
            Paragraph::new("welcome")
                .style(Style::default().fg(accent).add_modifier(Modifier::BOLD))
                .alignment(Alignment::Left)
                .render(Rect::new(inner.x, title_y, inner.width, 1), buf);

            let sub1_y = title_y + 2;
            Paragraph::new("multi-model coding agent")
                .style(Style::default().fg(text_primary))
                .alignment(Alignment::Left)
                .render(Rect::new(inner.x, sub1_y, inner.width, 1), buf);

            let sub2_y = sub1_y + 1;
            Paragraph::new("configure providers, models, keys")
                .style(Style::default().fg(text_muted))
                .alignment(Alignment::Left)
                .render(Rect::new(inner.x, sub2_y, inner.width, 1), buf);

            // P0-2/P2-3 FIX: Add CTA footer with Enter hint
            let footer_y = inner.y + inner.height.saturating_sub(3);
            Paragraph::new("Press Enter to begin →")
                .style(Style::default().fg(accent))
                .alignment(Alignment::Left)
                .render(Rect::new(inner.x, footer_y, inner.width, 1), buf);

            let skip_y = footer_y + 1;
            Paragraph::new("Esc to skip setup")
                .style(Style::default().fg(text_muted).add_modifier(Modifier::DIM))
                .alignment(Alignment::Left)
                .render(Rect::new(inner.x, skip_y, inner.width, 1), buf);
        });
}

// ─── Provider Select Step ────────────────────────────────────────────────────

fn render_provider_select(area: Rect, buf: &mut Buffer, theme: &ThemeWrapper, onboarding: &Onboarding) {
    let text_muted = theme_color("text.muted", theme);
    let text_primary = theme_color("text.primary", theme);
    let accent = theme_color("accent.primary", theme);

    let display_indices = if onboarding.filtered_provider_indices.is_empty() {
        (0..onboarding.providers.len()).collect::<Vec<_>>()
    } else {
        onboarding.filtered_provider_indices.clone()
    };
    let display_count = display_indices.len();
    let list_h = display_count as u16;
    let dialog_h = list_h + 6;
    let dialog_w = 40;

    let dialog_area = centered_rect(area, dialog_w, dialog_h);

    Panel::new()
        .border_gradient(accent, text_muted)
        .render(dialog_area, buf, |inner, buf| {
            let inner = Rect::new(inner.x + 2, inner.y + 1, inner.width.saturating_sub(4), inner.height.saturating_sub(2));
            let start_y = inner.y;

            let title_y = start_y;
            Paragraph::new("select provider")
                .style(Style::default().fg(accent).add_modifier(Modifier::BOLD))
                .alignment(Alignment::Left)
                .render(Rect::new(inner.x, title_y, inner.width, 1), buf);

            let list_y = title_y + 2;
            for (i, &provider_idx) in display_indices.iter().enumerate() {
                let row_y = list_y + i as u16;
                let provider = &onboarding.providers[provider_idx];
                let is_selected = i == onboarding.selected_item;
                let radio = if is_selected { "◉" } else { "○" };
                let radio_style = if is_selected {
                    Style::default().fg(accent)
                } else {
                    Style::default().fg(text_muted)
                };
                let name_style = if is_selected {
                    Style::default().fg(text_primary).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(text_primary)
                };
                let line = Line::from(vec![
                    Span::styled("  ", Style::default().fg(text_muted)),
                    Span::styled(radio, radio_style),
                    Span::styled("  ", Style::default().fg(text_muted)),
                    Span::styled(provider.name.to_lowercase(), name_style),
                ]);
                Paragraph::new(line).render(Rect::new(inner.x, row_y, inner.width, 1), buf);
            }

            if let Some(ref err) = onboarding.error_message {
                let err_y = list_y + list_h + 1;
                Paragraph::new(err.as_str())
                    .style(Style::default().fg(theme_color("error", theme)))
                    .alignment(Alignment::Left)
                    .render(Rect::new(inner.x, err_y, inner.width, 1), buf);
            }
        });
}

// ─── Model Select Step ────────────────────────────────────────────────────────

fn render_model_select(area: Rect, buf: &mut Buffer, theme: &ThemeWrapper, onboarding: &Onboarding) {
    let text_muted = theme_color("text.muted", theme);
    let text_primary = theme_color("text.primary", theme);
    let accent = theme_color("accent.primary", theme);

    let display_indices = if onboarding.filtered_model_indices.is_empty() {
        (0..onboarding.models.len()).collect::<Vec<_>>()
    } else {
        onboarding.filtered_model_indices.clone()
    };
    let display_count = display_indices.len();
    let list_h = display_count as u16;
    let dialog_h = list_h + 6;
    let dialog_w = 40;

    let dialog_area = centered_rect(area, dialog_w, dialog_h);

    let provider_name = onboarding.get_current_provider()
        .map(|p| p.name.to_lowercase())
        .unwrap_or_default();

    Panel::new()
        .border_gradient(accent, text_muted)
        .render(dialog_area, buf, |inner, buf| {
            let inner = Rect::new(inner.x + 2, inner.y + 1, inner.width.saturating_sub(4), inner.height.saturating_sub(2));
            let start_y = inner.y;

            let title_y = start_y;
            Paragraph::new(format!("{}  >  select models", provider_name))
                .style(Style::default().fg(accent).add_modifier(Modifier::BOLD))
                .alignment(Alignment::Left)
                .render(Rect::new(inner.x, title_y, inner.width, 1), buf);

            let list_y = title_y + 2;
            for (i, &model_idx) in display_indices.iter().enumerate() {
                let row_y = list_y + i as u16;
                let model = &onboarding.models[model_idx];
                let is_selected = i == onboarding.selected_item;
                let radio = if is_selected { "◉" } else { "○" };
                let radio_style = if is_selected {
                    Style::default().fg(accent)
                } else {
                    Style::default().fg(text_muted)
                };
                let name_style = if is_selected {
                    Style::default().fg(text_primary).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(text_primary)
                };
                let line = Line::from(vec![
                    Span::styled("  ", Style::default().fg(text_muted)),
                    Span::styled(radio, radio_style),
                    Span::styled("  ", Style::default().fg(text_muted)),
                    Span::styled(model.name.to_lowercase(), name_style),
                ]);
                Paragraph::new(line).render(Rect::new(inner.x, row_y, inner.width, 1), buf);
            }

            if let Some(ref err) = onboarding.error_message {
                let err_y = list_y + list_h + 1;
                Paragraph::new(err.as_str())
                    .style(Style::default().fg(theme_color("error", theme)))
                    .alignment(Alignment::Left)
                    .render(Rect::new(inner.x, err_y, inner.width, 1), buf);
            }
        });
}

// ─── Key Input Step ─────────────────────────────────────────────────────────

fn render_key_input(area: Rect, buf: &mut Buffer, theme: &ThemeWrapper, onboarding: &Onboarding) {
    let text_muted = theme_color("text.muted", theme);
    let text_primary = theme_color("text.primary", theme);
    let accent = theme_color("accent.primary", theme);
    let success = theme_color("success", theme);

    let provider_name = onboarding.get_current_provider()
        .map(|p| p.name.to_lowercase())
        .unwrap_or_default();

    let dialog_area = centered_rect(area, 40, 9);

    Panel::new()
        .border_gradient(accent, text_muted)
        .render(dialog_area, buf, |inner, buf| {
            let inner = Rect::new(inner.x + 2, inner.y + 1, inner.width.saturating_sub(4), inner.height.saturating_sub(2));
            let start_y = inner.y;

            let title_y = start_y;
            Paragraph::new(format!("{}  >  api key", provider_name))
                .style(Style::default().fg(accent).add_modifier(Modifier::BOLD))
                .alignment(Alignment::Left)
                .render(Rect::new(inner.x, title_y, inner.width, 1), buf);

            let key_y = title_y + 2;
            let key = &onboarding.api_key_input;
            let masked = if key.is_empty() {
                String::new()
            } else if key.len() <= 6 {
                key.clone()
            } else {
                format!("{}...{}", &key[..3], &key[key.len()-2..])
            };
            Paragraph::new(masked)
                .style(Style::default().fg(text_primary))
                .alignment(Alignment::Left)
                .render(Rect::new(inner.x, key_y, inner.width, 1), buf);

            let verify_y = key_y + 2;
            let (verify_text, verify_style) = if onboarding.is_fetching_models {
                ("loading models...", Style::default().fg(text_muted))
            } else if let Some(ref err) = onboarding.fetch_error {
                // P1-1 FIX: Show fetch error instead of validation status
                (err.as_str(), Style::default().fg(theme_color("error", theme)))
            } else {
                let is_valid = onboarding.validate_key();
                if is_valid {
                    ("[✓] valid", Style::default().fg(success))
                } else {
                    ("[ ] verify", Style::default().fg(text_muted))
                }
            };
            Paragraph::new(verify_text)
                .style(verify_style)
                .alignment(Alignment::Left)
                .render(Rect::new(inner.x, verify_y, inner.width, 1), buf);
            
            // P1-1 FIX: Show retry hint if fetch failed
            if onboarding.fetch_error.is_some() {
                let retry_y = verify_y + 1;
                Paragraph::new("press Enter to retry or Esc to go back")
                    .style(Style::default().fg(text_muted))
                    .alignment(Alignment::Left)
                    .render(Rect::new(inner.x, retry_y, inner.width, 1), buf);
            }
        });
}

// ─── Complete Step ───────────────────────────────────────────────────────────

fn render_complete(area: Rect, buf: &mut Buffer, theme: &ThemeWrapper, onboarding: &Onboarding) {
    let text_muted = theme_color("text.muted", theme);
    let text_primary = theme_color("text.primary", theme);
    let accent = theme_color("accent.primary", theme);

    let provider_name = onboarding.get_current_provider()
        .map(|p| p.name.to_lowercase())
        .unwrap_or_default();

    let model_count = onboarding.models.len();

    let dialog_area = centered_rect(area, 40, 10);

    Panel::new()
        .border_gradient(accent, text_muted)
        .render(dialog_area, buf, |inner, buf| {
            let inner = Rect::new(inner.x + 2, inner.y + 1, inner.width.saturating_sub(4), inner.height.saturating_sub(2));
            let start_y = inner.y;

            let configured_y = start_y;
            Paragraph::new(format!("{} configured   {} model", provider_name, model_count))
                .style(Style::default().fg(text_primary))
                .alignment(Alignment::Left)
                .render(Rect::new(inner.x, configured_y, inner.width, 1), buf);

            let prompt_y = configured_y + 2;
            Paragraph::new("add another provider?")
                .style(Style::default().fg(text_muted))
                .alignment(Alignment::Left)
                .render(Rect::new(inner.x, prompt_y, inner.width, 1), buf);

            let options = vec![("yes", "add another provider"), ("no, finish", "complete setup")];
            let option_y = prompt_y + 2;
            for (i, (label, _)) in options.iter().enumerate() {
                let row_y = option_y + i as u16;
                let is_selected = i == onboarding.selected_item;
                let radio = if is_selected { "◉" } else { "○" };
                let style = if is_selected { accent } else { text_muted };
                let line = Line::from(vec![
                    Span::styled(radio, Style::default().fg(style)),
                    Span::styled("  ", Style::default().fg(text_muted)),
                    Span::styled(*label, Style::default().fg(style)),
                ]);
                Paragraph::new(line)
                    .alignment(Alignment::Left)
                    .render(Rect::new(inner.x, row_y, inner.width, 1), buf);
            }
        });
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
        assert!(content.iter().any(|c| c.symbol() == "w"));
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
        assert!(content.iter().any(|c| c.symbol() == "○" || c.symbol() == "◉"));
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
        assert!(content.iter().any(|c| c.symbol() == "●" || c.symbol() == "s"));
    }

    #[test]
    fn test_model_select_renders() {
        let theme = make_theme();
        let mut onboarding = Onboarding::new();
        onboarding.step = OnboardingStep::ProviderSelect;
        onboarding.update_search(""); // Populate filtered_provider_indices
        onboarding.select_provider(0); // Anthropic (index 0 after alphabetical sort)
        onboarding.step = OnboardingStep::ModelSelect;
        onboarding.update_search(""); // Populate filtered_model_indices
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        render_ref(&onboarding, area, &mut buf, &theme);
        let content = buf.content();
        assert!(content.iter().any(|c| c.symbol() == "◉" || c.symbol() == "○"));
    }
}
