use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::{Block, Borders, Paragraph, Widget},
};
use runie_core::types::ThemeKind;

/// Pure projection of the actor-selected shortcut overlay.
pub fn render(area: Rect, buf: &mut Buffer, theme: ThemeKind) {
    let width = 38.min(area.width.saturating_sub(2));
    let height = 8.min(area.height.saturating_sub(2));
    if width < 10 || height < 3 {
        return;
    }
    let panel = Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    };
    Paragraph::new("Enter  send\nShift+Tab  cycle mode\nCtrl+C  clear / abort\nEsc  clear prompt\nCtrl+L  file search\ne  fold/unfold feed")
        .block(Block::default().style(crate::appearance::base_style_for(theme)).title(" Shortcuts ").borders(Borders::ALL))
        .render(panel, buf);
}
