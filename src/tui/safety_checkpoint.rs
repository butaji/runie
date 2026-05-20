use ratatui::{
    widgets::Paragraph,
    style::{Style, Color, Modifier},
    text::{Line, Span},
};

/// Safety checkpoint — shown when an agent attempts a high-risk action
pub struct SafetyCheckpoint {
    pub visible: bool,
    pub selected: usize, // 0=Y/Approve, 1=n/Reject, 2=e/Edit
    pub reason: String,
    pub diff_preview: Vec<String>,
    pub risk_level: RiskLevel,
    pub confidence: f64,
}

#[derive(Clone, Debug)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl RiskLevel {
    fn label(&self) -> &'static str {
        match self {
            RiskLevel::Low => "LOW",
            RiskLevel::Medium => "MEDIUM",
            RiskLevel::High => "HIGH",
            RiskLevel::Critical => "CRITICAL",
        }
    }

    fn color(&self) -> Color {
        match self {
            RiskLevel::Low => Color::Green,
            RiskLevel::Medium => Color::Yellow,
            RiskLevel::High => Color::Red,
            RiskLevel::Critical => Color::Red,
        }
    }

    /// Style for rendering this risk level (includes bold for critical)
    fn style(&self) -> Style {
        match self {
            RiskLevel::Low => Style::new().fg(Color::Green),
            RiskLevel::Medium => Style::new().fg(Color::Yellow),
            RiskLevel::High => Style::new().fg(Color::Red),
            RiskLevel::Critical => Style::new().fg(Color::Red).add_modifier(Modifier::BOLD),
        }
    }
}

impl SafetyCheckpoint {
    pub fn new() -> Self {
        Self {
            visible: false,
            selected: 0,
            reason: "Agent wants to delete 147 lines from src/auth.rs".to_string(),
            diff_preview: vec![
                "  - function oldAuth() { ... }".to_string(),
                "  - function legacyLogin() { ... }".to_string(),
                "  + function newOAuth2() { ... }".to_string(),
            ],
            risk_level: RiskLevel::High,
            confidence: 0.72,
        }
    }

    /// Show checkpoint with specific details
    pub fn show_with(&mut self, reason: String, diff: Vec<String>, risk: RiskLevel) {
        self.visible = true;
        self.selected = 0;
        self.reason = reason;
        self.diff_preview = diff;
        self.risk_level = risk;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Returns true if user approved (selected 0)
    pub fn is_approved(&self) -> bool {
        self.selected == 0
    }

    pub fn render(&self) -> Paragraph<'_> {
        if !self.visible {
            return Paragraph::new("");
        }

        let _choices = ["Y", "n", "e"];
        let _descriptions = ["Approve and continue", "Reject and pause", "Edit plan"];

        let mut lines = vec![
            Line::from(vec![
                Span::raw("┌─ "),
                Span::styled("⚠ SAFETY CHECKPOINT", Style::new().fg(Color::Red).add_modifier(Modifier::BOLD)),
                Span::raw(" ─────────────────────────────────────"),
            ]),
            Line::from("│".to_string()),
            Line::from(vec![
                Span::raw("│  "),
                Span::styled(&self.reason, Style::new().fg(Color::White)),
            ]),
            Line::from("│".to_string()),
            Line::from(vec![
                Span::raw("│  "),
                Span::styled("Diff preview:", Style::new().fg(Color::DarkGray)),
            ]),
        ];

        for diff_line in &self.diff_preview {
            let style = if diff_line.trim().starts_with('+') {
                Style::new().fg(Color::Green)
            } else if diff_line.trim().starts_with('-') {
                Style::new().fg(Color::Red)
            } else {
                Style::new().fg(Color::DarkGray)
            };
            lines.push(Line::from(vec![
                Span::raw("│    ".to_string()),
                Span::styled(diff_line, style),
            ]));
        }

        lines.push(Line::from("│".to_string()));
        lines.push(Line::from(vec![
            Span::raw("│  "),
            Span::styled("Risk:", Style::new().fg(Color::DarkGray)),
            Span::raw("  "),
            Span::styled(self.risk_level.label(), self.risk_level.style()),
        ]));
        lines.push(Line::from(vec![
            Span::raw("│  "),
            Span::styled("Confidence:", Style::new().fg(Color::DarkGray)),
            Span::raw(format!("  {:.2} (below 0.80 threshold)", self.confidence)),
        ]));
        lines.push(Line::from("│".to_string()));
        lines.push(Line::from(vec![
            Span::raw("│  "),
            Span::styled(
                "[Y] Approve and continue",
                if self.selected == 0 {
                    Style::new().fg(Color::Green).add_modifier(Modifier::BOLD)
                } else {
                    Style::new().fg(Color::DarkGray)
                },
            ),
        ]));
        lines.push(Line::from(vec![
            Span::raw("│  "),
            Span::styled(
                "[n] Reject and pause",
                if self.selected == 1 {
                    Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::new().fg(Color::DarkGray)
                },
            ),
        ]));
        lines.push(Line::from(vec![
            Span::raw("│  "),
            Span::styled(
                "[e] Edit plan",
                if self.selected == 2 {
                    Style::new().fg(Color::Blue).add_modifier(Modifier::BOLD)
                } else {
                    Style::new().fg(Color::DarkGray)
                },
            ),
        ]));
        lines.push(Line::from("│".to_string()));
        lines.push(Line::from(vec![
            Span::raw("└─ "),
            Span::styled("←/→", Style::new().fg(Color::Blue)),
            Span::raw(" navigate  "),
            Span::styled("Enter", Style::new().fg(Color::Blue)),
            Span::raw(" confirm  "),
            Span::styled("Esc", Style::new().fg(Color::Blue)),
            Span::raw(" cancel"),
        ]));

        Paragraph::new(lines).style(Style::new().fg(Color::White))
    }

    pub fn handle_key(&mut self, key: crossterm::event::KeyCode) -> Option<CheckpointAction> {
        match key {
            crossterm::event::KeyCode::Left | crossterm::event::KeyCode::Char('h') => {
                self.selected = self.selected.saturating_sub(1);
            }
            crossterm::event::KeyCode::Right | crossterm::event::KeyCode::Char('l')
                if self.selected < 2 => {
                    self.selected += 1;
                }
            crossterm::event::KeyCode::Up | crossterm::event::KeyCode::Char('k') => {
                self.selected = self.selected.saturating_sub(1);
            }
            crossterm::event::KeyCode::Down | crossterm::event::KeyCode::Char('j')
                if self.selected < 2 => {
                    self.selected += 1;
                }
            crossterm::event::KeyCode::Enter => {
                let action = match self.selected {
                    0 => Some(CheckpointAction::Approve),
                    1 => Some(CheckpointAction::Reject),
                    2 => Some(CheckpointAction::EditPlan),
                    _ => None,
                };
                self.hide();
                return action;
            }
            crossterm::event::KeyCode::Esc => {
                self.hide();
            }
            _ => {}
        }
        None
    }
}

#[derive(Clone, Debug)]
pub enum CheckpointAction {
    Approve,
    Reject,
    EditPlan,
}

impl Default for SafetyCheckpoint {
    fn default() -> Self {
        Self::new()
    }
}
