use crate::components::MessageItem;
use crate::tui::state::{AppState, Msg, TuiMode};
use super::ChatCmd;
use super::submit::handle_submit;

pub fn handle_slash_menu_msg(state: &mut AppState, msg: crate::tui::state::Msg) -> Vec<ChatCmd> {
    use crate::tui::state::Msg;
    match msg {
        Msg::SlashMenuUp => { state.slash_menu.move_up(); vec![] }
        Msg::SlashMenuDown => { state.slash_menu.move_down(); vec![] }
        Msg::SlashMenuConfirm => {
            if let Some(cmd) = state.slash_menu.selected_command() {
                state.textarea.select_all();
                state.textarea.delete_line_by_end();
                state.textarea.insert_str(&cmd);
                state.slash_menu.close();
                return handle_submit(state);
            }
            state.slash_menu.close();
            vec![]
        }
        Msg::CloseSlashMenu => { state.slash_menu.close(); vec![] }
        _ => vec![],
    }
}

pub fn handle_textarea_key(state: &mut AppState, key: crossterm::event::KeyEvent) -> Vec<ChatCmd> {
    state.textarea.input(super::key_to_textarea_input(key));
    update_slash_menu_from_input(state);
    vec![]
}

pub fn update_slash_menu_from_input(state: &mut AppState) {
    let text = state.textarea.lines().join("\n");
    if text.starts_with('/') {
        state.file_picker.close();
        if !state.slash_menu.is_open() {
            state.slash_menu.open(&text);
        } else {
            state.slash_menu.set_filter(text.strip_prefix('/').unwrap_or(""));
        }
    } else if text.starts_with('@') {
        state.slash_menu.close();
        if !state.file_picker.is_open() {
            state.file_picker.open();
        }
        let filter = text.strip_prefix('@').unwrap_or("");
        state.file_picker.filter = filter.to_string();
        state.file_picker.update_filtered();
    } else {
        state.slash_menu.close();
        state.file_picker.close();
    }
}

pub fn handle_shortcuts_panel_msg(state: &mut AppState, msg: crate::tui::state::Msg) -> Vec<ChatCmd> {
    use crate::tui::state::Msg;
    match msg {
        Msg::OpenShortcutsPanel => { state.shortcuts_panel.open(); vec![] }
        Msg::CloseShortcutsPanel => { state.shortcuts_panel.close(); vec![] }
        Msg::ShortcutsPanelFilterInput(c) => shortcuts_filter_input(state, c),
        Msg::ShortcutsPanelFilterBackspace => shortcuts_filter_backspace(state),
        _ => shortcuts_panel_navigation(&msg, state).unwrap_or(vec![]),
    }
}

fn shortcuts_panel_navigation(msg: &Msg, state: &mut AppState) -> Option<Vec<ChatCmd>> {
    match msg {
        Msg::ShortcutsPanelUp => { state.shortcuts_panel.move_up(); Some(vec![]) }
        Msg::ShortcutsPanelDown => { state.shortcuts_panel.move_down(); Some(vec![]) }
        Msg::ShortcutsPanelToggleSection => { state.shortcuts_panel.toggle_selected_section(); Some(vec![]) }
        Msg::ShortcutsPanelToggleFilter => { state.shortcuts_panel.toggle_filter(); Some(vec![]) }
        _ => None,
    }
}

fn shortcuts_filter_input(state: &mut AppState, c: char) -> Vec<ChatCmd> {
    if state.shortcuts_panel.filter_mode {
        let new_filter = format!("{}{}", state.shortcuts_panel.filter, c);
        state.shortcuts_panel.set_filter(&new_filter);
    }
    vec![]
}

fn shortcuts_filter_backspace(state: &mut AppState) -> Vec<ChatCmd> {
    if state.shortcuts_panel.filter_mode {
        let new_filter = state.shortcuts_panel.filter.chars().next_back()
            .map(|_| state.shortcuts_panel.filter[..state.shortcuts_panel.filter.len()-1].to_string())
            .unwrap_or_default();
        state.shortcuts_panel.set_filter(&new_filter);
    }
    vec![]
}

pub fn handle_settings_modal_msg(state: &mut AppState, msg: crate::tui::state::Msg) -> Vec<ChatCmd> {
    use crate::tui::state::Msg;
    match msg {
        Msg::OpenSettingsModal => { state.settings_modal.open(); vec![] }
        Msg::CloseSettingsModal => { state.settings_modal.close(); vec![] }
        Msg::SettingsModalUp => { state.settings_modal.move_up(); vec![] }
        Msg::SettingsModalDown => { state.settings_modal.move_down(); vec![] }
        Msg::SettingsModalNextTab => { state.settings_modal.next_tab(); vec![] }
        Msg::SettingsModalPrevTab => { state.settings_modal.prev_tab(); vec![] }
        Msg::SettingsModalSelect => settings_modal_select(state),
        _ => vec![],
    }
}

fn settings_modal_select(state: &mut AppState) -> Vec<ChatCmd> {
    if state.settings_modal.selected_tab == 0 {
        if let Some((name, _)) = crate::components::settings_modal::THEMES.get(state.settings_modal.selected_item) {
            state.messages.push(MessageItem::System {
                text: format!("Theme switched to {}", name),
            });
        }
    }
    vec![]
}

pub fn handle_home_screen_msg(state: &mut AppState, msg: crate::tui::state::Msg) -> Vec<ChatCmd> {
    use crate::tui::state::Msg;
    match msg {
        Msg::HomeScreenUp => { state.home_screen.move_up(); vec![] }
        Msg::HomeScreenDown => { state.home_screen.move_down(); vec![] }
        Msg::HomeScreenSelect => home_screen_select(state),
        Msg::HomeScreenToggleSessions => { state.home_screen.toggle_sessions(); vec![] }
        Msg::CloseHomeScreen => home_screen_close(state),
        _ => vec![],
    }
}

fn home_screen_select(state: &mut AppState) -> Vec<ChatCmd> {
    let action = state.home_screen.selected_action().to_string();
    match action.as_str() {
        "New worktree" => {
            state.messages.clear();
            state.messages.push(MessageItem::System { text: "New session started".to_string() });
            // Show home screen after creating worktree, don't switch to chat
            state.home_screen.show();
            state.mode = TuiMode::HomeScreen;
        }
        "Resume session" => {
            state.home_screen.hide();
            state.mode = TuiMode::Chat;
            state.messages.push(MessageItem::System { text: "Resuming last session".to_string() });
        }
        "Settings" => { state.settings_modal.open(); }
        "Help" => { state.shortcuts_panel.open(); }
        "Quit" => { state.running = false; }
        _ => {
            state.home_screen.hide();
            state.mode = TuiMode::Chat;
        }
    }
    vec![]
}

fn home_screen_close(state: &mut AppState) -> Vec<ChatCmd> {
    state.home_screen.hide();
    state.mode = TuiMode::Chat;
    vec![]
}

pub fn handle_file_picker_msg(state: &mut AppState, msg: crate::tui::state::Msg) -> Vec<ChatCmd> {
    use crate::tui::state::Msg;
    match msg {
        Msg::FilePickerUp => { state.file_picker.move_up(); vec![] }
        Msg::FilePickerDown => { state.file_picker.move_down(); vec![] }
        Msg::FilePickerConfirm => file_picker_confirm(state),
        Msg::FilePickerFilter(c) => { state.file_picker.push_filter(c); vec![] }
        Msg::FilePickerBackspace => { state.file_picker.pop_filter(); vec![] }
        Msg::CloseFilePicker => { state.file_picker.close(); vec![] }
        _ => vec![],
    }
}

fn file_picker_confirm(state: &mut AppState) -> Vec<ChatCmd> {
    if let Some(file) = state.file_picker.selected_file() {
        let text = state.textarea.lines().join("\n");
        let new_text = if text.starts_with('@') { file } else { format!("{} {}", text, file) };
        state.textarea = ratatui_textarea::TextArea::new(vec![new_text]);
    }
    state.file_picker.close();
    vec![]
}
