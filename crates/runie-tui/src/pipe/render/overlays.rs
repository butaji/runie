//! RenderPipe overlay rendering.

use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

use crate::components::CommandPalette;
use crate::layout::{centered_rect, right_aligned_rect};
use crate::theme::ThemeColors;
use crate::theme::ThemeWrapper;
use crate::tui::AppState;

use super::helpers::{dim_background, blit_buffer};

pub fn render_overlays(
    buf: &mut Buffer,
    state: &AppState,
    palette: &CommandPalette,
    padded: Rect,
    area: Rect,
    theme: &ThemeWrapper,
    theme_colors: &ThemeColors,
) {
    let mode = state.mode.clone();
    if mode == crate::tui::TuiMode::Permission && state.permission_modal.tool.is_some() {
        render_permission_modal(buf, state, padded, area, theme, theme_colors);
    }
    if mode == crate::tui::TuiMode::CommandPalette {
        render_command_palette(buf, padded, area, theme, palette, theme_colors);
    }
    if state.shortcuts_panel.is_open() {
        render_shortcuts_panel(buf, state, padded, area, theme, theme_colors);
    }
    if state.settings_modal.is_open() {
        render_settings_modal(buf, state, padded, area, theme, theme_colors);
    }
    if state.context_usage_modal.is_open() {
        render_context_usage_modal(buf, state, padded, area, theme, theme_colors);
    }
    if mode == crate::tui::TuiMode::Overlay {
        render_overlay_mode(buf, state, area, theme);
    }
    if mode == crate::tui::TuiMode::DiffViewer {
        render_diff_viewer(buf, state, area, theme, theme_colors);
    }
    if mode == crate::tui::TuiMode::SessionTree {
        render_session_tree(buf, state, area, theme, theme_colors);
    }
    if !state.history_search_matches.is_empty() {
        render_history_search(buf, state, area, theme);
    }
    if let Some(ref questionnaire) = state.questionnaire {
        if questionnaire.visible {
            render_questionnaire(buf, questionnaire, padded, area, theme, theme_colors);
        }
    }
    if state.subagent_panel.visible {
        render_subagent_panel(buf, state, padded, area, theme, theme_colors);
    }
    if state.plan_modal.is_open() {
        render_plan_modal(buf, state, padded, area, theme, theme_colors);
    }
}

fn render_permission_modal(
    buf: &mut Buffer,
    state: &AppState,
    padded: Rect,
    area: Rect,
    theme: &ThemeWrapper,
    theme_colors: &ThemeColors,
) {
    dim_background(buf, area, theme_colors);
    let modal_area = centered_rect(padded, 50, 14);
    let mut modal = crate::components::PermissionModal::new(
        state.permission_modal.tool.as_deref().unwrap_or(""),
        state.permission_modal.args.as_deref().unwrap_or(""),
        state.permission_modal.desc.as_deref().unwrap_or(""),
    );
    const TIMEOUT_SECS: u64 = 300;
    modal.timeout_secs = state.permission_modal.timeout_start.map(|start| {
        let elapsed = start.elapsed().as_secs();
        TIMEOUT_SECS.saturating_sub(elapsed)
    });
    modal.render_ref(modal_area, buf, theme);
}

fn render_command_palette(
    buf: &mut Buffer,
    padded: Rect,
    area: Rect,
    theme: &ThemeWrapper,
    palette: &CommandPalette,
    theme_colors: &ThemeColors,
) {
    dim_background(buf, area, theme_colors);
    let palette_area = right_aligned_rect(padded, 70, 20);
    palette.render_ref(palette_area, buf, theme);
}

fn render_shortcuts_panel(
    buf: &mut Buffer,
    state: &AppState,
    padded: Rect,
    area: Rect,
    theme: &ThemeWrapper,
    theme_colors: &ThemeColors,
) {
    dim_background(buf, area, theme_colors);
    let panel_area = right_aligned_rect(padded, 70, 25);
    crate::components::shortcuts_panel::render_shortcuts_panel(
        &state.shortcuts_panel, panel_area, buf, theme);
}

fn render_settings_modal(
    buf: &mut Buffer,
    state: &AppState,
    padded: Rect,
    area: Rect,
    theme: &ThemeWrapper,
    theme_colors: &ThemeColors,
) {
    dim_background(buf, area, theme_colors);
    let modal_area = right_aligned_rect(padded, 60, 20);
    crate::components::settings_modal::render_settings_modal(
        &state.settings_modal, modal_area, buf, theme);
}

fn render_context_usage_modal(
    buf: &mut Buffer,
    state: &AppState,
    padded: Rect,
    area: Rect,
    theme: &ThemeWrapper,
    theme_colors: &ThemeColors,
) {
    dim_background(buf, area, theme_colors);
    let modal_area = right_aligned_rect(padded, 50, 22);
    crate::components::context_usage_modal::render_context_usage_modal(
        &state.context_usage_modal, state, modal_area, buf, theme);
}

fn render_plan_modal(
    buf: &mut Buffer,
    state: &AppState,
    padded: Rect,
    area: Rect,
    _theme: &ThemeWrapper,
    theme_colors: &ThemeColors,
) {
    dim_background(buf, area, theme_colors);
    let modal_width = 60u16.min(padded.width.saturating_sub(4));
    let modal_height = 20u16.min(padded.height.saturating_sub(4));
    let modal_area = centered_rect(padded, modal_width, modal_height);
    use ratatui::widgets::Widget;
    (&state.plan_modal).render(modal_area, buf);
}

fn render_overlay_mode(buf: &mut Buffer, state: &AppState, area: Rect, theme: &ThemeWrapper) {
    let overlay_area = crate::components::Overlay::centered((70, 25), area);
    let mut overlay_buf = Buffer::empty(overlay_area);

    if let Some(ref picker) = state.model_picker {
        picker.render_ref(overlay_area, &mut overlay_buf, theme);
    }

    if let Some(ref ext_modal) = state.extensions_modal {
        ext_modal.render_ref(overlay_area, &mut overlay_buf, theme);
    }

    blit_buffer(buf, area, overlay_area, &overlay_buf);
}

fn render_diff_viewer(
    buf: &mut Buffer,
    state: &AppState,
    area: Rect,
    theme: &ThemeWrapper,
    theme_colors: &ThemeColors,
) {
    dim_background(buf, area, theme_colors);
    let diff_area = centered_rect(area, 80, 25);
    if let Some(ref diff) = state.diff_viewer {
        diff.render_ref(diff_area, buf, theme);
    }
}

fn render_session_tree(
    buf: &mut Buffer,
    state: &AppState,
    area: Rect,
    theme: &ThemeWrapper,
    theme_colors: &ThemeColors,
) {
    dim_background(buf, area, theme_colors);
    let tree_area = right_aligned_rect(area, 70, 25);
    state.session_tree.render_ref(tree_area, buf, theme);
}

fn render_history_search(
    buf: &mut Buffer,
    state: &AppState,
    area: Rect,
    theme: &ThemeWrapper,
) {
    use ratatui::style::{Modifier, Style};
    use crate::style::layout::SEARCH_OVERLAY_HEIGHT;

    let search_h = SEARCH_OVERLAY_HEIGHT.min(area.height.saturating_sub(4));
    let search_area = Rect {
        x: area.x + 2,
        y: area.y + area.height.saturating_sub(search_h + 3),
        width: area.width.saturating_sub(4),
        height: search_h,
    };

    // Background
    let bg_panel: ratatui::style::Color = theme.color("bg.panel").into();
    for y in search_area.top()..search_area.bottom() {
        for x in search_area.left()..search_area.right() {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_char(' ');
                cell.set_bg(bg_panel);
            }
        }
    }

    // Border
    let border: ratatui::style::Color = theme.color("border.unfocused").into();
    for x in search_area.left()..search_area.right() {
        if let Some(cell) = buf.cell_mut((x, search_area.top())) {
            cell.set_fg(border);
        }
        if let Some(cell) = buf.cell_mut((x, search_area.bottom().saturating_sub(1))) {
            cell.set_fg(border);
        }
    }

    let inner = Rect {
        x: search_area.x + 1,
        y: search_area.y + 1,
        width: search_area.width.saturating_sub(2),
        height: search_area.height.saturating_sub(2),
    };

    // Header
    let header = format!("(reverse-i-search) `{}': ", state.history_search_query);
    let header_style = Style::default().fg(ratatui::style::Color::Cyan).add_modifier(Modifier::BOLD);
    buf.set_string(inner.x, inner.y, &header, header_style);

    // Show current match
    let match_text = if let Some(&idx) = state.history_search_matches.get(state.history_search_index) {
        state.input_history.get(idx).map(|s| s.as_str()).unwrap_or("")
    } else {
        "no matches"
    };

    let match_style = Style::default().fg(ratatui::style::Color::White);
    let header_len = header.len() as u16;
    if header_len < inner.width {
        buf.set_string(inner.x + header_len, inner.y, match_text, match_style);
    }

    // Counter
    let counter = format!("{} / {}", state.history_search_index + 1, state.history_search_matches.len());
    let counter_style = Style::default().fg(ratatui::style::Color::Gray);
    let counter_x = inner.x + inner.width.saturating_sub(counter.len() as u16);
    if inner.height > 2 {
        buf.set_string(counter_x, inner.y + inner.height.saturating_sub(1), counter, counter_style);
    }
}

fn render_questionnaire(
    buf: &mut Buffer,
    questionnaire: &crate::components::questionnaire_panel::QuestionnaireState,
    padded: Rect,
    area: Rect,
    _theme: &ThemeWrapper,
    theme_colors: &ThemeColors,
) {
    dim_background(buf, area, theme_colors);
    let panel_area = centered_rect(padded, 60, 16);
    use ratatui::widgets::Widget;
    questionnaire.render(panel_area, buf);
}

fn render_subagent_panel(
    buf: &mut Buffer,
    state: &AppState,
    padded: Rect,
    _area: Rect,
    _theme: &ThemeWrapper,
    _theme_colors: &ThemeColors,
) {
    use crate::components::subagent_panel::SUBAGENT_PANEL_WIDTH;
    let panel_area = Rect {
        x: padded.x + padded.width.saturating_sub(SUBAGENT_PANEL_WIDTH),
        y: padded.y,
        width: SUBAGENT_PANEL_WIDTH,
        height: padded.height.saturating_sub(3),
    };
    use ratatui::widgets::Widget;
    (&state.subagent_panel).render(panel_area, buf);
}
