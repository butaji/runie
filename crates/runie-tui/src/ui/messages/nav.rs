use ratatui::{layout::Rect, Frame};
use runie_core::Snapshot;
use runie_core::{view::PostKind, Element};

pub(crate) fn compute_scroll_offset(snap: &Snapshot, row_to_element: &[usize], visible_height: usize) -> u16 {
    let mut offset = snap.scroll_offset(visible_height);
    if snap.vim_nav_mode {
        if let Some(selected_post) = snap.selected_post {
            if let Some(post_offset) = post_actual_offset(snap, row_to_element, visible_height, selected_post) {
                offset = post_offset;
            }
        }
    }
    offset
}

pub(crate) fn highlight_selected_post(
    f: &mut Frame,
    snap: &Snapshot,
    area: Rect,
    row_to_element: &[usize],
    offset: u16,
) {
    if let Some(selected_post) = snap.selected_post {
        draw_post_background(f, snap, area, row_to_element, offset, selected_post);
        draw_post_left_line(f, snap, area, row_to_element, offset, selected_post);
    }
}

/// Fill the selected post's area with a subtle accent-colored background
/// at 10% opacity. The highlight spans exactly the same rows as the left
/// selection line (content + adjacent spacers/margins) so the selection
/// is readable and visually consistent.
fn draw_post_background(
    f: &mut Frame,
    snap: &Snapshot,
    area: Rect,
    row_to_element: &[usize],
    offset: u16,
    selected_post: usize,
) {
    let Some((visible_start, visible_end)) = visible_row_range(row_to_element, offset, area.height) else {
        return;
    };
    let Some((start, end)) = selected_post_row_range(snap, row_to_element, selected_post) else {
        return;
    };

    let bg = crate::theme::color_accent_bg();
    let first_visible = start.max(visible_start);
    let last_visible = end.min(visible_end);

    // Paint the band across the full app width, edge to edge — the same span
    // as the user-card band.
    let full_width = f.area().width;
    for row in first_visible..last_visible {
        let y = area.y + (row - visible_start) as u16;
        if y >= area.y + area.height {
            break;
        }
        for x in 0..full_width {
            let cell = &mut f.buffer_mut()[(x, y)];
            let _ = cell.set_bg(bg);
        }
    }
}

/// Draw a thin accent vertical line in the leftmost terminal column for
/// every visible row of the selected post. The line spans exactly the same
/// rows as the accent background, giving the selection a clean left edge.
fn draw_post_left_line(
    f: &mut Frame,
    snap: &Snapshot,
    area: Rect,
    row_to_element: &[usize],
    offset: u16,
    selected_post: usize,
) {
    let Some((visible_start, visible_end)) = visible_row_range(row_to_element, offset, area.height) else {
        return;
    };
    let Some((start, end)) = selected_post_row_range(snap, row_to_element, selected_post) else {
        return;
    };

    let first_visible = start.max(visible_start);
    let last_visible = end.min(visible_end);
    if first_visible >= last_visible {
        return;
    }

    let accent = crate::theme::color_accent();

    for row in first_visible..last_visible {
        let y = area.y + (row - visible_start) as u16;
        if y >= area.y + area.height {
            break;
        }
        // Draw the thin selection line one column left of the feed area so
        // it hugs the terminal edge and does not steal horizontal space
        // from message content.
        let x = area.x.saturating_sub(1);
        let cell = &mut f.buffer_mut()[(x, y)];
        // Use a thin left-side block so the visual line sits on the left
        // edge of the cell without looking heavy.
        let _ = cell.set_char('▎');
        let _ = cell.set_fg(accent);
    }
}

fn visible_row_range(row_to_element: &[usize], offset: u16, area_height: u16) -> Option<(usize, usize)> {
    let visible_start = offset as usize;
    let visible_end = (offset as usize + area_height as usize).min(row_to_element.len());
    if visible_start >= visible_end {
        return None;
    }
    Some((visible_start, visible_end))
}

/// Compute the inclusive start and exclusive end rows of the selected
/// post's highlight area. This includes the post's content rows plus the
/// adjacent spacer/margin rows that the highlight extends into, so the
/// returned range is exactly the height of the selection.
fn selected_post_row_range(snap: &Snapshot, row_to_element: &[usize], selected_post: usize) -> Option<(usize, usize)> {
    let post = snap.posts.get(selected_post)?;
    let (elem_start_rows, elem_line_counts) = element_row_map(row_to_element);
    let (start, end) = post_content_range(snap, post, &elem_start_rows, &elem_line_counts)?;
    Some(extend_with_spacers(
        snap,
        row_to_element,
        start,
        end,
        post.kind,
    ))
}

fn post_content_range(
    snap: &Snapshot,
    post: &runie_core::view::Post,
    elem_start_rows: &[usize],
    elem_line_counts: &[usize],
) -> Option<(usize, usize)> {
    let mut bracket_start: Option<usize> = None;
    let mut bracket_end: Option<usize> = None;
    for elem_idx in post.start..post.end {
        let elem = snap.elements.get(elem_idx)?;
        if matches!(elem, Element::Spacer { .. }) {
            continue;
        }
        let start = elem_start_rows[elem_idx];
        let end = start + elem_line_counts[elem_idx];
        bracket_start = Some(bracket_start.map_or(start, |s| s.min(start)));
        bracket_end = Some(bracket_end.map_or(end, |e| e.max(end)));
    }
    Some((bracket_start?, bracket_end?))
}

fn extend_with_spacers(
    snap: &Snapshot,
    row_to_element: &[usize],
    start: usize,
    end: usize,
    kind: PostKind,
) -> (usize, usize) {
    if kind == PostKind::UserInput {
        return (start, end);
    }
    let new_start = if start > 0 && is_spacer_at_row(snap, row_to_element, start - 1) {
        start - 1
    } else {
        start
    };
    let new_end = if end < row_to_element.len() && is_spacer_at_row(snap, row_to_element, end) {
        end + 1
    } else {
        end
    };
    (new_start, new_end)
}

fn is_spacer_at_row(snap: &Snapshot, row_to_element: &[usize], row: usize) -> bool {
    let elem_idx = row_to_element.get(row).copied().unwrap_or(usize::MAX);
    matches!(snap.elements.get(elem_idx), Some(Element::Spacer { .. }))
}

/// Compute the actual wrapped-row offset that places the start of the
/// selected post at the top of the viewport when possible, or keeps it
/// visible near the bottom when the post is lower in the feed.
fn post_actual_offset(
    snap: &Snapshot,
    row_to_element: &[usize],
    visible_height: usize,
    selected_post: usize,
) -> Option<u16> {
    let post = snap.posts.get(selected_post)?;
    let (starts, _) = element_row_map(row_to_element);
    let first_content =
        (post.start..post.end).find(|&i| !matches!(snap.elements.get(i), Some(Element::Spacer { .. })))?;
    // Scroll so the full bracket is visible. Non-user posts extend into
    // the spacer above them (leading spacer for the first post, trailing
    // spacer of the previous post otherwise). User messages already have
    // internal margins, so their bracket starts at the content element.
    let target_top = if post.kind == PostKind::UserInput {
        starts[first_content]
    } else {
        starts[first_content].saturating_sub(1)
    };
    let max_offset = row_to_element.len().saturating_sub(visible_height);
    Some(target_top.min(max_offset).min(u16::MAX as usize) as u16)
}

/// From a flat `row -> element` mapping, derive the start row and line
/// count for each element index.
fn element_row_map(row_to_element: &[usize]) -> (Vec<usize>, Vec<usize>) {
    let elem_count = row_to_element.iter().copied().max().map_or(0, |m| m + 1);
    let mut starts = vec![0usize; elem_count];
    let mut counts = vec![0usize; elem_count];
    for (row, &elem_idx) in row_to_element.iter().enumerate() {
        counts[elem_idx] += 1;
        if row == 0 || elem_idx != row_to_element[row - 1] {
            starts[elem_idx] = row;
        }
    }
    (starts, counts)
}
