use ratatui::{
    widgets::Paragraph,
    style::{Style, Color, Modifier},
    text::Line,
};

pub struct Header {
    pub repo: String,
    pub branch: String,
    pub agent_count: usize,
    pub cost_spent: f64,
    pub cost_budget: f64,
    pub entry_count: usize,
}

impl Header {
    pub fn new() -> Self {
        Self {
            repo: "anvil".to_string(),
            branch: "main".to_string(),
            agent_count: 4,
            cost_spent: 12.40,
            cost_budget: 100.00,
            entry_count: 50,
        }
    }

    pub fn update_stream_count(&mut self, count: usize) {
        self.entry_count = count;
    }

    pub fn render(&self) -> Paragraph<'_> {
        let line = Line::from(vec![
            ratatui::text::Span::raw("┌─ "),
            ratatui::text::Span::styled(&self.repo, Style::new().fg(Color::Green).add_modifier(Modifier::BOLD)),
            ratatui::text::Span::raw("/"),
            ratatui::text::Span::styled(&self.branch, Style::new().fg(Color::Yellow)),
            ratatui::text::Span::raw("  user/folder/"),
            ratatui::text::Span::styled(&self.repo, Style::new().fg(Color::Green)),
            ratatui::text::Span::raw(" ───────── "),
            ratatui::text::Span::styled(
                format!("{} {} ", self.entry_count, "entries"),
                Style::new().fg(Color::Blue),
            ),
            ratatui::text::Span::raw(format!("{} agents ↓  ", self.agent_count)),
            ratatui::text::Span::styled(
                format!("${:.2}/{:.2}", self.cost_spent, self.cost_budget),
                Style::new().fg(Color::Cyan),
            ),
            ratatui::text::Span::raw(" ─┐"),
        ]);

        Paragraph::new(line)
            .style(Style::new().fg(Color::White))
    }
}

impl Default for Header {
    fn default() -> Self {
        Self::new()
    }
}
