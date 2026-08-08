use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::{Block, Borders, Paragraph, Widget, Wrap};

use crate::appearance;
use runie_core::types::ThemeKind;

/// Pure projection of the local changelog capability. The repository has no
/// packaged changelog entries, so the Pi-compatible empty result is explicit
/// rather than an I/O side effect during rendering.
pub struct ChangelogWidget {
    theme: ThemeKind,
}

impl ChangelogWidget {
    pub fn new() -> Self {
        Self {
            theme: ThemeKind::GrokNight,
        }
    }

    pub fn with_theme(mut self, theme: ThemeKind) -> Self {
        self.theme = theme;
        self
    }
}

impl Default for ChangelogWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for ChangelogWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let width = 56.min(area.width.saturating_sub(4));
        let height = 8.min(area.height.saturating_sub(4));
        if width < 24 || height < 5 {
            return;
        }
        let panel = Rect {
            x: area.x + area.width.saturating_sub(width) / 2,
            y: area.y + area.height.saturating_sub(height) / 2,
            width,
            height,
        };
        Paragraph::new("What's New\n\nNo changelog entries found.")
            .style(appearance::base_style_for(self.theme))
            .wrap(Wrap { trim: false })
            .block(
                Block::default()
                    .title(" Changelog ")
                    .title_style(appearance::accent_style_for(self.theme))
                    .borders(Borders::ALL)
                    .border_style(appearance::accent_style_for(self.theme)),
            )
            .render(panel, buf);
    }
}
