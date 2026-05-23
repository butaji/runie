use crate::components::MessageItem;
use crate::tui::state::{AppState, Msg, TuiMode};

pub fn handle_tree_nav(state: &mut AppState, msg: &Msg) {
    match msg {
        Msg::SessionTreeUp => state.session_tree.move_up(),
        Msg::SessionTreeDown => state.session_tree.move_down(),
        _ => {}
    }
}

pub fn handle_tree_confirm(state: &mut AppState) {
    if let Some(id) = state.session_tree.get_selected_id() {
        state.messages.push(MessageItem::System { text: format!("Jumped to message: {}", &id[..8]) });
    }
    state.session_tree.hide();
    state.mode = TuiMode::Chat;
}
