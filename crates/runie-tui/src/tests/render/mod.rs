//! TUI rendering tests — visuals, margins, styling

use runie_core::{AppState, Event};
use crate::ui::view;
use ratatui::{backend::TestBackend, Terminal};

pub(crate) fn find_input_box_bounds(buf: &ratatui::buffer::Buffer) -> (u16, u16) {
    let mut top = None;
    let mut bottom = None;
    for y in 0..buf.area().height {
        let line: String = (0..buf.area().width)
            .map(|x| buf[(x, y)].symbol())
            .collect();
        if line.contains("mock/") || line.contains("/echo") || line.contains("openai/") {
            bottom = Some(y);
            if top.is_none() {
                for ty in (0..y).rev() {
                    let tline: String = (0..buf.area().width)
                        .map(|x| buf[(x, ty)].symbol())
                        .collect();
                    if tline.contains('❯') {
                        top = Some(ty);
                        break;
                    }
                }
            }
        }
    }
    (top.unwrap_or(0), bottom.unwrap_or(0))
}

mod input;
mod transient;
mod timestamps;
mod input_box;
mod popup_bg;
