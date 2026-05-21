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

pub use state::{AppState, TuiMode, Msg, Cmd, TuiAction};
pub use update::update;
pub use events::event_to_msg;
use render::{render_top_bar, render_status_bar, render_agent_list};

pub struct Tui {
    pub config: TuiConfig,
    pub terminal: Terminal<CrosstermBackend<io::Stdout>>,
    pub state: AppState,
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
        update(&mut self.state, msg)
    }

    /// Calculate the height needed for the input bar based on its content
    fn input_bar_height(&self) -> u16 {
        // Each logical line = 1 visual line (no wrapping)
        let visual_lines = self.state.input_lines.len().max(1);
        // 2 for borders + visual lines for content
        (visual_lines as u16) + 2
    }

    pub fn render(&mut self) -> io::Result<()> {
        let size = self.terminal.size()?;
        let area = Rect::new(0, 0, size.width, size.height);

        let padded_area = Rect {
            x: area.x + 2,
            y: area.y + 1,
            width: area.width.saturating_sub(4),
            height: area.height.saturating_sub(2),
        };

        // Calculate dynamic input bar height
        let input_height = self.input_bar_height();

        // Extract values needed in closure to avoid borrow conflicts
        let show_sidebar = self.state.show_sidebar;
        let show_top_bar = self.config.show_top_bar;
        let show_status_bar = self.config.show_status_bar;
        let mode = self.state.mode.clone();
        let state_clone = self.state.clone();

        self.terminal.draw(|frame| {
            let theme = &self.config.theme;

            // Clear entire frame with bg.base
            let bg_base: ratatui::style::Color = theme.color("bg.base").into();
            for y in 0..area.height {
                for x in 0..area.width {
                    frame.buffer_mut().get_mut(x, y).set_style(Style::default().bg(bg_base));
                }
            }

            // Main vertical layout: TopBar | ContentArea | InputBar | StatusBar
            let main_constraints = [
                if show_top_bar { Constraint::Length(1) } else { Constraint::Length(0) },
                Constraint::Min(1),  // Content area (will be split horizontally)
                Constraint::Length(input_height),
                if show_status_bar { Constraint::Length(1) } else { Constraint::Length(0) },
            ];
            let main_areas: [Rect; 4] = Layout::vertical(main_constraints).areas(padded_area);

            // Render top bar using standalone function
            if show_top_bar {
                render_top_bar(&state_clone, main_areas[0], frame.buffer_mut(), theme);
            }

            // Split content area horizontally
            let content_area = main_areas[1];
            let mut h_constraints = vec![];
            h_constraints.push(Constraint::Min(20));
            if show_sidebar && content_area.width >= SIDEBAR_WIDTH + 20 {
                h_constraints.push(Constraint::Length(SIDEBAR_WIDTH));
            }
            let h_areas = Layout::horizontal(h_constraints.as_slice()).split(content_area);

            // Render message list directly from state (no widget instance needed)
            if show_sidebar && content_area.width >= SIDEBAR_WIDTH + 20 {
                MessageList::render_ref(&state_clone.messages, state_clone.feed_scroll_offset, h_areas[0], frame.buffer_mut(), theme);
                render_agent_list(h_areas[1], frame.buffer_mut(), theme);
            } else {
                MessageList::render_ref(&state_clone.messages, state_clone.feed_scroll_offset, h_areas[0], frame.buffer_mut(), theme);
            }

            // Render input bar using standalone function
            let input_bar = InputBar {
                prompt: "\u{276F} ".to_string(),
                lines: state_clone.input_lines.clone(),
                cursor_line: state_clone.cursor_row.min(state_clone.input_lines.len().saturating_sub(1)),
                cursor_col: state_clone.cursor_col,
                mode: crate::components::InputMode::Normal,
                right_info: state_clone.input_right_info.clone(),
            };
            input_bar.render_ref(main_areas[2], frame.buffer_mut(), theme);
            let cursor_pos = input_bar.cursor_screen_pos(main_areas[2]);
            frame.set_cursor_position(cursor_pos);

            // Render status bar using standalone function
            if show_status_bar {
                render_status_bar(&state_clone, main_areas[3], frame.buffer_mut(), theme);
            }

            // Permission modal (render from state if active)
            if mode == TuiMode::Permission && state_clone.permission_modal_tool.is_some() {
                let bg_base: ratatui::style::Color = theme.color("bg.base").into();
                // Dim background
                for y in 0..area.height {
                    for x in 0..area.width {
                        if let Some(cell) = frame.buffer_mut().cell_mut((x, y)) {
                            cell.set_style(Style::default().bg(bg_base));
                        }
                    }
                }
                // Center modal
                let modal_w = 50u16;
                let modal_h = 12u16;
                let modal_x = padded_area.x + (padded_area.width.saturating_sub(modal_w)) / 2;
                let modal_y = padded_area.y + (padded_area.height.saturating_sub(modal_h)) / 2;
                let modal_area = Rect::new(modal_x, modal_y, modal_w, modal_h);

                // Draw shadow
                Self::render_shadow(modal_area, frame.buffer_mut(), theme);

                // Render permission modal from state
                let modal = PermissionModal::new(
                    state_clone.permission_modal_tool.as_deref().unwrap_or(""),
                    state_clone.permission_modal_args.as_deref().unwrap_or(""),
                    state_clone.permission_modal_desc.as_deref().unwrap_or(""),
                );
                modal.render_ref(modal_area, frame.buffer_mut(), theme);
            }

            // Command palette (simplified - just show a placeholder)
            if mode == TuiMode::CommandPalette {
                let bg_base: ratatui::style::Color = theme.color("bg.base").into();
                // Dim background
                for y in 0..area.height {
                    for x in 0..area.width {
                        if let Some(cell) = frame.buffer_mut().cell_mut((x, y)) {
                            cell.set_style(Style::default().bg(bg_base));
                        }
                    }
                }
                // Center palette
                let palette_w = 70u16;
                let palette_h = 20u16;
                let palette_x = padded_area.x + (padded_area.width.saturating_sub(palette_w)) / 2;
                let palette_y = padded_area.y + (padded_area.height.saturating_sub(palette_h)) / 2;
                let palette_area = Rect::new(palette_x, palette_y, palette_w, palette_h);

                // Draw shadow
                Self::render_shadow(palette_area, frame.buffer_mut(), theme);

                // Render command palette widget
                let palette = CommandPalette::new();
                palette.render_ref(palette_area, frame.buffer_mut(), theme);
            }

            // Overlay mode (simplified - just show a centered box)
            if mode == TuiMode::Overlay {
                let overlay_area = Overlay::centered((60, 20), frame.area());

                // Draw shadow first
                Self::render_shadow(overlay_area, frame.buffer_mut(), theme);

                let mut overlay_buf = Buffer::empty(overlay_area);
                let overlay = Overlay::default();
                overlay.render_ref(overlay_area, &mut overlay_buf, theme);
                for y in 0..overlay_buf.area.height {
                    for x in 0..overlay_buf.area.width {
                        let cell = overlay_buf.get(x, y);
                        let tx = overlay_area.x + x;
                        let ty = overlay_area.y + y;
                        if tx < area.width && ty < area.height {
                            if let Some(target) = frame.buffer_mut().cell_mut((tx, ty)) {
                                target.set_style(cell.style());
                                if let Some(ch) = cell.symbol().chars().next() {
                                    target.set_char(ch);
                                }
                            }
                        }
                    }
                }
            }
        })?;
        Ok(())
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
                            PermissionDecision::Allow => PermissionAction::Confirm,
                            PermissionDecision::Deny => PermissionAction::Cancel,
                            PermissionDecision::AllowAlways => PermissionAction::Always,
                            PermissionDecision::Skip => PermissionAction::Skip,
                        };
                        return Some(TuiAction::ToolPermission {
                            tool: self.state.permission_modal_tool.clone().unwrap_or_default(),
                            action: permission_action,
                        });
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

    pub fn add_message(&mut self, item: MessageItem) {
        self.state.messages.push(item);
    }

    pub fn on_agent_event(&mut self, event: AgentEvent) -> Vec<Cmd> {
        // Use the update() reducer for all agent events
        self.update(Msg::AgentEvent(event.clone()))
    }

    pub fn show_overlay(&mut self, _overlay: Overlay) {
        self.state.mode = TuiMode::Overlay;
    }

    pub fn hide_overlay(&mut self) {
        self.state.mode = TuiMode::Chat;
    }

    pub fn request_permission(&mut self, tool_name: &str, tool_args: &str, description: &str) {
        self.state.permission_modal_tool = Some(tool_name.to_string());
        self.state.permission_modal_args = Some(tool_args.to_string());
        self.state.permission_modal_desc = Some(description.to_string());
        self.state.mode = TuiMode::Permission;
    }

    pub fn is_permission_modal_active(&self) -> bool {
        self.state.permission_modal_tool.is_some() && self.state.mode == TuiMode::Permission
    }

    pub fn toggle_sidebar(&mut self) {
        self.update(Msg::ToggleSidebar);
    }

    /// Draw a subtle shadow around a modal area (1 cell right, 1 cell down)
    fn render_shadow(modal_area: Rect, buf: &mut ratatui::buffer::Buffer, theme: &ThemeWrapper) {
        let shadow_bg: ratatui::style::Color = theme.color("bg.base").into();
        let shadow_fg: ratatui::style::Color = theme.color("text.dim").into();

        // Shadow on the right side (1 column to the right of modal)
        let shadow_x = modal_area.x + modal_area.width;
        if shadow_x < buf.area.width {
            for y in modal_area.y + 1..modal_area.y + modal_area.height + 1 {
                if y < buf.area.height {
                    if let Some(cell) = buf.cell_mut((shadow_x, y)) {
                        cell.set_char('░');
                        cell.set_style(Style::default().fg(shadow_fg).bg(shadow_bg));
                    }
                }
            }
        }

        // Shadow on the bottom (1 row below modal)
        let shadow_y = modal_area.y + modal_area.height;
        if shadow_y < buf.area.height {
            for x in modal_area.x + 1..modal_area.x + modal_area.width + 1 {
                if x < buf.area.width {
                    if let Some(cell) = buf.cell_mut((x, shadow_y)) {
                        cell.set_char('░');
                        cell.set_style(Style::default().fg(shadow_fg).bg(shadow_bg));
                    }
                }
            }
        }

        // Corner shadow (diagonal)
        let corner_x = modal_area.x + modal_area.width;
        let corner_y = modal_area.y + modal_area.height;
        if corner_x < buf.area.width && corner_y < buf.area.height {
            if let Some(cell) = buf.cell_mut((corner_x, corner_y)) {
                cell.set_char('▒');
                cell.set_style(Style::default().fg(shadow_fg).bg(shadow_bg));
            }
        }
    }
}

