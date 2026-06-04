//! RenderPipe transforms ViewModels into terminal frames.

use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    buffer::Buffer,
    layout::Rect,
};
use std::io;

use crate::components::CommandPalette;
use crate::style::helpers::padded_area;
use crate::theme::ThemeWrapper;
use crate::theme::ThemeColors;
use crate::tui::view_models::ViewModels;
use crate::tui::AppState;

mod layout;
pub mod modes;
pub mod overlays;
pub mod helpers;

// Import from sibling modules at pipe/ level
use super::render_content;
use super::render_input;

pub struct RenderPipe;

impl RenderPipe {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    pub fn render(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        state: &AppState,
        view_models: ViewModels,
        _config: &crate::tui::TuiConfig,
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
            let padded_area = padded_area(area);
            let main_areas = Self::layout_main(padded_area, show_status_bar, input_height);

            if is_onboarding {
                Self::render_onboarding_mode(frame.buffer_mut(), area, state, &view_models, main_areas, show_status_bar, &theme, &theme_colors);
            } else if state.home_screen.is_visible() || matches!(state.mode, crate::tui::TuiMode::HomeScreen) {
                Self::render_home_screen_mode(frame.buffer_mut(), area, state, &view_models, main_areas, &theme, &theme_colors);
            } else {
                Self::render_normal_mode(frame.buffer_mut(), area, state, &view_models, main_areas, show_sidebar, show_status_bar, &palette, padded_area, &theme, &theme_colors);
            }
        })?;
        Ok(())
    }

    pub fn input_bar_height(state: &AppState) -> u16 {
        // No attachments yet in pipe render path
        crate::components::input_bar::input_bar_height(&state.textarea, false)
    }

    pub fn layout_main(padded: Rect, show_status: bool, input_h: u16) -> [Rect; 6] {
        use ratatui::layout::{Constraint, Layout};
        let constraints = [
            Constraint::Length(3),        // topbar + 2 blank lines padding
            Constraint::Min(1),           // feed
            Constraint::Length(1),       // global_tags
            Constraint::Length(input_h),  // input
            Constraint::Length(1),       // version separator (blank line)
            if show_status { Constraint::Length(1) } else { Constraint::Length(0) }, // hotkeys
        ];
        Layout::vertical(constraints).areas(padded)
    }

    pub fn render_onboarding_mode(
        buf: &mut Buffer,
        area: Rect,
        state: &AppState,
        vms: &ViewModels,
        main_areas: [Rect; 6],
        show_status_bar: bool,
        theme: &ThemeWrapper,
        theme_colors: &ThemeColors,
    ) {
        use self::modes::render_onboarding_mode;
        render_onboarding_mode(buf, area, state, vms, main_areas, show_status_bar, theme, theme_colors)
    }

    pub fn render_home_screen_mode(
        buf: &mut Buffer,
        area: Rect,
        state: &AppState,
        vms: &ViewModels,
        main_areas: [Rect; 6],
        theme: &ThemeWrapper,
        theme_colors: &ThemeColors,
    ) {
        use self::modes::render_home_screen_mode;
        render_home_screen_mode(buf, area, state, vms, main_areas, theme, theme_colors)
    }

    pub fn render_normal_mode(
        buf: &mut Buffer,
        area: Rect,
        state: &AppState,
        vms: &ViewModels,
        main_areas: [Rect; 6],
        show_sidebar: bool,
        show_status_bar: bool,
        palette: &CommandPalette,
        padded: Rect,
        theme: &ThemeWrapper,
        theme_colors: &ThemeColors,
    ) {
        use self::modes::render_normal_mode;
        render_normal_mode(buf, area, state, vms, main_areas, show_sidebar, show_status_bar, palette, padded, theme, theme_colors)
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
        use self::overlays::render_overlays;
        render_overlays(buf, state, palette, padded, area, theme, theme_colors)
    }

    fn clear_background(buf: &mut Buffer, area: Rect, bg_color: ratatui::style::Color) {
        use self::helpers::clear_background;
        clear_background(buf, area, bg_color)
    }

    fn dim_background(buf: &mut Buffer, area: Rect, theme_colors: &ThemeColors) {
        use self::helpers::dim_background;
        dim_background(buf, area, theme_colors)
    }

    fn blit_buffer(buf: &mut Buffer, area: Rect, src_area: Rect, src: &Buffer) {
        use self::helpers::blit_buffer;
        blit_buffer(buf, area, src_area, src)
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
        let mut buf = setup_test_buffer(area);

        let state = create_onboarding_state();
        let (vms, theme, theme_colors, main_areas) = setup_render_context(area, &state);

        RenderPipe::render_onboarding_mode(
            &mut buf, area, &state, &vms, main_areas, true, &theme, &theme_colors,
        );

        assert_no_background_leakage(&buf, area);
    }

    fn setup_test_buffer(area: Rect) -> Buffer {
        let mut buf = Buffer::empty(area);
        for y in 0..area.height {
            for x in 0..area.width {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_char('⠋'); // braille spinner char
                    cell.set_style(Style::default().fg(ratatui::style::Color::White));
                }
            }
        }
        buf
    }

    fn create_onboarding_state() -> AppState {
        let mut state = AppState::default();
        state.mode = TuiMode::Onboarding;
        let mut onboarding = Onboarding::new(false);
        onboarding.matrix_rain = Some(crate::components::onboarding::MatrixRain::new(0, 0));
        onboarding.step = OnboardingStep::Welcome;
        state.onboarding = Some(onboarding);
        state
    }

    fn setup_render_context(
        area: Rect,
        state: &AppState,
    ) -> (crate::tui::view_models::ViewModels, ThemeWrapper, ThemeColors, [Rect; 6]) {
        let palette = crate::components::CommandPalette::new();
        let wrap_cache = crate::components::message_list::render::WrapCache::new();
        let vms = crate::tui::view_models::ViewModels::from_app_state(state, &palette, wrap_cache);
        let theme = ThemeWrapper::default();
        let theme_colors = ThemeColors::from(&theme);
        let main_areas = RenderPipe::layout_main(area, true, 3);
        (vms, theme, theme_colors, main_areas)
    }

    fn assert_no_background_leakage(buf: &Buffer, area: Rect) {
        for y in 0..area.height {
            for x in 0..area.width {
                if let Some(cell) = buf.cell((x, y)) {
                    if is_leaked_cell(cell, y, x, area.height) {
                        panic!(
                            "Onboarding background should clear previous frame characters outside rendered elements"
                        );
                    }
                }
            }
        }
    }

    fn is_leaked_cell(cell: &ratatui::buffer::Cell, y: u16, x: u16, height: u16) -> bool {
        let is_top_bar = y < 3;
        let is_status_bar = y >= height - 3;
        let is_dialog = y >= 5 && y <= 19 && x >= 18 && x <= 62;
        !is_top_bar && !is_status_bar && !is_dialog && cell.symbol() != " "
    }
}
