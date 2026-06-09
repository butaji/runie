//! Immutable frame description — the UI DSL.
//! The event loop builds snapshots; the render actor draws them.
//! Zero blocking I/O in the event loop by design.

use crate::model::VisibleRegion;
use crate::ui::elements::Element;

#[derive(Clone)]
pub struct Snapshot {
    pub elements: Vec<Element>,
    pub line_counts: Vec<usize>,
    pub total_lines: usize,
    pub input: String,
    pub cursor_pos: usize,
    pub hint_text: String,
    pub at_suggestions: Option<Vec<String>>,
    pub at_selected: Option<usize>,
    pub turn_active: bool,
    pub spinner_frame: char,
    pub scroll: usize,
    /// Elapsed seconds since turn started. Captured at snapshot creation time.
    pub turn_elapsed_secs: Option<f64>,
    pub provider: String,
    pub model: String,
    /// Active theme name for the render actor
    pub theme_name: String,
    /// Current thinking level for status display
    pub thinking_level: crate::model::ThinkingLevel,
    /// Read-only mode active — only safe tools exposed to LLM
    pub read_only: bool,
    /// Flash countdown for invalid input feedback.
    pub input_flash: u8,
    /// Placeholder text shown when input is empty.
    pub placeholder: String,
    /// Queue count (pending messages in queue)
    pub queue_count: usize,
    /// Currently open dialog state for rendering overlays.
    pub dialog: Option<crate::commands::DialogState>,
    /// Filtered command list for palette rendering (name, description, category).
    pub palette_items: Vec<(String, String, String)>,
}

impl Snapshot {
    pub fn element_count(&self) -> usize {
        self.elements.len()
    }

    pub fn visible(&self, skip: usize, take: usize) -> &[Element] {
        let start = skip.min(self.elements.len());
        let end = (start + take).min(self.elements.len());
        &self.elements[start..end]
    }

    pub fn visible_scroll(&self, visible_height: usize) -> VisibleRegion<'_> {
        if self.elements.is_empty() || visible_height == 0 {
            return VisibleRegion { elements: &[], skip_lines: 0 };
        }

        let total = self.total_lines;
        let max_scroll = total.saturating_sub(visible_height);
        let scroll = self.scroll.min(max_scroll);

        let viewport_end = total.saturating_sub(scroll);
        let viewport_start = viewport_end.saturating_sub(visible_height);

        let mut cum = 0usize;
        let mut start_idx = 0;
        let mut skip_lines = 0;

        for (i, count) in self.line_counts.iter().enumerate() {
            let next_cum = cum + count;
            if next_cum > viewport_start {
                start_idx = i;
                skip_lines = viewport_start.saturating_sub(cum);
                break;
            }
            cum = next_cum;
        }

        let mut end_idx = self.elements.len();
        cum = 0;
        for (i, count) in self.line_counts.iter().enumerate() {
            cum += count;
            if cum >= viewport_end {
                end_idx = i + 1;
                break;
            }
        }

        VisibleRegion {
            elements: &self.elements[start_idx..end_idx.min(self.elements.len())],
            skip_lines,
        }
    }

    pub fn scroll_offset(&self, visible_height: usize) -> u16 {
        let max_scroll = self.total_lines.saturating_sub(visible_height);
        let scroll = self.scroll.min(max_scroll);
        max_scroll.saturating_sub(scroll).min(u16::MAX as usize) as u16
    }

    pub fn scrollbar_metrics(&self, visible_height: usize) -> (usize, usize) {
        let total = self.total_lines;
        if total <= visible_height || visible_height == 0 {
            return (0, 0);
        }
        let max_scroll = total.saturating_sub(visible_height);
        let scroll = self.scroll.min(max_scroll);
        let position = max_scroll.saturating_sub(scroll);
        let track = visible_height;
        let thumb = (visible_height * visible_height / total).max(1);
        #[allow(clippy::manual_checked_ops)]
        let thumb_offset = if max_scroll > 0 {
            position * (track - thumb) / max_scroll
        } else {
            0
        };
        (thumb, thumb_offset)
    }
}
