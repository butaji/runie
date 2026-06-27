//! Test helper for computing the visible region of the element cache.

use runie_core::model::AppState;
use runie_core::view::elements::Element;

pub struct TestViewport {
    pub elements: Vec<Element>,
    pub skip_lines: usize,
}

pub fn compute_viewport(state: &mut AppState, visible_height: usize) -> TestViewport {
    let snap = state.snapshot();
    if snap.elements.is_empty() || visible_height == 0 {
        return TestViewport {
            elements: vec![],
            skip_lines: 0,
        };
    }

    let total = snap.total_lines;
    let (viewport_start, viewport_end) = viewport_bounds(total, visible_height, snap.scroll);
    let (start_idx, skip_lines) = find_start_index(&snap.line_counts, viewport_start);
    let end_idx = find_end_index(&snap.line_counts, viewport_end, snap.elements.len());

    TestViewport {
        elements: snap.elements[start_idx..end_idx.min(snap.elements.len())].to_vec(),
        skip_lines,
    }
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
