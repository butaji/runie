//! Test helper for computing the visible region of the element cache.

use crate::model::AppState;
use crate::view::elements::Element;

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
    let (viewport_start, viewport_end) = viewport_bounds(total, visible_height, state.view.scroll);
    let (start_idx, skip_lines) = find_start_index(state.view.line_counts(), viewport_start);
    let end_idx = find_end_index(state.view.line_counts(), viewport_end, cache.len());
    let end_idx = trim_trailing_spacers(cache, start_idx, end_idx);

    TestViewport {
        elements: &cache[start_idx..end_idx.min(cache.len())],
        skip_lines,
    }
}

fn trim_trailing_spacers(cache: &[Element], start_idx: usize, end_idx: usize) -> usize {
    let mut end = end_idx.min(cache.len());
    while end > start_idx && matches!(cache.get(end - 1), Some(Element::Spacer { .. })) {
        end -= 1;
    }
    end
}

fn viewport_bounds(total: usize, visible_height: usize, scroll: usize) -> (usize, usize) {
    let max_scroll = total.saturating_sub(visible_height);
    let scroll = scroll.min(max_scroll);
    let viewport_end = total.saturating_sub(scroll);
    let viewport_start = viewport_end.saturating_sub(visible_height);
    (viewport_start, viewport_end)
}

fn find_start_index(line_counts: &[usize], viewport_start: usize) -> (usize, usize) {
    let mut cum = 0usize;
    for (i, count) in line_counts.iter().enumerate() {
        let next_cum = cum + count;
        if next_cum > viewport_start {
            return (i, viewport_start.saturating_sub(cum));
        }
        cum = next_cum;
    }
    (0, 0)
}

fn find_end_index(line_counts: &[usize], viewport_end: usize, cache_len: usize) -> usize {
    let mut cum = 0usize;
    for (i, count) in line_counts.iter().enumerate() {
        cum += count;
        if cum >= viewport_end {
            return i + 1;
        }
    }
    cache_len
}
