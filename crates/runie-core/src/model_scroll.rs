//! Scroll and viewport calculations for AppState.
use crate::model::AppState;

impl AppState {
    pub fn scroll_offset(&self, visible_height: usize) -> u16 {
        let max_scroll = self.view.total_lines.saturating_sub(visible_height);
        let scroll = self.view.scroll.min(max_scroll);
        max_scroll.saturating_sub(scroll).min(u16::MAX as usize) as u16
    }

    pub fn scrollbar_metrics(&self, visible_height: usize) -> (usize, usize) {
        let total = self.view.total_lines;
        if total <= visible_height || visible_height == 0 {
            return (0, 0);
        }
        let max_scroll = total.saturating_sub(visible_height);
        let scroll = self.view.scroll.min(max_scroll);
        let position = max_scroll.saturating_sub(scroll);
        let track_f = visible_height as f64;
        // Match ratatui's rounding formula exactly (with clamping):
        let thumb_start = (position as f64 * track_f / total as f64)
            .round()
            .clamp(0.0, track_f - 1.0) as usize;
        let thumb_end = ((position + visible_height) as f64 * track_f / total as f64)
            .round()
            .clamp(0.0, track_f) as usize;
        let thumb = thumb_end.saturating_sub(thumb_start).max(1);
        (thumb, thumb_start)
    }

}
