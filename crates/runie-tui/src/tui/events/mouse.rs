//! Event mouse handling.

use crossterm::event::MouseEventKind;
use crate::tui::state::Msg;

pub(super) fn mouse_event_to_msg(mouse: crossterm::event::MouseEvent) -> Vec<Msg> {
    match mouse.kind {
        MouseEventKind::ScrollUp => vec![Msg::ScrollUp],
        MouseEventKind::ScrollDown => vec![Msg::ScrollDown],
        MouseEventKind::Down(_) => vec![Msg::MouseClick { x: mouse.column, y: mouse.row, button: 0 }],
        _ => vec![],
    }
}
