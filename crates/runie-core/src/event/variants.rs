//! Event enum — top-level wrapper over focused sub-enums.
//!
//! The flat `Event` enum was split into sub-enums to keep match arms small,
//! improve error messages, and make the dispatcher more readable. Each sub-enum
//! lives in its own file under `event/`.

use serde::{Deserialize, Serialize};
use super::names::EVENT_NAMES;

use super::{
    AgentEvent, CommandEvent, ControlEvent, DialogEvent, EditEvent, InputEvent,
    LoginFlowEvent, ModelConfigEvent, OrchestratorEvent, ScrollEvent, SessionEvent,
    SidebarEvent, SystemEvent,
};

/// The top-level event type for the entire application.
///
/// Variants wrap sub-enums so that the compiler enforces exhaustive handling
/// at each dispatch layer. Call sites construct events with the sub-enum,
/// e.g. `Event::Input(InputEvent::Submit)`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum Event {
    Input(InputEvent),
    Agent(AgentEvent),
    Scroll(ScrollEvent),
    Control(ControlEvent),
    ModelConfig(ModelConfigEvent),
    Dialog(DialogEvent),
    Edit(EditEvent),
    System(SystemEvent),
    Session(SessionEvent),
    LoginFlow(LoginFlowEvent),
    Command(CommandEvent),
    Sidebar(SidebarEvent),
    Orchestrator(OrchestratorEvent),
}

impl Event {
    /// Convert this event to a durable core event for JSONL persistence.
    /// Returns `None` for transient-only events (keystrokes, scroll, streaming deltas).
    pub fn to_durable(&self) -> Option<super::DurableCoreEvent> {
        use super::DurableCoreEvent;
        match self {
            // ResponseDelta is transient — not persisted
            Event::Agent(AgentEvent::ResponseDelta { .. }) => None,
            // Complete Response is persisted
            Event::Agent(AgentEvent::Response { id, content }) => {
                Some(DurableCoreEvent::MessageSent {
                    id: id.clone(),
                    role: "assistant".into(),
                    content: content.clone(),
                    timestamp: crate::model::now(),
                })
            }
            Event::Agent(AgentEvent::ToolStart { id, name, input }) => {
                Some(DurableCoreEvent::ToolCalled {
                    id: id.clone(),
                    name: name.clone(),
                    input: input.clone(),
                })
            }
            Event::Agent(AgentEvent::ToolEnd { id, output, .. }) => {
                Some(DurableCoreEvent::ToolResult {
                    id: id.clone(),
                    output: output.clone(),
                    success: true,
                })
            }
            Event::ModelConfig(ModelConfigEvent::SwitchModel { provider, model }) => {
                Some(DurableCoreEvent::ModelSwitched {
                    provider: provider.clone(),
                    model: model.clone(),
                })
            }
            Event::Command(CommandEvent::RunNameCommand { name }) => {
                Some(DurableCoreEvent::SessionRenamed { name: name.clone() })
            }
            // SidebarEvent is transient UI state — not persisted
            Event::Sidebar(_) => None,
            // OrchestratorEvent drives sidebar state — not persisted
            Event::Orchestrator(_) => None,
            _ => None,
        }
    }

    /// Canonical string name for bindable variants (those in EVENT_NAMES).
    pub fn name(&self) -> Option<&'static str> {
        match self {
            Event::Input(i) => i.variant_name(),
            Event::Control(c) => c.variant_name(),
            Event::Dialog(d) => d.variant_name(),
            Event::ModelConfig(m) => m.variant_name(),
            Event::System(s) => s.variant_name(),
            _ => None,
        }
    }

    /// Build an Event from its canonical name. Supports `Input:<char>` prefix.
    pub fn from_name(name: &str) -> Option<Event> {
        if let Some(rest) = name.strip_prefix("Input:") {
            let c = rest.chars().next()?;
            return Some(Event::Input(InputEvent::Input(c)));
        }
        for (n, ctor) in EVENT_NAMES {
            if *n == name {
                return Some(ctor());
            }
        }
        None
    }
}

// ── Convenience constructors ───────────────────────────────────────────────────

impl Event {
    pub fn input(c: char) -> Self {
        Event::Input(InputEvent::Input(c))
    }
    pub fn backspace() -> Self {
        Event::Input(InputEvent::Backspace)
    }
    pub fn newline() -> Self {
        Event::Input(InputEvent::Newline)
    }
    pub fn submit() -> Self {
        Event::Input(InputEvent::Submit)
    }
    pub fn scroll_up() -> Self {
        Event::Scroll(ScrollEvent::Up)
    }
    pub fn scroll_down() -> Self {
        Event::Scroll(ScrollEvent::Down)
    }
    pub fn page_up() -> Self {
        Event::Scroll(ScrollEvent::PageUp)
    }
    pub fn page_down() -> Self {
        Event::Scroll(ScrollEvent::PageDown)
    }
    pub fn go_to_top() -> Self {
        Event::Scroll(ScrollEvent::GoToTop)
    }
    pub fn go_to_bottom() -> Self {
        Event::Scroll(ScrollEvent::GoToBottom)
    }
    pub fn quit() -> Self {
        Event::Control(ControlEvent::Quit)
    }
    pub fn reset() -> Self {
        Event::Control(ControlEvent::Reset)
    }
    pub fn abort() -> Self {
        Event::Control(ControlEvent::Abort)
    }
    pub fn switch_model(provider: String, model: String) -> Self {
        Event::ModelConfig(ModelConfigEvent::SwitchModel { provider, model })
    }
    pub fn switch_theme(name: String) -> Self {
        Event::ModelConfig(ModelConfigEvent::SwitchTheme { name })
    }
    pub fn agent_thinking(id: String) -> Self {
        Event::Agent(AgentEvent::Thinking { id })
    }
    pub fn agent_thought_done(id: String) -> Self {
        Event::Agent(AgentEvent::ThoughtDone { id })
    }
    pub fn agent_tool_start(id: String, name: String, input: serde_json::Value) -> Self {
        Event::Agent(AgentEvent::ToolStart { id, name, input })
    }
    pub fn agent_tool_end(id: String, duration_secs: f64, output: String) -> Self {
        Event::Agent(AgentEvent::ToolEnd { id, duration_secs, output })
    }
    pub fn agent_response(id: String, content: String) -> Self {
        Event::Agent(AgentEvent::Response { id, content })
    }
    pub fn agent_turn_complete(id: String, duration_secs: f64) -> Self {
        Event::Agent(AgentEvent::TurnComplete { id, duration_secs })
    }
    pub fn agent_done(id: String) -> Self {
        Event::Agent(AgentEvent::Done { id })
    }
    pub fn agent_error(id: String, message: String) -> Self {
        Event::Agent(AgentEvent::Error { id, message })
    }

    // Additional convenience constructors
    pub fn paste(s: String) -> Self {
        Event::Input(InputEvent::Paste(s))
    }
    pub fn set_thinking_level(level: crate::model::ThinkingLevel) -> Self {
        Event::ModelConfig(ModelConfigEvent::SetThinkingLevel(level))
    }
    pub fn palette_select() -> Self {
        Event::Dialog(DialogEvent::PaletteSelect)
    }
    pub fn palette_filter(c: char) -> Self {
        Event::Dialog(DialogEvent::PaletteFilter(c))
    }
    pub fn palette_close() -> Self {
        Event::Dialog(DialogEvent::PaletteClose)
    }
    pub fn palette_down() -> Self {
        Event::Dialog(DialogEvent::PaletteDown)
    }
    pub fn settings_close() -> Self {
        Event::ModelConfig(ModelConfigEvent::SettingsClose)
    }
    pub fn show_diagnostics() -> Self {
        Event::System(SystemEvent::ShowDiagnostics)
    }
    pub fn dialog(event: DialogEvent) -> Self {
        Event::Dialog(event)
    }
    pub fn toggle_command_palette() -> Self {
        Event::Dialog(DialogEvent::ToggleCommandPalette)
    }
    pub fn dialog_back() -> Self {
        Event::Dialog(DialogEvent::DialogBack)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::DurableCoreEvent;

    #[test]
    fn event_input_equality() {
        assert_eq!(
            Event::Input(InputEvent::Input('a')),
            Event::Input(InputEvent::Input('a')),
        );
        assert_ne!(
            Event::Input(InputEvent::Input('a')),
            Event::Input(InputEvent::Input('b')),
        );
    }

    #[test]
    fn event_agent_equality() {
        let id = "test.1".to_string();
        assert_eq!(
            Event::Agent(AgentEvent::Thinking { id: id.clone() }),
            Event::Agent(AgentEvent::Thinking { id: "test.1".to_string() }),
        );
    }

    #[test]
    fn event_scroll_equality() {
        assert_eq!(Event::Scroll(ScrollEvent::Up), Event::Scroll(ScrollEvent::Up));
        assert_ne!(Event::Scroll(ScrollEvent::Up), Event::Scroll(ScrollEvent::Down));
    }

    #[test]
    fn durable_conversion_message_sent() {
        let evt = Event::Agent(AgentEvent::Response {
            id: "r1".into(),
            content: "hello".into(),
        });
        let durable = evt.to_durable();
        assert!(matches!(durable, Some(DurableCoreEvent::MessageSent { .. })));
    }

    #[test]
    fn durable_conversion_tool_call() {
        let input = serde_json::json!({"command": "ls" });
        let evt = Event::Agent(AgentEvent::ToolStart {
            id: "t1".into(),
            name: "bash".into(),
            input: input.clone(),
        });
        let durable = evt.to_durable();
        assert!(
            matches!(durable, Some(DurableCoreEvent::ToolCalled { id, name, input: persisted }) if id == "t1" && name == "bash" && persisted == input)
        );
    }

    #[test]
    fn durable_conversion_tool_result_preserves_id() {
        let evt = Event::Agent(AgentEvent::ToolEnd {
            id: "t1".into(),
            duration_secs: 1.0,
            output: "done".into(),
        });
        let durable = evt.to_durable();
        assert!(
            matches!(durable, Some(DurableCoreEvent::ToolResult { id, output, success }) if id == "t1" && output == "done" && success)
        );
    }

    #[test]
    fn durable_conversion_non_durable_returns_none() {
        let evt = Event::Control(ControlEvent::Quit);
        assert!(evt.to_durable().is_none());
    }

    #[test]
    fn all_sub_enums_have_variants() {
        // Smoke test: each sub-enum can be constructed
        let _ = Event::Input(InputEvent::Submit);
        let _ = Event::Agent(AgentEvent::Done { id: "x".into() });
        let _ = Event::Scroll(ScrollEvent::Up);
        let _ = Event::Control(ControlEvent::Quit);
        let _ = Event::ModelConfig(ModelConfigEvent::SwitchModel {
            provider: "openai".into(),
            model: "gpt-4".into(),
        });
        let _ = Event::Dialog(DialogEvent::ToggleCommandPalette);
        let _ = Event::Edit(EditEvent::PendingEdit {
            path: "x".into(),
            original: "a".into(),
            proposed: "b".into(),
        });
        let _ = Event::System(SystemEvent::ClearTransient);
        let _ = Event::Session(SessionEvent::CloneSession);
        let _ = Event::Command(CommandEvent::RunNameCommand { name: "test".into() });
    }

    #[test]
    fn convenience_constructors() {
        assert!(matches!(Event::input('x'), Event::Input(InputEvent::Input('x'))));
        assert!(matches!(Event::submit(), Event::Input(InputEvent::Submit)));
        assert!(matches!(Event::scroll_up(), Event::Scroll(ScrollEvent::Up)));
        assert!(matches!(Event::quit(), Event::Control(ControlEvent::Quit)));
        assert!(matches!(
            Event::switch_model("anthropic".into(), "claude-3".into()),
            Event::ModelConfig(ModelConfigEvent::SwitchModel { .. })
        ));
        assert!(matches!(
            Event::switch_theme("dracula".into()),
            Event::ModelConfig(ModelConfigEvent::SwitchTheme { .. })
        ));
        assert!(matches!(
            Event::agent_thinking("x".into()),
            Event::Agent(AgentEvent::Thinking { .. })
        ));
        assert!(matches!(
            Event::agent_tool_start("t1".into(), "bash".into(), serde_json::Value::Null),
            Event::Agent(AgentEvent::ToolStart { .. })
        ));
        assert!(matches!(
            Event::agent_response("r1".into(), "hi".into()),
            Event::Agent(AgentEvent::Response { .. })
        ));
    }

    /// Layer 1: every event that claims a name round-trips correctly.
    /// (Some EVENT_NAMES entries have no stable name — skip those silently,
    /// matching the existing keybindings::event_name_roundtrip test.)
    #[test]
    fn event_name_round_trip() {
        for (name, ctor) in super::EVENT_NAMES {
            let evt = ctor();
            if let Some(got) = evt.name() {
                assert_eq!(got, *name, "{}: Event::name() returned wrong name", name);
            }
            // from_name must resolve for every entry in EVENT_NAMES
            let roundtrip = Event::from_name(name);
            assert!(
                roundtrip.is_some(),
                "{}: Event::from_name({:?}) returned None",
                name,
                name
            );
        }
    }

    /// Layer 1: the Event enum has an exhaustive match arm for every variant.
    /// If a new variant is added but the match is not updated, this will fail to compile.
    #[test]
    fn dispatcher_handles_all_variants() {
        fn assert_exhaustive(e: Event) -> Event {
            match e {
                Event::Input(_) => Event::Input(InputEvent::Submit),
                Event::Agent(_) => Event::Agent(AgentEvent::Done { id: "x".into() }),
                Event::Scroll(_) => Event::Scroll(ScrollEvent::Up),
                Event::Control(_) => Event::Control(ControlEvent::Quit),
                Event::ModelConfig(_) => {
                    Event::ModelConfig(ModelConfigEvent::CycleModelNext)
                }
                Event::Dialog(_) => Event::Dialog(DialogEvent::PaletteClose),
                Event::Edit(_) => Event::Edit(EditEvent::RejectEdit),
                Event::System(_) => Event::System(SystemEvent::ClearTransient),
                Event::Session(_) => Event::Session(SessionEvent::CloneSession),
                Event::LoginFlow(_) => Event::LoginFlow(LoginFlowEvent::Cancel),
                Event::Command(_) => Event::Command(CommandEvent::RunNameCommand {
                    name: "test".into(),
                }),
                Event::Sidebar(_) => Event::Sidebar(SidebarEvent::Hide),
                Event::Orchestrator(_) => {
                    Event::Orchestrator(OrchestratorEvent::Cancelled)
                }
            }
        }
        // Compile-time check: the match is exhaustive for all Event variants
        let _ = assert_exhaustive(Event::Input(InputEvent::Submit));
    }
}
