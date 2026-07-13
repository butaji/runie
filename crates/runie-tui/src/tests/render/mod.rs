//! TUI rendering tests — visuals, margins, styling

// Re-export types used by child test modules so they can `use super::*;`
pub use super::{
    AppState, ChatMessage, DialogKind, Event, Part, Role, ScopedModel, Snapshot, view,
};

mod feed_style;
mod render_at;
mod render_basic;
mod render_model_selector;
mod render_scrollbar;
mod render_slash;
mod render_toggle;

/// Find the top and bottom rows of the input box. Provider-agnostic: we
/// locate the `❯` input prompt (always present) and walk outward to the
/// box borders, so the helper works in production (empty provider) and
/// dev (mock/echo) alike.
pub(crate) fn find_input_box_bounds(buf: &ratatui::buffer::Buffer) -> (u16, u16) {
    let Some(prompt) = find_prompt_line(buf) else {
        return (0, 0);
    };
    let top = find_border_above(buf, prompt);
    let bottom = find_border_below(buf, prompt);
    (top, bottom)
}

fn find_prompt_line(buf: &ratatui::buffer::Buffer) -> Option<u16> {
    for y in 0..buf.area().height {
        let line = row_text(buf, y);
        if line.contains('❯') {
            return Some(y);
        }
    }
    None
}

fn find_border_above(buf: &ratatui::buffer::Buffer, prompt: u16) -> u16 {
    for ty in (0..prompt).rev() {
        if is_border_line(&row_text(buf, ty)) {
            return ty;
        }
    }
    prompt
}

fn find_border_below(buf: &ratatui::buffer::Buffer, prompt: u16) -> u16 {
    for ty in (prompt + 1)..buf.area().height {
        if is_border_line(&row_text(buf, ty)) {
            return ty;
        }
    }
    prompt
}

fn row_text(buf: &ratatui::buffer::Buffer, y: u16) -> String {
    (0..buf.area().width)
        .map(|x| buf[(x, y)].symbol())
        .collect()
}

fn is_border_line(line: &str) -> bool {
    line.contains('─')
        || line.contains('┌')
        || line.contains('└')
        || line.contains('┐')
        || line.contains('┘')
}

mod flow;
mod form;
mod input;
mod input_box;
mod no_model;
mod panel_list;
mod permission_dialog;
mod popup_bg;
mod scoped_models;
mod scrollbar;
mod timestamps;
mod toggle_expand;
mod tool_truncation;
mod tools;
mod transient;
mod trust_banner;
mod vim_nav;
