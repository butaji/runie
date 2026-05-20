use ratatui::{
    widgets::Paragraph,
    style::{Style, Color, Modifier},
    text::{Line, Span},
};
use crate::router::ModelDatabase;

/// Cost HUD modal — expanded cost breakdown shown when ^$ is pressed
pub struct CostHud {
    pub visible: bool,
}

impl CostHud {
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

    /// Build a progress bar string from a percentage (0–100)
    fn progress_bar(pct: f64, width: usize) -> String {
        let filled = ((pct / 100.0) * width as f64).round() as usize;
        let empty = width.saturating_sub(filled);
        format!("{}{}", "█".repeat(filled), "░".repeat(empty))
    }

    pub fn render<'a>(&self, db: &'a ModelDatabase) -> Paragraph<'a> {
        if !self.visible {
            return Paragraph::new("");
        }

        let total = db.total_spent();
        let budget = 100.0;
        let pct = if budget > 0.0 { (total / budget) * 100.0 } else { 0.0 };
        let bar = Self::progress_bar(pct, 20);

        let bar_color = if pct >= 95.0 {
            Color::Red
        } else if pct >= 80.0 {
            Color::Yellow
        } else {
            Color::Green
        };

        let mut lines = vec![
            Line::from(vec![
                Span::raw("┌─ "),
                Span::styled("Cost Breakdown", Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(" ───────────────────────────────────────"),
            ]),
            Line::from(vec![
                Span::raw("│  "),
                Span::styled(format!("${:.2}/{:.2}", total, budget), Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw("  "),
                Span::styled(format!("[{}]  {:.0}%", bar, pct), Style::new().fg(bar_color)),
            ]),
            Line::from("├────────────────────────────────────────────────".to_string()),
        ];

        // Per-model breakdown
        let mut has_any = false;
        for (id, status) in &db.statuses {
            if status.spent > 0.0 || db.models.contains_key(id) {
                has_any = true;
                let model_name = db.models.get(id).map(|m| m.name.as_str()).unwrap_or(id);
                let model_pct = if total > 0.0 {
                    (status.spent / total) * 100.0
                } else {
                    0.0
                };
                let health = status.health.dots();
                let model_bar = Self::progress_bar(model_pct, 10);

                let _health_color = match status.health {
                    crate::router::HealthLevel::Healthy | crate::router::HealthLevel::Good => Color::Green,
                    crate::router::HealthLevel::Degraded => Color::Yellow,
                    crate::router::HealthLevel::Critical => Color::Red,
                };

                lines.push(Line::from(vec![
                    Span::raw("│  "),
                    Span::styled(model_name, Style::new().fg(Color::White)),
                    Span::raw(format!("  ${:.2}", status.spent)),
                    Span::raw(format!("  ({:.0}%)  ", model_pct)),
                    Span::styled(model_bar.to_string(), Style::new().fg(bar_color)),
                    Span::raw(format!("  {}", health)),
                ]));
            }
        }

        if !has_any {
            lines.push(Line::from(vec![
                Span::raw("│  "),
                Span::styled("(no spending yet)", Style::new().fg(Color::DarkGray)),
            ]));
        }

        lines.push(Line::from("├────────────────────────────────────────────────".to_string()));
        lines.push(Line::from(vec![
            Span::raw("│  "),
            Span::styled("This session:", Style::new().fg(Color::DarkGray)),
            Span::raw(format!("  ${:.2}", total)),
        ]));
        lines.push(Line::from(vec![
            Span::raw("│  "),
            Span::styled("Monthly budget:", Style::new().fg(Color::DarkGray)),
            Span::raw(format!("  ${:.2}", budget)),
        ]));
        lines.push(Line::from(vec![
            Span::raw("│  "),
            Span::styled("Remaining:", Style::new().fg(Color::DarkGray)),
            Span::raw(format!("  ${:.2}", (budget - total).max(0.0))),
        ]));

        // Warning when >80%
        if pct >= 80.0 {
            let warning = if pct >= 95.0 {
                "⚠ Budget critical — stop spending!"
            } else {
                "⚠ Approaching budget limit"
            };
            let warn_color = if pct >= 95.0 { Color::Red } else { Color::Yellow };
            lines.push(Line::from("│".to_string()));
            lines.push(Line::from(vec![
                Span::raw("│  "),
                Span::styled(warning, Style::new().fg(warn_color).add_modifier(Modifier::BOLD)),
            ]));
        }

        lines.push(Line::from("└────────────────────────────────────────────────".to_string()));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("^$", Style::new().fg(Color::Blue)),
            Span::raw(" to close"),
        ]));

        Paragraph::new(lines).style(Style::new().fg(Color::White))
    }
}

impl Default for CostHud {
    fn default() -> Self {
        Self::new()
    }
}
