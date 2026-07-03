//! Submit content dispatch logic.

use crate::ui_actor::UiActor;

/// Dispatch submit content (slash command, form submission, steering, or user message).
///
/// This function handles the full submit flow:
/// 1. If a form dialog is open and chat input is empty, submit the form
/// 2. Close any open dialog (e.g., command palette)
/// 3. Handle slash commands
/// 4. Route steering/follow-up during active turns through TurnActor
/// 5. Normal user message submission
pub(crate) async fn dispatch(ui: &mut UiActor, content: String) {
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
    *ui.state.open_dialog_mut() = None;
    // Slash command handling.
    if let Some(result) = ui.state.handle_slash(&content) {
        // Extract Abort/ClearQueues from CommandResult::Events before applying,
        // so UiActor flags are cleared even though handle_event_inner is bypassed.
        let has_abort = matches!(
            &result,
            runie_core::commands::CommandResult::Events(evts) if evts.iter().any(|e| matches!(e, runie_core::Event::Abort))
        );
        ui.state.apply_command_result(result);
        if has_abort {
            ui.clear_turn_state(true).await;
        }
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
