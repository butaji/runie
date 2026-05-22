use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Layout, Rect},
    buffer::Buffer,
    style::Style,
    widgets::Widget,
};
use crossterm::{
    cursor::{SetCursorStyle, Show},
    event::{Event, KeyCode, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use std::io::{self, stdout};

use crate::{
    theme::ThemeWrapper,
    components::{
        MessageList,
        MessageItem,
        InputBar,
        Overlay,
        PermissionModal,
        PermissionAction,
        AgentStatus,
        AgentList,
        AgentItem,
        ContextPanel,
        GitChange,
        GitStatus,
        CommandPalette,
        DiffViewer,
        SessionTreeNavigator,
    },
};
use runie_agent::events::{AgentEvent, AgentMessage, ContentPart, PermissionDecision};

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
            show_status_bar: true,
        }
    }
}

const SIDEBAR_WIDTH: u16 = 28;

// Module declarations
pub mod state;
pub mod update;
pub mod render;
pub mod events;
pub mod tests;

pub use state::{AppState, TuiMode, Msg, Cmd, TuiAction, RenderState};
pub use update::update;
pub use events::event_to_msg;
use render::{render_top_bar, render_status_bar, render_agent_list};

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
    pub fn new(config: TuiConfig) -> io::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = stdout();
        stdout.execute(EnterAlternateScreen)?;
        stdout.execute(Show)?;
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
        self.terminal.backend_mut().execute(SetCursorStyle::DefaultUserShape)?;
        self.terminal.backend_mut().execute(LeaveAlternateScreen)?;
        Ok(())
    }

    /// Dispatch a msg to update state (unidirectional data flow)
    /// Returns Vec<Cmd> to be executed by the runtime
    pub fn update(&mut self, msg: Msg) -> Vec<Cmd> {
        self.log_action(&msg);
        update(&mut self.state, msg)
    }

    fn log_action(&mut self, msg: &Msg) {
        if self.action_log.len() >= self.action_log_capacity {
            self.action_log.remove(0);
        }
        self.action_log.push(msg.clone());
    }

    /// Calculate the height needed for the input bar based on its content
    fn input_bar_height(&self) -> u16 {
        // Each logical line = 1 visual line (no wrapping)
        let visual_lines = self.state.input_lines.len().max(1);
        // 2 for borders + visual lines for content
        (visual_lines as u16) + 2
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
        let show_status_bar = self.config.show_status_bar;

        // Extract render state - only clone what we need
        let theme = self.config.theme.clone();
        let render_state = RenderState::from(&self.state);
        let palette = self.command_palette.clone();

        self.terminal.draw(|frame| {
            let theme = &theme;
            Self::clear_background(frame, area, theme);
            let main_areas = Self::layout_main(padded_area, show_top_bar, show_status_bar, input_height);
            let state = &render_state;

            if show_top_bar {
                render_top_bar(state, main_areas[0], frame.buffer_mut(), theme);
            }
            Self::render_content(frame, state, show_sidebar, main_areas[1], theme);
            Self::render_input(frame, state, main_areas[2], theme);
            if show_status_bar {
                render_status_bar(state, main_areas[3], frame.buffer_mut(), theme);
            }
            Self::render_overlays(frame, state, &palette, padded_area, area, theme);
        })?;
        Ok(())
    }

    fn clear_background(frame: &mut ratatui::Frame, area: Rect, theme: &ThemeWrapper) {
        let bg_base: ratatui::style::Color = theme.color("bg.base").into();
        for y in 0..area.height {
            for x in 0..area.width {
                if let Some(cell) = frame.buffer_mut().cell_mut((x, y)) {
                    cell.set_style(Style::default().bg(bg_base));
                }
            }
        }
    }

    fn layout_main(padded: Rect, show_top: bool, show_status: bool, input_h: u16) -> [Rect; 4] {
        let constraints = [
            if show_top { Constraint::Length(1) } else { Constraint::Length(0) },
            Constraint::Min(1),
            Constraint::Length(input_h),
            if show_status { Constraint::Length(1) } else { Constraint::Length(0) },
        ];
        Layout::vertical(constraints).areas(padded)
    }

    fn render_content(frame: &mut ratatui::Frame, state: &RenderState, show_sidebar: bool, area: Rect, theme: &ThemeWrapper) {
        let mut h_constraints = vec![Constraint::Min(20)];
        if show_sidebar && area.width >= SIDEBAR_WIDTH + 20 {
            h_constraints.push(Constraint::Length(SIDEBAR_WIDTH));
        }
        let h_areas = Layout::horizontal(h_constraints.as_slice()).split(area);
        MessageList::render_ref(&state.messages, state.scroll.feed_offset, h_areas[0], frame.buffer_mut(), theme, &state.animation, state.agent_running);
        if show_sidebar && area.width >= SIDEBAR_WIDTH + 20 {
            render_agent_list(h_areas[1], frame.buffer_mut(), theme);
        }
    }

    fn render_input(frame: &mut ratatui::Frame, state: &RenderState, area: Rect, theme: &ThemeWrapper) {
        let input_bar = InputBar {
            prompt: "\u{276F} ".to_string(),
            lines: state.input_lines.clone(),
            cursor_line: state.cursor_row.min(state.input_lines.len().saturating_sub(1)),
            cursor_col: state.cursor_col,
            mode: crate::components::InputMode::Normal,
            right_info: state.input_right_info.clone(),
        };
        input_bar.render_ref(area, frame.buffer_mut(), theme);
        frame.set_cursor_position(input_bar.cursor_screen_pos(area));
    }

    fn render_overlays(frame: &mut ratatui::Frame, state: &RenderState, palette: &CommandPalette, padded: Rect, area: Rect, theme: &ThemeWrapper) {
        let mode = state.mode.clone();
        if mode == TuiMode::Permission && state.permission_modal.tool.is_some() {
            Self::render_permission_modal(frame, state, padded, area, theme);
        }
        if mode == TuiMode::CommandPalette {
            Self::render_command_palette(frame, state, padded, area, theme, palette);
        }
        if mode == TuiMode::Overlay {
            Self::render_overlay_mode(frame, area, theme);
        }
        if mode == TuiMode::DiffViewer {
            Self::render_diff_viewer(frame, state, area, theme);
        }
        if mode == TuiMode::SessionTree {
            Self::render_session_tree(frame, state, area, theme);
        }
    }

    fn render_permission_modal(frame: &mut ratatui::Frame, state: &RenderState, padded: Rect, area: Rect, theme: &ThemeWrapper) {
        Self::dim_background(frame, area, theme);
        let modal_area = Self::centered_rect(padded, 50, 12);
        Self::render_shadow(modal_area, frame.buffer_mut(), theme);
        let modal = PermissionModal::new(
            state.permission_modal.tool.as_deref().unwrap_or(""),
            state.permission_modal.args.as_deref().unwrap_or(""),
            state.permission_modal.desc.as_deref().unwrap_or(""),
        );
        modal.render_ref(modal_area, frame.buffer_mut(), theme);
    }

    fn render_command_palette(frame: &mut ratatui::Frame, state: &RenderState, padded: Rect, area: Rect, theme: &ThemeWrapper, palette: &CommandPalette) {
        Self::dim_background(frame, area, theme);
        let palette_area = Self::centered_rect(padded, 70, 20);
        Self::render_shadow(palette_area, frame.buffer_mut(), theme);
        palette.render_ref(palette_area, frame.buffer_mut(), theme);
    }

    fn render_overlay_mode(frame: &mut ratatui::Frame, area: Rect, theme: &ThemeWrapper) {
        let overlay_area = Overlay::centered((60, 20), frame.area());
        Self::render_shadow(overlay_area, frame.buffer_mut(), theme);
        let mut overlay_buf = Buffer::empty(overlay_area);
        Overlay::default().render_ref(overlay_area, &mut overlay_buf, theme);
        Self::blit_buffer(frame, area, overlay_area, &overlay_buf);
    }

    fn render_diff_viewer(frame: &mut ratatui::Frame, state: &RenderState, area: Rect, theme: &ThemeWrapper) {
        Self::dim_background(frame, area, theme);
        let diff_area = Self::centered_rect(area, 80, 25);
        Self::render_shadow(diff_area, frame.buffer_mut(), theme);
        if let Some(ref diff) = state.diff_viewer {
            diff.render_ref(diff_area, frame.buffer_mut(), theme);
        }
    }

    fn render_session_tree(frame: &mut ratatui::Frame, state: &RenderState, area: Rect, theme: &ThemeWrapper) {
        Self::dim_background(frame, area, theme);
        let tree_area = Self::centered_rect(area, 70, 25);
        Self::render_shadow(tree_area, frame.buffer_mut(), theme);
        state.session_tree.render_ref(tree_area, frame.buffer_mut(), theme);
    }

    fn dim_background(frame: &mut ratatui::Frame, area: Rect, theme: &ThemeWrapper) {
        let bg_base: ratatui::style::Color = theme.color("bg.base").into();
        for y in 0..area.height {
            for x in 0..area.width {
                if let Some(cell) = frame.buffer_mut().cell_mut((x, y)) {
                    cell.set_style(Style::default().bg(bg_base));
                }
            }
        }
    }

    fn centered_rect(padded: Rect, w: u16, h: u16) -> Rect {
        let x = padded.x + (padded.width.saturating_sub(w)) / 2;
        let y = padded.y + (padded.height.saturating_sub(h)) / 2;
        Rect::new(x, y, w, h)
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

    pub fn handle_event(&mut self, event: Event) -> Option<TuiAction> {
        match event {
            Event::Key(key) => self.handle_key(key),
            Event::Resize(_, _) => None,
            _ => None,
        }
    }

    fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> Option<TuiAction> {
        // Convert key event to Msg and apply update
        if let Some(msg) = events::key_to_msg(key, &self.state) {
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
                    Cmd::SaveSession { .. } | Cmd::LoadSession { .. } | Cmd::SlashCommand(_) => {
                        // These are handled by the CLI runtime, not the TUI
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

    fn render_shadow(area: Rect, buf: &mut ratatui::buffer::Buffer, theme: &ThemeWrapper) {
        let bg: ratatui::style::Color = theme.color("bg.base").into();
        let fg: ratatui::style::Color = theme.color("text.dim").into();
        Self::draw_v_shadow(area, buf, fg, bg);
        Self::draw_h_shadow(area, buf, fg, bg);
        Self::draw_corner_shadow(area, buf, fg, bg);
    }

    fn draw_v_shadow(area: Rect, buf: &mut ratatui::buffer::Buffer, fg: ratatui::style::Color, bg: ratatui::style::Color) {
        let x = area.x + area.width;
        if x >= buf.area.width { return; }
        for y in area.y + 1..area.y + area.height + 1 {
            if y < buf.area.height {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_char('░');
                    cell.set_style(Style::default().fg(fg).bg(bg));
                }
            }
        }
    }

    fn draw_h_shadow(area: Rect, buf: &mut ratatui::buffer::Buffer, fg: ratatui::style::Color, bg: ratatui::style::Color) {
        let y = area.y + area.height;
        if y >= buf.area.height { return; }
        for x in area.x + 1..area.x + area.width + 1 {
            if x < buf.area.width {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_char('░');
                    cell.set_style(Style::default().fg(fg).bg(bg));
                }
            }
        }
    }

    fn draw_corner_shadow(area: Rect, buf: &mut ratatui::buffer::Buffer, fg: ratatui::style::Color, bg: ratatui::style::Color) {
        let x = area.x + area.width;
        let y = area.y + area.height;
        if x < buf.area.width && y < buf.area.height {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_char('▒');
                cell.set_style(Style::default().fg(fg).bg(bg));
            }
        }
    }
}

