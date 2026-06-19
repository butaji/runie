//! Event update handlers — merged dispatcher (formerly split between mod.rs and dispatch.rs).

use crate::event::DialogEvent;
use crate::model::AppState;
use crate::Event;

// Re-export for backward compatibility
pub use crate::tool_markers::has_tool_markers as content_has_tool_markers;
pub use crate::tool_markers::strip_tool_markers;

mod agent;
pub(crate) mod command;
pub(crate) mod dialog;
pub(crate) mod dialog_input;
mod dispatch;
pub(crate) mod input;
pub(crate) mod login_flow;
mod permission;
mod session;
mod system;
mod tools;

// These are still separate (not merged):
mod path_complete;
pub mod settings_dialog;

pub(crate) use crate::message::now;

impl AppState {
    /// Main event dispatcher — merged from update() and dispatch_event().
    pub fn update(&mut self, event: Event) {
        if self.try_handle_dialog_event_input(&event) {
            return;
        }
        if self.try_handle_vim_dialog_back_input(&event) {
            return;
        }
        if self.try_handle_vim_nav_event_input(&event) {
            return;
        }
        if dispatch::is_dialog_event(&event) {
            self.handle_dialog_event(&event);
        } else {
            dispatch::dispatch_event(self, event);
        }
    }

    fn handle_dialog_event(&mut self, event: &Event) {
        if is_login_flow_dialog_event(event) || is_providers_dialog_event(event) {
            dispatch::dispatch_event(self, event.clone());
            return;
        }
        if self.login_flow.is_some() && matches!(event, DialogEvent::DialogBack) {
            login_flow::login_flow_cancel(self);
            return;
        }
        if self.try_handle_dialog_event_dialog(event) {
            return;
        }
        dispatch::dispatch_event(self, event.clone());
    }
}

fn is_login_flow_dialog_event(event: &DialogEvent) -> bool {
    matches!(event, DialogEvent::ProvidersAdd)
}

fn is_providers_dialog_event(event: &DialogEvent) -> bool {
    matches!(
        event,
        DialogEvent::ProvidersDialog
            | DialogEvent::ProvidersSelectModel { .. }
            | DialogEvent::ProvidersDisconnect { .. }
            | DialogEvent::ProvidersAdd
    )
}

