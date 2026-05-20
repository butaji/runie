use ratatui::{
    widgets::Paragraph,
    style::{Style, Color, Modifier},
    text::{Line, Span},
};

/// Help overlay — shown when user presses ?
pub struct HelpOverlay {
    pub visible: bool,
}

impl HelpOverlay {
    pub fn new() -> Self {
        Self { visible: false }
    }

    pub fn show(&mut self) {
        self.visible = true;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    pub fn render(&self) -> Paragraph<'_> {
        if !self.visible {
            return Paragraph::new("");
        }

        let lines = vec![
            Line::from(vec![
                Span::raw("┌─ "),
                Span::styled("Anvil Help", Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(" ────────────────────────────────────────────"),
            ]),
            Line::from("│".to_string()),
            Line::from(vec![
                Span::raw("│  "),
                Span::styled("Navigation", Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::raw("│  "),
                Span::styled("j / k", Style::new().fg(Color::Green)),
                Span::raw("          Navigate stream entries"),
            ]),
            Line::from(vec![
                Span::raw("│  "),
                Span::styled("Space", Style::new().fg(Color::Green)),
                Span::raw("          Expand / collapse entry"),
            ]),
            Line::from(vec![
                Span::raw("│  "),
                Span::styled("PgUp / PgDn", Style::new().fg(Color::Green)),
                Span::raw("    Page through stream"),
            ]),
            Line::from(vec![
                Span::raw("│  "),
                Span::styled("^h", Style::new().fg(Color::Green)),
                Span::raw("          Jump to top of stream"),
            ]),
            Line::from("│".to_string()),
            Line::from(vec![
                Span::raw("│  "),
                Span::styled("Panels & Modes", Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::raw("│  "),
                Span::styled("/", Style::new().fg(Color::Green)),
                Span::raw(" or "),
                Span::styled(">", Style::new().fg(Color::Green)),
                Span::raw("       Open command palette"),
            ]),
            Line::from(vec![
                Span::raw("│  "),
                Span::styled("?", Style::new().fg(Color::Green)),
                Span::raw("          Toggle this help"),
            ]),
            Line::from(vec![
                Span::raw("│  "),
                Span::styled("Tab", Style::new().fg(Color::Green)),
                Span::raw("          Cycle focus (input ↔ stream)"),
            ]),
            Line::from(vec![
                Span::raw("│  "),
                Span::styled("Esc", Style::new().fg(Color::Green)),
                Span::raw("          Close panel / deselect"),
            ]),
            Line::from("│".to_string()),
            Line::from(vec![
                Span::raw("│  "),
                Span::styled("Agents & Models", Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::raw("│  "),
                Span::styled("^a", Style::new().fg(Color::Green)),
                Span::raw("          Toggle agent swarm view"),
            ]),
            Line::from(vec![
                Span::raw("│  "),
                Span::styled("^p", Style::new().fg(Color::Green)),
                Span::raw("          Cycle to previous model"),
            ]),
            Line::from(vec![
                Span::raw("│  "),
                Span::styled("^l", Style::new().fg(Color::Green)),
                Span::raw("          Open model selector"),
            ]),
            Line::from(vec![
                Span::raw("│  "),
                Span::styled("^$", Style::new().fg(Color::Green)),
                Span::raw("          Toggle cost HUD"),
            ]),
            Line::from("│".to_string()),
            Line::from(vec![
                Span::raw("│  "),
                Span::styled("Execution", Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::raw("│  "),
                Span::styled("Enter", Style::new().fg(Color::Green)),
                Span::raw("          Send command / confirm"),
            ]),
            Line::from(vec![
                Span::raw("│  "),
                Span::styled("^c", Style::new().fg(Color::Green)),
                Span::raw("          Cancel current task"),
            ]),
            Line::from(vec![
                Span::raw("│  "),
                Span::styled("^q", Style::new().fg(Color::Green)),
                Span::raw("          Quit anvil"),
            ]),
            Line::from("│".to_string()),
            Line::from(vec![
                Span::raw("│  "),
                Span::styled("Command Palette", Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::raw("│  "),
                Span::styled("/spawn", Style::new().fg(Color::Green)),
                Span::raw("        Spawn a new agent"),
            ]),
            Line::from(vec![
                Span::raw("│  "),
                Span::styled("/models", Style::new().fg(Color::Green)),
                Span::raw("        Open model selector"),
            ]),
            Line::from(vec![
                Span::raw("│  "),
                Span::styled("/cost", Style::new().fg(Color::Green)),
                Span::raw("          Show cost breakdown"),
            ]),
            Line::from(vec![
                Span::raw("│  "),
                Span::styled("/pause", Style::new().fg(Color::Green)),
                Span::raw("         Pause all agents"),
            ]),
            Line::from(vec![
                Span::raw("│  "),
                Span::styled("/resume", Style::new().fg(Color::Green)),
                Span::raw("         Resume paused agents"),
            ]),
            Line::from("│".to_string()),
            Line::from(vec![
                Span::raw("└─ "),
                Span::styled("Esc", Style::new().fg(Color::Blue)),
                Span::raw(" or "),
                Span::styled("?", Style::new().fg(Color::Blue)),
                Span::raw(" to close"),
            ]),
        ];

        Paragraph::new(lines).style(Style::new().fg(Color::White))
    }
}

impl Default for HelpOverlay {
    fn default() -> Self {
        Self::new()
    }
}
