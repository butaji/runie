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
    event::Event,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use std::io::{self, stdout};

use crate::{
    theme::{ThemeWrapper, ThemeColors},
    components::{
        MessageList,
        Overlay,
        PermissionModal,
        PermissionAction,
        CommandPalette,
    },
};
use crate::components::onboarding::render::render_onboarding;
use crate::components::render_top_bar;
use crate::tui::view_models::ViewModels;
use runie_agent::events::{AgentEvent, PermissionDecision};

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

pub use state::{AppState, TuiMode, Msg, Cmd, TuiAction, RenderState, Onboarding};
pub use update::update;
pub use events::event_to_msg;
use render::{render_status_bar, render_agent_list};


pub struct Tui {
    pub config: TuiConfig,
    pub terminal: Terminal<CrosstermBackend<io::Stdout>>,
    pub state: AppState,
    command_palette: CommandPalette,
    dirty: bool,
    action_log: Vec<Msg>,
    action_log_capacity: usize,
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
            dirty: true,
            action_log: Vec::new(),
            action_log_capacity: 1000,
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
        self.dirty = true;
        update(&mut self.state, &mut self.command_palette, msg)
    }

    fn log_action(&mut self, msg: &Msg) {
        if self.action_log.len() >= self.action_log_capacity {
            self.action_log.remove(0);
        }
        self.action_log.push(msg.clone());
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Calculate the height needed for the input bar based on its content
    fn input_bar_height(&self) -> u16 {
        crate::components::input_bar::input_bar_height(&self.state.textarea)
    }

    pub fn render(&mut self) -> io::Result<()> {
        if !self.dirty {
            return Ok(());
        }
        self.dirty = false;

        let size = self.terminal.size()?;
        let area = Rect::new(0, 0, size.width, size.height);
        let padded_area = Rect {
            x: area.x + 2, y: area.y + 1,
            width: area.width.saturating_sub(4),
            height: area.height.saturating_sub(2),
        };
        let input_height = self.input_bar_height();
        let show_sidebar = self.state.show_sidebar;
        let show_top_bar = self.config.show_top_bar;
        let show_status_bar = true; // Always show hotkeys bar per UX requirement

        // Extract render state - only clone what we need
        let theme = self.config.theme.clone();
        let theme_colors = ThemeColors::from(&self.config.theme);
        let render_state = RenderState::from(&self.state);
        let palette = self.command_palette.clone();
        
        // Build all ViewModels from render state (Elm/TEA: Model -> ViewModel)
        let view_models = ViewModels::from_render_state(&render_state, &palette);

        self.terminal.draw(|frame| {
            let theme = &theme;
            let theme_colors = &theme_colors;
            let main_areas = Self::layout_main(padded_area, show_top_bar, show_status_bar, input_height);
            let state = &render_state;
            let vms = &view_models;
            let is_onboarding = matches!(state.mode, TuiMode::Onboarding);

            if is_onboarding {
                Self::render_onboarding_mode(frame, area, state, vms, main_areas, show_status_bar, theme, theme_colors);
            } else {
                Self::render_normal_mode(frame, area, state, vms, main_areas, show_sidebar, show_top_bar, show_status_bar, &palette, padded_area, theme, theme_colors);
            }
        })?;
        Ok(())
    }

    fn clear_background(frame: &mut ratatui::Frame, area: Rect, bg_color: ratatui::style::Color) {
        ratatui::widgets::Paragraph::new("")
            .style(Style::default().bg(bg_color))
            .render(area, frame.buffer_mut());
    }

    fn render_onboarding_mode(frame: &mut ratatui::Frame, area: Rect, _state: &RenderState, vms: &ViewModels, main_areas: [Rect; 4], show_status_bar: bool, theme: &ThemeWrapper, theme_colors: &ThemeColors) {
        Self::clear_background(frame, area, theme_colors.bg_base);
        if show_status_bar {
            render_status_bar(&vms.status_bar, main_areas[3], frame.buffer_mut(), theme_colors);
        }
        if let Some(ref onboarding) = _state.onboarding {
            let onboarding_area = Rect {
                x: area.x,
                y: area.y,
                width: area.width,
                height: area.height - if show_status_bar { 1 } else { 0 },
            };
            render_onboarding(onboarding, onboarding_area, frame.buffer_mut(), theme);
        }
    }

    fn render_normal_mode(frame: &mut ratatui::Frame, area: Rect, state: &RenderState, vms: &ViewModels, main_areas: [Rect; 4], show_sidebar: bool, show_top_bar: bool, show_status_bar: bool, palette: &CommandPalette, padded: Rect, theme: &ThemeWrapper, theme_colors: &ThemeColors) {
        Self::clear_background(frame, area, theme_colors.bg_base);
        if show_top_bar {
            render_top_bar(&vms.top_bar, main_areas[0], frame.buffer_mut(), theme_colors);
        }
        Self::render_content(frame, state, vms, show_sidebar, main_areas[1], theme, theme_colors);
        Self::render_input(frame, state, main_areas[2], theme);
        if show_status_bar {
            render_status_bar(&vms.status_bar, main_areas[3], frame.buffer_mut(), theme_colors);
        }
        Self::render_overlays(frame, state, palette, padded, area, theme, theme_colors);
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

    fn render_content(frame: &mut ratatui::Frame, _state: &RenderState, vms: &ViewModels, show_sidebar: bool, area: Rect, theme: &ThemeWrapper, theme_colors: &ThemeColors) {
        let mut h_constraints = vec![Constraint::Min(20)];
        if show_sidebar && area.width >= SIDEBAR_WIDTH + 20 {
            h_constraints.push(Constraint::Length(SIDEBAR_WIDTH));
        }
        let h_areas = Layout::horizontal(h_constraints.as_slice()).split(area);
        MessageList::render_ref(&vms.message_list, h_areas[0], frame.buffer_mut(), theme);
        if show_sidebar && area.width >= SIDEBAR_WIDTH + 20 {
            render_agent_list(&vms.agent_list, h_areas[1], frame.buffer_mut(), theme_colors);
        }
    }

    fn render_input(frame: &mut ratatui::Frame, state: &RenderState, area: Rect, theme: &ThemeWrapper) {
        let mut textarea = state.textarea.clone();
        let accent_color = theme.color("accent.primary").into();
        let text_primary = theme.color("text.primary").into();
        textarea.set_style(Style::default().fg(text_primary));
        textarea.set_cursor_style(Style::default().fg(accent_color).bg(accent_color));
        textarea.set_cursor_line_style(Style::default().remove_modifier(ratatui::style::Modifier::UNDERLINED));
        crate::components::input_bar::render_input_bar(
            &textarea,
            "\u{276F} ",
            &state.input_right_info,
            area,
            frame.buffer_mut(),
            theme,
        );
        // Note: TextArea widget renders its own cursor; no need for frame.set_cursor_position()
    }

    fn render_overlays(frame: &mut ratatui::Frame, state: &RenderState, palette: &CommandPalette, padded: Rect, area: Rect, theme: &ThemeWrapper, theme_colors: &ThemeColors) {
        let mode = state.mode.clone();
        if mode == TuiMode::Permission && state.permission_modal.tool.is_some() {
            Self::render_permission_modal(frame, state, padded, area, theme, theme_colors);
        }
        if mode == TuiMode::CommandPalette {
            Self::render_command_palette(frame, state, padded, area, theme, palette, theme_colors);
        }
        if mode == TuiMode::Overlay {
            Self::render_overlay_mode(frame, area, theme);
        }
        if mode == TuiMode::DiffViewer {
            Self::render_diff_viewer(frame, state, area, theme, theme_colors);
        }
        if mode == TuiMode::SessionTree {
            Self::render_session_tree(frame, state, area, theme, theme_colors);
        }
    }

    fn render_permission_modal(frame: &mut ratatui::Frame, state: &RenderState, padded: Rect, area: Rect, theme: &ThemeWrapper, theme_colors: &ThemeColors) {
        Self::dim_background(frame, area, theme_colors);
        let modal_area = Self::centered_rect(padded, 50, 12);
        let modal = PermissionModal::new(
            state.permission_modal.tool.as_deref().unwrap_or(""),
            state.permission_modal.args.as_deref().unwrap_or(""),
            state.permission_modal.desc.as_deref().unwrap_or(""),
        );
        modal.render_ref(modal_area, frame.buffer_mut(), theme);
    }

    fn render_command_palette(frame: &mut ratatui::Frame, _state: &RenderState, padded: Rect, area: Rect, theme: &ThemeWrapper, palette: &CommandPalette, theme_colors: &ThemeColors) {
        Self::dim_background(frame, area, theme_colors);
        let palette_area = Self::centered_rect(padded, 70, 20);
        palette.render_ref(palette_area, frame.buffer_mut(), theme);
    }

    fn render_overlay_mode(frame: &mut ratatui::Frame, area: Rect, theme: &ThemeWrapper) {
        let overlay_area = Overlay::centered((60, 20), frame.area());
        let mut overlay_buf = Buffer::empty(overlay_area);
        Overlay::default().render_ref(overlay_area, &mut overlay_buf, theme);
        Self::blit_buffer(frame, area, overlay_area, &overlay_buf);
    }

    fn render_diff_viewer(frame: &mut ratatui::Frame, state: &RenderState, area: Rect, theme: &ThemeWrapper, theme_colors: &ThemeColors) {
        Self::dim_background(frame, area, theme_colors);
        let diff_area = Self::centered_rect(area, 80, 25);
        if let Some(ref diff) = state.diff_viewer {
            diff.render_ref(diff_area, frame.buffer_mut(), theme);
        }
    }

    fn render_session_tree(frame: &mut ratatui::Frame, state: &RenderState, area: Rect, theme: &ThemeWrapper, theme_colors: &ThemeColors) {
        Self::dim_background(frame, area, theme_colors);
        let tree_area = Self::centered_rect(area, 70, 25);
        state.session_tree.render_ref(tree_area, frame.buffer_mut(), theme);
    }

    fn dim_background(frame: &mut ratatui::Frame, area: Rect, theme_colors: &ThemeColors) {
        // Dim by darkening the base background color
        let dim_color = match theme_colors.bg_base {
            ratatui::style::Color::Rgb(r, g, b) => {
                ratatui::style::Color::Rgb(
                    (r as f32 * 0.5).round() as u8,
                    (g as f32 * 0.5).round() as u8,
                    (b as f32 * 0.5).round() as u8,
                )
            }
            ratatui::style::Color::Indexed(idx) => {
                // For indexed colors, darken by using a darker shade
                ratatui::style::Color::Indexed(idx.saturating_sub(8))
            }
            _ => ratatui::style::Color::Black,
        };
        ratatui::widgets::Paragraph::new("")
            .style(Style::default().bg(dim_color))
            .render(area, frame.buffer_mut());
    }

    fn centered_rect(padded: Rect, w: u16, h: u16) -> Rect {
        let x = padded.x + (padded.width.saturating_sub(w)) / 2;
        let y = padded.y + (padded.height.saturating_sub(h)) / 2;
        Rect::new(x, y, w.min(padded.width), h.min(padded.height))
    }

    fn blit_buffer(frame: &mut ratatui::Frame, area: Rect, src_area: Rect, src: &Buffer) {
        for y in 0..src.area.height {
            for x in 0..src.area.width {
                let cell = src.cell((x, y));
                let tx = src_area.x + x;
                let ty = src_area.y + y;
                if tx < area.width && ty < area.height {
                    if let (Some(src_cell), Some(target)) = (cell, frame.buffer_mut().cell_mut((tx, ty))) {
                        target.set_style(src_cell.style());
                        if let Some(ch) = src_cell.symbol().chars().next() {
                            target.set_char(ch);
                        }
                    }
                }
            }
        }
    }

    /// Handle an event.
    /// Returns TuiAction to be processed by caller, or None for no action.
    /// Note: handle_event handles TextareaKey internally via handle_key.
    /// For the tui_run.rs event loop which calls update() directly, TextareaKey
    /// is handled in update() itself.
    pub fn handle_event(&mut self, event: Event) -> Option<TuiAction> {
        match event {
            Event::Key(key) => self.handle_key(key),
            _ => None,
        }
    }

    fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> Option<TuiAction> {
        // Check if key should go to textarea (most keys)
        if let Some(msg) = events::key_to_msg(key, &self.state) {
            if matches!(msg, Msg::TextareaKey(_)) {
                // Convert crossterm KeyEvent to ratatui-textarea Input.
                // Manual conversion needed because project crossterm (0.28)
                // differs from ratatui-textarea's crossterm (0.29).
                let input = key_to_textarea_input(key);
                self.state.textarea.input(input);
                self.dirty = true;
                return None;
            }

            // For non-textarea keys, go through update
            let cmds = self.update(msg);
            // Process cmds to determine TuiAction
            for cmd in cmds {
                match cmd {
                    Cmd::SpawnAgent { .. } => {
                        // Would return Submit action - but text already captured
                    }
                    Cmd::SendPermission { decision } => {
                        let permission_action = match decision {
                            PermissionDecision::Allow { .. } => PermissionAction::Confirm,
                            PermissionDecision::Deny { .. } => PermissionAction::Cancel,
                            PermissionDecision::AllowAlways { .. } => PermissionAction::Always,
                            PermissionDecision::Skip { .. } => PermissionAction::Skip,
                        };
                        return Some(TuiAction::ToolPermission {
                            tool: self.state.permission_modal.tool.clone().unwrap_or_default(),
                            action: permission_action,
                        });
                    }
                    Cmd::SaveSession { .. } | Cmd::LoadSession { .. } | Cmd::SlashCommand(_) | Cmd::SaveSettings { .. } | Cmd::FetchModels { .. } => {
                        // These are handled by the CLI runtime, not the TUI
                    }
                    // P1-4 FIX: Rollback is handled by the runtime/tool executor
                    Cmd::Rollback { .. } => {}
                    // P0-1 FIX: Interrupt cancels the agent task
                    Cmd::Interrupt => {
                        return Some(TuiAction::Cancel);
                    }
                }
            }
            // Check if we should quit
            if !self.state.running {
                return Some(TuiAction::Quit);
            }
        }
        None
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

