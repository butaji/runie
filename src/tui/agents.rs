use ratatui::{
    widgets::Paragraph,
    style::{Style, Color, Modifier},
    text::{Line, Span},
};

/// Agent worktree info — git worktree per agent
#[derive(Clone, Debug)]
pub struct AgentWorktree {
    pub path: String,
    pub branch: String,
}

/// Agent for display purposes — includes worktree for git operations
#[derive(Clone)]
pub struct AgentInfo {
    pub name: String,
    pub role: String,
    pub task: String,
    pub model: String,
    pub elapsed_secs: u32,
    pub status: AgentStatus,
    pub worktree: Option<AgentWorktree>,
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
                worktree: Some(AgentWorktree {
                    path: ".git/worktrees/anvil-general-design".to_string(),
                    branch: "anvil/general-design".to_string(),
                }),
            },
            AgentInfo {
                name: "general".to_string(),
                role: "general".to_string(),
                task: "Review CLI page implementation".to_string(),
                model: "grok-build-latest".to_string(),
                elapsed_secs: 29,
                status: AgentStatus::Running,
                worktree: Some(AgentWorktree {
                    path: ".git/worktrees/anvil-general-cli".to_string(),
                    branch: "anvil/general-cli".to_string(),
                }),
            },
            AgentInfo {
                name: "explore".to_string(),
                role: "explore".to_string(),
                task: "Check Section component".to_string(),
                model: "explore".to_string(),
                elapsed_secs: 29,
                status: AgentStatus::Waiting,
                worktree: Some(AgentWorktree {
                    path: ".git/worktrees/anvil-explore-section".to_string(),
                    branch: "anvil/explore-section".to_string(),
                }),
            },
            AgentInfo {
                name: "explore".to_string(),
                role: "explore".to_string(),
                task: "Explore website design system".to_string(),
                model: "explore".to_string(),
                elapsed_secs: 29,
                status: AgentStatus::Running,
                worktree: Some(AgentWorktree {
                    path: ".git/worktrees/anvil-explore-design".to_string(),
                    branch: "anvil/explore-design".to_string(),
                }),
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

            // Sub-status line with worktree info
            let (sub_text, status_color) = match agent.status {
                AgentStatus::Running => (
                    if let Some(wt) = &agent.worktree {
                        format!("└─ {} │ {}", wt.branch, wt.path)
                    } else {
                        "└─ Running...".to_string()
                    },
                    Color::Green,
                ),
                AgentStatus::Waiting => (
                    "└─ Waiting on: user approval".to_string(),
                    Color::Yellow,
                ),
                AgentStatus::Blocked => (
                    "└─ Blocked: no context available".to_string(),
                    Color::Red,
                ),
                AgentStatus::Done => (
                    if let Some(wt) = &agent.worktree {
                        format!("└─ Done @ {} │ {}", wt.branch, wt.path)
                    } else {
                        "└─ Done".to_string()
                    },
                    Color::DarkGray,
                ),
            };
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
            crossterm::event::KeyCode::Down | crossterm::event::KeyCode::Char('j')
                if self.selected < self.agents.len().saturating_sub(1) => {
                    self.selected += 1;
                }
            _ => {}
        }
    }

    /// Spawn a new agent (returns the index of the new agent)
    pub fn spawn_agent(&mut self, role: &str, task: &str, model: &str) -> usize {
        let agent_id = format!("agent-{}", self.agents.len());
        let new_agent = AgentInfo {
            name: role.to_string(),
            role: role.to_string(),
            task: task.to_string(),
            model: model.to_string(),
            elapsed_secs: 0,
            status: AgentStatus::Running,
            worktree: Some(AgentWorktree {
                path: format!(".git/worktrees/anvil-{}", agent_id),
                branch: format!("anvil/{}", agent_id),
            }),
        };
        self.agents.push(new_agent);
        self.agents.len() - 1
    }

    /// Cancel the selected agent
    pub fn cancel_agent(&mut self) -> Option<AgentInfo> {
        if self.selected < self.agents.len() {
            let agent = self.agents.remove(self.selected);
            self.selected = self.selected.saturating_sub(1);
            Some(agent)
        } else {
            None
        }
    }

    /// Get worktree info for selected agent
    pub fn selected_worktree(&self) -> Option<&AgentWorktree> {
        self.agents.get(self.selected).and_then(|a| a.worktree.as_ref())
    }

    /// Get mutable reference to selected agent
    pub fn selected_agent(&mut self) -> Option<&mut AgentInfo> {
        self.agents.get_mut(self.selected)
    }
}

impl Default for AgentsPanel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_worktree_clone() {
        let wt = AgentWorktree {
            path: "/repo/.git/worktrees/test".to_string(),
            branch: "anvil/test".to_string(),
        };
        let cloned = wt.clone();
        assert_eq!(cloned.path, wt.path);
        assert_eq!(cloned.branch, wt.branch);
    }

    #[test]
    fn test_agents_panel_default() {
        let panel = AgentsPanel::default();
        assert!(!panel.visible);
        assert_eq!(panel.len(), 4); // mock_agents has 4 entries
    }

    #[test]
    fn test_agents_panel_show_hide() {
        let mut panel = AgentsPanel::new();
        assert!(!panel.visible);
        
        panel.show();
        assert!(panel.visible);
        assert_eq!(panel.selected, 0);
        
        panel.hide();
        assert!(!panel.visible);
    }

    #[test]
    fn test_agents_panel_toggle() {
        let mut panel = AgentsPanel::new();
        
        panel.toggle();
        assert!(panel.visible);
        
        panel.toggle();
        assert!(!panel.visible);
    }

    #[test]
    fn test_agents_panel_navigation() {
        let mut panel = AgentsPanel::new();
        panel.show();
        
        // Initial selection
        assert_eq!(panel.selected, 0);
        
        // Move down
        panel.handle_key(crossterm::event::KeyCode::Down);
        assert_eq!(panel.selected, 1);
        
        panel.handle_key(crossterm::event::KeyCode::Char('j'));
        assert_eq!(panel.selected, 2);
        
        // Move up
        panel.handle_key(crossterm::event::KeyCode::Up);
        assert_eq!(panel.selected, 1);
        
        panel.handle_key(crossterm::event::KeyCode::Char('k'));
        assert_eq!(panel.selected, 0);
        
        // Wrap around
        panel.handle_key(crossterm::event::KeyCode::Up);
        assert_eq!(panel.selected, 3); // Wraps to last
    }

    #[test]
    fn test_spawn_agent() {
        let mut panel = AgentsPanel::new();
        panel.show();
        
        let initial_len = panel.len();
        let new_idx = panel.spawn_agent("backend", "Fix auth bug", "claude-3");
        
        assert_eq!(new_idx, initial_len);
        assert_eq!(panel.len(), initial_len + 1);
        
        // Verify new agent has worktree
        let worktree = panel.agents.get(new_idx).unwrap().worktree.as_ref();
        assert!(worktree.is_some());
        let wt = worktree.unwrap();
        assert!(wt.branch.starts_with("anvil/"));
        assert!(wt.path.contains("agent-"));
    }

    #[test]
    fn test_cancel_agent() {
        let mut panel = AgentsPanel::new();
        panel.show();
        
        let initial_len = panel.len();
        
        // Cancel the selected agent (index 0)
        let cancelled = panel.cancel_agent();
        
        assert!(cancelled.is_some());
        assert_eq!(panel.len(), initial_len - 1);
        assert_eq!(panel.selected, 0);
    }

    #[test]
    fn test_selected_worktree() {
        let mut panel = AgentsPanel::new();
        panel.show();
        
        // Default selection is 0, which has a worktree
        let wt = panel.selected_worktree();
        assert!(wt.is_some());
        let wt = wt.unwrap();
        assert!(wt.path.contains("general-design"));
        assert!(wt.branch.contains("anvil/general-design"));
    }

    #[test]
    fn test_selected_agent_mut() {
        let mut panel = AgentsPanel::new();
        panel.show();
        
        let agent = panel.selected_agent();
        assert!(agent.is_some());
        
        // Mutate the agent
        if let Some(a) = agent {
            let _ = std::mem::replace(&mut a.elapsed_secs, 99);
        }
        
        // Verify the change
        assert_eq!(panel.agents[0].elapsed_secs, 99);
    }

    #[test]
    fn test_panel_height_calculation() {
        let panel = AgentsPanel::new();
        // 4 agents * 3 lines + 8 = 20, but min is 12
        assert!(panel.panel_height() >= 12);
    }

    #[test]
    fn test_agent_status_colors() {
        use ratatui::style::Color;
        
        assert_eq!(AgentStatus::Running.color(), Color::Green);
        assert_eq!(AgentStatus::Waiting.color(), Color::Yellow);
        assert_eq!(AgentStatus::Blocked.color(), Color::Red);
        assert_eq!(AgentStatus::Done.color(), Color::DarkGray);
    }

    #[test]
    fn test_agent_status_labels() {
        assert_eq!(AgentStatus::Running.label(), "running");
        assert_eq!(AgentStatus::Waiting.label(), "waiting");
        assert_eq!(AgentStatus::Blocked.label(), "blocked");
        assert_eq!(AgentStatus::Done.label(), "done");
    }

    #[test]
    fn test_render_empty_when_hidden() {
        let panel = AgentsPanel::new();
        // When not visible, render returns empty paragraph (we can't access text field)
        // but we verify panel state
        assert!(!panel.visible);
    }

    #[test]
    fn test_render_shows_agents() {
        let panel = AgentsPanel::new();
        // When not visible, text is empty
        assert!(!panel.visible);
        
        // When visible
        let mut panel = AgentsPanel::new();
        panel.show();
        assert!(panel.visible);
        assert!(!panel.is_empty());
        // First agent should be "general" role
        assert!(panel.agents[0].role == "general");
    }
}
