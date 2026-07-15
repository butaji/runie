//! Durable event types for session persistence.
//!
//! These events are stored in `SessionStore` (JSONL) under
//! `data_dir/runie/sessions/<id>.jsonl` and can be replayed to reconstruct a
//! session.
//!
//! Derivable from the canonical `Event` via `Event::to_durable()` (which
//! delegates to `DurableCoreEvent::try_from`). Non-durable `Event` variants
//! return `None`.

use crate::proto::message::Part;
use crate::Event;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

/// Turn journal phases for crash recovery.
///
/// These phases track the lifecycle of a turn and enable recovery after crashes.
/// Each phase is persisted to the session JSONL and can be replayed to determine
/// where a turn was interrupted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TurnPhase {
    /// Turn has begun (user message received).
    TurnStarted,
    /// LLM provider request has been sent.
    ProviderCalled,
    /// Tool calls have been recorded from the LLM response.
    ToolRequestsRecorded,
    /// Streaming response has started.
    ResponseDelta,
    /// Turn completed successfully.
    TurnCommitted,
    /// Turn was aborted or failed.
    TurnAborted,
}

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
            | Event::CompactionTriggered { .. }
            | Event::StreamStarted { .. }
            | Event::TurnCompleted
            | Event::TurnErrored { .. }
            | Event::TurnConstraintError { .. }
            | Event::UserMessageSubmitted { .. }
            | Event::QueueAborted { .. }
            | Event::QueuesCleared
            | Event::QueueFollowUpAdded { .. }
            | Event::QueueSteeringAdded { .. }
            | Event::SteeringDelivered { .. }
            | Event::FollowUpDelivered { .. }
            | Event::MessageDequeued { .. }
            | Event::IdGenerated(_)
            | Event::AssistantMessageReady { .. }
            | Event::PatternWorkerSpawned { .. }
            | Event::PatternWorkerFinished { .. }
            | Event::Error { .. }
            | Event::ToggleTasksPane
            // Init load events — not persisted, only used during bootstrap
            | Event::SkillsLoaded { .. }
            | Event::SkillCreated { .. }
            | Event::SkillDeleted { .. }
            | Event::SkillError { .. }
            | Event::AuthLoaded { .. } => None,
            // Durable: message
            // Durable: assistant response (uses stored role/timestamp/provider)
            Event::Response { id, content, role, timestamp, provider } => Some(D::MessageSent {
                id: id.clone(),
                role: if role.is_empty() { "assistant".into() } else { role.clone() },
                content: content.clone(),
                timestamp: if *timestamp == 0.0 { crate::model::now() } else { *timestamp },
                provider: provider.clone(),
                // Event::Response only carries flat content; parts preserved via session save path.
                parts: Vec::new(),
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
                // Event::MessageReplayed only carries flat content; parts preserved via session save path.
                parts: Vec::new(),
            }),
            // Durable: tool call
            Event::ToolStart { id, name, input } => Some(D::ToolCalled {
                id: id.clone(),
                name: name.clone(),
                input: input.clone(),
            }),
            // Durable: tool result — duration_secs omitted (computed from timing data at runtime, not available during replay)
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
            // Durable: the per-model reasoning flow's switch is also a model
            // switch for session replay (the level override lives in config,
            // not in the session journal).
            Event::SwitchModelWithLevel { provider, model, .. } => Some(D::ModelSwitched {
                provider: provider.clone(),
                model: model.clone(),
            }),
            // Durable: session config
            Event::RunNameCommand { name } => Some(D::SessionRenamed { name: name.clone() }),
            Event::SwitchTheme { name } => Some(D::ThemeSwitched { name: name.clone() }),
            Event::SetThinkingLevel(level) => Some(D::ThinkingLevelSet { level: *level }),
            // Durable: session tree
            Event::SessionTreeSnapshot { snapshot } => Some(D::TreeSnapshot { snapshot: snapshot.clone() }),

            // Durable: turn journal phases
            Event::TurnStarted { request_id, .. } => Some(D::TurnPhaseChanged {
                phase: TurnPhase::TurnStarted,
                request_id: request_id.clone(),
            }),
            Event::TurnComplete { .. } => Some(D::TurnPhaseChanged {
                phase: TurnPhase::TurnCommitted,
                request_id: String::new(),
            }),
            Event::Done { .. } => Some(D::TurnPhaseChanged {
                phase: TurnPhase::TurnCommitted,
                request_id: String::new(),
            }),
            Event::TurnAborted => Some(D::TurnPhaseChanged {
                phase: TurnPhase::TurnAborted,
                request_id: String::new(),
            }),
            Event::ToolRequestsRecorded { request_id } => Some(D::TurnPhaseChanged {
                phase: TurnPhase::ToolRequestsRecorded,
                request_id: request_id.clone(),
            }),
            Event::ResponseDeltaStarted { request_id } => Some(D::TurnPhaseChanged {
                phase: TurnPhase::ResponseDelta,
                request_id: request_id.clone(),
            }),

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
            | Event::MouseScrollUp
            | Event::MouseScrollDown
            | Event::FocusGained
            | Event::FocusLost
            | Event::TerminalSize { .. }
            | Event::Start
            | Event::PermissionRequest { .. }
            | Event::PermissionResponse { .. }
            | Event::PermissionRequestDismissed
            | Event::PermissionAllow { .. }
            | Event::PermissionDeny { .. }
            | Event::PermissionAlwaysAllow { .. }
            | Event::PermissionSessionAllow { .. }
            // AskUserQuestion dialog — not persisted
            | Event::AskUserQuestion { .. }
            | Event::QuestionAnswer { .. }
            | Event::QuestionSkip { .. }
            | Event::QuestionSubmit { .. }
            | Event::PermissionOnce { .. } => None,
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
            | Event::SessionMessageAdded { .. }
            | Event::SessionMessageUpdated { .. }
            | Event::SessionMetadataUpdated { .. }
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
            | Event::SelectModel { .. }
            | Event::ToggleSettingsDialog
            | Event::SettingsUp
            | Event::SettingsDown
            | Event::SettingsLeft
            | Event::SettingsRight
            | Event::SettingsSelect
            | Event::SettingsClose
            | Event::SettingsSwitchCategory { .. }
            | Event::CycleThinkingLevel
            | Event::SetMode { .. }
            | Event::SetSwarmVariant { .. }
            | Event::SetModeAndSwarmVariant { .. }
            | Event::ToggleReadOnly
            | Event::TrustProject
            | Event::UntrustProject
            | Event::ReloadAll
            | Event::KeybindingsReloaded
            | Event::SetPrompt { .. }
            | Event::PlanModeEnabled { .. }
            | Event::PlanModeDisabled
            | Event::ToggleMcpServersDialog
            | Event::ToggleSkillsDialog
            | Event::McpServerAction { .. }
            | Event::SkillAction { .. }
            | Event::GoalCreate { .. }
            | Event::GoalComplete { .. }
            | Event::GoalPause
            | Event::GoalResume
            | Event::GoalCancel
            | Event::GoalStatus { .. } => None,
        }
    }
}

/// Derive a durable event from a canonical `Event`.
/// Returns `None` for transient-only events.
impl TryFrom<&Event> for DurableCoreEvent {
    type Error = ();

    fn try_from(
        event: &Event,
    ) -> Result<DurableCoreEvent, <DurableCoreEvent as TryFrom<&Event>>::Error> {
        Self::try_from_event(event).ok_or(())
    }
}

/// Convert a durable event back to a canonical `Event`.
impl TryFrom<&DurableCoreEvent> for Event {
    type Error = ();

    fn try_from(
        event: &DurableCoreEvent,
    ) -> Result<Event, <Event as TryFrom<&DurableCoreEvent>>::Error> {
        use DurableCoreEvent as D;
        match event {
            D::MessageSent {
                id,
                role,
                content,
                timestamp,
                provider,
                parts,
            } => {
                Ok(Event::MessageReplayed {
                    id: id.clone(),
                    role: role.clone(),
                    content: if parts.is_empty() {
                        content.clone()
                    } else {
                        // Reconstruct content from parts for backward compatibility.
                        parts
                            .iter()
                            .filter_map(|p| match p {
                                Part::Text { content } => Some(content.as_str()),
                                _ => None,
                            })
                            .collect::<Vec<_>>()
                            .join("")
                    },
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
                input: None,
                // duration_secs not available during replay (timing data lost); set to 0.0.
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
            D::TreeSnapshot { snapshot } => Ok(Event::SessionTreeSnapshot {
                snapshot: snapshot.clone(),
            }),
            // TurnPhaseChanged is used for crash recovery but doesn't directly
            // translate to a canonical event (phase is reconstructed from other events).
            D::TurnPhaseChanged { .. } => Err(()),
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
        /// Structured parts (text, reasoning, tool calls, tool results).
        /// When present, supersedes `content` for replay.
        #[serde(default)]
        parts: Vec<Part>,
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
        // duration_secs omitted — computed from timing data at runtime, not persisted
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
    /// Session tree structure snapshot (edges and branch).
    TreeSnapshot {
        snapshot: crate::session::tree::SessionTreeSnapshot,
    },
    /// Turn journal phase changed.
    ///
    /// Used for crash recovery: records the current phase of turn execution.
    /// Scan the last `TurnPhaseChanged` event to determine where a turn was
    /// interrupted and what recovery action is needed.
    TurnPhaseChanged {
        phase: TurnPhase,
        #[serde(default, rename = "requestId")]
        request_id: String,
    },
}

#[cfg(test)]
mod tests;
