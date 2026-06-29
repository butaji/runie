//! Event update handlers — merged dispatcher (formerly split between mod.rs and dispatch.rs).

use crate::model::AppState;
use crate::Event;

// Re-export for backward compatibility
pub use crate::tool_markers::has_tool_markers as content_has_tool_markers;
pub use crate::tool_markers::strip_tool_markers;

/// Strip `<think>...</think>` thinking tags from content.
/// Returns only the visible text, dropping the reasoning content.
/// Handles unclosed tags by stripping from the last `<think>` to end of input.
pub fn strip_thinking_tags(content: &str) -> String {
    static THINK_BLOCK_REGEX: std::sync::LazyLock<regex::Regex> =
        std::sync::LazyLock::new(|| regex::Regex::new(r"(?s)<think>.*?</think>").unwrap());
    static THINK_OPEN_REGEX: std::sync::LazyLock<regex::Regex> =
        std::sync::LazyLock::new(|| regex::Regex::new(r"<think>").unwrap());

    let caps: Vec<_> = THINK_BLOCK_REGEX.captures_iter(content).collect();
    let has_unclosed = THINK_OPEN_REGEX.find_iter(content).count() > caps.len();

    if has_unclosed {
        strip_with_unclosed(content, &caps)
    } else {
        THINK_BLOCK_REGEX.replace_all(content, "").to_string()
    }
}

fn strip_with_unclosed(content: &str, caps: &[regex::Captures]) -> String {
    // Find the position after the last complete block
    let after_last_block = caps
        .last()
        .and_then(|c| c.get(0))
        .map(|m| m.end())
        .unwrap_or(0);

    // Look for unclosed <think> after the last complete block
    let remaining = &content[after_last_block..];
    if let Some(pos) = remaining.find("<think>") {
        // Content before last block + content before unclosed tag
        let before_block = &content[..after_last_block];
        let before_unclosed = &remaining[..pos];
        // Strip complete blocks from the before_block portion
        static THINK_BLOCK_REGEX: std::sync::LazyLock<regex::Regex> =
            std::sync::LazyLock::new(|| regex::Regex::new(r"(?s)<think>.*?</think>").unwrap());
        let stripped_before = THINK_BLOCK_REGEX.replace_all(before_block, "");
        format!("{stripped_before}{before_unclosed}")
    } else {
        content.to_string()
    }
}

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

mod agent;
pub(crate) mod command;
pub mod dialog;
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
