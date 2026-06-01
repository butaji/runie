use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Layout, Rect},
    buffer::Buffer,
    style::Style,
    widgets::Widget,
};
use std::io;

use crate::tui::view_models::ViewModels;
use crate::tui::AppState;
use crate::theme::ThemeWrapper;
use crate::theme::ThemeColors;
use crate::components::CommandPalette;
use crate::components::MessageList;
use crate::layout::centered_rect;

const SIDEBAR_WIDTH: u16 = 28;

/// RenderPipe transforms ViewModels into terminal frames.
pub struct RenderPipe;

impl RenderPipe {
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
        let theme = config.theme.clone();
        let theme_colors = ThemeColors::from(&config.theme);
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
            } else {
                Self::render_normal_mode(frame.buffer_mut(), area, state, &view_models, main_areas, show_sidebar, show_status_bar, &palette, padded_area, &theme, &theme_colors);
            }
        })?;
        Ok(())
    }

    fn input_bar_height(state: &AppState) -> u16 {
        crate::components::input_bar::input_bar_height(&state.textarea)
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
        Self::render_content(buf, vms, show_sidebar, main_areas[1], theme, theme_colors);
        crate::components::global_tags::render_global_tags(&vms.global_tags, main_areas[2], buf, theme_colors);
        Self::render_input(buf, state, main_areas[3], theme);
        if show_status_bar {
            crate::components::status_bar::render_ref(&vms.status_bar, main_areas[4], buf, theme_colors);
        }
        Self::render_overlays(buf, state, palette, padded, area, theme, theme_colors);
    }

    fn render_content(buf: &mut Buffer, vms: &ViewModels, show_sidebar: bool, area: Rect, theme: &ThemeWrapper, theme_colors: &ThemeColors) {
        let mut h_constraints = vec![Constraint::Min(20)];
        if show_sidebar && area.width >= SIDEBAR_WIDTH + 20 {
            h_constraints.push(Constraint::Length(SIDEBAR_WIDTH));
        }
        let h_areas = Layout::horizontal(h_constraints.as_slice()).split(area);
        MessageList::render_ref(&vms.message_list, h_areas[0], buf, theme);
        if show_sidebar && area.width >= SIDEBAR_WIDTH + 20 {
            crate::tui::render::render_agent_list(&vms.agent_list, h_areas[1], buf, theme_colors);
        }
    }

    fn render_input(buf: &mut Buffer, state: &AppState, area: Rect, theme: &ThemeWrapper) {
        let mut textarea = state.textarea.clone();
        let accent_color = theme.color("accent.primary").into();
        let text_primary = theme.color("text.primary").into();
        textarea.set_style(Style::default().fg(text_primary));
        textarea.set_cursor_style(Style::default().fg(accent_color).bg(accent_color));
        textarea.set_cursor_line_style(Style::default().remove_modifier(ratatui::style::Modifier::UNDERLINED));
        let prompt = format!("{ch} ", ch = crate::glyphs::CHEVRON);
        crate::components::input_bar::render_input_bar(&textarea, &prompt, &state.input_right_info, area, buf, theme);
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
        if mode == crate::tui::TuiMode::Overlay {
            Self::render_overlay_mode(buf, state, area, theme);
        }
        if mode == crate::tui::TuiMode::DiffViewer {
            Self::render_diff_viewer(buf, state, area, theme, theme_colors);
        }
        if mode == crate::tui::TuiMode::SessionTree {
            Self::render_session_tree(buf, state, area, theme, theme_colors);
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
        let palette_area = centered_rect(padded, 70, 20);
        palette.render_ref(palette_area, buf, theme);
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
        let tree_area = centered_rect(area, 70, 25);
        state.session_tree.render_ref(tree_area, buf, theme);
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
