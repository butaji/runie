//! Durable event types for session persistence.
//!
//! These events are stored in `SessionStore` (JSONL) under
//! `data_dir/runie/sessions/<id>.jsonl` and can be replayed to reconstruct a
//! session.
//!
//! Derivable from the canonical `Event` via `Event::to_durable()` (which
//! delegates to `DurableCoreEvent::try_from`). Non-durable `Event` variants
//! return `None`.

use crate::Event;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

impl DurableCoreEvent {
    /// Convert a canonical `Event` to a durable event for JSONL persistence.
    /// Returns `None` for transient-only events (keystrokes, scroll, streaming deltas).
    pub fn try_from_event(event: &Event) -> Option<Self> {
        use DurableCoreEvent as D;
        match event {
            // Transient streaming — not persisted
            Event::ResponseDelta { .. }
            | Event::TextStart { .. }
            | Event::TextEnd { .. }
            | Event::ThinkingStart { .. }
            | Event::ThinkingDelta { .. }
            | Event::ThinkingEnd { .. }
            | Event::Thinking { .. }
            | Event::ThoughtDone { .. }
            | Event::ToolInputDelta { .. }
            | Event::TokenStatsUpdated { .. }
            | Event::StreamStarted { .. }
            | Event::TurnStarted { .. }
            | Event::TurnComplete { .. }
            | Event::Done { .. }
            | Event::TurnAborted
            | Event::TurnCompleted
            | Event::TurnErrored { .. }
            | Event::TurnConstraintError { .. }
            | Event::UserMessageSubmitted { .. }
            | Event::QueueAborted { .. }
            | Event::QueuesCleared
            | Event::SteeringDelivered { .. }
            | Event::FollowUpDelivered { .. }
            | Event::MessageDequeued { .. }
            | Event::IdGenerated(_)
            | Event::AssistantMessageReady { .. }
            | Event::Error { .. } => None,
            // Durable: message
            // Durable: assistant response (uses now() since Response doesn't carry role/timestamp)
            Event::Response { id, content } => Some(D::MessageSent {
                id: id.clone(),
                role: "assistant".into(),
                content: content.clone(),
                timestamp: crate::model::now(),
                provider: String::new(),
            }),
            // Durable: replayed message (carries full metadata from session)
            Event::MessageReplayed {
                id,
                role,
                content,
                timestamp,
                provider,
            } => Some(D::MessageSent {
                id: id.clone(),
                role: role.clone(),
                content: content.clone(),
                timestamp: *timestamp,
                provider: provider.clone(),
            }),
            // Durable: tool call
            Event::ToolStart { id, name, input } => Some(D::ToolCalled {
                id: id.clone(),
                name: name.clone(),
                input: input.clone(),
            }),
            // Durable: tool result
            Event::ToolEnd { id, output, .. } => Some(D::ToolResult {
                id: id.clone(),
                output: output.clone(),
                success: true,
            }),
            // Durable: model switch
            Event::SwitchModel { provider, model, .. } => Some(D::ModelSwitched {
                provider: provider.clone(),
                model: model.clone(),
            }),
            // Durable: session config
            Event::RunNameCommand { name } => Some(D::SessionRenamed { name: name.clone() }),
            Event::SwitchTheme { name } => Some(D::ThemeSwitched { name: name.clone() }),
            Event::SetThinkingLevel(level) => Some(D::ThinkingLevelSet { level: *level }),
            // Input, scroll, permission — not persisted
            Event::Input(_)
            | Event::Backspace
            | Event::Newline
            | Event::Submit
            | Event::Escape
            | Event::CursorLeft
            | Event::CursorRight
            | Event::CursorStart
            | Event::CursorEnd
            | Event::DeleteWord
            | Event::DeleteToEnd
            | Event::DeleteToStart
            | Event::KillChar
            | Event::HistoryPrev
            | Event::HistoryNext
            | Event::Undo
            | Event::Redo
            | Event::CursorWordLeft
            | Event::CursorWordRight
            | Event::PageUp
            | Event::PageDown
            | Event::GoToTop
            | Event::GoToBottom
            | Event::Paste(_)
            | Event::PasteImage
            | Event::MouseClick { .. }
            | Event::MouseRelease { .. }
            | Event::MouseDrag { .. }
            | Event::MouseMove { .. }
            | Event::MouseScrollUp
            | Event::MouseScrollDown
            | Event::FocusGained
            | Event::FocusLost
            | Event::TerminalSize { .. }
            | Event::Start
            | Event::PermissionRequest { .. }
            | Event::PermissionResponse { .. }
            | Event::PermissionRequestDismissed => None,
            // PermissionResponse / PermissionRequest — not persisted
            // Other facts — not persisted
            Event::InputChanged { .. }
            | Event::ViewChanged { .. }
            | Event::CompletionChanged { .. }
            | Event::TrustLoaded { .. }
            | Event::TrustChanged { .. }
            | Event::TrustSet { .. }
            | Event::ReadOnlyChanged { .. }
            | Event::HistoryLoaded { .. }
            | Event::HistoryAppend { .. }
            | Event::SessionLoaded { .. }
            | Event::SessionSaved { .. }
            | Event::SessionDeleted { .. }
            | Event::SessionImported { .. }
            | Event::SessionExported { .. }
            | Event::SessionList { .. }
            | Event::SessionOperationFailed { .. }
            | Event::SessionChanged { .. }
            | Event::TransientMessage { .. }
            | Event::TransientError { .. }
            | Event::ClearTransient
            | Event::ShowDiagnostics
            | Event::SystemMessage { .. }
            | Event::ConfigLoaded { .. }
            | Event::ProcessResumed
            | Event::BashOutput { .. }
            | Event::FilesWritten { .. }
            | Event::EnvDetected { .. }
            | Event::FffSearchResult { .. }
            | Event::ToolConstraintError { .. } => None,
            // Command intents — handled directly in replay, not via durable_to_event
            Event::RunLoadCommand { .. }
            | Event::RunSaveCommand { .. }
            | Event::RunDeleteCommand { .. }
            | Event::RunImportCommand { .. }
            | Event::RunExportCommand { .. }
            | Event::RunSkillCommand { .. }
            | Event::RunLoginCommand { .. }
            | Event::RunLogoutCommand { .. }
            | Event::RunForkCommand { .. }
            | Event::RunCompactCommand { .. }
            | Event::RunPromptCommand { .. }
            | Event::RunThinkingCommand { .. }
            | Event::RunPaletteCommand { .. } => None,
            // UI intents — not persisted
            Event::Quit
            | Event::ForceQuit
            | Event::Reset
            | Event::Abort
            | Event::ClearQueues
            | Event::FollowUp
            | Event::ToggleExpand
            | Event::Dequeue
            | Event::OpenExternalEditor
            | Event::ExternalEditorDone { .. }
            | Event::ShareSession
            | Event::Suspend
            | Event::ToggleVimMode
            | Event::CopyLastResponse
            | Event::OpenSessionList
            | Event::NewSession
            | Event::ResumeSession
            | Event::SelectSession { .. }
            | Event::StarSession { .. }
            | Event::RenameSession { .. }
            | Event::DeleteSession { .. }
            | Event::ToggleWelcome
            | Event::ToggleCommandPalette
            | Event::PaletteFilter(_)
            | Event::PaletteBackspace
            | Event::PaletteUp
            | Event::PaletteDown
            | Event::PaletteSelect
            | Event::PaletteClose
            | Event::ToggleModelSelector
            | Event::ModelSelectorFilter(_)
            | Event::ModelSelectorBackspace
            | Event::ModelSelectorUp
            | Event::ModelSelectorDown
            | Event::ModelSelectorSelect
            | Event::ModelSelectorClose
            | Event::TogglePathCompletion
            | Event::PathCompletionUp
            | Event::PathCompletionDown
            | Event::PathCompletionSelect
            | Event::PathCompletionClose
            | Event::CommandFormInput(_)
            | Event::CommandFormBackspace
            | Event::CommandFormUp
            | Event::CommandFormDown
            | Event::CommandFormSubmit
            | Event::CommandFormClose
            | Event::DialogBack
            | Event::ProvidersDialog
            | Event::ProvidersSelectModel { .. }
            | Event::ProvidersDisconnect { .. }
            | Event::ProvidersAdd
            | Event::ProvidersEditModels { .. }
            | Event::CopyToClipboard(_)
            | Event::CopySelectedBlock
            | Event::CopyBlockMetadata
            | Event::AtFilePicker
            | Event::InsertAtRef(_)
            | Event::PendingEdit { .. }
            | Event::ApproveEdit
            | Event::RejectEdit
            | Event::GistShared { .. }
            | Event::ExternalEditorClosed { .. }
            | Event::ClipboardWritten { .. }
            | Event::ClipboardRead { .. }
            | Event::Up
            | Event::Down
            | Event::ForkSession { .. }
            | Event::CloneSession
            | Event::ToggleSessionTree
            | Event::SessionTreeFilterCycle
            | Event::SessionTreeSelect { .. }
            | Event::SelectProvider { .. }
            | Event::SubmitKey { .. }
            | Event::ToggleModel { .. }
            | Event::Save
            | Event::Cancel
            | Event::ValidationFailed { .. }
            | Event::ModelsFetched { .. }
            | Event::CycleModelNext
            | Event::CycleModelPrev
            | Event::ToggleScopedModelsDialog
            | Event::ScopedModelToggle { .. }
            | Event::ScopedModelEnableAll
            | Event::ScopedModelDisableAll
            | Event::ScopedModelToggleProvider { .. }
            | Event::ToggleSettingsDialog
            | Event::SettingsUp
            | Event::SettingsDown
            | Event::SettingsLeft
            | Event::SettingsRight
            | Event::SettingsSelect
            | Event::SettingsClose
            | Event::SettingsSwitchCategory { .. }
            | Event::CycleThinkingLevel
            | Event::ToggleReadOnly
            | Event::TrustProject
            | Event::UntrustProject
            | Event::ReloadAll
            | Event::KeybindingsReloaded
            | Event::SetPrompt { .. } => None,
        }
    }
}

/// Derive a durable event from a canonical `Event`.
/// Returns `None` for transient-only events.
impl TryFrom<&Event> for DurableCoreEvent {
    type Error = ();

    fn try_from(event: &Event) -> Result<DurableCoreEvent, <DurableCoreEvent as TryFrom<&Event>>::Error> {
        Self::try_from_event(event).ok_or(())
    }
}

/// Convert a durable event back to a canonical `Event`.
impl TryFrom<&DurableCoreEvent> for Event {
    type Error = ();

    fn try_from(event: &DurableCoreEvent) -> Result<Event, <Event as TryFrom<&DurableCoreEvent>>::Error> {
        use DurableCoreEvent as D;
        match event {
            D::MessageSent { id, role, content, timestamp, provider } => {
                Ok(Event::MessageReplayed {
                    id: id.clone(),
                    role: role.clone(),
                    content: content.clone(),
                    timestamp: *timestamp,
                    provider: provider.clone(),
                })
            }
            D::ToolCalled { id, name, input } => Ok(Event::ToolStart {
                id: id.clone(),
                name: name.clone(),
                input: input.clone(),
            }),
            D::ToolResult { id, output, .. } => Ok(Event::ToolEnd {
                id: id.clone(),
                duration_secs: 0.0,
                output: output.clone(),
            }),
            D::ModelSwitched { provider, model } => Ok(Event::SwitchModel {
                provider: provider.clone(),
                model: model.clone(),
                explicit: false,
            }),
            D::ThemeSwitched { name } => Ok(Event::SwitchTheme { name: name.clone() }),
            D::ThinkingLevelSet { level } => Ok(Event::SetThinkingLevel(*level)),
            // SessionRenamed and ReadOnlySet are handled directly in replay_event
            D::SessionRenamed { .. } | D::ReadOnlySet { .. } => Err(()),
        }
    }
}

/// Events that are persisted to the session store.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "event", rename_all = "camelCase")]
pub enum DurableCoreEvent {
    /// A message sent by the user or the assistant.
    MessageSent {
        id: String,
        role: String,
        content: String,
        timestamp: f64,
        #[serde(default)]
        provider: String,
    },
    /// An LLM invoked a tool.
    ToolCalled {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    /// A tool returned its result.
    ToolResult {
        id: String,
        output: String,
        success: bool,
    },
    /// The user switched the active model or provider.
    ModelSwitched { provider: String, model: String },
    /// The session was renamed by the user.
    SessionRenamed { name: String },
    /// The user switched the active theme.
    ThemeSwitched { name: String },
    /// The user changed the thinking level.
    ThinkingLevelSet { level: crate::model::ThinkingLevel },
    /// The user toggled read-only mode.
    ReadOnlySet { read_only: bool },
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Event → DurableCoreEvent ─────────────────────────────────────────────

    #[test]
    fn durable_from_response() {
        let event = Event::Response {
            id: "r1".into(),
            content: "hello".into(),
        };
        let durable = DurableCoreEvent::try_from_event(&event);
        assert!(durable.is_some());
        let durable = durable.unwrap();
        assert!(matches!(
            durable,
            DurableCoreEvent::MessageSent { role, content, .. }
            if role == "assistant" && content == "hello"
        ));
    }

    #[test]
    fn durable_from_tool_start() {
        let event = Event::ToolStart {
            id: "t1".into(),
            name: "bash".into(),
            input: serde_json::json!({"cmd": "ls"}),
        };
        let durable = DurableCoreEvent::try_from_event(&event);
        assert!(durable.is_some());
        let durable = durable.unwrap();
        assert!(matches!(
            durable,
            DurableCoreEvent::ToolCalled { id, name, .. } if id == "t1" && name == "bash"
        ));
    }

    #[test]
    fn durable_from_tool_end() {
        let event = Event::ToolEnd {
            id: "t1".into(),
            duration_secs: 1.5,
            output: "done".into(),
        };
        let durable = DurableCoreEvent::try_from_event(&event);
        assert!(durable.is_some());
        let durable = durable.unwrap();
        assert!(matches!(
            durable,
            DurableCoreEvent::ToolResult { id, output, success: true }
            if id == "t1" && output == "done"
        ));
    }

    #[test]
    fn durable_from_switch_model() {
        let event = Event::SwitchModel {
            provider: "anthropic".into(),
            model: "claude-3".into(),
            explicit: true,
        };
        let durable = DurableCoreEvent::try_from_event(&event);
        assert!(durable.is_some());
        assert!(matches!(
            durable.unwrap(),
            DurableCoreEvent::ModelSwitched { provider, model }
            if provider == "anthropic" && model == "claude-3"
        ));
    }

    #[test]
    fn durable_from_run_name_command() {
        let event = Event::RunNameCommand { name: "my session".into() };
        let durable = DurableCoreEvent::try_from_event(&event);
        assert!(durable.is_some());
        assert!(matches!(
            durable.unwrap(),
            DurableCoreEvent::SessionRenamed { name } if name == "my session"
        ));
    }

    #[test]
    fn durable_from_switch_theme() {
        let event = Event::SwitchTheme { name: "dark".into() };
        let durable = DurableCoreEvent::try_from_event(&event);
        assert!(durable.is_some());
        assert!(matches!(
            durable.unwrap(),
            DurableCoreEvent::ThemeSwitched { name } if name == "dark"
        ));
    }

    #[test]
    fn durable_from_set_thinking_level() {
        let event = Event::SetThinkingLevel(crate::model::ThinkingLevel::Medium);
        let durable = DurableCoreEvent::try_from_event(&event);
        assert!(durable.is_some());
        assert!(matches!(
            durable.unwrap(),
            DurableCoreEvent::ThinkingLevelSet { level: crate::model::ThinkingLevel::Medium }
        ));
    }

    #[test]
    fn durable_from_message_replayed() {
        let event = Event::MessageReplayed {
            id: "r1".into(),
            role: "user".into(),
            content: "hello".into(),
            timestamp: 1234.5,
            provider: "anthropic".into(),
        };
        let durable = DurableCoreEvent::try_from_event(&event);
        assert!(durable.is_some());
        assert!(matches!(
            durable.unwrap(),
            DurableCoreEvent::MessageSent {
                id,
                role,
                content,
                timestamp,
                provider
            }
            if id == "r1" && role == "user" && content == "hello"
                && timestamp == 1234.5 && provider == "anthropic"
        ));
    }

    #[test]
    fn transient_events_return_none() {
        let cases: Vec<Event> = vec![
            Event::ResponseDelta { id: "".into(), content: "x".into() },
            Event::TextStart { id: "".into() },
            Event::TextEnd { id: "".into() },
            Event::ThinkingDelta { id: "".into(), content: "".into() },
            Event::ThinkingStart { id: "".into() },
            Event::ThinkingEnd { id: "".into() },
            Event::Thinking { id: "".into() },
            Event::ThoughtDone { id: "".into() },
            Event::TokenStatsUpdated { tokens_in: 0, tokens_out: 0, speed_tps: 0.0 },
            Event::Quit,
            Event::Input('x'),
        ];
        for event in cases {
            assert!(
                DurableCoreEvent::try_from_event(&event).is_none(),
                "{:?} should not become durable",
                event
            );
        }
    }

    // ── DurableCoreEvent → Event (reverse) ───────────────────────────────────

    #[test]
    fn event_from_message_sent() {
        let durable = DurableCoreEvent::MessageSent {
            id: "r1".into(),
            role: "assistant".into(),
            content: "hello".into(),
            timestamp: 100.0,
            provider: "openai".into(),
        };
        let event: Result<Event, _> = Event::try_from(&durable);
        assert!(event.is_ok());
        let event = event.unwrap();
        assert!(matches!(
            event,
            Event::MessageReplayed { id, role, content, timestamp, provider }
            if id == "r1" && role == "assistant" && content == "hello"
                && timestamp == 100.0 && provider == "openai"
        ));
    }

    #[test]
    fn event_from_tool_called() {
        let durable = DurableCoreEvent::ToolCalled {
            id: "t1".into(),
            name: "read".into(),
            input: serde_json::json!({"path": "/tmp"}),
        };
        let event: Result<Event, _> = Event::try_from(&durable);
        assert!(event.is_ok());
        assert!(matches!(
            event.unwrap(),
            Event::ToolStart { id, name, .. } if id == "t1" && name == "read"
        ));
    }

    #[test]
    fn event_from_tool_result() {
        let durable = DurableCoreEvent::ToolResult {
            id: "t1".into(),
            output: "result".into(),
            success: true,
        };
        let event: Result<Event, _> = Event::try_from(&durable);
        assert!(event.is_ok());
        assert!(matches!(
            event.unwrap(),
            Event::ToolEnd { id, output, .. } if id == "t1" && output == "result"
        ));
    }

    #[test]
    fn event_from_model_switched() {
        let durable = DurableCoreEvent::ModelSwitched {
            provider: "openai".into(),
            model: "gpt-4".into(),
        };
        let event: Result<Event, _> = Event::try_from(&durable);
        assert!(event.is_ok());
        assert!(matches!(
            event.unwrap(),
            Event::SwitchModel { provider, model, explicit: false }
            if provider == "openai" && model == "gpt-4"
        ));
    }

    #[test]
    fn event_from_session_renamed_is_err() {
        let durable = DurableCoreEvent::SessionRenamed { name: "x".into() };
        let event: Result<Event, _> = Event::try_from(&durable);
        assert!(event.is_err()); // handled directly in replay_event
    }

    #[test]
    fn event_from_read_only_set_is_err() {
        let durable = DurableCoreEvent::ReadOnlySet { read_only: true };
        let event: Result<Event, _> = Event::try_from(&durable);
        assert!(event.is_err()); // handled directly in replay_event
    }

    // ── Serde roundtrip for DurableCoreEvent ─────────────────────────────────

    #[test]
    fn durable_message_sent_roundtrips_through_json() {
        let durable = DurableCoreEvent::MessageSent {
            id: "r1".into(),
            role: "user".into(),
            content: "test".into(),
            timestamp: 999.0,
            provider: "".into(),
        };
        let json = serde_json::to_string(&durable).unwrap();
        let parsed: DurableCoreEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, durable);
    }

    #[test]
    fn durable_tool_called_roundtrips_through_json() {
        let durable = DurableCoreEvent::ToolCalled {
            id: "t1".into(),
            name: "bash".into(),
            input: serde_json::json!({"cmd": "ls"}),
        };
        let json = serde_json::to_string(&durable).unwrap();
        let parsed: DurableCoreEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, durable);
    }

    #[test]
    fn durable_preserves_existing_jsonl_format() {
        // Simulate a JSONL line from an existing session file
        let json = r#"{"event":"modelSwitched","provider":"openai","model":"gpt-4"}"#;
        let durable: DurableCoreEvent = serde_json::from_str(json).unwrap();
        assert!(matches!(
            durable,
            DurableCoreEvent::ModelSwitched { provider, model }
            if provider == "openai" && model == "gpt-4"
        ));
    }
}
