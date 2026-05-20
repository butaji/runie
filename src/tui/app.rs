use crate::core::executor::ExecEvent;
use crate::core::safety::{SafetyConfig, SafetyEnvelope};
use crate::router::ModelDatabase;
use crate::tui::{
    AgentsPanel, CheckpointAction, CommandAction, CommandPalette, CostHud, Header,
    HelpOverlay, Input, ModelSelector, SafetyCheckpoint, SkillsPanel, Stream,
};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::Rect,
    widgets::Clear,
    Terminal,
};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io;

/// Application modes
#[derive(Clone, Copy, PartialEq)]
enum AppMode {
    /// Typing in the command input bar
    Input,
    /// Navigating the stream with j/k
    Navigation,
    /// Command palette is open
    CommandPalette,
    /// Model selector is open
    ModelSelector,
    /// Cost HUD expanded
    CostHud,
    /// Agent swarm panel open
    Agents,
    /// Skills panel open (Tab cycles tabs)
    Skills,
    /// Help overlay open
    Help,
    /// Safety checkpoint blocking execution
    SafetyCheckpoint,
}

/// Main TUI application state
pub struct App {
    header: Header,
    stream: Stream,
    input: Input,
    command_palette: CommandPalette,
    model_selector: ModelSelector,
    cost_hud: CostHud,
    agents_panel: AgentsPanel,
    help_overlay: HelpOverlay,
    safety_checkpoint: SafetyCheckpoint,
    skills_panel: SkillsPanel,
    models: ModelDatabase,
    safety: SafetyEnvelope,
    mode: AppMode,
    paused: bool,
    current_model: String,
    /// Whether to auto-scroll stream to bottom on next draw
    first_draw: bool,
    /// Receives ExecEvents from the background executor thread
    exec_rx: tokio::sync::mpsc::Receiver<ExecEvent>,
    /// Sends user input to the background executor thread
    input_tx: tokio::sync::mpsc::Sender<crate::core::executor::ExecutorInput>,
}

impl App {
    fn new(
        exec_rx: tokio::sync::mpsc::Receiver<ExecEvent>,
        input_tx: tokio::sync::mpsc::Sender<crate::core::executor::ExecutorInput>,
    ) -> Self {
        Self {
            header: Header::new(),
            stream: Stream::new(),
            input: Input::new(),
            command_palette: CommandPalette::new(),
            model_selector: ModelSelector::new(),
            cost_hud: CostHud::new(),
            agents_panel: AgentsPanel::new(),
            help_overlay: HelpOverlay::new(),
            skills_panel: SkillsPanel::new(),
            safety_checkpoint: SafetyCheckpoint::new(),
            models: ModelDatabase::new(),
            safety: SafetyEnvelope::new(SafetyConfig::default()),
            mode: AppMode::Input,
            paused: false,
            current_model: "anthropic/claude-sonnet-4".to_string(),
            first_draw: true,
            exec_rx,
            input_tx,
        }
    }

    /// Returns true when the app should quit
    fn handle_key(&mut self, key: KeyEvent) -> bool {
        // Safety checkpoint intercepts all input
        if self.safety_checkpoint.visible {
            if let Some(action) = self.safety_checkpoint.handle_key(key.code) {
                self.handle_checkpoint_action(action);
            }
            return false;
        }

        match key.code {
            // ── Global quit ──────────────────────────────────
            KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => return true,

            // ── Help overlay ────────────────────────────────
            KeyCode::Char('?') => {
                self.help_overlay.toggle();
                if self.help_overlay.visible {
                    self.mode = AppMode::Help;
                } else {
                    self.mode = AppMode::Input;
                }
            }

            // ── Tab: toggle skills panel ──────────────────────
            KeyCode::Tab => {
                self.skills_panel.toggle();
                if self.skills_panel.visible {
                    self.mode = AppMode::Skills;
                    self.command_palette.hide();
                    self.model_selector.hide();
                    self.cost_hud.hide();
                    self.agents_panel.hide();
                    self.help_overlay.hide();
                } else {
                    self.mode = AppMode::Input;
                }
            }

            // ── Shift-Tab: switch to navigation mode ────────
            KeyCode::BackTab => {
                self.skills_panel.hide();
                self.mode = AppMode::Navigation;
            }

            // ── ^h: jump to top ─────────────────────────────
            KeyCode::Char('h') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.stream.selected = 0;
                self.stream.scroll_offset = 0;
            }

            // ── ^c: cancel one agent ────────────────────────
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.header.agent_count = self.header.agent_count.saturating_sub(1);
            }

            // ── ^$: cost HUD ────────────────────────────────
            KeyCode::Char('$') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.cost_hud.toggle();
                if self.cost_hud.visible {
                    self.mode = AppMode::CostHud;
                    self.command_palette.hide();
                    self.model_selector.hide();
                    self.agents_panel.hide();
                    self.help_overlay.hide();
                } else {
                    self.mode = AppMode::Input;
                }
            }

            // ── ^a: agent swarm ─────────────────────────────
            KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.agents_panel.toggle();
                if self.agents_panel.visible {
                    self.mode = AppMode::Agents;
                    self.command_palette.hide();
                    self.model_selector.hide();
                    self.cost_hud.hide();
                    self.help_overlay.hide();
                } else {
                    self.mode = AppMode::Input;
                }
            }

            // ── ^p: cycle previous model ────────────────────
            KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.cycle_model(-1);
            }

            // ── ^l: model selector ──────────────────────────
            KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.mode = AppMode::ModelSelector;
                self.model_selector.show();
                self.command_palette.hide();
                self.cost_hud.hide();
                self.agents_panel.hide();
                self.help_overlay.hide();
            }

            // ── / or >: command palette ─────────────────────
            KeyCode::Char('/') | KeyCode::Char('>') => {
                self.mode = AppMode::CommandPalette;
                self.command_palette.show();
                self.model_selector.hide();
                self.cost_hud.hide();
                self.agents_panel.hide();
                self.help_overlay.hide();
            }

            // ── Esc: close all panels ───────────────────────
            KeyCode::Esc => {
                self.command_palette.hide();
                self.model_selector.hide();
                self.cost_hud.hide();
                self.agents_panel.hide();
                self.help_overlay.hide();
                self.skills_panel.hide();
                self.mode = AppMode::Input;
            }

            // ── Mode-specific handlers ───────────────────────
            _ => match self.mode {
                AppMode::Input => {
                    self.input.handle_key(key);
                    // Enter → execute command input
                    if let KeyCode::Enter = key.code {
                        self.execute_input();
                    }
                }
                AppMode::Navigation => {
                    let term_height = 30; // estimate; actual used in draw
                    self.stream.handle_key(key.code, term_height);
                }
                AppMode::CommandPalette => {
                    if let Some(action) = self.command_palette.handle_key(key) {
                        self.execute_command(action);
                    }
                }
                AppMode::ModelSelector => {
                    self.model_selector
                        .handle_key(key.code, self.models.models.len());
                    if let KeyCode::Enter = key.code {
                        self.select_model();
                    }
                }
                AppMode::Agents => {
                    self.agents_panel.handle_key(key.code);
                }
                AppMode::Skills => {
                    self.skills_panel.handle_key(key);
                }
                AppMode::CostHud | AppMode::Help => {
                    // Only Esc closes these
                }
                AppMode::SafetyCheckpoint => {
                    // Handled above
                }
            },
        }
        false
    }

    fn execute_input(&mut self) {
        let text = self.input.text().trim();
        if text.is_empty() {
            return;
        }

        // Route commands
        if let Some(cmd) = text.strip_prefix('/') {
            let action = match cmd.trim_start_matches("spawn").trim() {
                s if s.starts_with("spawn") => Some(CommandAction::Spawn),
                "models" | "model" => Some(CommandAction::Models),
                "cost" | "budget" => Some(CommandAction::Cost),
                "pause" | "stop" => Some(CommandAction::Pause),
                "resume" | "continue" => Some(CommandAction::Resume),
                "cancel" | "abort" => Some(CommandAction::Cancel),
                "help" | "?" => {
                    self.help_overlay.show();
                    self.mode = AppMode::Help;
                    return;
                }
                "quit" | "exit" => {
                    // Would quit; for now just clear input
                    self.input.clear();
                    return;
                }
                _ => None,
            };

            if let Some(action) = action {
                self.execute_command(action);
            }
        } else {
            // Natural language input → intent → plan → DAG → stream
            // Add immediately as pending entry so user sees something happened
            self.stream.push_input_entry(text);
            // Send to background executor for processing
            self.send_to_executor(text);
        }

        self.input.clear();
    }

    fn execute_command(&mut self, action: CommandAction) {
        match action {
            CommandAction::Spawn => {
                self.input.set_text("/spawn ");
                self.mode = AppMode::Input;
            }
            CommandAction::Models => {
                self.mode = AppMode::ModelSelector;
                self.model_selector.show();
            }
            CommandAction::Cost => {
                self.cost_hud.show();
                self.mode = AppMode::CostHud;
            }
            CommandAction::Pause => {
                self.paused = true;
                self.header.agent_count = 0;
                self.mode = AppMode::Input;
            }
            CommandAction::Resume => {
                self.paused = false;
                self.header.agent_count = 4;
                self.mode = AppMode::Input;
            }
            CommandAction::Cancel => {
                if self.header.agent_count > 0 {
                    self.header.agent_count -= 1;
                }
                self.mode = AppMode::Input;
            }
            CommandAction::Help => {
                self.help_overlay.show();
                self.mode = AppMode::Help;
            }
            CommandAction::Quit => {
                // Would set quit flag
                self.mode = AppMode::Input;
            }
        }
    }

    fn cycle_model(&mut self, delta: i32) {
        let models: Vec<_> = self.models.models.keys().collect();
        if let Some(cur) = models.iter().position(|m| *m == &self.current_model) {
            let new_idx = ((cur as i32 + delta) % models.len() as i32)
                .wrapping_add(models.len() as i32) as usize
                % models.len();
            self.current_model = models[new_idx].clone();
            self.update_active_model();
        }
    }

    fn select_model(&mut self) {
        let models: Vec<_> = self.models.models.keys().collect();
        if self.model_selector.selected < models.len() {
            self.current_model = models[self.model_selector.selected].clone();
            self.update_active_model();
        }
        self.model_selector.hide();
        self.mode = AppMode::Input;
    }

    fn update_active_model(&mut self) {
        for (id, status) in self.models.statuses.iter_mut() {
            status.is_active = id == &self.current_model;
        }
        // Update header repo to show current model
        self.header.repo = self
            .current_model
            .split('/')
            .next_back()
            .unwrap_or("unknown")
            .to_string();
    }

    fn handle_checkpoint_action(&mut self, action: CheckpointAction) {
        match action {
            CheckpointAction::Approve => {
                // Safety envelope approved — continue
                self.safety_checkpoint.hide();
                self.mode = AppMode::Input;
            }
            CheckpointAction::Reject => {
                // Pause all agents
                self.paused = true;
                self.header.agent_count = 0;
                self.safety_checkpoint.hide();
                self.mode = AppMode::Input;
            }
            CheckpointAction::EditPlan => {
                // Return to input to edit plan
                self.safety_checkpoint.hide();
                self.input.set_text("/plan ");
                self.mode = AppMode::Input;
            }
        }
    }

    /// Track cost for the current model and update safety envelope
    fn track_cost(&mut self, amount: f64) {
        self.models.track_spend(&self.current_model, amount);
        if let Err(violation) = self.safety.check_cost(amount) {
            // Trigger safety checkpoint
            if let crate::core::safety::SafetyViolation::CostExceeded { limit, actual } = violation {
                self.safety_checkpoint.show_with(
                    format!(
                        "Cost limit exceeded: ${:.2} > ${:.2}",
                        actual, limit
                    ),
                    vec![],
                    crate::tui::safety_checkpoint::RiskLevel::High,
                );
                self.mode = AppMode::SafetyCheckpoint;
            }
        } else {
            self.safety.track_spend(amount);
        }
    }

    fn update_header(&mut self) {
        self.header.cost_spent = self.models.total_spent();
        self.header.entry_count = self.stream.entry_count();
    }

    /// Process any pending executor events and push to stream
    fn process_exec_events(&mut self, viewport_height: usize) {
        while let Ok(event) = self.exec_rx.try_recv() {
            let entry = event.to_stream_entry();
            self.stream.entries.push(entry);
            // Auto-scroll: keep near bottom
            if self.stream.entries.len() > 10 {
                self.stream.scroll_offset = self.stream.entries.len().saturating_sub(viewport_height / 2);
            }
            self.stream.selected = self.stream.entries.len().saturating_sub(1);
            // Track cost if step completed
            if let ExecEvent::StepCompleted { cost, .. } = &event {
                self.track_cost(*cost);
            }
        }
    }

    /// Send user input to the executor for processing
    fn send_to_executor(&self, text: &str) {
        let _ = self.input_tx.try_send(crate::core::executor::ExecutorInput::Execute(text.to_string()));
    }
}

impl Default for App {
    fn default() -> Self {
        // Default impl for tests — creates disconnected channels
        let (_, exec_rx) = tokio::sync::mpsc::channel::<ExecEvent>(32);
        let (input_tx, _) = tokio::sync::mpsc::channel(32);
        Self::new(exec_rx, input_tx)
    }
}

/// Compute stream height from terminal height
fn stream_height(terminal_height: u16) -> usize {
    (terminal_height.saturating_sub(6)) as usize
}

/// Run the TUI event loop
pub fn run() -> anyhow::Result<()> {
    // ── Executor channels ──────────────────────────────────
    // exec_rx: TUI receives ExecEvents from executor
    // input_tx: TUI sends user input text to executor for processing
    let (exec_rx, input_tx) = crate::core::executor::start_executor_thread();

    // ── TUI setup ─────────────────────────────────────────
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(exec_rx, input_tx);

    loop {
        app.update_header();

        // Process executor events (non-blocking)
        if let Ok(size) = terminal.size() {
            app.process_exec_events(stream_height(size.height));
        }

        // Auto-scroll to bottom on first draw
        if app.first_draw {
            if let Ok(size) = terminal.size() {
                let sh = stream_height(size.height);
                app.stream.scroll_to_bottom(sh);
            }
            app.first_draw = false;
        }

        terminal.draw(|f| {
            let size = f.area();
            let header_height = 3u16;
            let input_height = 3u16;
            let stream_height = size
                .height
                .saturating_sub(header_height + input_height);

            // ── Header ──────────────────────────────────────
            let header_area = Rect::new(0, 0, size.width, header_height);
            f.render_widget(app.header.render(), header_area);

            // ── Stream ─────────────────────────────────────
            let stream_area = Rect::new(0, header_height, size.width, stream_height);
            let sh = stream_height as usize;
            let (para, _scrollbar, _sb_state) = app.stream.render(sh as u16);
            f.render_widget(para, stream_area);

            // ── Input bar ───────────────────────────────────
            let input_area = Rect::new(0, size.height - input_height, size.width, input_height);
            f.render_widget(app.input.render(), input_area);

            // ── Help overlay ─────────────────────────────────
            if app.help_overlay.visible {
                let pw = 50.min(size.width.saturating_sub(4));
                let ph = 35.min(size.height.saturating_sub(4));
                let px = (size.width - pw) / 2;
                let py = (size.height - ph) / 2;
                let area = Rect::new(px, py, pw, ph);
                f.render_widget(Clear, area);
                f.render_widget(app.help_overlay.render(), area);
            }

            // ── Cost HUD ────────────────────────────────────
            if app.cost_hud.visible {
                let cw = 48.min(size.width.saturating_sub(4));
                let ch = 18.min(size.height.saturating_sub(4));
                let cx = (size.width - cw) / 2;
                let cy = (size.height - ch) / 2;
                let area = Rect::new(cx, cy, cw, ch);
                f.render_widget(Clear, area);
                f.render_widget(app.cost_hud.render(&app.models), area);
            }

            // ── Agent swarm panel ───────────────────────────
            if app.agents_panel.visible {
                let aw = 50.min(size.width.saturating_sub(4));
                let ah = app.agents_panel.panel_height()
                    .min(size.height.saturating_sub(4));
                let ax = (size.width - aw) / 2;
                let ay = (size.height - ah) / 2;
                let area = Rect::new(ax, ay, aw, ah);
                f.render_widget(Clear, area);
                f.render_widget(app.agents_panel.render(), area);
            }

            // ── Skills panel ─────────────────────────────────
            if app.skills_panel.visible {
                let sw = 55.min(size.width.saturating_sub(4));
                let sh = app.skills_panel.panel_height()
                    .min(size.height.saturating_sub(4));
                let sx = (size.width - sw) / 2;
                let sy = (size.height - sh) / 2;
                let area = Rect::new(sx, sy, sw, sh);
                f.render_widget(Clear, area);
                app.skills_panel.render(f, area);
            }

            // ── Command palette ─────────────────────────────
            if app.command_palette.visible {
                let pw = 50.min(size.width.saturating_sub(4));
                let ph = 15.min(size.height.saturating_sub(4));
                let px = (size.width - pw) / 2;
                let py = (size.height - ph) / 2;
                let area = Rect::new(px, py, pw, ph);
                f.render_widget(Clear, area);
                f.render_widget(app.command_palette.render(), area);
            }

            // ── Model selector ──────────────────────────────
            if app.model_selector.visible {
                let sw = 55.min(size.width.saturating_sub(4));
                let sh = ((app.models.models.len() + 5) as u16)
                    .min(size.height.saturating_sub(4));
                let sx = (size.width - sw) / 2;
                let sy = (size.height - sh) / 2;
                let area = Rect::new(sx, sy, sw, sh);
                f.render_widget(Clear, area);
                f.render_widget(app.model_selector.render(&app.models), area);
            }

            // ── Safety checkpoint ────────────────────────────
            if app.safety_checkpoint.visible {
                let sw = 58.min(size.width.saturating_sub(4));
                let sh = 22.min(size.height.saturating_sub(4));
                let sx = (size.width - sw) / 2;
                let sy = (size.height - sh) / 2;
                let area = Rect::new(sx, sy, sw, sh);
                f.render_widget(Clear, area);
                f.render_widget(app.safety_checkpoint.render(), area);
            }
        })?;

        // ── Keyboard input ──────────────────────────────────
        if crossterm::event::poll(std::time::Duration::from_millis(100))? {
            if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
                if app.handle_key(key) {
                    break;
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    Ok(())
}
