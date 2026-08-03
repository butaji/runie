//! Submit content dispatch logic.

use crate::ui_actor::UiActor;

/// Dispatch submit content (slash command, form submission, steering, or user message).
///
/// This function handles the full submit flow:
/// 1. If a form dialog is open and chat input is empty, submit the form
/// 2. Close any open dialog (e.g., command palette)
/// 3. Handle quit commands ("quit", "exit", ":q")
/// 4. Handle slash commands
/// 5. Route steering/follow-up during active turns through TurnActor
/// 6. Normal user message submission
pub(crate) async fn dispatch(ui: &mut UiActor, content: String) {
    if runie_core::update::input::is_quit_command(&content) {
        *ui.state.should_quit_mut() = true;
        return;
    }

    // If a form dialog is open and chat input is empty, this is a form submission
    // (the form field content lives in the panel, not the chat input).
    // Route through handle_form_dialog so Enter on the submit button works.
    if ui.state.open_dialog().is_some() && content.is_empty() {
        let form_handled = ui.maybe_submit_form();
        if form_handled {
            // Form was submitted → dialog is now closed, command dispatched.
            ui.state.view_mut().scroll = 0;
            ui.state.view_mut().dirty = true;
            return;
        }
        // Not a form panel — fall through to close dialog and handle as slash command.
    }
    // Close any open dialog (e.g., command palette) before executing the command.
    // Restore the file-picker backup first (Esc/@-pick closes would otherwise
    // wipe the typed prefix — the dialog router does the same on DialogBack).
    if ui.state.open_dialog().is_some() {
        runie_core::update::dialog::restore_file_picker_backup(&mut ui.state);
        *ui.state.open_dialog_mut() = None;
    }
    // `/` opens an ephemeral palette, so executing a command must consume the
    // trigger draft as well. The palette can be closed here (rather than via
    // DialogBack) on Enter, which previously left an invisible slash in the
    // authoritative composer.
    if ui.state.command_palette_from_input {
        ui.state.input_mut().input.clear();
        ui.state.input_mut().cursor_pos = 0;
        ui.state.input_mut().chips.clear();
        ui.send_input_msg(runie_core::actors::InputMsg::Clear).await;
        ui.state.command_palette_from_input = false;
    }
    // Slash command handling.
    if let Some(result) = ui.state.handle_slash(&content) {
        // Extract Abort/ClearQueues from CommandResult::Events before applying,
        // so UiActor flags are cleared even though handle_event_inner is bypassed.
        let has_abort = matches!(
            &result,
            runie_core::commands::CommandResult::Events(evts) if evts.iter().any(|e| matches!(e, runie_core::Event::Abort))
        );
        ui.state.apply_command_result(result);
        // Palette filters are not chat drafts. Once a slash command executes,
        // clear both projections so the filter cannot echo back into the
        // composer and make the next `/` look like `status/`.
        ui.state.input_mut().input.clear();
        ui.state.input_mut().cursor_pos = 0;
        ui.state.input_mut().chips.clear();
        ui.send_input_msg(runie_core::actors::InputMsg::Clear).await;
        ui.state.command_palette_from_input = false;
        if has_abort {
            ui.clear_turn_state(true).await;
        }
        ui.state.view_mut().scroll = 0;
        ui.state.view_mut().dirty = true;
        return;
    }
    // Keep the production UiActor submit path aligned with AppState::submit:
    // bang-prefixed input is a shell command, not an agent prompt. The actor
    // path dispatches submits directly and therefore must perform this check
    // before steering or normal user-message routing.
    if ui.state.try_handle_bang_command(&content).is_some() {
        ui.state.view_mut().scroll = 0;
        ui.state.view_mut().dirty = true;
        return;
    }
    // Steering (follow-up during active turn): route through TurnActor to
    // maintain authoritative queue state. When the turn completes,
    // UiActor::handle_event_inner calls DeliverQueued + RunIfQueued to start
    // the queued turn.
    if ui.state.agent_state().turn_active {
        ui.state.queue_steering_and_update_history(content);
        return;
    }
    // Normal user message submission.
    ui.state.submit_user_message_and_update_history(content);
}
