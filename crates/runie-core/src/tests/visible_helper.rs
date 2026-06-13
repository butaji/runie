//! Test helper for computing the visible region of the element cache.

use crate::model::AppState;
use crate::ui::elements::Element;

#[derive(Clone, Copy)]
pub struct TestViewport<'a> {
    pub elements: &'a [Element],
    pub skip_lines: usize,
}

pub fn compute_viewport(state: &AppState, visible_height: usize) -> TestViewport<'_> {
    let cache = state.view.elements_cache();
    if cache.is_empty() || visible_height == 0 {
        return TestViewport {
            elements: &[],
            skip_lines: 0,
        };
    }

    let total = state.view.total_lines();
    let max_scroll = total.saturating_sub(visible_height);
    let scroll = state.view.scroll.min(max_scroll);

    let viewport_end = total.saturating_sub(scroll);
    let viewport_start = viewport_end.saturating_sub(visible_height);

    let mut cum = 0usize;
    let mut start_idx = 0;
    let mut skip_lines = 0;

    for (i, count) in state.view.line_counts().iter().enumerate() {
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
    for (i, count) in state.view.line_counts().iter().enumerate() {
        cum += count;
        if cum >= viewport_end {
            end_idx = i + 1;
            break;
        }
    }

    TestViewport {
        elements: &cache[start_idx..end_idx.min(cache.len())],
        skip_lines,
    }
}
