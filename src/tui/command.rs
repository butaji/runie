use ratatui::{
    widgets::Paragraph,
    style::{Style, Color, Modifier},
    text::{Line, Span},
};

#[derive(Clone)]
pub struct Command {
    pub name: String,
    pub description: String,
    pub aliases: Vec<String>,
    pub action: CommandAction,
}

#[derive(Clone)]
pub enum CommandAction {
    Spawn,
    Models,
    Cost,
    Pause,
    Resume,
    Cancel,
    Help,
    Quit,
}

pub struct CommandPalette {
    pub commands: Vec<Command>,
    pub selected: usize,
    pub query: String,
    pub visible: bool,
}

impl CommandPalette {
    pub fn new() -> Self {
        let commands = vec![
            Command {
                name: "spawn".to_string(),
                description: "Spawn a new agent".to_string(),
                aliases: vec!["/spawn".to_string(), "new".to_string()],
                action: CommandAction::Spawn,
            },
            Command {
                name: "models".to_string(),
                description: "Show model selector".to_string(),
                aliases: vec!["/models".to_string(), "model".to_string()],
                action: CommandAction::Models,
            },
            Command {
                name: "cost".to_string(),
                description: "Show cost breakdown".to_string(),
                aliases: vec!["/cost".to_string(), "budget".to_string()],
                action: CommandAction::Cost,
            },
            Command {
                name: "pause".to_string(),
                description: "Pause all agents".to_string(),
                aliases: vec!["/pause".to_string(), "stop".to_string()],
                action: CommandAction::Pause,
            },
            Command {
                name: "resume".to_string(),
                description: "Resume paused agents".to_string(),
                aliases: vec!["/resume".to_string(), "continue".to_string()],
                action: CommandAction::Resume,
            },
            Command {
                name: "cancel".to_string(),
                description: "Cancel current task".to_string(),
                aliases: vec!["/cancel".to_string(), "abort".to_string()],
                action: CommandAction::Cancel,
            },
            Command {
                name: "help".to_string(),
                description: "Show keyboard shortcuts".to_string(),
                aliases: vec!["/help".to_string(), "?".to_string()],
                action: CommandAction::Help,
            },
            Command {
                name: "quit".to_string(),
                description: "Exit anvil".to_string(),
                aliases: vec!["/quit".to_string(), "exit".to_string()],
                action: CommandAction::Quit,
            },
        ];

        Self {
            commands,
            selected: 0,
            query: String::new(),
            visible: false,
        }
    }

    pub fn show(&mut self) {
        self.visible = true;
        self.query.clear();
        self.selected = 0;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn filtered_commands(&self) -> Vec<&Command> {
        if self.query.is_empty() {
            return self.commands.iter().collect();
        }

        let query_lower = self.query.to_lowercase();
        self.commands
            .iter()
            .filter(|cmd| {
                cmd.name.to_lowercase().contains(&query_lower)
                    || cmd.description.to_lowercase().contains(&query_lower)
                    || cmd.aliases.iter().any(|a| a.to_lowercase().contains(&query_lower))
            })
            .collect()
    }

    pub fn render(&self) -> Paragraph<'_> {
        if !self.visible {
            return Paragraph::new("");
        }

        let mut lines = vec![
            Line::from(vec![
                Span::raw("┌─ "),
                Span::styled("Command Palette", Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(" ────────────────────────────────"),
            ]),
            Line::from(vec![
                Span::raw("│  "),
                Span::styled(">", Style::new().fg(Color::Green)),
                Span::raw(format!(" {}", self.query)),
                Span::raw(" _"),
            ]),
            Line::from("├───────────────────────────────────────────".to_string()),
        ];

        for (i, cmd) in self.filtered_commands().iter().enumerate() {
            let selected = i == self.selected;
            let prefix = if selected { "▶" } else { " " };
            let style = if selected {
                Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::new().fg(Color::White)
            };

            lines.push(Line::from(vec![
                Span::raw(format!("{} │  ", prefix)),
                Span::styled(format!("/{}", cmd.name), style),
                Span::raw("  "),
                Span::styled(cmd.description.clone(), Style::new().fg(Color::DarkGray)),
            ]));
        }

        lines.push(Line::from("├───────────────────────────────────────────".to_string()));
        lines.push(Line::from(vec![
            Span::raw("   "),
            Span::styled("↑/↓", Style::new().fg(Color::Blue)),
            Span::raw(" navigate  "),
            Span::styled("Enter", Style::new().fg(Color::Blue)),
            Span::raw(" execute  "),
            Span::styled("Esc", Style::new().fg(Color::Blue)),
            Span::raw(" close"),
        ]));
        lines.push(Line::from("└───────────────────────────────────────────".to_string()));

        Paragraph::new(lines)
            .style(Style::new().fg(Color::White))
    }

    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> Option<CommandAction> {
        match key.code {
            crossterm::event::KeyCode::Esc => {
                self.hide();
            }
            crossterm::event::KeyCode::Char('k') => {
                let filtered = self.filtered_commands();
                if self.selected > 0 {
                    self.selected -= 1;
                } else {
                    self.selected = filtered.len().saturating_sub(1);
                }
            }
            crossterm::event::KeyCode::Char('j') => {
                let filtered = self.filtered_commands();
                if self.selected < filtered.len().saturating_sub(1) {
                    self.selected += 1;
                } else {
                    self.selected = 0;
                }
            }
            crossterm::event::KeyCode::Up => {
                let filtered = self.filtered_commands();
                if self.selected > 0 {
                    self.selected -= 1;
                } else {
                    self.selected = filtered.len().saturating_sub(1);
                }
            }
            crossterm::event::KeyCode::Down => {
                let filtered = self.filtered_commands();
                if self.selected < filtered.len().saturating_sub(1) {
                    self.selected += 1;
                } else {
                    self.selected = 0;
                }
            }
            crossterm::event::KeyCode::Char(c) => {
                self.query.push(c);
                self.selected = 0;
            }
            crossterm::event::KeyCode::Backspace => {
                self.query.pop();
                self.selected = 0;
            }
            crossterm::event::KeyCode::Enter => {
                let filtered = self.filtered_commands();
                if !filtered.is_empty() && self.selected < filtered.len() {
                    let action = filtered[self.selected].action.clone();
                    self.hide();
                    return Some(action);
                }
            }
            _ => {}
        }
        None
    }
}

impl Default for CommandPalette {
    fn default() -> Self {
        Self::new()
    }
}
