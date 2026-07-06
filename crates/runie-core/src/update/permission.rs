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
        state.view_mut().dirty = true;
    }
}

/// Project permission events to AppState.
pub(crate) fn permission_event(state: &mut AppState, event: Event) {
    match event {
        Event::PermissionRequest {
            request_id,
            tool,
            input,
        } => {
            let req = PermissionRequestState {
                request_id,
                tool,
                input,
            };
            *state.permission_request_mut() = Some(req.clone());
            *state.open_dialog_mut() = Some(open_permission_dialog(&req));
            state.view_mut().input_receiver = InputReceiver::Dialog;
            // Pause the visible agent loop while waiting for the user decision.
            // The PermissionActor already blocks the real tool execution; this
            // stops the streaming spinner so the UI does not look alive.
            state.agent_state_mut().streaming = false;
            state.view_mut().dirty = true;
        }
        Event::PermissionResponse {
            request_id,
            action: _,
        } => {
            // Projection: the PermissionActor resolved the request.
            // Clear the request UI after resolution.
            clear_matching_request(state, &request_id);
        }
        Event::PermissionRequestDismissed => {
            *state.permission_request_mut() = None;
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
    use crate::model::AppState;

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

        state.update(Event::PermissionAllow {
            request_id: "req-1".into(),
        });

        assert!(state.permission_request_opt().is_none());
    }

    #[test]
    fn permission_deny_event_clears_request() {
        let mut state = AppState::default();
        open_sample_permission_dialog(&mut state);

        state.update(Event::PermissionDeny {
            request_id: "req-1".into(),
        });

        assert!(state.permission_request_opt().is_none());
    }

    #[test]
    fn permission_always_allow_event_clears_request() {
        let mut state = AppState::default();
        open_sample_permission_dialog(&mut state);

        state.update(Event::PermissionAlwaysAllow {
            request_id: "req-1".into(),
            tool: "list_dir".into(),
        });

        assert!(state.permission_request_opt().is_none());
    }
}
