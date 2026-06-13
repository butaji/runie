//! TUI rendering tests — visuals, margins, styling

/// Find the top and bottom rows of the input box. Provider-agnostic: we
/// locate the `❯` input prompt (always present) and walk outward to the
/// box borders, so the helper works in production (empty provider) and
/// dev (mock/echo) alike.
pub(crate) fn find_input_box_bounds(buf: &ratatui::buffer::Buffer) -> (u16, u16) {
    // First, find the line with the input prompt `❯`.
    let mut prompt_line: Option<u16> = None;
    for y in 0..buf.area().height {
        let line: String = (0..buf.area().width)
            .map(|x| buf[(x, y)].symbol())
            .collect();
        if line.contains('❯') {
            prompt_line = Some(y);
            break;
        }
    }
    let Some(prompt) = prompt_line else {
        return (0, 0);
    };

    // The input box has a top border (───) ~2 rows above the prompt
    // and a bottom border ~1 row below. Walk outward to find the borders.
    let mut top = prompt;
    for ty in (0..prompt).rev() {
        let line: String = (0..buf.area().width)
            .map(|x| buf[(x, ty)].symbol())
            .collect();
        if line.contains('─') || line.contains('┌') || line.contains('└') {
            top = ty;
            break;
        }
    }
    let mut bottom = prompt;
    for ty in (prompt + 1)..buf.area().height {
        let line: String = (0..buf.area().width)
            .map(|x| buf[(x, ty)].symbol())
            .collect();
        if line.contains('─') || line.contains('┐') || line.contains('┘') {
            bottom = ty;
            break;
        }
    }
    (top, bottom)
}

mod form;
mod input;
mod input_box;
mod panel_list;
mod popup_bg;
mod scoped_models;
mod scrollbar;
mod timestamps;
mod toggle_expand;
mod transient;
mod vim_nav;
