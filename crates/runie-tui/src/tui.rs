use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Layout, Rect},
    buffer::Buffer,
    style::Style,
    widgets::Widget,
};
use crossterm::{
    cursor::{SetCursorStyle, Show, Hide},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use std::collections::VecDeque;
use std::io::{self, stdout};

use crate::{
    theme::{ThemeWrapper, ThemeColors},
    components::{
        Overlay,
        PermissionModal,
        CommandPalette,
        component::InputBar,
    },
};
use crate::components::component::Component;
use crate::components::message_list::render::WrapCache;
use crate::tui::view_models::ViewModels;
use runie_agent::events::AgentEvent;

pub struct TuiConfig {
    pub theme: ThemeWrapper,
    pub show_top_bar: bool,
    pub show_status_bar: bool,
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            theme: ThemeWrapper::default(),
            show_top_bar: true,
            show_status_bar: true, // Always visible - hotkeys are context-aware and essential
        }
    }
}

const SIDEBAR_WIDTH: u16 = 28;

// Module declarations
pub mod state;
pub mod update;
pub mod render;
pub mod events;
pub mod view_models;
#[cfg(test)]
pub mod tests;
#[cfg(test)]
pub mod tests_hotkeys;
#[cfg(test)]
pub mod tests_statusbar;
#[cfg(test)]
pub mod tests_onboarding;
// tests_input module was deleted - intentionally commented out
// #[cfg(test)]
// pub mod tests_input; // intentionally commented out - module was deleted
// #[cfg(test)]
// pub mod tests_input;

pub use state::{AppState, TuiMode, Msg, Cmd, RenderState, Onboarding, OnboardingStep};
pub use update::update;
pub use events::event_to_msg;


pub struct Tui {
    pub config: TuiConfig,
    pub terminal: Terminal<CrosstermBackend<io::Stdout>>,
    pub state: AppState,
    command_palette: CommandPalette,
    action_log: VecDeque<Msg>,
    action_log_capacity: usize,
    /// Cache for text wrapping to avoid recomputing every frame
    wrap_cache: WrapCache,
}

impl Tui {
    /// Install a panic hook that restores the terminal before printing the panic.
    /// Uses std::sync::Once to ensure it only runs once even if Tui::new is called multiple times.
    pub fn install_panic_hook() {
        use std::sync::Once;
        static INIT: Once = Once::new();
        INIT.call_once(|| {
            let original_hook = std::panic::take_hook();
            std::panic::set_hook(Box::new(move |info| {
                // Best-effort terminal cleanup — ignore errors since we're already panicking
                let _ = disable_raw_mode();
                let _ = stdout().execute(LeaveAlternateScreen);
                let _ = stdout().execute(Hide);
                let _ = stdout().execute(SetCursorStyle::DefaultUserShape);
                // Now run the default hook which prints the panic + backtrace
                original_hook(info);
            }));
        });
    }

    pub fn new(config: TuiConfig) -> io::Result<Self> {
        Self::install_panic_hook();
        enable_raw_mode()?;
        let mut stdout = stdout();
        stdout.execute(EnterAlternateScreen)?;
        stdout.execute(Hide)?;
        stdout.execute(SetCursorStyle::SteadyBar)?;

        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        Ok(Self {
            config,
            terminal,
            state: AppState::default(),
            command_palette: CommandPalette::new(),
            action_log: VecDeque::new(),
            action_log_capacity: 1000,
            wrap_cache: WrapCache::new(),
        })
    }

    pub fn cleanup(&mut self) -> io::Result<()> {
        disable_raw_mode()?;
        self.terminal.backend_mut().execute(Show)?;
        self.terminal.backend_mut().execute(SetCursorStyle::DefaultUserShape)?;
        self.terminal.backend_mut().execute(LeaveAlternateScreen)?;
        Ok(())
    }

    /// Dispatch a msg to update state (unidirectional data flow)
    /// Returns Vec<Cmd> to be executed by the runtime
    pub fn update(&mut self, msg: Msg) -> Vec<Cmd> {
        self.log_action(&msg);
        update(&mut self.state, &mut self.command_palette, msg)
    }

    fn log_action(&mut self, msg: &Msg) {
        if self.action_log.len() >= self.action_log_capacity {
            self.action_log.pop_front();
        }
        self.action_log.push_back(msg.clone());
    }

    /// Calculate the height needed for the input bar based on its content
    fn input_bar_height(&self) -> u16 {
        crate::components::input_bar::input_bar_height(&self.state.textarea)
    }

    /// Render to terminal. Terminal I/O happens here.
    pub fn render(&mut self) -> io::Result<()> {
        let input_height = self.input_bar_height();
        let show_sidebar = self.state.show_sidebar;
        let show_top_bar = self.config.show_top_bar;
        let show_status_bar = true;
        let theme = self.config.theme.clone();
        let theme_colors = ThemeColors::from(&self.config.theme);
        let render_state = RenderState::from(&self.state);
        let palette = self.command_palette.clone();
        let view_models = ViewModels::from_render_state(&render_state, &palette, self.wrap_cache.clone());
        let is_onboarding = matches!(render_state.mode, TuiMode::Onboarding);

        self.terminal.draw(|frame| {
            let area = frame.area();
            let padded_area = Rect {
                x: area.x + 2,
                y: area.y + 1,
                width: area.width.saturating_sub(4),
                height: area.height.saturating_sub(2),
            };
            let main_areas = Self::layout_main(padded_area, show_top_bar, show_status_bar, input_height);

            if is_onboarding {
                Self::render_onboarding_mode(frame.buffer_mut(), area, &render_state, &view_models, main_areas, show_status_bar, &theme, &theme_colors);
            } else {
                Self::render_normal_mode(frame.buffer_mut(), area, &render_state, &view_models, main_areas, show_sidebar, show_top_bar, show_status_bar, &palette, padded_area, &theme, &theme_colors);
            }
        })?;
        Ok(())
    }

    fn render_onboarding_mode(buf: &mut Buffer, area: Rect, _state: &RenderState, vms: &ViewModels, main_areas: [Rect; 4], show_status_bar: bool, theme: &ThemeWrapper, _theme_colors: &ThemeColors) {
        // MatrixBg fills its own background; skip clear_background to avoid overdraw
        if show_status_bar {
            Component::render(&vms.status_bar, &vms.status_bar, main_areas[3], buf, theme);
        }
        if let Some(ref onboarding) = _state.onboarding {
            let onboarding_area = Rect {
                x: area.x,
                y: area.y,
                width: area.width,
                height: area.height - if show_status_bar { 2 } else { 0 },
            };
            // Use render_onboarding_screen for Welcome step (animated matrix rain + ASCII art)
            if matches!(onboarding.step, OnboardingStep::Welcome) {
                use crate::components::onboarding::{render_onboarding_screen, MatrixRain};
                let accent = theme.color("accent.primary").into();
                let bg_base = theme.color("bg.base").into();
                let default_rain = MatrixRain::new(onboarding_area.width, onboarding_area.height);
                let rain = onboarding.matrix_rain.as_ref().unwrap_or(&default_rain);
                render_onboarding_screen(rain, buf, onboarding_area, accent, bg_base);
            } else {
                // Render step-specific UI for other onboarding steps
                Component::render(onboarding, &(), onboarding_area, buf, theme);
            }
        }
    }

    fn render_normal_mode(buf: &mut Buffer, area: Rect, state: &RenderState, vms: &ViewModels, main_areas: [Rect; 4], show_sidebar: bool, show_top_bar: bool, show_status_bar: bool, palette: &CommandPalette, padded: Rect, theme: &ThemeWrapper, theme_colors: &ThemeColors) {
        Self::clear_background(buf, area, theme_colors.bg_base);
        if show_top_bar {
            Component::render(&vms.top_bar, &vms.top_bar, main_areas[0], buf, theme);
        }
        Self::render_content(buf, vms, show_sidebar, main_areas[1], theme);
        Self::render_input(buf, state, main_areas[2], theme);
        if show_status_bar {
            Component::render(&vms.status_bar, &vms.status_bar, main_areas[3], buf, theme);
        }
        Self::render_overlays(buf, state, palette, padded, area, theme, theme_colors);
    }

    fn layout_main(padded: Rect, show_top: bool, show_status: bool, input_h: u16) -> [Rect; 4] {
        let constraints = [
            if show_top { Constraint::Length(2) } else { Constraint::Length(0) },
            Constraint::Min(1),
            Constraint::Length(input_h),
            if show_status { Constraint::Length(1) } else { Constraint::Length(0) },
        ];
        Layout::vertical(constraints).areas(padded)
    }

    fn render_content(buf: &mut Buffer, vms: &ViewModels, show_sidebar: bool, area: Rect, theme: &ThemeWrapper) {
        let mut h_constraints = vec![Constraint::Min(20)];
        if show_sidebar && area.width >= SIDEBAR_WIDTH + 20 {
            h_constraints.push(Constraint::Length(SIDEBAR_WIDTH));
        }
        let h_areas = Layout::horizontal(h_constraints.as_slice()).split(area);
        Component::render(&vms.message_list, &vms.message_list, h_areas[0], buf, theme);
        if show_sidebar && area.width >= SIDEBAR_WIDTH + 20 {
            Component::render(&vms.agent_list, &vms.agent_list, h_areas[1], buf, theme);
        }
    }

    fn render_input(buf: &mut Buffer, state: &RenderState, area: Rect, theme: &ThemeWrapper) {
        let mut textarea = state.textarea.clone();
        let accent_color = theme.color("accent.primary").into();
        let text_primary = theme.color("text.primary").into();
        textarea.set_style(Style::default().fg(text_primary));
        textarea.set_cursor_style(Style::default().fg(accent_color).bg(accent_color));
        textarea.set_cursor_line_style(Style::default().remove_modifier(ratatui::style::Modifier::UNDERLINED));
        let vm = crate::tui::view_models::InputBarViewModel {
            textarea,
            prompt: "\u{276F} ".to_string(),
            right_info: state.input_right_info.clone(),
        };
        Component::render(&InputBar, &vm, area, buf, theme);
    }

    fn render_overlays(buf: &mut Buffer, state: &RenderState, palette: &CommandPalette, padded: Rect, area: Rect, theme: &ThemeWrapper, theme_colors: &ThemeColors) {
        let mode = state.mode.clone();
        if mode == TuiMode::Permission && state.permission_modal.tool.is_some() {
            Self::render_permission_modal(buf, state, padded, area, theme, theme_colors);
        }
        if mode == TuiMode::CommandPalette {
            Self::render_command_palette(buf, state, padded, area, theme, palette, theme_colors);
        }
        if mode == TuiMode::Overlay {
            Self::render_overlay_mode(buf, state, area, theme);
        }
        if mode == TuiMode::DiffViewer {
            Self::render_diff_viewer(buf, state, area, theme, theme_colors);
        }
        if mode == TuiMode::SessionTree {
            Self::render_session_tree(buf, state, area, theme, theme_colors);
        }
    }

    fn render_permission_modal(buf: &mut Buffer, state: &RenderState, padded: Rect, area: Rect, theme: &ThemeWrapper, theme_colors: &ThemeColors) {
        Self::dim_background(buf, area, theme_colors);
        let modal_area = Self::centered_rect(padded, 50, 14);
        let mut modal = PermissionModal::new(
            state.permission_modal.tool.as_deref().unwrap_or(""),
            state.permission_modal.args.as_deref().unwrap_or(""),
            state.permission_modal.desc.as_deref().unwrap_or(""),
        );
        const TIMEOUT_SECS: u64 = 300;
        modal.timeout_secs = state.permission_modal.timeout_start.map(|start| {
            let elapsed = start.elapsed().as_secs();
            TIMEOUT_SECS.saturating_sub(elapsed)
        });
        Component::render(&modal, &(), modal_area, buf, theme);
    }

    fn render_command_palette(buf: &mut Buffer, _state: &RenderState, padded: Rect, area: Rect, theme: &ThemeWrapper, palette: &CommandPalette, theme_colors: &ThemeColors) {
        Self::dim_background(buf, area, theme_colors);
        let palette_area = Self::centered_rect(padded, 70, 20);
        Component::render(palette, &(), palette_area, buf, theme);
    }

    fn render_overlay_mode(buf: &mut Buffer, state: &RenderState, area: Rect, theme: &ThemeWrapper) {
        let overlay_area = Overlay::centered((70, 25), area);
        let mut overlay_buf = Buffer::empty(overlay_area);

        if let Some(ref picker) = state.model_picker {
            Component::render(picker, &(), overlay_area, &mut overlay_buf, theme);
        }

        Self::blit_buffer(buf, area, overlay_area, &overlay_buf);
    }

    fn render_diff_viewer(buf: &mut Buffer, state: &RenderState, area: Rect, theme: &ThemeWrapper, theme_colors: &ThemeColors) {
        Self::dim_background(buf, area, theme_colors);
        let diff_area = Self::centered_rect(area, 80, 25);
        if let Some(ref diff) = state.diff_viewer {
            Component::render(diff, &(), diff_area, buf, theme);
        }
    }

    fn render_session_tree(buf: &mut Buffer, state: &RenderState, area: Rect, theme: &ThemeWrapper, theme_colors: &ThemeColors) {
        Self::dim_background(buf, area, theme_colors);
        let tree_area = Self::centered_rect(area, 70, 25);
        Component::render(&state.session_tree, &(), tree_area, buf, theme);
    }

    fn clear_background(buf: &mut Buffer, area: Rect, bg_color: ratatui::style::Color) {
        ratatui::widgets::Paragraph::new("")
            .style(Style::default().bg(bg_color))
            .render(area, buf);
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

    fn centered_rect(padded: Rect, w: u16, h: u16) -> Rect {
        let x = padded.x + (padded.width.saturating_sub(w)) / 2;
        let y = padded.y + (padded.height.saturating_sub(h)) / 2;
        Rect::new(x, y, w.min(padded.width), h.min(padded.height))
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

    pub fn on_agent_event(&mut self, event: AgentEvent) -> Vec<Cmd> {
        // Use the update() reducer for all agent events
        self.update(Msg::AgentEvent(event))
    }

    pub fn is_permission_modal_active(&self) -> bool {
        self.state.permission_modal.tool.is_some() && self.state.mode == TuiMode::Permission
    }

    pub fn toggle_sidebar(&mut self) {
        self.update(Msg::ToggleSidebar);
    }
}

/// Convert crossterm KeyEvent to ratatui-textarea Input.
/// Manual conversion needed because project crossterm (0.28) differs
/// from ratatui-textarea's internal crossterm (0.29) via ratatui-crossterm.
pub fn key_to_textarea_input(key: crossterm::event::KeyEvent) -> ratatui_textarea::Input {
    use crossterm::event::KeyCode;
    use ratatui_textarea::{Input, Key};

    let key_code = match key.code {
        KeyCode::Char(c) => Key::Char(c),
        KeyCode::Backspace => Key::Backspace,
        KeyCode::Enter => Key::Enter,
        KeyCode::Left => Key::Left,
        KeyCode::Right => Key::Right,
        KeyCode::Up => Key::Up,
        KeyCode::Down => Key::Down,
        KeyCode::Home => Key::Home,
        KeyCode::End => Key::End,
        KeyCode::Delete => Key::Delete,
        KeyCode::Tab => Key::Tab,
        KeyCode::Esc => Key::Esc,
        KeyCode::PageUp => Key::PageUp,
        KeyCode::PageDown => Key::PageDown,
        KeyCode::F(n) => Key::F(n),
        KeyCode::Null => Key::Null,
        _ => Key::Null,
    };

    let ctrl = key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL);
    let alt = key.modifiers.contains(crossterm::event::KeyModifiers::ALT);
    let shift = key.modifiers.contains(crossterm::event::KeyModifiers::SHIFT);

    Input { key: key_code, ctrl, alt, shift }
}
