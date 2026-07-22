//! Permission approval prompt handling.
//!
//! This module handles permission events by projecting them to AppState.
//! The PermissionActor owns the ApprovalRegistry and permission_request state;
//! this module handles the event projection.

use crate::model::{AppState, InputReceiver, PermissionRequestState};
use crate::permissions::PermissionAction;
use crate::update::permission_dialog::open_permission_dialog;
use crate::Event;

/// Clear the permission request UI if it matches the given request id.
fn clear_matching_request(state: &mut AppState, request_id: &str) {
    if state
        .permission_request_opt()
        .map(|r| r.request_id == request_id)
        .unwrap_or(false)
    {
        *state.permission_request_mut() = None;
        close_permission_dialog(state);
        state.view_mut().dirty = true;
    }
}

/// Close the permission dialog if it is currently open, returning focus to
/// the chat input. Resolution can arrive without a user choice (timeout,
/// cancellation, turn abort), in which case the form machinery never runs
/// and the dialog would stay open otherwise.
fn close_permission_dialog(state: &mut AppState) {
    let is_permission = state
        .open_dialog()
        .and_then(|d| d.panel_stack())
        .and_then(|s| s.current())
        .map(|p| p.id == "permission")
        .unwrap_or(false);
    if is_permission {
        *state.open_dialog_mut() = None;
        state.view_mut().input_receiver = InputReceiver::ChatInput;
        state.view_mut().dirty = true;
    }
}

/// Project permission events to AppState.
#[allow(clippy::too_many_lines)]
pub(crate) fn permission_event(state: &mut AppState, event: Event) {
    match event {
        Event::PermissionRequest { request_id, tool, input } => {
            let req = PermissionRequestState { request_id, tool, input };
            *state.permission_request_mut() = Some(req.clone());
            *state.open_dialog_mut() = Some(open_permission_dialog(&req));
            state.view_mut().input_receiver = InputReceiver::Dialog;
            // Pause the visible agent loop while waiting for the user decision.
            // The PermissionActor already blocks the real tool execution; this
            // stops the streaming spinner so the UI does not look alive.
            state.agent_state_mut().streaming = false;
            state.view_mut().dirty = true;
        }
        Event::PermissionResponse { request_id, action: _ } => {
            // Projection: the PermissionActor resolved the request.
            // Clear the request UI after resolution.
            clear_matching_request(state, &request_id);
        }
        Event::PermissionRequestDismissed => {
            *state.permission_request_mut() = None;
            close_permission_dialog(state);
            state.view_mut().dirty = true;
        }
        Event::PermissionAllow { request_id } => {
            if let Some(handles) = state.actor_handles() {
                handles
                    .permission
                    .try_resolve_permission(request_id.clone(), PermissionAction::Allow);
            }
            clear_matching_request(state, &request_id);
        }
        Event::PermissionDeny { request_id } => {
            if let Some(handles) = state.actor_handles() {
                handles
                    .permission
                    .try_resolve_permission(request_id.clone(), PermissionAction::Deny);
            }
            clear_matching_request(state, &request_id);
        }
        Event::PermissionAlwaysAllow { request_id, tool } => {
            if let Some(handles) = state.actor_handles() {
                handles
                    .permission
                    .try_upsert_rule(tool.clone(), PermissionAction::Allow);
                handles
                    .permission
                    .try_resolve_permission(request_id.clone(), PermissionAction::Allow);
            }
            clear_matching_request(state, &request_id);
        }
        Event::PermissionSessionAllow { request_id, tool } => {
            if let Some(handles) = state.actor_handles() {
                // SessionAllow: add rule with Session scope for this session
                handles
                    .permission
                    .try_upsert_session_rule(tool.clone(), PermissionAction::Allow);
                handles
                    .permission
                    .try_resolve_permission(request_id.clone(), PermissionAction::Allow);
            }
            clear_matching_request(state, &request_id);
        }
        Event::PermissionOnce { request_id } => {
            // Once: just allow this single request, no rule persistence
            if let Some(handles) = state.actor_handles() {
                handles
                    .permission
                    .try_resolve_permission(request_id.clone(), PermissionAction::Allow);
            }
            clear_matching_request(state, &request_id);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::{DialogKind, DialogState};
    use crate::dialog::{Panel, PanelStack};
    use crate::model::{AppState, InputReceiver};

    fn open_sample_permission_dialog(state: &mut AppState) {
        state.update(Event::PermissionRequest {
            request_id: "req-1".into(),
            tool: "list_dir".into(),
            input: serde_json::json!({"path": "."}),
        });
    }

    #[test]
    fn permission_request_opens_hosted_dialog() {
        let mut state = AppState::default();
        open_sample_permission_dialog(&mut state);
        assert!(state.permission_request_opt().is_some());
        let dialog = state.open_dialog().expect("dialog should be open");
        let stack = dialog.panel_stack().expect("panel stack");
        let panel = stack.current().expect("panel");
        assert_eq!(panel.title, " Permission Required ");
    }

    #[test]
    fn permission_request_pauses_streaming() {
        let mut state = AppState::default();
        state.agent_state_mut().streaming = true;
        open_sample_permission_dialog(&mut state);
        assert!(
            !state.agent_state().streaming,
            "streaming should pause while waiting for permission"
        );
    }

    #[test]
    fn hosted_permission_dialog_starts_with_allow_selected() {
        let mut state = AppState::default();
        open_sample_permission_dialog(&mut state);
        let dialog = state.open_dialog().expect("dialog should be open");
        let stack = dialog.panel_stack().expect("panel stack");
        let panel = stack.current().expect("panel");
        assert_eq!(panel.selected, 0);
    }

    #[test]
    fn arrow_keys_navigate_hosted_permission_dialog() {
        let mut state = AppState::default();
        open_sample_permission_dialog(&mut state);

        state.update(Event::HistoryNext);
        let dialog = state.open_dialog().expect("dialog should still be open");
        let stack = dialog.panel_stack().expect("panel stack");
        assert_eq!(stack.current().unwrap().selected, 1);

        state.update(Event::HistoryPrev);
        let dialog = state.open_dialog().expect("dialog should still be open");
        let stack = dialog.panel_stack().expect("panel stack");
        assert_eq!(stack.current().unwrap().selected, 0);
    }

    #[test]
    fn submit_activates_allow_and_clears_request() {
        let mut state = AppState::default();
        open_sample_permission_dialog(&mut state);

        state.update(Event::Submit);

        assert!(state.open_dialog().is_none());
        assert!(state.permission_request_opt().is_none());
    }

    #[test]
    fn permission_allow_event_clears_request() {
        let mut state = AppState::default();
        open_sample_permission_dialog(&mut state);

        state.update(Event::PermissionAllow { request_id: "req-1".into() });

        assert!(state.permission_request_opt().is_none());
    }

    #[test]
    fn permission_deny_event_clears_request() {
        let mut state = AppState::default();
        open_sample_permission_dialog(&mut state);

        state.update(Event::PermissionDeny { request_id: "req-1".into() });

        assert!(state.permission_request_opt().is_none());
    }

    #[test]
    fn permission_always_allow_event_clears_request() {
        let mut state = AppState::default();
        open_sample_permission_dialog(&mut state);

        state.update(Event::PermissionAlwaysAllow { request_id: "req-1".into(), tool: "list_dir".into() });

        assert!(state.permission_request_opt().is_none());
    }

    #[test]
    fn permission_request_dismissed_closes_dialog_and_restores_focus() {
        // Timeout / cancellation path: the actor emits PermissionRequestDismissed
        // when the pending request is cancelled without a user choice.
        let mut state = AppState::default();
        open_sample_permission_dialog(&mut state);
        assert!(state.open_dialog().is_some());
        assert_eq!(state.view().input_receiver, InputReceiver::Dialog);

        state.update(Event::PermissionRequestDismissed);

        assert!(state.permission_request_opt().is_none());
        assert!(
            state.open_dialog().is_none(),
            "dialog must close when the request is dismissed"
        );
        assert_eq!(
            state.view().input_receiver,
            InputReceiver::ChatInput,
            "focus must return to the chat input"
        );
    }

    #[test]
    fn permission_response_closes_dialog_and_restores_focus() {
        // Resolution observed via the actor's PermissionResponse event (e.g. the
        // request was resolved from another path while the dialog was open).
        let mut state = AppState::default();
        open_sample_permission_dialog(&mut state);
        assert!(state.open_dialog().is_some());

        state.update(Event::PermissionResponse { request_id: "req-1".into(), action: PermissionAction::Deny });

        assert!(state.permission_request_opt().is_none());
        assert!(
            state.open_dialog().is_none(),
            "dialog must close when the request is resolved"
        );
        assert_eq!(
            state.view().input_receiver,
            InputReceiver::ChatInput,
            "focus must return to the chat input"
        );
    }

    #[test]
    fn permission_dismissal_leaves_unrelated_dialog_open() {
        // Edge: a dismissal that arrives after the permission dialog already
        // closed (or for a different dialog) must not clobber an unrelated dialog.
        let mut state = AppState::default();
        *state.open_dialog_mut() = Some(DialogState::Active {
            kind: DialogKind::Generic,
            panels: PanelStack::new(Panel::new("settings", "Settings")),
        });
        state.view_mut().input_receiver = InputReceiver::Dialog;

        state.update(Event::PermissionRequestDismissed);

        assert!(
            state.open_dialog().is_some(),
            "unrelated dialog must stay open"
        );
        assert_eq!(state.view().input_receiver, InputReceiver::Dialog);
    }
}
