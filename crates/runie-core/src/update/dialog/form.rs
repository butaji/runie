//! Form Action Types and panel form handling (merged from dialog_form.rs).

use crate::dialog::{ItemAction, Panel, PanelItem};
use crate::event::TransientLevel;
use crate::model::AppState;
use crate::Event;

use super::panel_handler::toggle_selected_checkbox;

/// What a form panel should do in response to an event.
#[derive(Debug, Clone)]
pub enum FormAction {
    /// Keep the form open, persist the panel state.
    KeepOpen,
    /// Close the form and dispatch the submit event.
    Submit(Option<crate::Event>),
    /// Go back one step: if the stack is deeper than the root, pop the
    /// current panel and keep the dialog open; if at the root, close
    /// the dialog. This is the semantic of ESC / back.
    Back,
}

/// Map a single event to an action on a form panel.
pub fn form_panel_action(state: &mut AppState, panel: &mut Panel, event: Event) -> FormAction {
    use FormAction as A;
    match &event {
        crate::Event::SettingsClose
        | crate::Event::CommandFormClose
        | crate::Event::DialogBack => A::Back,
        crate::Event::CommandFormUp
        | crate::Event::HistoryPrev
        | crate::Event::SettingsUp
        | crate::Event::PaletteUp
        | crate::Event::ModelSelectorUp => {
            let _ = panel.select_up();
            A::KeepOpen
        }
        crate::Event::CommandFormDown
        | crate::Event::HistoryNext
        | crate::Event::SettingsDown
        | crate::Event::PaletteDown
        | crate::Event::ModelSelectorDown => {
            let _ = panel.select_down();
            A::KeepOpen
        }
        crate::Event::CommandFormInput(c) => handle_form_input(state, panel, *c),
        crate::Event::Input(' ') => handle_form_space(state, panel),
        crate::Event::Input(c) => handle_form_input(state, panel, *c),
        crate::Event::Paste(text) => handle_form_paste(panel, text),
        crate::Event::CommandFormBackspace | crate::Event::Backspace => {
            form_panel_delete_before_cursor(panel);
            A::KeepOpen
        }
        crate::Event::CommandFormSubmit
        | crate::Event::Submit
        | crate::Event::SettingsSelect
        | crate::Event::PaletteSelect => handle_form_submit(state, panel),
        _ => form_panel_edit_action(panel, &event),
    }
}

fn form_panel_edit_action(panel: &mut Panel, event: &Event) -> FormAction {
    use FormAction as A;
    match event {
        crate::Event::KillChar => form_panel_delete_at_cursor(panel),
        crate::Event::DeleteWord => form_panel_delete_word_before(panel),
        crate::Event::DeleteToEnd => form_panel_delete_to_end(panel),
        crate::Event::DeleteToStart => form_panel_delete_to_start(panel),
        crate::Event::CursorLeft => form_panel_move_cursor(panel, CursorDir::Left),
        crate::Event::CursorRight => form_panel_move_cursor(panel, CursorDir::Right),
        crate::Event::CursorStart | crate::Event::CursorWordLeft => {
            form_panel_move_cursor(panel, CursorDir::Start)
        }
        crate::Event::CursorEnd | crate::Event::CursorWordRight => {
            form_panel_move_cursor(panel, CursorDir::End)
        }
        // intentionally ignored: other input events fall through
        _ => {}
    }
    A::KeepOpen
}

fn handle_form_input(state: &mut AppState, panel: &mut Panel, c: char) -> FormAction {
    use FormAction as A;
    if panel.selected_form_field().is_some() {
        form_panel_edit_char(panel, c);
        return A::KeepOpen;
    }
    if let Some(ItemAction::Emit(evt)) = panel.find_button_by_accel(c) {
        if panel.id == "login-key" && is_empty_submit_key(evt, panel) {
            state.set_transient("API key is required.".into(), TransientLevel::Warning);
            return A::KeepOpen;
        }
        return A::Submit(Some(evt.clone()));
    }
    A::KeepOpen
}

fn handle_form_space(state: &mut AppState, panel: &mut Panel) -> FormAction {
    use FormAction as A;
    if toggle_selected_checkbox(state, panel) {
        return A::KeepOpen;
    }
    handle_form_input(state, panel, ' ')
}

fn handle_form_paste(panel: &mut Panel, text: &str) -> FormAction {
    use FormAction as A;
    if panel.selected_form_field().is_some() {
        form_panel_paste(panel, text);
    }
    A::KeepOpen
}

fn form_panel_paste(panel: &mut Panel, text: &str) {
    let Some(idx) = panel.selected_form_field() else {
        return;
    };
    let PanelItem::FormField {
        value,
        key,
        cursor_pos,
        ..
    } = &mut panel.items[idx]
    else {
        return;
    };
    value.insert_str(*cursor_pos, text);
    *cursor_pos = (*cursor_pos + text.len()).min(value.len());
    panel.form_values.insert(key.clone(), value.clone());
}

fn is_empty_submit_key(evt: &crate::Event, panel: &Panel) -> bool {
    matches!(
        evt,
        crate::Event::SubmitKey { key, .. } if key.trim().is_empty() && key_field_empty(panel)
    )
}

fn handle_form_submit(state: &mut AppState, panel: &mut Panel) -> FormAction {
    use FormAction as A;
    if panel.id == "login-key" && key_field_empty(panel) {
        state.set_transient("API key is required.".into(), TransientLevel::Warning);
        return A::KeepOpen;
    }
    match panel.selected_item().cloned() {
        Some(PanelItem::Action {
            action: ItemAction::Emit(evt),
            ..
        }) => A::Submit(Some(evt)),
        Some(PanelItem::Action { .. }) => A::Submit(None),
        Some(PanelItem::Toggle {
            action: ItemAction::Emit(crate::Event::ToggleModel { model }),
            ..
        }) if panel.id == "login-models" => {
            // In the login model selector, Enter confirms the selection and
            // saves. Make sure the focused model is selected before saving.
            if let Some(flow) = state.login_flow_mut().as_mut() {
                flow.selected_models.insert(model.clone());
            }
            A::Submit(Some(crate::Event::Save))
        }
        Some(PanelItem::Toggle { .. }) => {
            toggle_selected_checkbox(state, panel);
            A::KeepOpen
        }
        _ => A::Submit(form_build_submit(panel)),
    }
}

fn key_field_empty(panel: &Panel) -> bool {
    panel
        .form_values
        .get("key")
        .map(|v| v.trim().is_empty())
        .unwrap_or(true)
}

fn form_panel_edit_char(panel: &mut Panel, c: char) {
    let Some(idx) = panel.selected_form_field() else {
        return;
    };
    let PanelItem::FormField {
        value,
        key,
        cursor_pos,
        ..
    } = &mut panel.items[idx]
    else {
        return;
    };
    value.insert(*cursor_pos, c);
    *cursor_pos += c.len_utf8();
    panel.form_values.insert(key.clone(), value.clone());
}

fn form_panel_delete_before_cursor(panel: &mut Panel) {
    let Some(idx) = panel.selected_form_field() else {
        return;
    };
    let PanelItem::FormField {
        value,
        key,
        cursor_pos,
        ..
    } = &mut panel.items[idx]
    else {
        return;
    };
    if *cursor_pos == 0 {
        return;
    }
    let new_pos = prev_grapheme_boundary(value, *cursor_pos);
    value.drain(new_pos..*cursor_pos);
    *cursor_pos = new_pos;
    panel.form_values.insert(key.clone(), value.clone());
}

fn form_panel_delete_at_cursor(panel: &mut Panel) {
    let Some(idx) = panel.selected_form_field() else {
        return;
    };
    let PanelItem::FormField {
        value,
        key,
        cursor_pos,
        ..
    } = &mut panel.items[idx]
    else {
        return;
    };
    if *cursor_pos >= value.len() {
        return;
    }
    let end = next_grapheme_boundary(value, *cursor_pos);
    value.drain(*cursor_pos..end);
    panel.form_values.insert(key.clone(), value.clone());
}

fn form_panel_delete_word_before(panel: &mut Panel) {
    let Some(idx) = panel.selected_form_field() else {
        return;
    };
    let PanelItem::FormField {
        value,
        key,
        cursor_pos,
        ..
    } = &mut panel.items[idx]
    else {
        return;
    };
    if *cursor_pos == 0 {
        return;
    }
    let start = find_word_boundary_left(value, *cursor_pos);
    value.drain(start..*cursor_pos);
    *cursor_pos = start;
    panel.form_values.insert(key.clone(), value.clone());
}

fn form_panel_delete_to_end(panel: &mut Panel) {
    let Some(idx) = panel.selected_form_field() else {
        return;
    };
    let PanelItem::FormField {
        value,
        key,
        cursor_pos,
        ..
    } = &mut panel.items[idx]
    else {
        return;
    };
    value.truncate(*cursor_pos);
    panel.form_values.insert(key.clone(), value.clone());
}

fn form_panel_delete_to_start(panel: &mut Panel) {
    let Some(idx) = panel.selected_form_field() else {
        return;
    };
    let PanelItem::FormField {
        value,
        key,
        cursor_pos,
        ..
    } = &mut panel.items[idx]
    else {
        return;
    };
    if *cursor_pos == 0 {
        return;
    }
    value.drain(..*cursor_pos);
    *cursor_pos = 0;
    panel.form_values.insert(key.clone(), value.clone());
}

#[derive(Clone, Copy)]
enum CursorDir {
    Left,
    Right,
    Start,
    End,
}

fn form_panel_move_cursor(panel: &mut Panel, dir: CursorDir) {
    let Some(idx) = panel.selected_form_field() else {
        return;
    };
    let PanelItem::FormField { value, cursor_pos, .. } = &mut panel.items[idx] else {
        return;
    };
    *cursor_pos = match dir {
        CursorDir::Start => 0,
        CursorDir::End => value.len(),
        CursorDir::Left => prev_grapheme_boundary(value, *cursor_pos),
        CursorDir::Right => next_grapheme_boundary(value, *cursor_pos),
    };
}

fn prev_grapheme_boundary(s: &str, pos: usize) -> usize {
    crate::update::input::prev_grapheme_boundary(s, pos)
}

fn next_grapheme_boundary(s: &str, pos: usize) -> usize {
    crate::update::input::next_grapheme_boundary(s, pos)
}

fn find_word_boundary_left(s: &str, pos: usize) -> usize {
    crate::update::input::find_word_boundary_left(s, pos)
}

/// Apply a `FormAction` to the current dialog.
pub fn apply_form_action(state: &mut AppState, action: FormAction) {
    match action {
        FormAction::Submit(evt) => {
            *state.open_dialog_mut() = None;
            state.view_mut().input_receiver = crate::model::InputReceiver::ChatInput;
            state.view_mut().dirty = true;
            if let Some(e) = evt {
                state.update(e);
            }
        }
        FormAction::KeepOpen => {
            state.view_mut().dirty = true;
        }
        FormAction::Back => {}
    }
}

/// Build the submit event for a form panel by reading form values.
pub fn form_build_submit(panel: &mut Panel) -> Option<crate::Event> {
    let factory = panel.submit_factory?;
    let values = panel.get_form_values().clone();
    Some(factory(&values))
}

#[cfg(test)]
#[path = "form_tests.rs"]
mod form_tests;
