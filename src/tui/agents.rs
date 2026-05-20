use ratatui::{
    widgets::Paragraph,
    style::{Style, Color, Modifier},
    text::{Line, Span},
};

/// Mock agent for display purposes
#[derive(Clone)]
pub struct AgentInfo {
    pub name: String,
    pub role: String,
    pub task: String,
    pub model: String,
    pub elapsed_secs: u32,
    pub status: AgentStatus,
}

#[derive(Clone, Debug)]
pub enum AgentStatus {
    Running,
    Waiting,
    Blocked,
    Done,
}

impl AgentStatus {
    fn label(&self) -> &'static str {
        match self {
            AgentStatus::Running => "running",
            AgentStatus::Waiting => "waiting",
            AgentStatus::Blocked => "blocked",
            AgentStatus::Done => "done",
        }
    }

    fn color(&self) -> Color {
        match self {
            AgentStatus::Running => Color::Green,
            AgentStatus::Waiting => Color::Yellow,
            AgentStatus::Blocked => Color::Red,
            AgentStatus::Done => Color::DarkGray,
        }
    }
}

/// Agent swarm panel — shown when ^a is pressed
pub struct AgentsPanel {
    pub visible: bool,
    pub selected: usize,
    /// Mock agents for display
    agents: Vec<AgentInfo>,
}

impl AgentsPanel {
    pub fn new() -> Self {
        Self {
            visible: false,
            selected: 0,
            agents: Self::mock_agents(),
        }
    }

    fn mock_agents() -> Vec<AgentInfo> {
        vec![
            AgentInfo {
                name: "general".to_string(),
                role: "general".to_string(),
                task: "Suggest design improvements".to_string(),
                model: "grok-build-latest".to_string(),
                elapsed_secs: 29,
                status: AgentStatus::Running,
            },
            AgentInfo {
                name: "general".to_string(),
                role: "general".to_string(),
                task: "Review CLI page implementation".to_string(),
                model: "grok-build-latest".to_string(),
                elapsed_secs: 29,
                status: AgentStatus::Running,
            },
            AgentInfo {
                name: "explore".to_string(),
                role: "explore".to_string(),
                task: "Check Section component".to_string(),
                model: "explore".to_string(),
                elapsed_secs: 29,
                status: AgentStatus::Waiting,
            },
            AgentInfo {
                name: "explore".to_string(),
                role: "explore".to_string(),
                task: "Explore website design system".to_string(),
                model: "explore".to_string(),
                elapsed_secs: 29,
                status: AgentStatus::Running,
            },
        ]
    }

    pub fn show(&mut self) {
        self.visible = true;
        self.selected = 0;
        self.agents = Self::mock_agents();
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        if self.visible {
            self.selected = 0;
        }
    }

    /// Number of agents
    pub fn len(&self) -> usize {
        self.agents.len()
    }

    pub fn is_empty(&self) -> bool {
        self.agents.is_empty()
    }

    /// Approximate rendered height of the panel (in lines)
    pub fn panel_height(&self) -> u16 {
        ((self.agents.len() * 3 + 8) as u16).max(12)
    }

    pub fn render(&self) -> Paragraph<'_> {
        if !self.visible {
            return Paragraph::new("");
        }

        let running = self.agents.iter().filter(|a| matches!(a.status, AgentStatus::Running)).count();
        let waiting = self.agents.iter().filter(|a| matches!(a.status, AgentStatus::Waiting)).count();
        let blocked = self.agents.iter().filter(|a| matches!(a.status, AgentStatus::Blocked)).count();

        let mut lines = vec![
            Line::from(vec![
                Span::raw("┌─ "),
                Span::styled(format!("{} agents", self.agents.len()), Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(format!(" ({} running, {} waiting, {} blocked)", running, waiting, blocked)),
                Span::raw(" ─────────────────────────────────"),
            ]),
            Line::from("├────────────────────────────────────────────────".to_string()),
        ];

        for (i, agent) in self.agents.iter().enumerate() {
            let selected = i == self.selected;
            let prefix = if selected { "▶" } else { " " };
            let bullet = if selected { "•" } else { " " };

            let name_style = if selected {
                Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::new().fg(Color::White)
            };

            lines.push(Line::from(vec![
                Span::raw(format!("{} {} ", prefix, bullet)),
                Span::styled(&agent.role, name_style),
                Span::raw(format!("  {}", agent.task)),
                Span::raw("  "),
                Span::styled(&agent.model, Style::new().fg(Color::DarkGray)),
                Span::raw(format!("  [{}s]", agent.elapsed_secs)),
            ]));

            // Sub-status line
            let sub_text = match agent.status {
                AgentStatus::Running => "└─ Running...".to_string(),
                AgentStatus::Waiting => "└─ Waiting on: user approval".to_string(),
                AgentStatus::Blocked => "└─ Blocked: no context available".to_string(),
                AgentStatus::Done => "└─ Done".to_string(),
            };
            let status_color = agent.status.color();
            lines.push(Line::from(vec![
                Span::raw("       ".to_string()),
                Span::styled(sub_text, Style::new().fg(status_color)),
            ]));
        }

        lines.push(Line::from("├────────────────────────────────────────────────".to_string()));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("[+ Spawn subagent]", Style::new().fg(Color::Green)),
            Span::raw("  "),
            Span::styled("[▼ View worktree]", Style::new().fg(Color::Blue)),
            Span::raw("  "),
            Span::styled("[✕ Cancel]", Style::new().fg(Color::Red)),
        ]));
        lines.push(Line::from("├────────────────────────────────────────────────".to_string()));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("↑/↓", Style::new().fg(Color::Blue)),
            Span::raw(" select  "),
            Span::styled("^a", Style::new().fg(Color::Blue)),
            Span::raw(" toggle  "),
            Span::styled("Esc", Style::new().fg(Color::Blue)),
            Span::raw(" close"),
        ]));
        lines.push(Line::from("└────────────────────────────────────────────────".to_string()));

        Paragraph::new(lines).style(Style::new().fg(Color::White))
    }

    pub fn handle_key(&mut self, key: crossterm::event::KeyCode) {
        match key {
            crossterm::event::KeyCode::Up | crossterm::event::KeyCode::Char('k') => {
                if self.selected > 0 {
                    self.selected -= 1;
                } else {
                    self.selected = self.agents.len().saturating_sub(1);
                }
            }
            crossterm::event::KeyCode::Down | crossterm::event::KeyCode::Char('j') => {
                if self.selected < self.agents.len().saturating_sub(1) {
                    self.selected += 1;
                }
            }
            _ => {}
        }
    }
}

impl Default for AgentsPanel {
    fn default() -> Self {
        Self::new()
    }
}
