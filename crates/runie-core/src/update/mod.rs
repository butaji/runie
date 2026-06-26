//! Event update handlers — merged dispatcher (formerly split between mod.rs and dispatch.rs).

use crate::model::AppState;
use crate::Event;

// Re-export for backward compatibility
pub use crate::tool_markers::has_tool_markers as content_has_tool_markers;
pub use crate::tool_markers::strip_tool_markers;

/// Strip `<think>...</think>` thinking tags from content.
/// Returns only the visible text, dropping the reasoning content.
pub fn strip_thinking_tags(content: &str) -> String {
    let mut visible = String::new();
    let mut in_reasoning = false;
    let mut rest = content;
    loop {
        let marker = if in_reasoning { "</think>" } else { "<think>" };
        match rest.find(marker) {
            Some(idx) => {
                if !in_reasoning {
                    visible.push_str(&rest[..idx]);
                }
                rest = &rest[idx + marker.len()..];
                in_reasoning = !in_reasoning;
            }
            None => {
                if !in_reasoning {
                    visible.push_str(rest);
                }
                break;
            }
        }
    }
    if in_reasoning {
        // Unclosed thinking tag - drop the remaining content
    }
    visible
}

mod agent;
pub(crate) mod command;
pub(crate) mod dialog;
pub(crate) mod dialog_input;
mod dispatch;
pub(crate) mod input;

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
        if let Event::InputChanged { state } = event {
            *self.input_mut() = *state;
            return;
        }
        if let Event::ViewChanged { state } = event {
            *self.view_mut() = *state;
            return;
        }
        if let Event::ConfigLoaded { config } = event {
            self.apply_config(&config);
            return;
        }
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
        if self.login_flow().is_some() && matches!(event, Event::DialogBack) {
            crate::login_flow::login_flow_cancel(self);
            return;
        }
        if self.try_handle_dialog_event_dialog(event) {
            return;
        }
        dispatch::dispatch_event(self, event.clone());
    }
}

fn is_login_flow_dialog_event(event: &Event) -> bool {
    matches!(event, Event::ProvidersAdd)
}

fn is_providers_dialog_event(event: &Event) -> bool {
    matches!(
        event,
        Event::ProvidersDialog
            | Event::ProvidersSelectModel { .. }
            | Event::ProvidersDisconnect { .. }
            | Event::ProvidersAdd
            | Event::ProvidersEditModels { .. }
    )
}
