use ratatui::{
    Terminal,
    backend::CrosstermBackend,
};
use crossterm::{
    cursor::{SetCursorStyle, Show, Hide},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    event::{EnableMouseCapture, DisableMouseCapture},
    ExecutableCommand,
};
use std::collections::VecDeque;
use std::io::{self, stdout};

use crate::{
    pipe::{StateChange, ViewModelPipe, RenderPipe},
    theme::ThemeWrapper,
    components::CommandPalette,
};
use runie_agent::events::AgentEvent;
use runie_agent::PermissionDecision;
use runie_agent::loop_engine::PermissionState;
use std::sync::Arc;

pub struct TuiConfig {
    pub theme: ThemeWrapper,
    pub show_status_bar: bool,
}

pub struct Tui {
    pub config: TuiConfig,
    pub terminal: Terminal<CrosstermBackend<io::Stdout>>,
    pub state: AppState,
    command_palette: CommandPalette,
    view_model_pipe: ViewModelPipe,
    render_pipe: RenderPipe,
    action_log: VecDeque<Msg>,
    action_log_capacity: usize,
    pub permission_state: Arc<PermissionState>,
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            theme: ThemeWrapper::default(),
            show_status_bar: true, // Always visible - hotkeys are context-aware and essential
        }
    }
}

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


pub use state::{AppState, TuiMode, Msg, Cmd, Onboarding, OnboardingStep};
pub use update::update;
pub use events::event_to_msg;


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


    #[must_use]
    #[must_use]
    pub fn new(config: TuiConfig) -> io::Result<Self> {
        Self::install_panic_hook();
        enable_raw_mode()?;
        let mut stdout = stdout();
        stdout.execute(EnterAlternateScreen)?;
        stdout.execute(EnableMouseCapture)?;
        stdout.execute(Hide)?;
        stdout.execute(SetCursorStyle::SteadyBar)?;

        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        let state = AppState::default();
        let command_palette = CommandPalette::new();
        let view_model_pipe = ViewModelPipe::new();
        let render_pipe = RenderPipe::new();
        let permission_state = PermissionState::new();

        Ok(Self {
            config,
            terminal,
            state,
            command_palette,
            view_model_pipe,
            render_pipe,
            action_log: VecDeque::new(),
            action_log_capacity: 1000,
            permission_state,
        })
    }

    pub fn cleanup(&mut self) -> io::Result<()> {
        disable_raw_mode()?;
        self.terminal.backend_mut().execute(DisableMouseCapture)?;
        self.terminal.backend_mut().execute(Show)?;
        self.terminal.backend_mut().execute(SetCursorStyle::DefaultUserShape)?;
        self.terminal.backend_mut().execute(LeaveAlternateScreen)?;
        Ok(())
    }

    /// Dispatch a msg to update state (unidirectional data flow)
    /// Returns StateChange with commands to be executed and render decision
    pub fn update(&mut self, msg: Msg) -> StateChange {
        self.log_action(&msg);
        let cmds = update(&mut self.state, &mut self.command_palette, msg);
        StateChange {
            cmds,
            needs_render: true,
        }
    }

    fn log_action(&mut self, msg: &Msg) {
        if self.action_log.len() >= self.action_log_capacity {
            self.action_log.pop_front();
        }
        self.action_log.push_back(msg.clone());
    }

    /// Render to terminal. Terminal I/O happens here.
    pub fn render(&mut self) -> io::Result<()> {
        let vms = self.view_model_pipe.build(&self.state);
        self.render_pipe.render(
            &mut self.terminal,
            &self.state,
            vms,
            &self.config,
            &self.command_palette,
        )
    }

    pub fn on_agent_event(&mut self, event: AgentEvent) -> Vec<Cmd> {
        // Use the update() reducer for all agent events
        self.update(Msg::AgentEvent(event)).cmds
    }

    pub fn is_permission_modal_active(&self) -> bool {
        self.state.permission_modal.tool.is_some() && self.state.mode == TuiMode::Permission
    }

    pub fn toggle_sidebar(&mut self) {
        self.update(Msg::ToggleSidebar);
    }

    pub fn is_running(&self) -> bool {
        self.state.running
    }

    pub fn is_agent_running(&self) -> bool {
        self.state.agent_running
    }

    pub fn agent_start_time(&self) -> Option<std::time::Instant> {
        self.state.agent_start_time
    }

    pub fn messages(&self) -> &Vec<crate::components::MessageItem> {
        &self.state.messages
    }

    pub fn input_text(&self) -> String {
        self.state.textarea.lines().join("\n")
    }

    pub async fn set_permission(&self, decision: PermissionDecision) {
        self.permission_state.resolve(decision).await;
    }

    pub async fn clear_permission(&self) {
        self.permission_state.clear().await;
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
        code => {
            if let Some(k) = map_navigation_key(code) {
                k
            } else if let Some(k) = map_edit_key(code) {
                k
            } else if let Some(k) = map_special_key(code) {
                k
            } else {
                Key::Null
            }
        }
    };

    let ctrl = key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL);
    let alt = key.modifiers.contains(crossterm::event::KeyModifiers::ALT);
    let shift = key.modifiers.contains(crossterm::event::KeyModifiers::SHIFT);

    Input { key: key_code, ctrl, alt, shift }
}

// ─── Key mapping helpers ───────────────────────────────────────────────────────

fn map_arrow_keys(code: crossterm::event::KeyCode) -> Option<ratatui_textarea::Key> {
    use crossterm::event::KeyCode;
    use ratatui_textarea::Key;
    match code {
        KeyCode::Left => Some(Key::Left),
        KeyCode::Right => Some(Key::Right),
        KeyCode::Up => Some(Key::Up),
        KeyCode::Down => Some(Key::Down),
        _ => None,
    }
}

fn map_nav_page_keys(code: crossterm::event::KeyCode) -> Option<ratatui_textarea::Key> {
    use crossterm::event::KeyCode;
    use ratatui_textarea::Key;
    match code {
        KeyCode::Home => Some(Key::Home),
        KeyCode::End => Some(Key::End),
        KeyCode::PageUp => Some(Key::PageUp),
        KeyCode::PageDown => Some(Key::PageDown),
        _ => None,
    }
}

/// Map navigation keys (arrows, home, end, page up/down).
fn map_navigation_key(code: crossterm::event::KeyCode) -> Option<ratatui_textarea::Key> {
    map_arrow_keys(code).or_else(|| map_nav_page_keys(code))
}

/// Map edit keys (backspace, delete, tab, enter, escape).
fn map_edit_key(code: crossterm::event::KeyCode) -> Option<ratatui_textarea::Key> {
    use crossterm::event::KeyCode;
    use ratatui_textarea::Key;
    match code {
        KeyCode::Backspace => Some(Key::Backspace),
        KeyCode::Delete => Some(Key::Delete),
        KeyCode::Tab => Some(Key::Tab),
        KeyCode::Enter => Some(Key::Enter),
        KeyCode::Esc => Some(Key::Esc),
        _ => None,
    }
}

/// Map special keys (function keys, null).
fn map_special_key(code: crossterm::event::KeyCode) -> Option<ratatui_textarea::Key> {
    use crossterm::event::KeyCode;
    use ratatui_textarea::Key;
    match code {
        KeyCode::F(n) => Some(Key::F(n)),
        KeyCode::Null => Some(Key::Null),
        _ => None,
    }
}
