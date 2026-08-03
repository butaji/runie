//! Event update handlers — merged dispatcher (formerly split between mod.rs and dispatch.rs).

use crate::model::AppState;
use crate::Event;

// Re-export for backward compatibility
pub use crate::tool_markers::has_tool_markers as content_has_tool_markers;
pub use crate::tool_markers::strip_tool_markers;
pub mod think_tag;
pub use think_tag::strip_thinking_tags;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regex_strips_think_blocks() {
        // Single think block: strips content inside, keeps visible
        assert_eq!(
            strip_thinking_tags("<think>reasoning</think>answer"),
            "answer"
        );
        // Multiple think blocks: strips both
        assert_eq!(
            strip_thinking_tags("<think>reason1</think><think>reason2</think>visible"),
            "visible"
        );
        // Nested-like scenario: think content that looks like tags
        assert_eq!(
            strip_thinking_tags("<think>think content</think>answer"),
            "answer"
        );
    }

    #[test]
    fn regex_handles_unclosed_think() {
        // Unclosed opening tag: strips to end of input
        assert_eq!(strip_thinking_tags("<think>unclosed reasoning"), "");
        // Unclosed with visible content before: keeps visible
        assert_eq!(strip_thinking_tags("visible<think>unclosed"), "visible");
        // Closed then unclosed: keeps visible before, strips unclosed to end
        assert_eq!(
            strip_thinking_tags("<think>closed</think>visible<think>unclosed"),
            "visible"
        );
    }

    #[test]
    fn regex_preserves_text_without_tags() {
        assert_eq!(strip_thinking_tags("plain answer"), "plain answer");
    }
}

pub mod agent;
pub(crate) mod command;
pub mod dialog;
pub(crate) mod dialog_input;
mod dispatch;
pub mod input;

mod permission;
pub mod permission_dialog;
mod question;
pub mod question_dialog;
mod session;
mod system;
mod tools;

// These are still separate (not merged):
mod path_complete;
pub mod settings_dialog;

pub(crate) use crate::message::now;

impl AppState {
    /// Main event dispatcher — merged from update() and dispatch_event().
    #[allow(clippy::too_many_lines)]
    pub fn update(&mut self, event: Event) {
        if let Event::InputChanged { state } = event {
            // The InputActor owns text/cursor/history, but the file-picker
            // backup, range suffix, and chips are projection-only dialog
            // state the actor never sees. Preserve them across the wholesale
            // replace, or the Clear echo (sent when the picker opens) wipes
            // the typed prefix and the pick rewrites the whole input.
            // Chips are only preserved while the picker window is open
            // (backup present): outside it the actor is authoritative and
            // its echoed chips (e.g. a fresh paste chip) must win.
            let picker_backup = self.input().file_picker_backup.clone();
            let range_suffix = self.input().file_picker_range_suffix.clone();
            let chips = self.input().chips.clone();
            let picker_open = picker_backup.is_some();
            let mut projected = *state;
            // Inline editing seeds the visible prompt before the asynchronous
            // InputActor echo arrives. The actor may still publish its stale
            // empty buffer, or may publish only the newly typed suffix. Keep
            // that echo from erasing the selected prompt; a full prompt echo
            // remains authoritative when it already contains the original.
            if let Some(edit) = self.view().inline_edit.as_ref() {
                let incoming = projected.input.clone();
                let current = self.input().input.clone();
                if incoming.is_empty() && !edit.original.is_empty() {
                    projected.input = edit.original.clone();
                    projected.cursor_pos = projected.input.len();
                } else if current == edit.original
                    && !incoming.starts_with(&edit.original)
                    && !incoming.is_empty()
                {
                    projected.input = format!("{}{}", edit.original, incoming);
                    projected.cursor_pos = projected.input.len();
                }
            }
            *self.input_mut() = projected;
            let inline_text = self.input().input.clone();
            let inline_cursor = self.input().cursor_pos;
            if let Some(edit) = self.view_mut().inline_edit.as_mut() {
                edit.edited = inline_text;
                edit.cursor_pos = inline_cursor;
            }
            self.input_mut().file_picker_backup = picker_backup;
            self.input_mut().file_picker_range_suffix = range_suffix;
            if picker_open {
                self.input_mut().chips = chips;
            }
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
        if let Event::SkillsLoaded { skills } = event {
            self.set_skills(skills);
            return;
        }
        if let Event::AuthLoaded { providers } = event {
            self.set_auth_providers(providers);
            return;
        }
        if matches!(event, Event::FocusGained | Event::FocusLost) {
            self.view_mut().terminal_focused = matches!(event, Event::FocusGained);
            self.view_mut().dirty = true;
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
            // Safety net: the onboarding dialog must never be closed by Esc/DialogBack
            // while the login flow is still active. If a stale flag or bug somehow
            // closed it, reopen the dialog immediately.
            if self.login_flow().is_some() && self.open_dialog().is_none() {
                tracing::warn!("onboarding dialog was incorrectly closed by Esc/DialogBack; reopening");
                crate::login_flow::rebuild_login_dialog(self);
            }
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
