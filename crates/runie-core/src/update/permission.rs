//! Permission approval prompt handling.

use crate::model::{AppState, PermissionRequestState};
use crate::Event;

/// Store an incoming permission request so the UI can render a blocking modal.
pub(crate) fn permission_event(state: &mut AppState, event: Event) {
    if let Event::PermissionRequest {
        request_id,
        tool,
        input,
    } = event
    {
        state.permission_request = Some(PermissionRequestState {
            request_id,
            tool,
            input,
        });
        state.mark_dirty();
    }
}
