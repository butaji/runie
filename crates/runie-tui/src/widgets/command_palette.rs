use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::{Block, Borders, Paragraph, Widget, Wrap};
use runie_core::types::ThemeKind;
use runie_tui_model::PaletteAction;

use crate::appearance;

// Mirrors Grok's default palette vocabulary (modal.rs). Execution is wired
// separately through UiMsg so this view remains a pure projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandPaletteWidget {
    pub query: String,
    pub selected: usize,
    theme: ThemeKind,
}

impl CommandPaletteWidget {
    pub fn new(query: impl Into<String>, selected: usize) -> Self {
        Self {
            query: query.into(),
            selected,
            theme: ThemeKind::GrokNight,
        }
    }

    pub fn with_theme(mut self, theme: ThemeKind) -> Self {
        self.theme = theme;
        self
    }

    fn filtered(&self) -> Vec<&'static str> {
        PaletteAction::filtered_labels(&self.query)
    }

    pub fn selected_entry(query: &str, selected: usize) -> Option<&'static str> {
        PaletteAction::selected_label(query, selected)
    }

    pub fn entry_count(query: &str) -> usize {
        PaletteAction::entry_count(query)
    }
}

impl Widget for CommandPaletteWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let width = 56.min(area.width.saturating_sub(4));
        let height = 12.min(area.height.saturating_sub(4));
        if width < 24 || height < 6 {
            return;
        }
        let panel = Rect {
            x: area.x + area.width.saturating_sub(width) / 2,
            y: area.y + area.height.saturating_sub(height) / 2,
            width,
            height,
        };
        let entries = self.filtered();
        let mut text = format!("Commands\n\n> {}\n", self.query);
        for (index, entry) in entries.iter().enumerate() {
            let marker = if index == self.selected { "› " } else { "  " };
            text.push_str(&format!("\n{marker}{entry}"));
        }
        Paragraph::new(text)
            .style(appearance::base_style_for(self.theme))
            .wrap(Wrap { trim: false })
            .block(
                Block::default()
                    .title(" Command Palette ")
                    .title_style(appearance::accent_style_for(self.theme))
                    .borders(Borders::ALL)
                    .border_style(appearance::accent_style_for(self.theme)),
            )
            .render(panel, buf);
    }
}
