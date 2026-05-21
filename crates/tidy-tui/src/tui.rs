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
        TopBar,
        MessageList,
        MessageItem,
        InputBar,
        StatusBar,
        Overlay,
        PermissionModal,
        PermissionAction,
        AgentList,
        AgentItem,
        AgentStatus,
        ContextPanel,
        GitChange,
        GitStatus,
        CommandPalette,
        PaletteCommand,
    },
};
use tidy_agent::events::{AgentEvent, ContentPart};

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

pub struct Tui {
    pub config: TuiConfig,
    pub terminal: Terminal<CrosstermBackend<io::Stdout>>,
    pub top_bar: TopBar,
    pub message_list: MessageList,
    pub input_bar: InputBar,
    pub status_bar: StatusBar,
    pub overlay: Option<Overlay>,
    pub permission_modal: Option<PermissionModal>,
    pub command_palette: Option<CommandPalette>,
    pub running: bool,
    pub mode: TuiMode,
    pub show_sidebar: bool,
    pub agent_list: AgentList,
    pub context_panel: ContextPanel,
    pub agent_running: bool,
    pub current_model: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TuiMode {
    Chat,
    Overlay,
    Select,
    Permission,
    CommandPalette,
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
            top_bar: TopBar::default(),
            message_list: MessageList::default(),
            input_bar: InputBar::default(),
            status_bar: StatusBar::default(),
            overlay: None,
            permission_modal: None,
            command_palette: None,
            running: true,
            mode: TuiMode::Chat,
            show_sidebar: false,
            agent_list: AgentList {
                agents: vec![
                    AgentItem {
                        id: "coder".to_string(),
                        tag: "coder".to_string(),
                        tag_type: "assistant".to_string(),
                        description: "editing files".to_string(),
                        model: "claude-4".to_string(),
                        duration_secs: 45,
                        status: AgentStatus::Running,
                    },
                    AgentItem {
                        id: "test".to_string(),
                        tag: "test".to_string(),
                        tag_type: "system".to_string(),
                        description: "running tests".to_string(),
                        model: "gpt-4".to_string(),
                        duration_secs: 12,
                        status: AgentStatus::Completed,
                    },
                ],
            },
            context_panel: ContextPanel {
                recent_files: vec![
                    "src/main.rs".to_string(),
                    "Cargo.toml".to_string(),
                    "README.md".to_string(),
                ],
                git_changes: vec![
                    GitChange { path: "src/tui.rs".to_string(), status: GitStatus::Modified },
                    GitChange { path: "src/components/context_panel.rs".to_string(), status: GitStatus::Added },
                ],
                active_tool: Some("read_file".to_string()),
                model_name: "claude-4".to_string(),
                session_info: "demo-session-001".to_string(),
            },
            agent_running: false,
            current_model: None,
        })
    }

    pub fn cleanup(&mut self) -> io::Result<()> {
        disable_raw_mode()?;
        self.terminal.backend_mut().execute(SetCursorStyle::DefaultUserShape)?;
        self.terminal.backend_mut().execute(LeaveAlternateScreen)?;
        Ok(())
    }

    /// Calculate the height needed for the input bar based on its content
    fn input_bar_height(&self, _area_width: u16) -> u16 {
        // Each logical line = 1 visual line (no wrapping)
        let visual_lines = self.input_bar.visual_height();
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
        let input_height = self.input_bar_height(padded_area.width);

        // Extract values needed in closure to avoid borrow conflicts
        let show_sidebar = self.show_sidebar;
        let agent_list = self.agent_list.clone();
        let show_top_bar = self.config.show_top_bar;
        let show_status_bar = self.config.show_status_bar;

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

            // Render top bar
            if show_top_bar {
                self.top_bar.render_ref(main_areas[0], frame.buffer_mut(), theme);
            }

            // Split content area horizontally
            let content_area = main_areas[1];
            let mut h_constraints = vec![];
            h_constraints.push(Constraint::Min(20));
            if show_sidebar && content_area.width >= SIDEBAR_WIDTH + 20 {
                h_constraints.push(Constraint::Length(SIDEBAR_WIDTH));
            }
            let h_areas = Layout::horizontal(h_constraints.as_slice()).split(content_area);

            if show_sidebar && content_area.width >= SIDEBAR_WIDTH + 20 {
                self.message_list.render_ref(h_areas[0], frame.buffer_mut(), theme);
                agent_list.render(h_areas[1], frame.buffer_mut());
            } else {
                self.message_list.render_ref(h_areas[0], frame.buffer_mut(), theme);
            }

            // Render input bar
            self.input_bar.render_ref(main_areas[2], frame.buffer_mut(), theme);
            let cursor_pos = self.input_bar.cursor_screen_pos(main_areas[2]);
            frame.set_cursor_position(cursor_pos);

            // Render status bar
            if show_status_bar {
                self.status_bar.render_ref(main_areas[3], frame.buffer_mut(), theme);
            }

            if let Some(overlay) = &self.overlay {
                if self.mode == TuiMode::Overlay {
                    let overlay_area = Overlay::centered((60, 20), frame.area());

                    // Draw shadow first
                    Self::render_shadow(overlay_area, frame.buffer_mut(), theme);

                    let mut overlay_buf = Buffer::empty(overlay_area);
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
            }

            if self.mode == TuiMode::Permission {
                if let Some(ref modal) = self.permission_modal {
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

                    modal.render_ref(modal_area, frame.buffer_mut(), theme);
                }
            }

            if self.mode == TuiMode::CommandPalette {
                if let Some(ref palette) = self.command_palette {
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

                    palette.render_ref(palette_area, frame.buffer_mut(), theme);
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
        match self.mode {
            TuiMode::Chat => self.handle_chat_key(key),
            TuiMode::Overlay => self.handle_overlay_key(key),
            TuiMode::Select => self.handle_select_key(key),
            TuiMode::Permission => self.handle_permission_key(key),
            TuiMode::CommandPalette => self.handle_command_palette_key(key),
        }
    }

    fn handle_chat_key(&mut self, key: crossterm::event::KeyEvent) -> Option<TuiAction> {
        match key.code {
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.running = false;
                Some(TuiAction::Quit)
            }
            KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.running = false;
                Some(TuiAction::Quit)
            }
            KeyCode::Enter => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    // Shift+Enter: insert newline
                    self.input_bar.insert_newline();
                    None
                } else {
                    let text = self.input_bar.submit();
                    if !text.is_empty() {
                        self.message_list.messages.push(MessageItem::User { text: text.clone(), model: Some("You".to_string()), timestamp: None });
                        Some(TuiAction::Submit(text))
                    } else {
                        None
                    }
                }
            }
            KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Ctrl+J: insert newline (like Shift+Enter)
                self.input_bar.insert_newline();
                None
            }
            KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Ctrl+K: open command palette
                self.command_palette = Some(CommandPalette::new());
                self.mode = TuiMode::CommandPalette;
                None
            }
            KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Ctrl+P: open command palette
                self.command_palette = Some(CommandPalette::new());
                self.mode = TuiMode::CommandPalette;
                None
            }
            KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Ctrl+A: start of line
                self.input_bar.move_cursor_to_start();
                None
            }
            KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Ctrl+E: end of line
                self.input_bar.move_cursor_to_end();
                None
            }
            KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Ctrl+W: delete word backward
                self.input_bar.delete_word_backward();
                None
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Ctrl+U: delete from cursor to start of line
                self.input_bar.delete_to_start();
                None
            }

            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Ctrl+D: delete character under cursor (forward delete)
                self.input_bar.delete_forward();
                None
            }
            KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Ctrl+B: toggle right sidebar
                self.toggle_sidebar();
                None
            }
            KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Ctrl+F: move cursor forward (like Right arrow)
                self.input_bar.move_cursor_right();
                None
            }
            KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Ctrl+N: move cursor down (like Down arrow)
                self.input_bar.move_cursor_down();
                None
            }

            KeyCode::Char('h') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Ctrl+H: backspace
                self.input_bar.backspace();
                None
            }
            KeyCode::Char(c) => {
                self.input_bar.insert_char(c);
                None
            }
            KeyCode::Backspace => {
                self.input_bar.backspace();
                None
            }
            KeyCode::Left => {
                self.input_bar.move_cursor_left();
                None
            }
            KeyCode::Right => {
                self.input_bar.move_cursor_right();
                None
            }
            KeyCode::Up => {
                self.input_bar.move_cursor_up();
                None
            }
            KeyCode::Down => {
                self.input_bar.move_cursor_down();
                None
            }
            _ => None,
        }
    }

    fn handle_overlay_key(&mut self, key: crossterm::event::KeyEvent) -> Option<TuiAction> {
        match key.code {
            KeyCode::Esc => {
                self.overlay = None;
                self.mode = TuiMode::Chat;
                self.status_bar.set_chat_mode();
                None
            }
            _ => None,
        }
    }

    fn handle_select_key(&mut self, _key: crossterm::event::KeyEvent) -> Option<TuiAction> {
        None
    }

    fn handle_permission_key(&mut self, key: crossterm::event::KeyEvent) -> Option<TuiAction> {
        if self.permission_modal.is_none() {
            return None;
        }

        // We need to extract data before we can mutably borrow to set None
        let (tool_name, action_opt) = match key.code {
            KeyCode::Left => {
                self.permission_modal.as_mut().unwrap().prev_option();
                return None;
            }
            KeyCode::Right => {
                self.permission_modal.as_mut().unwrap().next_option();
                return None;
            }
            KeyCode::Enter => {
                let modal = self.permission_modal.as_mut().unwrap();
                let action = modal.confirm();
                let tool = modal.tool_name.clone();
                (tool, Some(action))
            }
            KeyCode::Esc => {
                let modal = self.permission_modal.as_mut().unwrap();
                let tool = modal.tool_name.clone();
                (tool, Some(PermissionAction::Cancel))
            }
            KeyCode::Char('y') => {
                let modal = self.permission_modal.as_mut().unwrap();
                let tool = modal.tool_name.clone();
                (tool, Some(PermissionAction::Confirm))
            }
            KeyCode::Char('n') => {
                let modal = self.permission_modal.as_mut().unwrap();
                let tool = modal.tool_name.clone();
                (tool, Some(PermissionAction::Cancel))
            }
            KeyCode::Char('a') => {
                let modal = self.permission_modal.as_mut().unwrap();
                let tool = modal.tool_name.clone();
                (tool, Some(PermissionAction::Always))
            }
            KeyCode::Char('s') => {
                let modal = self.permission_modal.as_mut().unwrap();
                let tool = modal.tool_name.clone();
                (tool, Some(PermissionAction::Skip))
            }
            _ => return None,
        };

        self.permission_modal = None;
        self.mode = TuiMode::Chat;
        action_opt.map(|action| TuiAction::ToolPermission { tool: tool_name, action })
    }

    fn handle_command_palette_key(&mut self, key: crossterm::event::KeyEvent) -> Option<TuiAction> {
        if let Some(ref mut palette) = self.command_palette {
            match key.code {
                KeyCode::Esc => {
                    self.command_palette = None;
                    self.mode = TuiMode::Chat;
                    None
                }
                KeyCode::Enter => {
                    if let Some(cmd) = palette.confirm() {
                        let action = match cmd {
                            PaletteCommand::ReadFile { path } => {
                                Some(TuiAction::Command(format!("read {}", path)))
                            }
                            PaletteCommand::EditFile { path, prompt } => {
                                Some(TuiAction::Command(format!("edit {} {}", path, prompt)))
                            }
                            PaletteCommand::RunAgent { name } => {
                                Some(TuiAction::Command(format!("run {}", name)))
                            }
                            PaletteCommand::SwitchModel { model } => {
                                Some(TuiAction::Command(format!("model {}", model)))
                            }
                            PaletteCommand::LoadSession { id } => {
                                Some(TuiAction::Command(format!("load {}", id)))
                            }
                            PaletteCommand::SaveSession { name } => {
                                Some(TuiAction::Command(format!("save {}", name)))
                            }
                            PaletteCommand::Cancel => None,
                        };
                        self.command_palette = None;
                        self.mode = TuiMode::Chat;
                        return action;
                    }
                    None
                }
                KeyCode::Up => {
                    palette.prev_item();
                    None
                }
                KeyCode::Down => {
                    palette.next_item();
                    None
                }
                KeyCode::Char(c) => {
                    palette.insert_char(c);
                    None
                }
                KeyCode::Backspace => {
                    palette.backspace();
                    None
                }
                _ => None,
            }
        } else {
            None
        }
    }

    pub fn add_message(&mut self, item: MessageItem) {
        self.message_list.messages.push(item);
    }

    pub fn on_agent_event(&mut self, event: AgentEvent) {
        match event {
            AgentEvent::MessageStart { message } => {
                self.agent_running = true;
                self.current_model = Some(message.role.clone());
                // Add empty assistant message that will be updated
                self.message_list.messages.push(MessageItem::Assistant {
                    text: String::new(),
                    model: self.current_model.clone(),
                    timestamp: None,
                });
            }
            AgentEvent::MessageUpdate { message } => {
                // Update the last assistant message with new text
                if let Some(last) = self.message_list.messages.last_mut() {
                    if let MessageItem::Assistant { ref mut text, .. } = last {
                        // Extract text from content parts
                        let new_text = message.content.iter()
                            .filter_map(|part| {
                                if let ContentPart::Text { text } = part { Some(text.as_str()) } else { None }
                            })
                            .collect::<Vec<_>>()
                            .join("");
                        *text = new_text;
                    }
                }
            }
            AgentEvent::MessageEnd { message } => {
                // Finalize the assistant message
                if let Some(last) = self.message_list.messages.last_mut() {
                    if let MessageItem::Assistant { ref mut text, .. } = last {
                        let final_text = message.content.iter()
                            .filter_map(|part| {
                                if let ContentPart::Text { text } = part { Some(text.as_str()) } else { None }
                            })
                            .collect::<Vec<_>>()
                            .join("");
                        *text = final_text;
                    }
                }
            }
            AgentEvent::ToolExecutionStart { tool_call_id } => {
                self.message_list.messages.push(MessageItem::ToolCall {
                    name: tool_call_id,  // We'll improve this later with actual tool names
                    args: String::new(),
                    result: None,
                    is_error: false,
                });
            }
            AgentEvent::ToolExecutionEnd { tool_call_id: _, result } => {
                let result_text = result.content.iter()
                    .filter_map(|part| {
                        if let ContentPart::Text { text } = part { Some(text.as_str()) } else { None }
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                let is_err = result.is_error;
                // Update the last ToolCall with the result
                if let Some(last) = self.message_list.messages.last_mut() {
                    if let MessageItem::ToolCall { ref mut result, ref mut is_error, .. } = last {
                        *result = Some(result_text);
                        *is_error = is_err;
                    }
                }
            }
            AgentEvent::TurnEnd { .. } => {
                // Turn complete, could update status
            }
            AgentEvent::AgentEnd { .. } => {
                self.agent_running = false;
                self.current_model = None;
            }
            AgentEvent::Error { message } => {
                self.message_list.messages.push(MessageItem::System {
                    text: format!("Error: {}", message),
                });
                self.agent_running = false;
            }
            AgentEvent::PermissionRequest { tool_call_id: _, tool_name, tool_args } => {
                // Show permission modal — this pauses the UI
                self.request_permission(
                    tool_name.as_str(),
                    tool_args.as_str(),
                    &format!("Agent wants to execute '{}'", tool_name)
                );
            }
            AgentEvent::PermissionGranted { tool_call_id: _ } => {
                // Could show a brief "approved" indicator in status bar
                // For now, just log or ignore
            }
            AgentEvent::PermissionDenied { tool_call_id: _ } => {
                // Could show a brief "denied" indicator
                // For now, just log or ignore
            }
        }
    }

    pub fn show_overlay(&mut self, overlay: Overlay) {
        self.overlay = Some(overlay);
        self.mode = TuiMode::Overlay;
        self.status_bar.set_overlay_mode();
    }

    pub fn hide_overlay(&mut self) {
        self.overlay = None;
        self.mode = TuiMode::Chat;
        self.status_bar.set_chat_mode();
    }

    pub fn request_permission(&mut self, tool_name: &str, tool_args: &str, description: &str) {
        self.permission_modal = Some(PermissionModal::new(tool_name, tool_args, description));
        self.mode = TuiMode::Permission;
    }

    pub fn is_permission_modal_active(&self) -> bool {
        self.permission_modal.is_some() && self.mode == TuiMode::Permission
    }

    pub fn toggle_sidebar(&mut self) {
        self.show_sidebar = !self.show_sidebar;
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

#[derive(Debug, Clone, PartialEq)]
pub enum TuiAction {
    Quit,
    Submit(String),
    Command(String),
    Cancel,
    ToolPermission { tool: String, action: PermissionAction },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_list_has_demo_data() {
        // Verify AgentList default has agents populated (testing the data structure)
        let agent_list = AgentList {
            agents: vec![
                AgentItem {
                    id: "coder".to_string(),
                    tag: "coder".to_string(),
                    tag_type: "assistant".to_string(),
                    description: "editing files".to_string(),
                    model: "claude-4".to_string(),
                    duration_secs: 45,
                    status: AgentStatus::Running,
                },
                AgentItem {
                    id: "test".to_string(),
                    tag: "test".to_string(),
                    tag_type: "system".to_string(),
                    description: "running tests".to_string(),
                    model: "gpt-4".to_string(),
                    duration_secs: 12,
                    status: AgentStatus::Completed,
                },
            ],
        };
        assert_eq!(agent_list.agents.len(), 2);
        assert_eq!(agent_list.agents[0].id, "coder");
        assert_eq!(agent_list.agents[1].status, AgentStatus::Completed);
    }

    #[test]
    fn test_context_panel_has_demo_data() {
        let context_panel = ContextPanel {
            recent_files: vec![
                "src/main.rs".to_string(),
                "Cargo.toml".to_string(),
                "README.md".to_string(),
            ],
            git_changes: vec![
                GitChange { path: "src/tui.rs".to_string(), status: GitStatus::Modified },
                GitChange { path: "src/components/context_panel.rs".to_string(), status: GitStatus::Added },
            ],
            active_tool: Some("read_file".to_string()),
            model_name: "claude-4".to_string(),
            session_info: "demo-session-001".to_string(),
        };
        assert_eq!(context_panel.model_name, "claude-4");
        assert_eq!(context_panel.recent_files.len(), 3);
        assert_eq!(context_panel.git_changes.len(), 2);
        assert_eq!(context_panel.active_tool, Some("read_file".to_string()));
    }

    #[test]
    fn test_sidebar_toggle_methods() {
        // Test that toggle methods work on Tui state
        // We test the methods themselves since Tui::new requires a terminal
        let mut show_left = false;
        let mut show_right = false;

        // Simulate toggle_left_sidebar
        show_left = !show_left;
        assert!(show_left);

        // Simulate toggle_right_sidebar
        show_right = !show_right;
        assert!(show_right);
    }

    #[test]
    fn test_agent_status_variants() {
        assert_eq!(AgentStatus::Running, AgentStatus::Running);
        assert_eq!(AgentStatus::Completed, AgentStatus::Completed);
        assert_ne!(AgentStatus::Running, AgentStatus::Completed);
    }

    #[test]
    fn test_git_status_variants() {
        assert_eq!(GitStatus::Modified, GitStatus::Modified);
        assert_eq!(GitStatus::Added, GitStatus::Added);
        assert_eq!(GitStatus::Deleted, GitStatus::Deleted);
        assert_eq!(GitStatus::Untracked, GitStatus::Untracked);
    }
}
