use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::Rect,
    widgets::Clear,
};
use crossterm::{
    execute,
    terminal::{enable_raw_mode, disable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    event::{EnableMouseCapture, DisableMouseCapture, KeyEvent, KeyCode},
};
use std::io;

use crate::router::ModelDatabase;
use crate::tui::{
    command::{CommandPalette, CommandAction},
    header::Header,
    input::Input,
    selector::ModelSelector,
    stream::Stream,
};

pub struct App {
    header: Header,
    stream: Stream,
    input: Input,
    command_palette: CommandPalette,
    model_selector: ModelSelector,
    models: ModelDatabase,
    mode: AppMode,
    paused: bool,
    current_model: String,
}

enum AppMode {
    Input,
    Navigation,
    CommandPalette,
    ModelSelector,
}

impl App {
    fn new() -> Self {
        Self {
            header: Header::new(),
            stream: Stream::new(),
            input: Input::new(),
            command_palette: CommandPalette::new(),
            model_selector: ModelSelector::new(),
            models: ModelDatabase::new(),
            mode: AppMode::Input,
            paused: false,
            current_model: "anthropic/claude-sonnet-4".to_string(),
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char('q') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                return true; // Quit
            }
            KeyCode::Tab => {
                self.mode = match self.mode {
                    AppMode::Input => AppMode::Navigation,
                    AppMode::Navigation => AppMode::Input,
                    AppMode::CommandPalette => AppMode::Input,
                    AppMode::ModelSelector => AppMode::Input,
                };
                self.command_palette.hide();
                self.model_selector.hide();
            }
            KeyCode::Char('h') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                self.stream.selected = 0; // Home
            }
            KeyCode::Char('l') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                self.mode = AppMode::ModelSelector;
                self.model_selector.show();
            }
            KeyCode::Char('/') | KeyCode::Char('>') => {
                self.mode = AppMode::CommandPalette;
                self.command_palette.show();
            }
            KeyCode::Esc => {
                self.mode = AppMode::Input;
                self.command_palette.hide();
                self.model_selector.hide();
            }
            _ => {
                match self.mode {
                    AppMode::Input => {
                        self.input.handle_key(key);
                    }
                    AppMode::Navigation => {
                        self.stream.handle_key(key.code);
                    }
                    AppMode::CommandPalette => {
                        if let Some(action) = self.command_palette.handle_key(key) {
                            self.execute_command(action);
                        }
                    }
                    AppMode::ModelSelector => {
                        self.model_selector.handle_key(key.code, self.models.models.len());
                        if let KeyCode::Enter = key.code {
                            self.select_model();
                        }
                    }
                }
            }
        }
        false
    }

    fn execute_command(&mut self, action: CommandAction) {
        match action {
            CommandAction::Spawn => {
                self.input.set_text("/spawn ");
            }
            CommandAction::Models => {
                self.mode = AppMode::ModelSelector;
                self.model_selector.show();
            }
            CommandAction::Cost => {
                // Mock cost update
                self.header.cost_spent += 0.01;
            }
            CommandAction::Pause => {
                self.paused = true;
                self.header.agent_count = 0;
            }
            CommandAction::Resume => {
                self.paused = false;
                self.header.agent_count = 4;
            }
            CommandAction::Cancel => {}
            CommandAction::Help => {}
            CommandAction::Quit => {}
        }
    }

    fn select_model(&mut self) {
        let models: Vec<_> = self.models.models.keys().collect();
        if self.model_selector.selected < models.len() {
            self.current_model = models[self.model_selector.selected].clone();
            
            // Update status
            for (id, status) in self.models.statuses.iter_mut() {
                status.is_active = id == &self.current_model;
            }
        }
        self.model_selector.hide();
        self.mode = AppMode::Input;
    }

    fn update_header(&mut self) {
        // Update header with current model info
        self.header.repo = self.current_model.split('/').last().unwrap_or("unknown").to_string();
        self.header.cost_spent = self.models.total_spent();
        
        // Count active models
        let active_count = self.models.statuses.values()
            .filter(|s| s.is_active)
            .count();
        if active_count > 0 {
            self.header.agent_count = active_count;
        }
    }
}

pub fn run() -> anyhow::Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new();

    // Fetch models in background (skip for now in skeleton)
    // tokio::spawn(async move {
    //     app.models.fetch_from_models_dev().await;
    // });

    // Main event loop
    loop {
        app.update_header();
        
        terminal.draw(|f| {
            let size = f.area();
            
            // Calculate layout
            let header_height = 3;
            let input_height = 3;
            let stream_height = size.height.saturating_sub(header_height + input_height);

            // Render header
            let header_area = Rect::new(0, 0, size.width, header_height);
            f.render_widget(app.header.render(), header_area);

            // Render stream
            let stream_area = Rect::new(0, header_height, size.width, stream_height);
            f.render_widget(app.stream.render(), stream_area);

            // Render input at bottom
            let input_area = Rect::new(0, size.height - input_height, size.width, input_height);
            f.render_widget(app.input.render(), input_area);

            // Render command palette overlay if visible
            if app.command_palette.visible {
                let palette_width = 50.min(size.width.saturating_sub(4));
                let palette_height = 15.min(size.height.saturating_sub(4));
                let palette_x = (size.width - palette_width) / 2;
                let palette_y = (size.height - palette_height) / 2;
                let palette_area = Rect::new(palette_x, palette_y, palette_width, palette_height);
                
                f.render_widget(Clear, palette_area);
                f.render_widget(app.command_palette.render(), palette_area);
            }

            // Render model selector overlay if visible
            if app.model_selector.visible {
                let selector_width = 55.min(size.width.saturating_sub(4));
                let selector_height = ((app.models.models.len() + 5) as u16).min(size.height.saturating_sub(4));
                let selector_x = (size.width - selector_width) / 2;
                let selector_y = (size.height - selector_height) / 2;
                let selector_area = Rect::new(selector_x, selector_y, selector_width, selector_height);
                
                f.render_widget(Clear, selector_area);
                f.render_widget(app.model_selector.render(&app.models), selector_area);
            }
        })?;

        // Handle input with timeout
        if crossterm::event::poll(std::time::Duration::from_millis(100))? {
            if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
                if app.handle_key(key) {
                    break;
                }
            }
        }
    }

    // Cleanup
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    Ok(())
}
