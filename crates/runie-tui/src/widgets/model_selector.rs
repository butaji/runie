use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::{Block, Borders, Paragraph, Widget, Wrap};
use runie_core::types::ThemeKind;

use crate::appearance;

/// Renderer-only projection of the UI actor's selector state. Catalog
/// ownership stays in `ModelCatalogActor`; this widget only paints facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelSelectorWidget {
    query: String,
    selected: usize,
    scoped_only: bool,
    result_count: usize,
    theme: ThemeKind,
}

impl ModelSelectorWidget {
    pub fn new(
        query: impl Into<String>,
        selected: usize,
        scoped_only: bool,
        result_count: usize,
    ) -> Self {
        Self {
            query: query.into(),
            selected,
            scoped_only,
            result_count,
            theme: ThemeKind::GrokNight,
        }
    }

    pub fn with_theme(mut self, theme: ThemeKind) -> Self {
        self.theme = theme;
        self
    }
}

impl Widget for ModelSelectorWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let width = 64.min(area.width.saturating_sub(4));
        let height = 12.min(area.height.saturating_sub(4));
        if width < 28 || height < 6 {
            return;
        }
        let panel = Rect {
            x: area.x + area.width.saturating_sub(width) / 2,
            y: area.y + area.height.saturating_sub(height) / 2,
            width,
            height,
        };
        let scope = if self.scoped_only { "scoped" } else { "all" };
        let text = format!(
            "Models ({scope})\n\n> {}\n\n› result {} of {}\n\nTab: toggle scope",
            self.query,
            self.selected.saturating_add(1),
            self.result_count
        );
        Paragraph::new(text)
            .style(appearance::base_style_for(self.theme))
            .wrap(Wrap { trim: false })
            .block(
                Block::default()
                    .title(" Model Selector ")
                    .title_style(appearance::accent_style_for(self.theme))
                    .borders(Borders::ALL)
                    .border_style(appearance::accent_style_for(self.theme)),
            )
            .render(panel, buf);
    }
}
