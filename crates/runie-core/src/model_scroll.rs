//! Scroll and viewport calculations for AppState.
use crate::model::AppState;
use crate::snapshot::VisibleRegion;

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

    pub fn visible_scroll(&self, visible_height: usize) -> VisibleRegion<'_> {
        let cache = self.view.elements_cache.as_ref();
        if cache.is_empty() || visible_height == 0 {
            return VisibleRegion {
                elements: &[],
                skip_lines: 0,
            };
        }

        let total = self.view.total_lines;
        let max_scroll = total.saturating_sub(visible_height);
        let scroll = self.view.scroll.min(max_scroll);

        let viewport_end = total.saturating_sub(scroll);
        let viewport_start = viewport_end.saturating_sub(visible_height);

        let mut cum = 0usize;
        let mut start_idx = 0;
        let mut skip_lines = 0;

        for (i, count) in self.view.line_counts.iter().enumerate() {
            let next_cum = cum + count;
            if next_cum > viewport_start {
                start_idx = i;
                skip_lines = viewport_start.saturating_sub(cum);
                break;
            }
            cum = next_cum;
        }

        let mut end_idx = cache.len();
        cum = 0;
        for (i, count) in self.view.line_counts.iter().enumerate() {
            cum += count;
            if cum >= viewport_end {
                end_idx = i + 1;
                break;
            }
        }

        VisibleRegion {
            elements: &cache[start_idx..end_idx.min(cache.len())],
            skip_lines,
        }
    }
}
