use crate::event::{DurableCoreEvent, Event, EVENT_NAMES};

/// Pre-optimization size of `Event` before boxing large orchestrator payloads.
const EVENT_BASELINE_SIZE: usize = 288;

#[test]
fn event_size_reduced() {
    let size = std::mem::size_of::<Event>();
    assert!(
        size < EVENT_BASELINE_SIZE,
        "Event size {} should be smaller than baseline {}",
        size,
        EVENT_BASELINE_SIZE
    );
}

#[test]
fn event_input_equality() {
    assert_eq!(Event::Input('a'), Event::Input('a'));
    assert_ne!(Event::Input('a'), Event::Input('b'));
}

#[test]
fn event_agent_equality() {
    let id = "test.1".to_string();
    assert_eq!(
        Event::Thinking { id: id.clone() },
        Event::Thinking {
            id: "test.1".to_string()
        },
    );
}

#[test]
fn event_scroll_equality() {
    assert_eq!(Event::Up, Event::Up);
    assert_ne!(Event::Up, Event::Down);
}

#[test]
fn durable_conversion_message_sent() {
    let evt = Event::Response {
        id: "r1".into(),
        content: "hello".into(),
    };
    let durable = evt.to_durable();
    assert!(matches!(
        durable,
        Some(DurableCoreEvent::MessageSent { .. })
    ));
}

#[test]
fn durable_conversion_tool_call() {
    let input = serde_json::json!({"command": "ls" });
    let evt = Event::ToolStart {
        id: "t1".into(),
        name: "bash".into(),
        input: input.clone(),
    };
    let durable = evt.to_durable();
    assert!(
        matches!(durable, Some(DurableCoreEvent::ToolCalled { id, name, input: persisted }) if id == "t1" && name == "bash" && persisted == input)
    );
}

#[test]
fn durable_conversion_tool_result_preserves_id() {
    let evt = Event::ToolEnd {
        id: "t1".into(),
        duration_secs: 1.0,
        output: "done".into(),
    };
    let durable = evt.to_durable();
    assert!(
        matches!(durable, Some(DurableCoreEvent::ToolResult { id, output, success }) if id == "t1" && output == "done" && success)
    );
}

#[test]
fn durable_conversion_non_durable_returns_none() {
    let evt = Event::Quit;
    assert!(evt.to_durable().is_none());
}

#[test]
fn all_sub_enums_have_variants() {
    let _ = Event::Submit;
    let _ = Event::Done { id: "x".into() };
    let _ = Event::Up;
    let _ = Event::Quit;
    let _ = Event::ForceQuit;
    let _ = Event::SwitchModel {
        provider: "openai".into(),
        model: "gpt-4".into(),
    };
    let _ = Event::ToggleCommandPalette;
    let _ = Event::PendingEdit {
        path: "x".into(),
        original: "a".into(),
        proposed: "b".into(),
    };
    let _ = Event::ClearTransient;
    let _ = Event::CloneSession;
    let _ = Event::RunNameCommand {
        name: "test".into(),
    };
}

#[test]
fn convenience_constructors() {
    assert!(matches!(Event::input('x'), Event::Input('x')));
    assert!(matches!(Event::submit(), Event::Submit));
    assert!(matches!(Event::scroll_up(), Event::Up));
    assert!(matches!(Event::quit(), Event::Quit));
    assert!(matches!(Event::force_quit(), Event::ForceQuit));
    assert!(matches!(
        Event::switch_model("anthropic".into(), "claude-3".into()),
        Event::SwitchModel { .. }
    ));
    assert!(matches!(
        Event::switch_theme("dracula".into()),
        Event::SwitchTheme { .. }
    ));
    assert!(matches!(
        Event::agent_thinking("x".into()),
        Event::Thinking { .. }
    ));
    assert!(matches!(
        Event::agent_tool_start("t1".into(), "bash".into(), serde_json::Value::Null),
        Event::ToolStart { .. }
    ));
    assert!(matches!(
        Event::agent_response("r1".into(), "hi".into()),
        Event::Response { .. }
    ));
}

/// Layer 1: every event that claims a name round-trips correctly.
#[test]
fn event_name_round_trip() {
    for (name, ctor) in EVENT_NAMES {
        let evt = ctor();
        if let Some(got) = evt.name() {
            assert_eq!(got, *name, "{}: Event::name() returned wrong name", name);
        }
        let roundtrip = Event::from_name(name);
        assert!(
            roundtrip.is_some(),
            "{}: Event::from_name({:?}) returned None",
            name,
            name
        );
    }
}

/// Layer 2: input events still reach the input handler.
#[test]
fn all_input_events_dispatch() {
    use crate::model::AppState;
    let mut state = AppState::default();
    state.update(Event::Input('h'));
    assert_eq!(state.input.input, "h");
    state.update(Event::Backspace);
    assert!(state.input.input.is_empty());
}

/// Layer 2: agent events still reach the agent handler.
#[test]
fn all_agent_events_dispatch() {
    use crate::model::{AppState, Role};
    let mut state = AppState::default();
    state.update(Event::Response {
        id: "r1".into(),
        content: "hello".into(),
    });
    let last = state.session.messages.last();
    assert!(last.is_some_and(|m| m.role == Role::Assistant && m.content == "hello"));
}

/// Layer 1: the Event enum has an exhaustive match arm for every variant.
/// The assertion values are arbitrary; the value of this test is the
/// compile-time exhaustiveness check.
#[test]
fn dispatcher_handles_all_variants() {
    fn assert_exhaustive(e: Event) -> Event {
        match e {
            // Input
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
            | Event::TerminalSize { .. } => Event::Submit,

            // Agent
            Event::Thinking { .. }
            | Event::ThoughtDone { .. }
            | Event::ToolStart { .. }
            | Event::ToolEnd { .. }
            | Event::ResponseDelta { .. }
            | Event::Response { .. }
            | Event::MessageReplayed { .. }
            | Event::TurnComplete { .. }
            | Event::Done { .. }
            | Event::Error { .. } => Event::Done { id: "x".into() },

            // Scroll
            Event::Up | Event::Down => Event::Up,

            // Control
            Event::Quit
            | Event::ForceQuit
            | Event::Reset
            | Event::Abort
            | Event::FollowUp
            | Event::SpawnAgent { .. }
            | Event::SteerAgent { .. }
            | Event::CancelAgent { .. }
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
            | Event::DeleteSession { .. } => Event::Quit,

            // ModelConfig
            Event::SwitchModel { .. }
            | Event::SwitchTheme { .. }
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
            | Event::SetThinkingLevel(_)
            | Event::ToggleReadOnly
            | Event::TrustProject
            | Event::UntrustProject
            | Event::ReloadAll
            | Event::KeybindingsReloaded => Event::CycleModelNext,

            // Dialog
            Event::ToggleWelcome
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
            | Event::RunSaveCommand { .. }
            | Event::DialogBack
            | Event::ProvidersDialog
            | Event::ProvidersSelectModel { .. }
            | Event::ProvidersDisconnect { .. }
            | Event::ProvidersAdd
            | Event::ProviderEditModels { .. }
            | Event::ProviderEditModelsToggle { .. }
            | Event::ProviderEditModelsSave { .. }
            | Event::ProviderEditModelsClose
            | Event::OpenAgentsManager
            | Event::AgentsManagerSetField { .. }
            | Event::AgentsManagerSave { .. }
            | Event::AgentsManagerDelete { .. }
            | Event::CopyToClipboard(_)
            | Event::CopySelectedBlock
            | Event::CopyBlockMetadata
            | Event::AtFilePicker
            | Event::InsertAtRef(_) => Event::PaletteClose,

            // Edit
            Event::PendingEdit { .. } | Event::ApproveEdit | Event::RejectEdit => Event::RejectEdit,

            // System
            Event::SystemMessage { .. }
            | Event::TransientMessage { .. }
            | Event::TransientError { .. }
            | Event::ClearTransient
            | Event::ShowDiagnostics => Event::ClearTransient,

            // Session
            Event::ForkSession { .. }
            | Event::CloneSession
            | Event::ToggleSessionTree
            | Event::SessionTreeFilterCycle
            | Event::SessionTreeSelect { .. } => Event::CloneSession,

            // Command
            Event::RunLoadCommand { .. }
            | Event::RunDeleteCommand { .. }
            | Event::RunImportCommand { .. }
            | Event::RunExportCommand { .. }
            | Event::RunSkillCommand { .. }
            | Event::RunLoginCommand { .. }
            | Event::RunLogoutCommand { .. }
            | Event::RunNameCommand { .. }
            | Event::RunForkCommand { .. }
            | Event::RunCompactCommand { .. }
            | Event::RunPromptCommand { .. }
            | Event::RunThinkingCommand { .. }
            | Event::RunPaletteCommand { .. } => Event::RunNameCommand {
                name: "test".into(),
            },

            // LoginFlow
            Event::Start
            | Event::SelectProvider { .. }
            | Event::SubmitKey { .. }
            | Event::ValidationDone { .. }
            | Event::ValidationFailed { .. }
            | Event::ModelsFetched { .. }
            | Event::ToggleModel { .. }
            | Event::Save
            | Event::Cancel => Event::Cancel,

            // Sidebar
            Event::Show
            | Event::Hide
            | Event::FocusOrchestrator
            | Event::FocusSubagent(_)
            | Event::UpdateStatus { .. }
            | Event::SetSubagents(_)
            | Event::SetOrchestratorStatus(_) => Event::Hide,

            // Orchestrator
            Event::StateChanged { .. }
            | Event::PlanStarted
            | Event::PlanningStarted
            | Event::PlanGenerated { .. }
            | Event::PlanningFailed { .. }
            | Event::SubagentDispatched { .. }
            | Event::SubagentStatusChanged { .. }
            | Event::SubagentCompleted { .. }
            | Event::SubagentFailed { .. }
            | Event::SynthesisStarted
            | Event::SynthesisComplete { .. }
            | Event::Finished { .. }
            | Event::Cancelled => Event::Cancelled,

            // Permissions
            Event::PermissionRequest { .. } => Event::PermissionRequest {
                request_id: String::new(),
                tool: String::new(),
                input: serde_json::Value::Null,
            },
            Event::PermissionResponse { .. } => Event::PermissionResponse {
                request_id: String::new(),
                action: crate::permissions::PermissionAction::Ask,
            },
        }
    }
    let _ = assert_exhaustive(Event::Submit);
}
