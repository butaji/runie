//! Permission approval prompt handling.
//!
//! This module handles permission events by projecting them to AppState.
//! The PermissionActor owns the ApprovalRegistry and permission_request state;
//! this module handles the event projection.

use crate::model::{AppState, PermissionRequestState};
use crate::Event;

/// Project permission events to AppState.
pub(crate) fn permission_event(state: &mut AppState, event: Event) {
    match event {
        Event::PermissionRequest {
            request_id,
            tool,
            input,
        } => {
            *state.permission_request_mut() = Some(PermissionRequestState {
                request_id,
                tool,
                input,
            });
            state.view_mut().dirty = true;
        }
        Event::PermissionResponse {
            request_id,
            action: _,
        } => {
            // Projection: the PermissionActor resolved the request.
            // Clear the request UI after resolution.
            if state
                .permission_request_opt()
                .map(|r| r.request_id == request_id)
                .unwrap_or(false)
            {
                *state.permission_request_mut() = None;
                state.view_mut().dirty = true;
            }
        }
        Event::PermissionRequestDismissed => {
            *state.permission_request_mut() = None;
            state.view_mut().dirty = true;
        }
        _ => {}
    }
}
