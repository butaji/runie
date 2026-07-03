use crate::config::Config;
use crate::event::Event;
use crate::model::{AppState, Role};

/// Layer 2: input events still reach the input handler.
#[test]
fn all_input_events_dispatch() {
    let mut state = AppState::default();
    state.update(Event::Input('h'));
    assert_eq!(state.input().input, "h");
    state.update(Event::Backspace);
    assert!(state.input().input.is_empty());
}

/// Layer 2: agent events still reach the agent handler.
#[test]
fn all_agent_events_dispatch() {
    let mut state = AppState::default();
    state.update(Event::Response {
        id: "r1".into(),
        content: "hello".into(),
        role: String::new(),
        timestamp: 0.0,
        provider: String::new(),
    });
    let last = state.session().messages.last();
    assert!(last.is_some_and(|m| m.role == Role::Assistant && m.content() == "hello"));
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
            | Event::ToolInputDelta { .. }
            | Event::ToolEnd { .. }
            | Event::ToolConstraintError { .. }
            | Event::ToolRequestsRecorded { .. }
            | Event::ResponseDeltaStarted { .. }
            | Event::ResponseDelta { .. }
            | Event::ThinkingDelta { .. }
            | Event::TextStart { .. }
            | Event::TextEnd { .. }
            | Event::ThinkingStart { .. }
            | Event::ThinkingEnd { .. }
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
            | Event::ProvidersEditModels { .. }
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

            // Config
            Event::ConfigLoaded { .. } => Event::ConfigLoaded {
                config: Box::new(Config::default()),
            },

            // Persistence
            Event::TrustLoaded { .. }
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
            | Event::SessionMessageAdded { .. }
            | Event::SessionMessageUpdated { .. }
            | Event::SessionMetadataUpdated { .. }
            | Event::SessionTreeSnapshot { .. }
            | Event::BashOutput { .. }
            | Event::FilesWritten { .. }
            | Event::EnvDetected { .. }
            | Event::InputChanged { .. }
            | Event::ViewChanged { .. }
            | Event::CompletionChanged { .. } => Event::HistoryAppend {
                entry: String::new(),
            },

            // Session
            Event::ForkSession { .. }
            | Event::CloneSession
            | Event::ToggleSessionTree
            | Event::SessionTreeFilterCycle
            | Event::SessionTreeSelect { .. } => Event::CloneSession,

            // Command
            Event::SetPrompt { .. }
            | Event::RunLoadCommand { .. }
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
            | Event::ValidationFailed { .. }
            | Event::ModelsFetched { .. }
            | Event::ToggleModel { .. }
            | Event::Save
            | Event::Cancel => Event::Cancel,

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
            Event::PermissionRequestDismissed => Event::PermissionRequestDismissed,

            // TurnActor
            Event::TurnStarted { .. } => Event::TurnStarted {
                id: String::new(),
                request_id: String::new(),
                content: String::new(),
            },
            Event::TurnAborted => Event::TurnAborted,
            Event::TurnCompleted => Event::TurnCompleted,
            Event::TurnErrored { .. } => Event::TurnErrored {
                id: String::new(),
                message: String::new(),
            },
            Event::TurnConstraintError { .. } => Event::TurnConstraintError {
                id: String::new(),
                tool: String::new(),
                message: String::new(),
            },
            Event::TokenStatsUpdated { .. } => Event::TokenStatsUpdated {
                tokens_in: 0,
                tokens_out: 0,
            },
            Event::CompactionTriggered { .. } => Event::CompactionTriggered {
                ratio: 0.7,
                tokens_in: 100,
                context_window: 128000,
            },
            Event::StreamStarted { .. } => Event::StreamStarted { id: String::new() },
            Event::UserMessageSubmitted { .. } => Event::UserMessageSubmitted {
                id: String::new(),
                content: String::new(),
            },
            Event::QueueAborted { .. } => Event::QueueAborted {
                content: String::new(),
            },
            Event::QueuesCleared => Event::QueuesCleared,
            Event::QueueFollowUpAdded { .. } => Event::QueueFollowUpAdded {
                id: String::new(),
                content: String::new(),
            },
            Event::QueueSteeringAdded { .. } => Event::QueueSteeringAdded {
                id: String::new(),
                content: String::new(),
            },
            Event::SteeringDelivered { .. } => Event::SteeringDelivered {
                content: String::new(),
                id: String::new(),
            },
            Event::FollowUpDelivered { .. } => Event::FollowUpDelivered {
                content: String::new(),
                id: String::new(),
            },
            Event::MessageDequeued { .. } => Event::MessageDequeued {
                content: String::new(),
            },
            Event::IdGenerated(_) => {
                Event::IdGenerated(crate::actors::turn::NextIdResponse { id: String::new() })
            }
            Event::AssistantMessageReady { .. } => Event::AssistantMessageReady {
                message: crate::ChatMessage::default(),
            },

            // IO
            Event::FffSearchResult { .. } => Event::FffSearchResult {
                request_id: 0,
                entries: Vec::new(),
                query: String::new(),
                indexed: false,
            },
            Event::SkillsLoaded { .. } => Event::HistoryAppend {
                entry: String::new(),
            },
            Event::AuthLoaded { .. } => Event::HistoryAppend {
                entry: String::new(),
            },

            // IO effects (results)
            Event::GistShared { .. } => Event::GistShared {
                result: Ok("https://gist.github.com/test".to_string()),
            },
            Event::ExternalEditorClosed { .. } => Event::ExternalEditorClosed {
                result: Ok("edited content".to_string()),
            },
            Event::ClipboardWritten { .. } => Event::ClipboardWritten { success: true },
            Event::ClipboardRead { .. } => Event::ClipboardRead {
                result: Ok("clipboard content".to_string()),
            },
            Event::ProcessResumed => Event::ProcessResumed,
            Event::PlanModeEnabled { content } => Event::PlanModeEnabled {
                content: content.clone(),
            },
            Event::PlanModeDisabled => Event::PlanModeDisabled,
        }
    }
    let _ = assert_exhaustive(Event::Submit);
}
