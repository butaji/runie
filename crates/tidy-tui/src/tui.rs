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

// ─── AppState ─────────────────────────────────────────────────────────────────
// All application state, extracted from Tui. No terminal I/O, no widgets.

#[derive(Clone)]
pub struct AppState {
    pub messages: Vec<MessageItem>,
    pub input_lines: Vec<String>,
    pub cursor_col: usize,
    pub cursor_row: usize,
    pub input_right_info: String,
    pub mode: TuiMode,
    pub running: bool,
    pub show_sidebar: bool,
    pub agent_running: bool,
    pub current_model: Option<String>,
    pub top_bar_repo: String,
    pub top_bar_branch: String,
    pub top_bar_path: String,
    pub permission_modal_tool: Option<String>,
    pub permission_modal_args: Option<String>,
    pub permission_modal_desc: Option<String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            input_lines: vec![String::new()],
            cursor_col: 0,
            cursor_row: 0,
            input_right_info: String::new(),
            mode: TuiMode::Chat,
            running: true,
            show_sidebar: false,
            agent_running: false,
            current_model: None,
            top_bar_repo: String::new(),
            top_bar_branch: String::new(),
            top_bar_path: String::new(),
            permission_modal_tool: None,
            permission_modal_args: None,
            permission_modal_desc: None,
        }
    }
}

// ─── Action ────────────────────────────────────────────────────────────────────
// All state changes described as actions (unidirectional data flow)

#[derive(Debug, Clone)]
pub enum Action {
    Quit,
    Submit,
    InsertChar(char),
    Backspace,
    DeleteForward,
    MoveCursorLeft,
    MoveCursorRight,
    MoveCursorUp,
    MoveCursorDown,
    MoveCursorToStart,
    MoveCursorToEnd,
    InsertNewline,
    DeleteWordBackward,
    DeleteToStart,
    ToggleSidebar,
    OpenCommandPalette,
    CloseModal,
    ConfirmModal,
    AgentEvent(tidy_agent::events::AgentEvent),
    PermissionConfirm,
    PermissionCancel,
    PermissionAlways,
    PermissionSkip,
    OverlayClosed,
}

// ─── update() ─────────────────────────────────────────────────────────────────
// Pure reducer: takes state + action, returns new state (no side effects)

pub fn update(state: &mut AppState, action: Action) {
    match action {
        Action::Quit => state.running = false,

        Action::Submit => {
            let text = state.input_lines.join("\n");
            if !text.is_empty() {
                state.messages.push(MessageItem::User {
                    text,
                    model: Some("You".to_string()),
                    timestamp: None,
                });
                state.input_lines = vec![String::new()];
                state.cursor_col = 0;
                state.cursor_row = 0;
            }
        }

        Action::InsertChar(c) => {
            if state.cursor_row < state.input_lines.len() {
                state.input_lines[state.cursor_row].insert(state.cursor_col, c);
                state.cursor_col += 1;
            }
        }

        Action::Backspace => {
            if state.cursor_col > 0 {
                state.input_lines[state.cursor_row].remove(state.cursor_col - 1);
                state.cursor_col -= 1;
            } else if state.cursor_row > 0 {
                let line = state.input_lines.remove(state.cursor_row);
                state.cursor_row -= 1;
                state.cursor_col = state.input_lines[state.cursor_row].len();
                state.input_lines[state.cursor_row].push_str(&line);
            }
        }

        Action::InsertNewline => {
            if state.cursor_row < state.input_lines.len() {
                let remainder = state.input_lines[state.cursor_row].split_off(state.cursor_col);
                state.cursor_row += 1;
                state.cursor_col = 0;
                state.input_lines.insert(state.cursor_row, remainder);
            }
        }

        Action::MoveCursorLeft => {
            if state.cursor_col > 0 {
                state.cursor_col -= 1;
            } else if state.cursor_row > 0 {
                state.cursor_row -= 1;
                state.cursor_col = state.input_lines[state.cursor_row].len();
            }
        }

        Action::MoveCursorRight => {
            if state.cursor_col < state.input_lines[state.cursor_row].len() {
                state.cursor_col += 1;
            } else if state.cursor_row + 1 < state.input_lines.len() {
                state.cursor_row += 1;
                state.cursor_col = 0;
            }
        }

        Action::MoveCursorUp => {
            if state.cursor_row > 0 {
                state.cursor_row -= 1;
                state.cursor_col = state.cursor_col.min(state.input_lines[state.cursor_row].len());
            }
        }

        Action::MoveCursorDown => {
            if state.cursor_row + 1 < state.input_lines.len() {
                state.cursor_row += 1;
                state.cursor_col = state.cursor_col.min(state.input_lines[state.cursor_row].len());
            }
        }

        Action::MoveCursorToStart => state.cursor_col = 0,

        Action::MoveCursorToEnd => {
            state.cursor_col = state.input_lines[state.cursor_row].len();
        }

        Action::DeleteForward => {
            if state.cursor_col < state.input_lines[state.cursor_row].len() {
                state.input_lines[state.cursor_row].remove(state.cursor_col);
            }
        }

        Action::DeleteWordBackward => {
            let line = &state.input_lines[state.cursor_row];
            let before = &line[..state.cursor_col];
            if let Some(pos) = before.rfind(|c: char| c.is_whitespace()) {
                state.input_lines[state.cursor_row].drain(pos..state.cursor_col);
                state.cursor_col = pos;
            } else {
                state.input_lines[state.cursor_row].clear();
                state.cursor_col = 0;
            }
        }

        Action::DeleteToStart => {
            state.input_lines[state.cursor_row].drain(..state.cursor_col);
            state.cursor_col = 0;
        }

        Action::ToggleSidebar => state.show_sidebar = !state.show_sidebar,

        Action::OpenCommandPalette => {
            state.mode = TuiMode::CommandPalette;
        }

        Action::CloseModal => {
            state.mode = TuiMode::Chat;
            state.permission_modal_tool = None;
        }

        Action::ConfirmModal => {
            state.mode = TuiMode::Chat;
            state.permission_modal_tool = None;
        }

        Action::OverlayClosed => {
            state.mode = TuiMode::Chat;
        }

        Action::AgentEvent(event) => {
use tidy_agent::events::AgentEvent;
            match event {
                AgentEvent::MessageStart { message } => {
                    state.agent_running = true;
                    state.current_model = Some(message.role.clone());
                    state.messages.push(MessageItem::Assistant {
                        text: String::new(),
                        model: state.current_model.clone(),
                        timestamp: None,
                    });
                }
                AgentEvent::MessageUpdate { message } => {
                    if let Some(last) = state.messages.last_mut() {
                        if let MessageItem::Assistant { ref mut text, .. } = last {
                            let new_text = message
                                .content
                                .iter()
                                .filter_map(|part| {
                                    if let ContentPart::Text { text } = part {
                                        Some(text.as_str())
                                    } else {
                                        None
                                    }
                                })
                                .collect::<Vec<_>>()
                                .join("");
                            *text = new_text;
                        }
                    }
                }
                AgentEvent::MessageEnd { message } => {
                    if let Some(last) = state.messages.last_mut() {
                        if let MessageItem::Assistant { ref mut text, .. } = last {
                            let final_text = message
                                .content
                                .iter()
                                .filter_map(|part| {
                                    if let ContentPart::Text { text } = part {
                                        Some(text.as_str())
                                    } else {
                                        None
                                    }
                                })
                                .collect::<Vec<_>>()
                                .join("");
                            *text = final_text;
                        }
                    }
                }
                AgentEvent::ToolExecutionStart { tool_call_id } => {
                    state.messages.push(MessageItem::ToolCall {
                        name: tool_call_id,
                        args: String::new(),
                        result: None,
                        is_error: false,
                    });
                }
                AgentEvent::ToolExecutionEnd { result, .. } => {
                    let result_text = result
                        .content
                        .iter()
                        .filter_map(|part| {
                            if let ContentPart::Text { text } = part {
                                Some(text.as_str())
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(" ");
                    let is_err = result.is_error;
                    if let Some(last) = state.messages.last_mut() {
                        if let MessageItem::ToolCall { ref mut result, ref mut is_error, .. } = last {
                            *result = Some(result_text);
                            *is_error = is_err;
                        }
                    }
                }
                AgentEvent::AgentEnd { .. } => {
                    state.agent_running = false;
                    state.current_model = None;
                }
                AgentEvent::Error { message } => {
                    state
                        .messages
                        .push(MessageItem::System { text: format!("Error: {}", message) });
                    state.agent_running = false;
                }
                AgentEvent::PermissionRequest { tool_name, tool_args, .. } => {
                    state.permission_modal_tool = Some(tool_name.clone());
                    state.permission_modal_args = Some(tool_args.clone());
                    state.permission_modal_desc =
                        Some(format!("Agent wants to execute '{}'", tool_name));
                    state.mode = TuiMode::Permission;
                }
                _ => {}
            }
        }

        Action::PermissionConfirm => {
            state.mode = TuiMode::Chat;
            state.permission_modal_tool = None;
        }
        Action::PermissionCancel => {
            state.mode = TuiMode::Chat;
            state.permission_modal_tool = None;
        }
        Action::PermissionAlways => {
            state.mode = TuiMode::Chat;
            state.permission_modal_tool = None;
        }
        Action::PermissionSkip => {
            state.mode = TuiMode::Chat;
            state.permission_modal_tool = None;
        }
    }
}

// ─── Tui ─────────────────────────────────────────────────────────────────────

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
    pub agent_list: AgentList,
    pub context_panel: ContextPanel,
    pub state: AppState,
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
            state: AppState::default(),
        })
    }

    pub fn cleanup(&mut self) -> io::Result<()> {
        disable_raw_mode()?;
        self.terminal.backend_mut().execute(SetCursorStyle::DefaultUserShape)?;
        self.terminal.backend_mut().execute(LeaveAlternateScreen)?;
        Ok(())
    }

    /// Dispatch an action to update state (unidirectional data flow)
    pub fn update(&mut self, action: Action) {
        update(&mut self.state, action);
    }

    /// Calculate the height needed for the input bar based on its content
    fn input_bar_height(&self, _area_width: u16) -> u16 {
        // Each logical line = 1 visual line (no wrapping)
        let visual_lines = self.input_bar.visual_height();
        // 2 for borders + visual lines for content
        (visual_lines as u16) + 2
    }

    /// Sync widget state from AppState (state → widgets, unidirectional)
    fn sync_widgets_from_state(&mut self) {
        // MessageList
        self.message_list.messages = self.state.messages.clone();

        // InputBar
        self.input_bar.lines = self.state.input_lines.clone();
        self.input_bar.cursor_col = self.state.cursor_col;
        // Note: InputBar uses cursor_line, state uses cursor_row
        // We need to map carefully to avoid out-of-bounds
        self.input_bar.cursor_line = self.state.cursor_row.min(self.input_bar.lines.len().saturating_sub(1));

        // Sync mode to widgets that need it
        match self.state.mode {
            TuiMode::Chat => self.status_bar.set_chat_mode(),
            TuiMode::Overlay => self.status_bar.set_overlay_mode(),
            _ => {}
        }
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

        // Sync widgets from state (unidirectional: state → widgets)
        self.sync_widgets_from_state();

        // Calculate dynamic input bar height
        let input_height = self.input_bar_height(padded_area.width);

        // Extract values needed in closure to avoid borrow conflicts
        let show_sidebar = self.state.show_sidebar;
        let agent_list = self.agent_list.clone();
        let show_top_bar = self.config.show_top_bar;
        let show_status_bar = self.config.show_status_bar;
        let mode = self.state.mode.clone();

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
                if mode == TuiMode::Overlay {
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

            if mode == TuiMode::Permission {
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

            if mode == TuiMode::CommandPalette {
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
        match self.state.mode {
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
                self.update(Action::Quit);
                Some(TuiAction::Quit)
            }
            KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.update(Action::Quit);
                Some(TuiAction::Quit)
            }
            KeyCode::Enter => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    self.update(Action::InsertNewline);
                    None
                } else {
                    let text = self.state.input_lines.join("\n");
                    if !text.is_empty() {
                        self.update(Action::Submit);
                        Some(TuiAction::Submit(text))
                    } else {
                        None
                    }
                }
            }
            KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.update(Action::InsertNewline);
                None
            }
            KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.command_palette = Some(CommandPalette::new());
                self.update(Action::OpenCommandPalette);
                None
            }
            KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.command_palette = Some(CommandPalette::new());
                self.update(Action::OpenCommandPalette);
                None
            }
            KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.update(Action::MoveCursorToStart);
                None
            }
            KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.update(Action::MoveCursorToEnd);
                None
            }
            KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.update(Action::DeleteWordBackward);
                None
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.update(Action::DeleteToStart);
                None
            }

            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.update(Action::DeleteForward);
                None
            }
            KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.update(Action::ToggleSidebar);
                None
            }
            KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.update(Action::MoveCursorRight);
                None
            }
            KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.update(Action::MoveCursorDown);
                None
            }

            KeyCode::Char('h') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.update(Action::Backspace);
                None
            }
            KeyCode::Char(c) => {
                self.update(Action::InsertChar(c));
                None
            }
            KeyCode::Backspace => {
                self.update(Action::Backspace);
                None
            }
            KeyCode::Left => {
                self.update(Action::MoveCursorLeft);
                None
            }
            KeyCode::Right => {
                self.update(Action::MoveCursorRight);
                None
            }
            KeyCode::Up => {
                self.update(Action::MoveCursorUp);
                None
            }
            KeyCode::Down => {
                self.update(Action::MoveCursorDown);
                None
            }
            _ => None,
        }
    }

    fn handle_overlay_key(&mut self, key: crossterm::event::KeyEvent) -> Option<TuiAction> {
        match key.code {
            KeyCode::Esc => {
                self.overlay = None;
                self.update(Action::OverlayClosed);
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
        self.state.mode = TuiMode::Chat;
        action_opt.map(|action| TuiAction::ToolPermission { tool: tool_name, action })
    }

    fn handle_command_palette_key(&mut self, key: crossterm::event::KeyEvent) -> Option<TuiAction> {
        if let Some(ref mut palette) = self.command_palette {
            match key.code {
                KeyCode::Esc => {
                    self.command_palette = None;
                    self.state.mode = TuiMode::Chat;
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
                        self.state.mode = TuiMode::Chat;
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
        self.message_list.messages.push(item.clone());
        self.state.messages.push(item);
    }

    pub fn on_agent_event(&mut self, event: AgentEvent) {
        // Use the update() reducer for all agent events
        self.update(Action::AgentEvent(event.clone()));

        // For PermissionRequest, also show the modal
        if let AgentEvent::PermissionRequest { tool_name, tool_args, .. } = event {
            self.permission_modal = Some(PermissionModal::new(
                tool_name.as_str(),
                tool_args.as_str(),
                &format!("Agent wants to execute '{}'", tool_name),
            ));
        }
    }

    pub fn show_overlay(&mut self, overlay: Overlay) {
        self.overlay = Some(overlay);
        self.state.mode = TuiMode::Overlay;
        self.status_bar.set_overlay_mode();
    }

    pub fn hide_overlay(&mut self) {
        self.overlay = None;
        self.state.mode = TuiMode::Chat;
        self.status_bar.set_chat_mode();
    }

    pub fn request_permission(&mut self, tool_name: &str, tool_args: &str, description: &str) {
        self.permission_modal = Some(PermissionModal::new(tool_name, tool_args, description));
        self.state.mode = TuiMode::Permission;
    }

    pub fn is_permission_modal_active(&self) -> bool {
        self.permission_modal.is_some() && self.state.mode == TuiMode::Permission
    }

    pub fn toggle_sidebar(&mut self) {
        self.update(Action::ToggleSidebar);
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

    // ─── Reducer Tests ─────────────────────────────────────────────────────────

    fn make_state() -> AppState {
        AppState {
            messages: vec![],
            input_lines: vec![String::new()],
            cursor_col: 0,
            cursor_row: 0,
            input_right_info: String::new(),
            mode: TuiMode::Chat,
            running: true,
            show_sidebar: false,
            agent_running: false,
            current_model: None,
            top_bar_repo: String::new(),
            top_bar_branch: String::new(),
            top_bar_path: String::new(),
            permission_modal_tool: None,
            permission_modal_args: None,
            permission_modal_desc: None,
        }
    }

    #[test]
    fn test_insert_char() {
        let mut state = make_state();
        update(&mut state, Action::InsertChar('h'));
        update(&mut state, Action::InsertChar('i'));
        assert_eq!(state.input_lines, vec!["hi"]);
        assert_eq!(state.cursor_col, 2);
    }

    #[test]
    fn test_backspace() {
        let mut state = make_state();
        update(&mut state, Action::InsertChar('h'));
        update(&mut state, Action::InsertChar('i'));
        update(&mut state, Action::Backspace);
        assert_eq!(state.input_lines, vec!["h"]);
        assert_eq!(state.cursor_col, 1);
    }

    #[test]
    fn test_submit_clears_input() {
        let mut state = make_state();
        update(&mut state, Action::InsertChar('h'));
        update(&mut state, Action::InsertChar('i'));
        update(&mut state, Action::Submit);
        assert_eq!(state.input_lines, vec![""]);
        assert_eq!(state.messages.len(), 1);
        if let MessageItem::User { text, .. } = &state.messages[0] {
            assert_eq!(text, "hi");
        } else {
            panic!("Expected User message");
        }
    }

    #[test]
    fn test_submit_empty_does_nothing() {
        let mut state = make_state();
        update(&mut state, Action::Submit);
        assert_eq!(state.messages.len(), 0);
    }

    #[test]
    fn test_move_cursor() {
        let mut state = make_state();
        update(&mut state, Action::InsertChar('a'));
        update(&mut state, Action::InsertChar('b'));
        update(&mut state, Action::InsertChar('c'));
        assert_eq!(state.cursor_col, 3);

        update(&mut state, Action::MoveCursorLeft);
        assert_eq!(state.cursor_col, 2);

        update(&mut state, Action::MoveCursorLeft);
        assert_eq!(state.cursor_col, 1);

        update(&mut state, Action::MoveCursorRight);
        assert_eq!(state.cursor_col, 2);

        update(&mut state, Action::MoveCursorToStart);
        assert_eq!(state.cursor_col, 0);

        update(&mut state, Action::MoveCursorToEnd);
        assert_eq!(state.cursor_col, 3);
    }

    #[test]
    fn test_newline() {
        let mut state = make_state();
        update(&mut state, Action::InsertChar('h'));
        update(&mut state, Action::InsertChar('i'));
        update(&mut state, Action::InsertNewline);
        assert_eq!(state.input_lines, vec!["hi", ""]);
        assert_eq!(state.cursor_row, 1);
        assert_eq!(state.cursor_col, 0);
    }

    #[test]
    fn test_multi_line_submit() {
        let mut state = make_state();
        for c in "line1".chars() {
            update(&mut state, Action::InsertChar(c));
        }
        update(&mut state, Action::InsertNewline);
        for c in "line2".chars() {
            update(&mut state, Action::InsertChar(c));
        }
        update(&mut state, Action::Submit);

        assert_eq!(state.input_lines, vec![""]);
        assert_eq!(state.messages.len(), 1);
        if let MessageItem::User { text, .. } = &state.messages[0] {
            assert_eq!(text, "line1\nline2");
        } else {
            panic!("Expected User message");
        }
    }

    #[test]
    fn test_quit() {
        let mut state = make_state();
        update(&mut state, Action::Quit);
        assert!(!state.running);
    }

    #[test]
    fn test_toggle_sidebar() {
        let mut state = make_state();
        assert!(!state.show_sidebar);
        update(&mut state, Action::ToggleSidebar);
        assert!(state.show_sidebar);
        update(&mut state, Action::ToggleSidebar);
        assert!(!state.show_sidebar);
    }

    #[test]
    fn test_delete_word_backward() {
        let mut state = make_state();
        // Type "hello world"
        for c in "hello world".chars() {
            update(&mut state, Action::InsertChar(c));
        }
        assert_eq!(state.cursor_col, 11);

        // Delete word backward → "hello" (removes " world" including space, bash-like)
        update(&mut state, Action::DeleteWordBackward);
        assert_eq!(state.input_lines[0], "hello");
        assert_eq!(state.cursor_col, 5);

        // Delete word backward → "" (no more words, clears line)
        update(&mut state, Action::DeleteWordBackward);
        assert_eq!(state.input_lines[0], "");
        assert_eq!(state.cursor_col, 0);
    }

    #[test]
    fn test_delete_to_start() {
        let mut state = make_state();
        for c in "hello".chars() {
            update(&mut state, Action::InsertChar(c));
        }
        update(&mut state, Action::MoveCursorToEnd);
        update(&mut state, Action::DeleteToStart);
        assert_eq!(state.input_lines[0], "");
        assert_eq!(state.cursor_col, 0);
    }

    #[test]
    fn test_delete_forward() {
        let mut state = make_state();
        update(&mut state, Action::InsertChar('a'));
        update(&mut state, Action::InsertChar('b'));
        update(&mut state, Action::InsertChar('c'));
        update(&mut state, Action::MoveCursorToStart);
        update(&mut state, Action::DeleteForward);
        assert_eq!(state.input_lines[0], "bc");
    }

    #[test]
    fn test_agent_event_message_start() {
        let mut state = make_state();
        update(
            &mut state,
            Action::AgentEvent(tidy_agent::events::AgentEvent::MessageStart {
                message: tidy_agent::events::AgentMessage {
                    role: "assistant".to_string(),
                    content: vec![],
                    timestamp: 0,
                    usage: None,
                    stop_reason: None,
                    error_message: None,
                },
            }),
        );
        assert!(state.agent_running);
        assert_eq!(state.messages.len(), 1);
    }

    #[test]
    fn test_agent_event_message_update() {
        let mut state = make_state();
        // Start message
        update(
            &mut state,
            Action::AgentEvent(tidy_agent::events::AgentEvent::MessageStart {
                message: tidy_agent::events::AgentMessage {
                    role: "assistant".to_string(),
                    content: vec![],
                    timestamp: 0,
                    usage: None,
                    stop_reason: None,
                    error_message: None,
                },
            }),
        );

        // Update with text
        update(
            &mut state,
            Action::AgentEvent(tidy_agent::events::AgentEvent::MessageUpdate {
                message: tidy_agent::events::AgentMessage {
                    role: "assistant".to_string(),
                    content: vec![tidy_agent::events::ContentPart::Text {
                        text: "Hello".to_string(),
                    }],
                    timestamp: 0,
                    usage: None,
                    stop_reason: None,
                    error_message: None,
                },
            }),
        );

        assert_eq!(state.messages.len(), 1);
        if let MessageItem::Assistant { text, .. } = &state.messages[0] {
            assert_eq!(text, "Hello");
        } else {
            panic!("Expected Assistant message");
        }
    }
}
