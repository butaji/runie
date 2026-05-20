use ratatui::{
    widgets::Paragraph,
    style::{Style, Color, Modifier},
    text::{Line, Span},
};
use crate::router::ModelDatabase;

pub struct ModelSelector {
    pub visible: bool,
    pub selected: usize,
}

impl ModelSelector {
    pub fn new() -> Self {
        Self {
            visible: false,
            selected: 0,
        }
    }

    pub fn show(&mut self) {
        self.visible = true;
        self.selected = 0;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn render<'a>(&self, db: &'a ModelDatabase) -> Paragraph<'a> {
        if !self.visible {
            return Paragraph::new("");
        }

        let mut lines = vec![
            Line::from(vec![
                Span::raw("┌─ "),
                Span::styled("Model Selector", Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(" ────────────────────────────────"),
            ]),
            Line::from("├───────────────────────────────────────────".to_string()),
        ];

        let models: Vec<_> = db.models.values().collect();
        for (i, model) in models.iter().enumerate() {
            let selected = i == self.selected;
            let status = db.statuses.get(&model.id);
            let is_active = status.map(|s| s.is_active).unwrap_or(false);
            
            let prefix = if selected { "▶" } else { " " };
            let name_style = if selected {
                Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::new().fg(Color::White)
            };

            let cost_display = if model.input_cost > 0.0 {
                format!("${:.2}/$15.00", model.input_cost)
            } else {
                "$0.00/$0.00".to_string()
            };

            let ctx_display = if model.context_length >= 1_000_000 {
                format!("{}M ctx", model.context_length / 1_000_000)
            } else {
                format!("{}K ctx", model.context_length / 1000)
            };

            let health = status
                .map(|s| s.health.dots())
                .unwrap_or("●●●●●");

            let status_text = if is_active { "[ACTIVE]" } else { "[STANDBY]" };
            let status_color = if is_active { Color::Green } else { Color::DarkGray };

            lines.push(Line::from(vec![
                Span::raw(format!("{} │  ", prefix)),
                Span::styled(model.name.clone(), name_style),
                Span::raw("  "),
                Span::styled(cost_display, Style::new().fg(Color::Blue)),
                Span::raw(format!("  {}  ", ctx_display)),
                Span::styled(health.to_string(), Style::new().fg(Color::Green)),
                Span::raw("  "),
                Span::styled(status_text.to_string(), Style::new().fg(status_color)),
            ]));
        }

        lines.push(Line::from("├───────────────────────────────────────────".to_string()));
        lines.push(Line::from(vec![
            Span::raw("   "),
            Span::styled("↑/↓", Style::new().fg(Color::Blue)),
            Span::raw(" select  "),
            Span::styled("Enter", Style::new().fg(Color::Blue)),
            Span::raw(" confirm  "),
            Span::styled("Esc", Style::new().fg(Color::Blue)),
            Span::raw(" cancel"),
        ]));
        lines.push(Line::from("└───────────────────────────────────────────".to_string()));

        Paragraph::new(lines)
            .style(Style::new().fg(Color::White))
    }

    pub fn handle_key(&mut self, key: crossterm::event::KeyCode, model_count: usize) {
        match key {
            crossterm::event::KeyCode::Up | crossterm::event::KeyCode::Char('k') => {
                if self.selected > 0 {
                    self.selected -= 1;
                } else {
                    self.selected = model_count.saturating_sub(1);
                }
            }
            crossterm::event::KeyCode::Down | crossterm::event::KeyCode::Char('j')
                if self.selected < model_count.saturating_sub(1) => {
                    self.selected += 1;
                }
            _ => {}
        }
    }
}

impl Default for ModelSelector {
    fn default() -> Self {
        Self::new()
    }
}
