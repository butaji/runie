use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::{Block, Borders, Paragraph, Widget, Wrap};
use runie_core::session::SessionSnapshot;
use runie_core::types::ThemeKind;

use crate::appearance;

/// Pure read-only projection of the session actor's immutable snapshot.
pub struct SessionInfoWidget<'a> {
    snapshot: &'a SessionSnapshot,
    theme: ThemeKind,
}

impl<'a> SessionInfoWidget<'a> {
    pub fn new(snapshot: &'a SessionSnapshot) -> Self {
        Self {
            snapshot,
            theme: ThemeKind::GrokNight,
        }
    }

    pub fn with_theme(mut self, theme: ThemeKind) -> Self {
        self.theme = theme;
        self
    }
}

impl Widget for SessionInfoWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let width = 52.min(area.width.saturating_sub(4));
        let height = 10.min(area.height.saturating_sub(4));
        if width < 24 || height < 6 {
            return;
        }
        let panel = Rect {
            x: area.x + area.width.saturating_sub(width) / 2,
            y: area.y + area.height.saturating_sub(height) / 2,
            width,
            height,
        };
        let stats = self.snapshot.stats();
        let text = format!(
            "Session\n\nMessages: {}\nTokens: {}\nCost: ${:.4}\nSequence: {}",
            stats.message_count, stats.total_tokens, stats.cost_total, self.snapshot.sequence
        );
        Paragraph::new(text)
            .style(appearance::base_style_for(self.theme))
            .wrap(Wrap { trim: false })
            .block(
                Block::default()
                    .title(" Session Info ")
                    .title_style(appearance::accent_style_for(self.theme))
                    .borders(Borders::ALL)
                    .border_style(appearance::accent_style_for(self.theme)),
            )
            .render(panel, buf);
    }
}
