//! Convenience constructors for common `Event` variants.

use crate::model::ThinkingLevel;

use super::Event;

impl Event {
    pub fn input(c: char) -> Self {
        Event::Input(c)
    }
    pub fn backspace() -> Self {
        Event::Backspace
    }
    pub fn newline() -> Self {
        Event::Newline
    }
    pub fn submit() -> Self {
        Event::Submit
    }
    pub fn scroll_up() -> Self {
        Event::Up
    }
    pub fn scroll_down() -> Self {
        Event::Down
    }
    pub fn page_up() -> Self {
        Event::PageUp
    }
    pub fn page_down() -> Self {
        Event::PageDown
    }
    pub fn go_to_top() -> Self {
        Event::GoToTop
    }
    pub fn go_to_bottom() -> Self {
        Event::GoToBottom
    }
    pub fn quit() -> Self {
        Event::Quit
    }
    pub fn force_quit() -> Self {
        Event::ForceQuit
    }
    pub fn reset() -> Self {
        Event::Reset
    }
    pub fn abort() -> Self {
        Event::Abort
    }
    pub fn switch_model(provider: String, model: String) -> Self {
        Event::SwitchModel {
            provider,
            model,
            explicit: false,
        }
    }
    pub fn switch_theme(name: String) -> Self {
        Event::SwitchTheme { name }
    }
    pub fn agent_thinking(id: String) -> Self {
        Event::Thinking { id }
    }
    pub fn agent_thought_done(id: String) -> Self {
        Event::ThoughtDone { id }
    }
    pub fn agent_tool_start(id: String, name: String, input: serde_json::Value) -> Self {
        Event::ToolStart { id, name, input }
    }
    pub fn agent_tool_end(id: String, duration_secs: f64, output: String) -> Self {
        Event::tool_end(id, duration_secs, output)
    }
    pub fn agent_response(id: String, content: String) -> Self {
        Event::response(id, content)
    }
    pub fn agent_turn_complete(id: String, duration_secs: f64) -> Self {
        Event::TurnComplete { id, duration_secs }
    }
    pub fn agent_done(id: String) -> Self {
        Event::Done { id }
    }
    pub fn agent_error(id: String, message: String) -> Self {
        Event::Error { id, message }
    }

    pub fn paste(s: String) -> Self {
        Event::Paste(s)
    }
    pub fn set_thinking_level(level: ThinkingLevel) -> Self {
        Event::SetThinkingLevel(level)
    }
    pub fn palette_select() -> Self {
        Event::PaletteSelect
    }
    pub fn palette_filter(c: char) -> Self {
        Event::PaletteFilter(c)
    }
    pub fn palette_close() -> Self {
        Event::PaletteClose
    }
    pub fn palette_down() -> Self {
        Event::PaletteDown
    }
    pub fn settings_close() -> Self {
        Event::SettingsClose
    }
    pub fn show_diagnostics() -> Self {
        Event::ShowDiagnostics
    }
    pub fn dialog(event: Event) -> Self {
        event
    }
    pub fn toggle_command_palette() -> Self {
        Event::ToggleCommandPalette
    }
    pub fn dialog_back() -> Self {
        Event::DialogBack
    }
}
