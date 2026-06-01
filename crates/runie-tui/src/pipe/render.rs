use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Layout, Rect},
    buffer::Buffer,
    style::{Style, Modifier},
    widgets::Widget,
};
use std::io;

use crate::tui::view_models::ViewModels;
use crate::tui::AppState;
use crate::theme::ThemeWrapper;
use crate::theme::ThemeColors;
use crate::components::CommandPalette;
use crate::components::MessageList;
use crate::layout::{centered_rect, right_aligned_rect};

const SIDEBAR_WIDTH: u16 = 28;

/// RenderPipe transforms ViewModels into terminal frames.
pub struct RenderPipe;

impl RenderPipe {

    #[must_use]
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    pub fn render(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        state: &AppState,
        view_models: ViewModels,
        config: &crate::tui::TuiConfig,
        command_palette: &CommandPalette,
    ) -> io::Result<()> {
        let input_height = Self::input_bar_height(state);
        let show_sidebar = state.show_sidebar;
        let show_status_bar = true;
        let theme = if state.current_theme == "silkcircuit_neon" {
            ThemeWrapper::silkcircuit_neon()
        } else {
            ThemeWrapper::crush_grok()
        };
        let theme_colors = ThemeColors::from(&theme);
        let is_onboarding = matches!(state.mode, crate::tui::TuiMode::Onboarding);
        let palette = command_palette.clone();

        terminal.draw(|frame| {
            let area = frame.area();
            let padded_area = Rect {
                x: area.x + 2,
                y: area.y + 1,
                width: area.width.saturating_sub(4),
                height: area.height.saturating_sub(2),
            };
            let main_areas = Self::layout_main(padded_area, show_status_bar, input_height);

            if is_onboarding {
                Self::render_onboarding_mode(frame.buffer_mut(), area, state, &view_models, main_areas, show_status_bar, &theme, &theme_colors);
            } else if state.home_screen.is_visible() || matches!(state.mode, crate::tui::TuiMode::HomeScreen) {
                Self::render_home_screen_mode(frame.buffer_mut(), area, state, &theme, &theme_colors);
            } else {
                Self::render_normal_mode(frame.buffer_mut(), area, state, &view_models, main_areas, show_sidebar, show_status_bar, &palette, padded_area, &theme, &theme_colors);
            }
        })?;
        Ok(())
    }

    fn input_bar_height(state: &AppState) -> u16 {
        // No attachments yet in pipe render path
        crate::components::input_bar::input_bar_height(&state.textarea, false)
    }

    fn layout_main(padded: Rect, show_status: bool, input_h: u16) -> [Rect; 5] {
        let constraints = [
            Constraint::Length(2),        // topbar + padding
            Constraint::Min(1),           // feed
            Constraint::Length(1),       // global_tags
            Constraint::Length(input_h),  // input
            if show_status { Constraint::Length(1) } else { Constraint::Length(0) }, // hotkeys
        ];
        Layout::vertical(constraints).areas(padded)
    }

    fn render_onboarding_mode(
        buf: &mut Buffer,
        area: Rect,
        state: &AppState,
        vms: &ViewModels,
        main_areas: [Rect; 5],
        show_status_bar: bool,
        theme: &ThemeWrapper,
        theme_colors: &ThemeColors,
    ) {
        let bg_base: ratatui::style::Color = theme.color("bg.base").into();
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_char(' ');
                    cell.set_style(Style::default().bg(bg_base));
                }
            }
        }

        // Render top bar
        crate::components::top_bar::render_top_bar(&vms.top_bar, main_areas[0], buf, theme_colors);

        if let Some(ref onboarding) = state.onboarding {
            let onboarding_area = Rect {
                x: area.x,
                y: area.y,
                width: area.width,
                height: area.height - if show_status_bar { 2 } else { 0 },
            };
            crate::components::onboarding::render::render_onboarding(onboarding, onboarding_area, buf, theme);
        }

        if show_status_bar {
            crate::components::status_bar::render_ref(&vms.status_bar, main_areas[4], buf, theme_colors);
        }
    }

    fn render_home_screen_mode(
        buf: &mut Buffer,
        area: Rect,
        state: &AppState,
        theme: &ThemeWrapper,
        theme_colors: &ThemeColors,
    ) {
        Self::clear_background(buf, area, theme_colors.bg_base);
        crate::components::home_screen::render_home_screen(&state.home_screen, area, buf, theme);
    }

    fn render_normal_mode(
        buf: &mut Buffer,
        area: Rect,
        state: &AppState,
        vms: &ViewModels,
        main_areas: [Rect; 5],
        show_sidebar: bool,
        show_status_bar: bool,
        palette: &CommandPalette,
        padded: Rect,
        theme: &ThemeWrapper,
        theme_colors: &ThemeColors,
    ) {
        Self::clear_background(buf, area, theme_colors.bg_base);
        // main_areas[0] = topbar, [1] = feed, [2] = global_tags, [3] = input, [4] = hotkeys
        crate::components::top_bar::render_top_bar(&vms.top_bar, main_areas[0], buf, theme_colors);
        Self::render_content(buf, vms, state, show_sidebar, main_areas[1], theme, theme_colors);
        crate::components::global_tags::render_global_tags(&vms.global_tags, main_areas[2], buf, theme_colors);
        if state.slash_menu.is_open() {
            let menu_h = 12u16.min(main_areas[1].height.saturating_sub(2));
            let menu_area = Rect {
                x: main_areas[1].x,
                y: main_areas[2].y.saturating_sub(menu_h),
                width: main_areas[1].width,
                height: menu_h,
            };
            crate::components::slash_menu::render_slash_menu(&state.slash_menu, menu_area, buf, theme);
        }
        if state.file_picker.is_open() {
            let picker_h = 16u16.min(main_areas[1].height.saturating_sub(2));
            let picker_area = Rect {
                x: main_areas[1].x,
                y: main_areas[2].y.saturating_sub(picker_h),
                width: main_areas[1].width,
                height: picker_h,
            };
            use ratatui::widgets::Widget;
            (&state.file_picker).render(picker_area, buf);
        }
        Self::render_input(buf, state, main_areas[3], theme, &theme_colors);
        if show_status_bar {
            crate::components::status_bar::render_ref(&vms.status_bar, main_areas[4], buf, theme_colors);
        }
        Self::render_overlays(buf, state, palette, padded, area, theme, theme_colors);
    }

    fn render_content(buf: &mut Buffer, vms: &ViewModels, state: &AppState, show_sidebar: bool, area: Rect, theme: &ThemeWrapper, theme_colors: &ThemeColors) {
        use crate::components::activity_panel::{ActivityPanel, ACTIVITY_PANEL_WIDTH, should_show_activity_panel, render_activity_panel};

        let show_activity = should_show_activity_panel(area.width);
        let activity_width = if show_activity { ACTIVITY_PANEL_WIDTH } else { 0 };

        let mut h_constraints = vec![Constraint::Min(20)];
        if show_sidebar && area.width >= SIDEBAR_WIDTH + 20 {
            h_constraints.push(Constraint::Length(SIDEBAR_WIDTH));
        }
        if activity_width > 0 {
            h_constraints.push(Constraint::Length(activity_width));
        }

        let h_areas = Layout::horizontal(h_constraints.as_slice()).split(area);
        let feed_area = h_areas[0];
        MessageList::render_ref(&vms.message_list, feed_area, buf, theme);

        if show_sidebar && area.width >= SIDEBAR_WIDTH + 20 {
            crate::tui::render::render_agent_list(&vms.agent_list, h_areas[1], buf, theme_colors);
        }

        // Render activity panel on the right
        if show_activity {
            let activity_area_idx = if show_sidebar && area.width >= SIDEBAR_WIDTH + 20 {
                2
            } else {
                1
            };
            if activity_area_idx < h_areas.len() {
                let activity_panel = ActivityPanel::with_jobs(state.background_jobs.clone());
                render_activity_panel(&activity_panel, h_areas[activity_area_idx], buf, theme_colors);
            }
        }
    }

    fn render_input(buf: &mut Buffer, state: &AppState, area: Rect, theme: &ThemeWrapper, theme_colors: &ThemeColors) {
        use crate::tui::state::PermissionMode;

        let mut textarea = state.textarea.clone();
        let accent_color = theme.color("accent.primary").into();
        let text_primary = theme.color("text.primary").into();
        textarea.set_style(Style::default().fg(text_primary));
        textarea.set_cursor_style(Style::default().fg(accent_color).bg(accent_color));
        textarea.set_cursor_line_style(Style::default().remove_modifier(ratatui::style::Modifier::UNDERLINED));
        let text = state.textarea.lines().join("\n");
        let prompt = if text.starts_with('!') {
            "! ".to_string()
        } else if text.starts_with('@') {
            "@ ".to_string()
        } else {
            format!("{ch} ", ch = crate::glyphs::CHEVRON)
        };

        // Build mode indicator
        let mode_indicator = match state.permission_mode {
            PermissionMode::Normal => "runie".to_string(),
            PermissionMode::Plan => "runie · plan".to_string(),
            PermissionMode::AutoApprove => "runie · yolo".to_string(),
        };

        // Calculate char count if text is long (>50% of context window)
        let char_count = {
            let text_len = text.len();
            let ctx_window = state.top_bar.context_window.unwrap_or(128_000);
            let estimated_tokens = text_len * 4;
            if estimated_tokens > ctx_window / 2 {
                Some(text_len)
            } else {
                None
            }
        };

        // Attached files (empty in pipe render path for now)
        let attached_files: Vec<String> = Vec::new();

        crate::components::input_bar::render_input_bar(
            &textarea,
            &prompt,
            &state.input_right_info,
            area,
            buf,
            theme_colors,
            &mode_indicator,
            &attached_files,
            char_count,
            !state.scroll.scroll_focused, // is_focused
        );
    }

    fn render_overlays(
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
            Self::render_permission_modal(buf, state, padded, area, theme, theme_colors);
        }
        if mode == crate::tui::TuiMode::CommandPalette {
            Self::render_command_palette(buf, padded, area, theme, palette, theme_colors);
        }
        if state.shortcuts_panel.is_open() {
            Self::render_shortcuts_panel(buf, state, padded, area, theme, theme_colors);
        }
        if state.settings_modal.is_open() {
            Self::render_settings_modal(buf, state, padded, area, theme, theme_colors);
        }
        if state.context_usage_modal.is_open() {
            Self::render_context_usage_modal(buf, state, padded, area, theme, theme_colors);
        }
        if mode == crate::tui::TuiMode::Overlay {
            Self::render_overlay_mode(buf, state, area, theme);
        }
        if mode == crate::tui::TuiMode::DiffViewer {
            Self::render_diff_viewer(buf, state, area, theme, theme_colors);
        }
        if mode == crate::tui::TuiMode::SessionTree {
            Self::render_session_tree(buf, state, area, theme, theme_colors);
        }
        if !state.history_search_matches.is_empty() {
            Self::render_history_search(buf, state, area, theme);
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
        Self::dim_background(buf, area, theme_colors);
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
        Self::dim_background(buf, area, theme_colors);
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
        Self::dim_background(buf, area, theme_colors);
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
        Self::dim_background(buf, area, theme_colors);
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
        Self::dim_background(buf, area, theme_colors);
        let modal_area = right_aligned_rect(padded, 50, 22);
        crate::components::context_usage_modal::render_context_usage_modal(
            &state.context_usage_modal, state, modal_area, buf, theme);
    }

    fn render_overlay_mode(buf: &mut Buffer, state: &AppState, area: Rect, theme: &ThemeWrapper) {
        let overlay_area = crate::components::Overlay::centered((70, 25), area);
        let mut overlay_buf = Buffer::empty(overlay_area);

        if let Some(ref picker) = state.model_picker {
            picker.render_ref(overlay_area, &mut overlay_buf, theme);
        }

        Self::blit_buffer(buf, area, overlay_area, &overlay_buf);
    }

    fn render_diff_viewer(
        buf: &mut Buffer,
        state: &AppState,
        area: Rect,
        theme: &ThemeWrapper,
        theme_colors: &ThemeColors,
    ) {
        Self::dim_background(buf, area, theme_colors);
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
        Self::dim_background(buf, area, theme_colors);
        let tree_area = right_aligned_rect(area, 70, 25);
        state.session_tree.render_ref(tree_area, buf, theme);
    }

    fn render_history_search(
        buf: &mut Buffer,
        state: &AppState,
        area: Rect,
        _theme: &ThemeWrapper,
    ) {
        let search_h = 10u16.min(area.height.saturating_sub(4));
        let search_area = Rect {
            x: area.x + 2,
            y: area.y + area.height.saturating_sub(search_h + 3),
            width: area.width.saturating_sub(4),
            height: search_h,
        };
        
        // Background
        for y in search_area.top()..search_area.bottom() {
            for x in search_area.left()..search_area.right() {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_char(' ');
                    cell.set_bg(ratatui::style::Color::Rgb(30, 30, 30));
                }
            }
        }
        
        // Border
        let border = ratatui::style::Color::DarkGray;
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
        let header_len = header.len() as u16;
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

    fn clear_background(buf: &mut Buffer, area: Rect, bg_color: ratatui::style::Color) {
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_char(' ');
                    cell.set_style(Style::default().bg(bg_color));
                }
            }
        }
    }

    fn dim_background(buf: &mut Buffer, area: Rect, theme_colors: &ThemeColors) {
        let dim_color = match theme_colors.bg_base {
            ratatui::style::Color::Rgb(r, g, b) => {
                ratatui::style::Color::Rgb(
                    (r as f32 * 0.5).round() as u8,
                    (g as f32 * 0.5).round() as u8,
                    (b as f32 * 0.5).round() as u8,
                )
            }
            ratatui::style::Color::Indexed(idx) => {
                ratatui::style::Color::Indexed(idx.saturating_sub(8))
            }
            _ => ratatui::style::Color::Black,
        };
        ratatui::widgets::Paragraph::new("")
            .style(Style::default().bg(dim_color))
            .render(area, buf);
    }

    fn blit_buffer(buf: &mut Buffer, area: Rect, src_area: Rect, src: &Buffer) {
        for y in 0..src.area.height {
            for x in 0..src.area.width {
                let cell = src.cell((x, y));
                let tx = src_area.x + x;
                let ty = src_area.y + y;
                if tx < area.width && ty < area.height {
                    if let (Some(src_cell), Some(target)) = (cell, buf.cell_mut((tx, ty))) {
                        *target = src_cell.clone();
                    }
                }
            }
        }
    }
}

impl Default for RenderPipe {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use crate::tui::state::{AppState, TuiMode, Onboarding, OnboardingStep};
    use crate::theme::ThemeWrapper;

    /// BUG-20 REGRESSION: Onboarding background must clear cell chars,
    /// not just set background color. Previous chat content (braille chars)
    /// would bleed through if only bg was set.
    #[test]
    fn test_onboarding_background_clears_characters() {
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);

        // Simulate previous frame content (chat with braille chars)
        for y in 0..area.height {
            for x in 0..area.width {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_char('⠋'); // braille spinner char
                    cell.set_style(Style::default().fg(ratatui::style::Color::White));
                }
            }
        }

        let mut state = AppState::default();
        state.mode = TuiMode::Onboarding;
        // Use empty matrix rain so we can verify background is cleared
        let mut onboarding = Onboarding::new(false);
        onboarding.matrix_rain = Some(crate::components::onboarding::MatrixRain::new(0, 0));
        onboarding.step = OnboardingStep::Welcome;
        state.onboarding = Some(onboarding);

        let palette = crate::components::CommandPalette::new();
        let wrap_cache = crate::components::message_list::render::WrapCache::new();
        let vms = crate::tui::view_models::ViewModels::from_app_state(&state, &palette, wrap_cache);
        let theme = ThemeWrapper::default();
        let theme_colors = ThemeColors::from(&theme);
        let main_areas = RenderPipe::layout_main(area, true, 3);

        RenderPipe::render_onboarding_mode(
            &mut buf, area, &state, &vms, main_areas, true, &theme, &theme_colors,
        );

        // Verify all cells have been cleared (char = ' ')
        // With empty matrix rain and minimal dialog, most cells should be space
        let mut found_leakage = false;
        for y in 0..area.height {
            for x in 0..area.width {
                if let Some(cell) = buf.cell((x, y)) {
                    // Skip cells that are part of rendered elements:
                    // - top bar (y=0..2)
                    // - welcome dialog with border (~y=5..19, x=18..62)
                    // - status bar (bottom rows)
                    let is_top_bar = y < 3;
                    let is_status_bar = y >= area.height - 3;
                    let is_dialog = y >= 5 && y <= 19 && x >= 18 && x <= 62;
                    if !is_top_bar && !is_status_bar && !is_dialog && cell.symbol() != " " {
                        found_leakage = true;
                        break;
                    }
                }
            }
            if found_leakage {
                break;
            }
        }

        assert!(
            !found_leakage,
            "Onboarding background should clear previous frame characters outside rendered elements"
        );
    }
}
